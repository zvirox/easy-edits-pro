use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{LazyLock, OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

static MEDIA_SERVER_PORT: AtomicU16 = AtomicU16::new(0);
static MEDIA_SERVER_TOKEN: OnceLock<String> = OnceLock::new();
static APP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();
static APP_CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
static ALLOWED_MEDIA_PATHS: LazyLock<RwLock<HashSet<PathBuf>>> =
    LazyLock::new(|| RwLock::new(HashSet::new()));

pub fn set_app_data_dir(dir: PathBuf) {
    if let Ok(canon) = dir.canonicalize() {
        let _ = APP_DATA_DIR.set(canon);
    } else {
        let _ = APP_DATA_DIR.set(dir);
    }
}

pub fn set_app_cache_dir(dir: PathBuf) {
    if let Ok(canon) = dir.canonicalize() {
        let _ = APP_CACHE_DIR.set(canon);
    } else {
        let _ = APP_CACHE_DIR.set(dir);
    }
}

pub fn register_allowed_media_path<P: AsRef<Path>>(path: P) {
    let p = path.as_ref();
    if let Ok(mut set) = ALLOWED_MEDIA_PATHS.write() {
        set.insert(p.to_path_buf());
        if let Ok(canon) = p.canonicalize() {
            set.insert(canon);
        }
    }
}

pub fn is_allowed_media_path<P: AsRef<Path>>(path: P) -> bool {
    let p = path.as_ref();
    if let Ok(set) = ALLOWED_MEDIA_PATHS.read() {
        if set.contains(p) {
            return true;
        }
        if let Ok(canon) = p.canonicalize() {
            if set.contains(&canon) {
                return true;
            }
        }
    }
    if let Some(app_dir) = APP_DATA_DIR.get() {
        if let Ok(canon) = p.canonicalize() {
            if canon.starts_with(app_dir) {
                return true;
            }
        } else if p.starts_with(app_dir) {
            return true;
        }
    }
    if let Some(cache_dir) = APP_CACHE_DIR.get() {
        if let Ok(canon) = p.canonicalize() {
            if canon.starts_with(cache_dir) {
                return true;
            }
        } else if p.starts_with(cache_dir) {
            return true;
        }
    }
    false
}

fn generate_session_token() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    let mut state = (now ^ ((pid as u128) << 32)) ^ 0x9e3779b97f4a7c15;

    let mut token = String::with_capacity(32);
    for _ in 0..32 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let hex_char = match (state & 0xF) as u8 {
            v @ 0..=9 => (b'0' + v) as char,
            v => (b'a' + (v - 10)) as char,
        };
        token.push(hex_char);
    }
    token
}

fn url_decode(input: &str) -> String {
    let mut result = Vec::new();
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(val) = u8::from_str_radix(hex_str, 16) {
                    result.push(val);
                    i += 3;
                    continue;
                }
            }
        }
        if bytes[i] == b'+' {
            result.push(b' ');
        } else {
            result.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&result).into_owned()
}

