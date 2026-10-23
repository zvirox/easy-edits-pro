use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{LazyLock, OnceLock, RwLock};

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
    let Ok(canon) = p.canonicalize() else {
        return false;
    };
    if let Some(app_dir) = APP_DATA_DIR.get() {
        if ["network_imports", "assets"]
            .iter()
            .any(|sub| canon.starts_with(app_dir.join(sub)))
        {
            return true;
        }
    }
    if let Some(cache_dir) = APP_CACHE_DIR.get() {
        if canon.starts_with(cache_dir.join("video-previews")) {
            return true;
        }
    }
    false
}

fn generate_session_token() -> String {
    let mut raw = [0u8; 16];
    getrandom::fill(&mut raw).expect("OS random number generator unavailable");
    raw.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn token_matches(provided: Option<&str>, expected: &str) -> bool {
    let Some(provided) = provided else {
        return false;
    };
    if expected.is_empty() || provided.len() != expected.len() {
        return false;
    }
    provided
        .bytes()
        .zip(expected.bytes())
        .fold(0u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

fn origin_allowed(origin: &str) -> bool {
    const FIXED: [&str; 3] = [
        "tauri://localhost",
        "http://tauri.localhost",
        "https://tauri.localhost",
    ];
    FIXED.contains(&origin)
        || (cfg!(debug_assertions)
            && (origin == "http://localhost:1420" || origin == "http://127.0.0.1:1420"))
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

    const MAX_HEADER_BYTES: usize = 16 * 1024;
    let mut buffer = Vec::with_capacity(2048);
    let mut chunk = [0u8; 2048];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
            Err(_) => return,
        }
        if buffer.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
        if buffer.len() > MAX_HEADER_BYTES {
            let _ = stream.write_all(
                b"HTTP/1.1 431 Request Header Fields Too Large\r\nConnection: close\r\n\r\n",
            );
            return;
        }
    }
    if buffer.is_empty() {
        return;
    }
    let request_str = String::from_utf8_lossy(&buffer);
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
        let _ =
            stream.write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nForbidden Host");
        return;
    }

    // 2. CORS origin validation
    let cors_origin = match origin_header.as_deref().map(str::trim) {
        None => "tauri://localhost",
        Some(origin) if origin_allowed(origin) => origin,
        Some(_) => {
            let _ = stream
                .write_all(b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nForbidden Origin");
            return;
        }
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
    if !token_matches(req_token.as_deref(), expected_token) {
        let _ = stream.write_all(
            b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nInvalid or missing session token",
        );
        return;
    }

    // 4. Path traversal & file check
    let Some(file_path_str) = req_path else {
        let _ = stream.write_all(
            b"HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\nMissing path parameter",
        );
        return;
    };
    let Ok(path) = PathBuf::from(&file_path_str).canonicalize() else {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n");
        return;
    };
    if !path.is_file() {
        let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nConnection: close\r\n\r\n");
        return;
    }

    // 5. Allow-list verification
    if !is_allowed_media_path(&path) {
        let _ = stream.write_all(
            b"HTTP/1.1 403 Forbidden\r\nConnection: close\r\n\r\nPath not in project allow-list",
        );
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
                if spec_parts[0].is_empty() {
                    if let Ok(suffix) = spec_parts[1].parse::<u64>() {
                        if suffix > 0 {
                            range_start = total_size.saturating_sub(suffix);
                            is_range = true;
                        }
                    }
                } else if let Ok(start) = spec_parts[0].parse::<u64>() {
                    range_start = start;
                    is_range = true;
                }
                if !spec_parts[0].is_empty() && !spec_parts[1].is_empty() {
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
            format!(
                "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{}\r\n\r\n",
                total_size
            )
            .as_bytes(),
        );
        return;
    }

    let content_len = range_end - range_start + 1;
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
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
X-Content-Type-Options: nosniff\r\n\
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
X-Content-Type-Options: nosniff\r\n\
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_is_random_hex_and_unique() {
        let first = generate_session_token();
        let second = generate_session_token();
        assert_eq!(first.len(), 32);
        assert!(first.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(first, second);
    }

    #[test]
    fn token_comparison_requires_equal_nonempty_values() {
        assert!(token_matches(Some("abc"), "abc"));
        assert!(!token_matches(Some("abd"), "abc"));
        assert!(!token_matches(Some("ab"), "abc"));
        assert!(!token_matches(None, "abc"));
        assert!(!token_matches(Some(""), ""));
    }

    #[test]
    fn origin_allowlist_is_exact() {
        assert!(origin_allowed("tauri://localhost"));
        assert!(origin_allowed("http://tauri.localhost"));
        assert!(!origin_allowed("http://localhost:8080"));
        assert!(!origin_allowed("http://127.0.0.1:9999"));
        assert!(!origin_allowed("http://localhost:1420.evil.com"));
        assert!(!origin_allowed("tauri://localhost.evil.com"));
    }

    #[test]
    fn allow_list_requires_explicit_registration() {
        let folder = tempfile::tempdir().unwrap();
        let inside = folder.path().join("scene.mp4");
        std::fs::write(&inside, b"media").unwrap();
        let outside = folder.path().join("secret.mp4");
        std::fs::write(&outside, b"secret").unwrap();

        assert!(!is_allowed_media_path(&inside), "nothing is allowed before registration");
        register_allowed_media_path(&inside);
        assert!(is_allowed_media_path(&inside));
        // Canonical and as-written forms must both match.
        let canon = inside.canonicalize().unwrap();
        assert!(is_allowed_media_path(&canon), "canonical form must be allowed");
        assert!(!is_allowed_media_path(&outside), "sibling file stays forbidden");
    }

    fn exchange(raw_request: &str) -> String {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_media_stream_client(stream);
        });
        let mut client = TcpStream::connect(addr).unwrap();
        client.write_all(raw_request.as_bytes()).unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        handle.join().unwrap();
        response
    }

    /// Sends one request through the real media-server handler and asserts
    /// the security chain fires in order: host header, CORS origin, session
    /// token, then the project media allow-list (planning safety packet).
    #[test]
    fn media_stream_request_chain_rejects_everything_untrusted() {
        const PORT: u16 = 45987;
        MEDIA_SERVER_PORT.store(PORT, Ordering::SeqCst);
        let token = MEDIA_SERVER_TOKEN.get_or_init(|| "eep-test-session-token".to_owned());

        let folder = tempfile::tempdir().unwrap();
        let media = folder.path().join("allowed.bin");
        std::fs::write(&media, b"frame-data").unwrap();
        register_allowed_media_path(&media);
        let secret = folder.path().join("unregistered.bin");
        std::fs::write(&secret, b"secret").unwrap();

        let get = |host: &str, origin: Option<&str>, token_param: Option<&str>, path: &str| {
            let mut request = format!("GET /media?path={path}");
            if let Some(token_param) = token_param {
                request.push_str(&format!("&token={token_param}"));
            }
            request.push_str(&format!(" HTTP/1.1\r\nHost: {host}\r\n"));
            if let Some(origin) = origin {
                request.push_str(&format!("Origin: {origin}\r\n"));
            }
            request.push_str("\r\n");
            exchange(&request)
        };
        let ok = format!("127.0.0.1:{PORT}");
        assert!(!token.is_empty());

        // 1. DNS-rebinding host is refused before anything else.
        let response = get("rebind.example:80", Some("tauri://localhost"), Some(token), "/x");
        assert!(response.starts_with("HTTP/1.1 403"), "bad host: {response}");

        // 2. Foreign web origin is refused.
        let response = get(&ok, Some("http://localhost:1420.evil.com"), Some(token), "/x");
        assert!(response.starts_with("HTTP/1.1 403"), "bad origin: {response}");

        // 3. Missing or wrong session token is refused.
        for broken in [None, Some("wrong-token-value0000")] {
            let response = get(&ok, Some("tauri://localhost"), broken, "/x");
            assert!(
                response.starts_with("HTTP/1.1 403") && response.contains("session token"),
                "token {broken:?}: {response}"
            );
        }

        // 4. A valid-looking request for a path the project never registered
        //    is still forbidden - allow-list, not existence, decides access.
        let response = get(
            &ok,
            Some("tauri://localhost"),
            Some(token),
            &percent_encode(&secret),
        );
        assert!(
            response.starts_with("HTTP/1.1 403") && response.contains("allow-list"),
            "unregistered path: {response}"
        );

        // 5. Registered + fully valid request streams the file.
        let response = get(
            &ok,
            Some("tauri://localhost"),
            Some(token),
            &percent_encode(&media),
        );
        assert!(response.starts_with("HTTP/1.1 200"), "registered path: {response}");
        assert!(response.contains("frame-data"), "body streamed: {response:?}");
    }

    fn percent_encode(path: &Path) -> String {
        path.to_string_lossy()
            .bytes()
            .map(|byte| {
                if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-') {
                    (byte as char).to_string()
                } else {
                    format!("%{byte:02X}")
                }
            })
            .collect()
    }
}
