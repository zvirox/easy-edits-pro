//! Preview-proxy lifecycle (planning.md M1.1): deterministic cache keys for
//! generated preview proxies and the stale-prune policy.
//!
//! Invalidation is deliberately age-based: the cache key binds the source
//! path, length, and modification time, but once a source disappears (media
//! deleted or relinked) its length/mtime are gone with it, so its old proxy
//! cannot be reconstructed and targeted. Stale files are therefore pruned by
//! age at startup.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Subdirectory of the app cache dir holding generated preview proxies.
pub const CACHE_SUBDIR: &str = "video-previews";

/// How long an untouched proxy is kept before the startup prune removes it.
pub const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// Deterministic proxy cache key for a source file: binds path, length, and
/// modification time so an edited source invalidates its own proxy. Must stay
/// byte-compatible with the hashes already on disk.
pub fn proxy_key(source: &Path, len: u64, modified_nanos: u128) -> String {
    let mut key = DefaultHasher::new();
    source.hash(&mut key);
    len.hash(&mut key);
    modified_nanos.hash(&mut key);
    format!("{:016x}", key.finish())
}

/// Full proxy path for a source inside the preview cache directory.
pub fn proxy_path(cache_dir: &Path, source: &Path, len: u64, modified_nanos: u128) -> PathBuf {
    cache_dir.join(format!("{}.webm", proxy_key(source, len, modified_nanos)))
}

/// Removes `*.webm` proxy files strictly older than `max_age`, measured from
/// `now`. Returns (removed count, reclaimed bytes). A missing cache directory
/// is not an error: there is simply nothing to prune.
pub fn prune_stale_proxies(cache_dir: &Path, max_age: Duration, now: SystemTime) -> (usize, u64) {
    let entries = match std::fs::read_dir(cache_dir) {
        Ok(entries) => entries,
        Err(_) => return (0, 0),
    };
    let mut removed = 0usize;
    let mut bytes = 0u64;
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("webm") {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        let Ok(age) = now.duration_since(modified) else {
            continue;
        };
        if age > max_age && std::fs::remove_file(&path).is_ok() {
            removed += 1;
            bytes += metadata.len();
        }
    }
    (removed, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_keys_bind_source_path_length_and_mtime() {
        let source = Path::new("/assets/interview.mp4");
        let key = proxy_key(source, 1024, 1_760_000_000_000);
        assert_eq!(key.len(), 16, "hex u64 key: {key}");
        assert_eq!(proxy_key(source, 1024, 1_760_000_000_000), proxy_key(source, 1024, 1_760_000_000_000));
        // Any change to length or modification time must move the proxy file
        // so a mutated source never plays a stale preview.
        assert_ne!(proxy_key(source, 2048, 1_760_000_000_000), key);
        assert_ne!(proxy_key(source, 1024, 1_760_000_000_001), key);
        assert_ne!(
            proxy_key(Path::new("/assets/other.mp4"), 1024, 1_760_000_000_000),
            key
        );
    }

    #[test]
    fn prune_removes_only_stale_proxies() {
        let folder = tempfile::tempdir().unwrap();
        let cache = folder.path().join(CACHE_SUBDIR);
        std::fs::create_dir_all(&cache).unwrap();
        let now = SystemTime::now();
        let stale = cache.join("aaaa1111bbbb2222.webm");
        let fresh = cache.join("cccc3333dddd4444.webm");
        let keep = cache.join("eeee5555ffff6666.txt");
        std::fs::write(&stale, b"old").unwrap();
        std::fs::write(&fresh, b"new").unwrap();
        std::fs::write(&keep, b"not a proxy").unwrap();
        // The stale proxy was last touched 31 days ago; the fresh one now.
        let old = now - Duration::from_secs(31 * 24 * 60 * 60);
        filetime::set_file_mtime(&stale, filetime::FileTime::from_system_time(old)).unwrap();
        filetime::set_file_mtime(&fresh, filetime::FileTime::from_system_time(now)).unwrap();

        let (removed, bytes) = prune_stale_proxies(&cache, MAX_AGE, now);
        assert_eq!(removed, 1, "only the stale proxy is removed");
        assert_eq!(bytes, 3, "the stale proxy was 3 bytes");
        assert!(!stale.exists());
        assert!(fresh.exists());
        assert!(keep.exists(), "non-proxy files are never pruned");

        // A missing cache directory is a no-op, not an error.
        let (removed, bytes) = prune_stale_proxies(&folder.path().join("absent"), MAX_AGE, now);
        assert_eq!((removed, bytes), (0, 0));
    }
}