fn handle_media_stream_client(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(15)));

    let mut buffer = [0u8; 4096];
    let bytes_read = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return,
    };
    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut lines = request_str.lines();
    let request_line = match lines.next() {
        Some(l) => l,
        None => return,
    };

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        return;
    }

    let mut range_header: Option<String> = None;
    let mut host_header: Option<String> = None;
    let mut origin_header: Option<String> = None;

    for line in lines {
        if line.is_empty() {
            break;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("range:") {
            range_header = Some(line["range:".len()..].trim().to_string());
        } else if lower.starts_with("host:") {
            host_header = Some(line["host:".len()..].trim().to_string());
        } else if lower.starts_with("origin:") {
            origin_header = Some(line["origin:".len()..].trim().to_string());
        }
    }

    // 1. Host header validation (blocks DNS rebinding)
    let port = MEDIA_SERVER_PORT.load(Ordering::SeqCst);
    let valid_hosts = [
        format!("127.0.0.1:{port}"),
        format!("localhost:{port}"),
        "127.0.0.1".to_string(),
        "localhost".to_string(),
    ];
    let is_valid_host = host_header.as_ref().map_or(false, |h| {
        valid_hosts.iter().any(|v| h.trim().eq_ignore_ascii_case(v))
    });
    if !is_valid_host {
        let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nForbidden Host");
        return;
    }

    // 2. CORS origin validation
    let is_origin_allowed = origin_header.as_ref().map_or(true, |orig| {
        let o = orig.trim();
        o == "tauri://localhost"
            || o == "http://tauri.localhost"
            || o == "https://tauri.localhost"
            || o.starts_with("http://localhost:")
            || o.starts_with("http://127.0.0.1:")
            || o.starts_with("https://localhost:")
            || o.starts_with("https://127.0.0.1:")
    });
    let cors_origin = if is_origin_allowed {
        origin_header.as_deref().unwrap_or("tauri://localhost")
    } else {
        "tauri://localhost"
    };

    if parts[0] == "OPTIONS" {
        let preflight = format!(
            "HTTP/1.1 204 No Content\r\n\
Access-Control-Allow-Origin: {cors_origin}\r\n\
Vary: Origin\r\n\
Access-Control-Allow-Methods: GET, HEAD, OPTIONS\r\n\
Access-Control-Allow-Headers: Range, Content-Type, Accept\r\n\
Access-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges\r\n\
Access-Control-Max-Age: 86400\r\n\
Connection: close\r\n\r\n"
        );
        let _ = stream.write_all(preflight.as_bytes());
        return;
    }

    if parts[0] != "GET" && parts[0] != "HEAD" {
        let _ = stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\nConnection: close\r\n\r\n");
        return;
    }
    let is_head = parts[0] == "HEAD";

    let target = parts[1];
    let query_start = match target.find('?') {
        Some(pos) => pos,
        None => {
            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n");
            return;
        }
    };

    let path_part = &target[..query_start];
    if path_part != "/media" {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n");
        return;
    }

    let query_str = &target[query_start + 1..];
    let mut req_path: Option<String> = None;
    let mut req_token: Option<String> = None;
    for pair in query_str.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == "path" {
                req_path = Some(url_decode(v));
            } else if k == "token" {
                req_token = Some(url_decode(v));
            }
        }
    }

    // 3. Token check
    let expected_token = MEDIA_SERVER_TOKEN.get().map(|s| s.as_str()).unwrap_or("");
    if req_token.as_deref() != Some(expected_token) {
        let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nInvalid or missing session token");
        return;
    }

    // 4. Path traversal & file check
    let Some(file_path_str) = req_path else {
        let _ = stream.write_all(b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\nMissing path parameter");
        return;
    };
    if file_path_str.contains("..") {
        let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nDirectory traversal detected");
        return;
    }

    let path = PathBuf::from(&file_path_str);
    if !path.is_file() {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n");
        return;
    }

    // 5. Allow-list verification
    if !is_allowed_media_path(&path) {
        let _ = stream.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nPath not in project allow-list");
        return;
    }

    let metadata = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(_) => {
            let _ = stream.write_all(b"HTTP/1.1 500 Internal Error\r\nConnection: close\r\n\r\n");
            return;
        }
    };
    let total_size = metadata.len();

    let mut range_start: u64 = 0;
    let mut range_end: u64 = total_size.saturating_sub(1);
    let mut is_range = false;

    if let Some(range_val) = range_header {
        if let Some(bytes_spec) = range_val.strip_prefix("bytes=") {
            let spec_parts: Vec<&str> = bytes_spec.split('-').collect();
            if spec_parts.len() == 2 {
                if let Ok(start) = spec_parts[0].parse::<u64>() {
                    range_start = start;
                    is_range = true;
                }
                if !spec_parts[1].is_empty() {
                    if let Ok(end) = spec_parts[1].parse::<u64>() {
                        if end < total_size {
                            range_end = end;
                        }
                    }
                }
            }
        }
    }

    if range_start > range_end || range_start >= total_size {
        let _ = stream.write_all(
            format!("HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\n\r\n", total_size).as_bytes()
        );
        return;
    }

    let content_len = range_end - range_start + 1;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let content_type = match ext.as_str() {
        "webm" => "video/webm",
        "mp4" | "m4v" => "video/mp4",
        "mkv" => "video/x-matroska",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "ts" => "video/mp2t",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "flac" => "audio/flac",
        "ogg" => "audio/ogg",
        "aac" => "audio/aac",
        "m4a" => "audio/mp4",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    };

    let response_headers = if is_range {
        format!(
            "HTTP/1.1 206 Partial Content\r\n\
Content-Type: {}\r\n\
Content-Length: {}\r\n\
Content-Range: bytes {}-{}/{}\r\n\
Accept-Ranges: bytes\r\n\
Access-Control-Allow-Origin: {}\r\n\
Vary: Origin\r\n\
Access-Control-Allow-Headers: Range, Content-Type, Accept\r\n\
Access-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges\r\n\
Connection: close\r\n\r\n",
            content_type, content_len, range_start, range_end, total_size, cors_origin
        )
    } else {
        format!(
            "HTTP/1.1 200 OK\r\n\
Content-Type: {}\r\n\
Content-Length: {}\r\n\
Accept-Ranges: bytes\r\n\
Access-Control-Allow-Origin: {}\r\n\
Vary: Origin\r\n\
Access-Control-Allow-Headers: Range, Content-Type, Accept\r\n\
Access-Control-Expose-Headers: Content-Range, Content-Length, Accept-Ranges\r\n\
Connection: close\r\n\r\n",
            content_type, total_size, cors_origin
        )
    };

    if stream.write_all(response_headers.as_bytes()).is_err() || is_head {
        return;
    }

    let mut file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return,
    };

    if file.seek(SeekFrom::Start(range_start)).is_err() {
        return;
    }

    let mut reader = file.take(content_len);
    let mut copy_buf = [0u8; 65536];
    loop {
        match reader.read(&mut copy_buf) {
            Ok(0) => break,
            Ok(n) => {
                if stream.write_all(&copy_buf[..n]).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

pub fn start_media_server() -> u16 {
    let token = generate_session_token();
    let _ = MEDIA_SERVER_TOKEN.set(token);

    let listener = match TcpListener::bind("127.0.0.1:0") {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind local media streaming server: {e}");
            return 0;
        }
    };
    let port = listener.local_addr().map(|a| a.port()).unwrap_or(0);
    MEDIA_SERVER_PORT.store(port, Ordering::SeqCst);

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                std::thread::spawn(move || {
                    handle_media_stream_client(stream);
                });
            }
        }
    });

    port
}

pub fn get_port() -> u16 {
    MEDIA_SERVER_PORT.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn get_media_server_port() -> u16 {
    MEDIA_SERVER_PORT.load(Ordering::SeqCst)
}

#[tauri::command]
pub fn get_media_server_token() -> String {
    MEDIA_SERVER_TOKEN.get().cloned().unwrap_or_default()
}
