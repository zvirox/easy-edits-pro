// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! BLAKE3 media fingerprints (schema v15).
//!
//! **This is a fingerprint, not a content hash** (plan-review constraint,
//! `docs/AI_NATIVE_PLAN_CHECKLIST.md` Packet 0.2, audit §4.10). It hashes the
//! first and last slice of a file plus its size, which is cheap for
//! multi-gigabyte media and good enough to *suggest* a relink when a project
//! folder moved. It never asserts two files are identical on its own: any
//! relink driven by it is a suggestion the user confirms through
//! `project.relink_media`.

use serde::{Deserialize, Serialize};
use std::io::{Read, Seek};
use std::path::Path;

/// Bytes hashed from the head and tail of a file. Small enough to stay cheap
/// on network storage, large enough that a content change anywhere in the
/// head or tail of typical media is very likely to change the fingerprint.
pub const SLICE_BYTES: u64 = 64 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaFingerprint {
    /// BLAKE3 hex of the first `SLICE_BYTES` (or fewer) bytes.
    pub head: String,
    /// BLAKE3 hex of the last `SLICE_BYTES` (or fewer) bytes. Files at or
    /// below `SLICE_BYTES` hash the same bytes twice.
    pub tail: String,
    /// File size in bytes; part of the key, never a store of content.
    pub size: u64,
}

impl MediaFingerprint {
    /// True only when every component matches. Serialized fingerprints from
    /// older schema versions survive as data; they either compare equal or
    /// they do not — a mismatch is a "no suggestion", never an error.
    pub fn matches(&self, other: &MediaFingerprint) -> bool {
        self.head == other.head && self.tail == other.tail && self.size == other.size
    }
}

/// Compute the fingerprint of a file on disk. Bounded reads: at most
/// `2 × SLICE_BYTES` is ever pulled from the file.
pub fn compute_file(path: &Path) -> Result<MediaFingerprint, String> {
    let mut file = std::fs::File::open(path)
        .map_err(|error| format!("Could not open {} for fingerprinting: {error}", path.display()))?;
    let size = file
        .metadata()
        .map_err(|error| format!("Could not stat {}: {error}", path.display()))?
        .len();
    let head_len = size.min(SLICE_BYTES) as usize;
    let tail_len = head_len; // symmetric by symmetry of the byte range
    let mut head = vec![0u8; head_len];
    file.read_exact(&mut head)
        .map_err(|error| format!("Could not read head of {}: {error}", path.display()))?;
    let tail = if size <= SLICE_BYTES {
        head.clone()
    } else {
        let mut tail = vec![0u8; tail_len];
        let tail_start = size - SLICE_BYTES as u64;
        file.seek(std::io::SeekFrom::Start(tail_start))
            .map_err(|error| format!("Could not seek in {}: {error}", path.display()))?;
        file.read_exact(&mut tail)
            .map_err(|error| format!("Could not read tail of {}: {error}", path.display()))?;
        tail
    };
    let head_hex = blake3::hash(&head).to_hex().to_string();
    let tail_hex = blake3::hash(&tail).to_hex().to_string();
    Ok(MediaFingerprint { head: head_hex, tail: tail_hex, size })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn small_files_hash_the_once_whole_only() {
        let file = tempfile::NamedTempFile::new().unwrap();
        file.as_file().write_all(b"hello easy edits").unwrap();
        let fingerprint = compute_file(file.path()).unwrap();
        assert_eq!(fingerprint.size, 16);
        assert_eq!(fingerprint.head, fingerprint.tail);
        assert_eq!(fingerprint, compute_file(file.path()).unwrap());
        assert!(fingerprint.matches(&compute_file(file.path()).unwrap()));
    }

    #[test]
    fn head_and_tail_cover_both_ends() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut bytes = vec![0u8; SLICE_BYTES as usize * 2];
        let last = bytes.len() - 1;
        bytes[0] = 0xAB;
        bytes[last] = 0xCD;
        file.as_file().write_all(&bytes).unwrap();
        let fingerprint = compute_file(file.path()).unwrap();
        assert_eq!(fingerprint.size, bytes.len() as u64);
        assert_ne!(fingerprint.head, fingerprint.tail);
        let _ = &file;
    }

    /// The documented limitation: a byte in the file's middle — outside the
    /// head and tail slices — is invisible to the fingerprint. This is why
    /// fingerprint matches are *suggestions*, confirmed by the user.
    #[test]
    fn middle_bytes_are_invisible_to_the_fingerprint() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut bytes = vec![0u8; SLICE_BYTES as usize * 3];
        bytes[0] = 0xAB;
        bytes[SLICE_BYTES as usize + 7] = 0x55; // in the uncovered middle
        file.as_file().write_all(&bytes).unwrap();
        let fingerprint = compute_file(file.path()).unwrap();

        let mut other_file = tempfile::NamedTempFile::new().unwrap();
        let mut other = bytes.clone();
        other[SLICE_BYTES as usize + 7] ^= 0xFF;
        other_file.write_all(&other).unwrap();
        assert!(fingerprint.matches(&compute_file(other_file.path()).unwrap()));
    }

    #[test]
    fn byte_changes_at_either_end_change_the_fingerprint() {
        use std::io::Write as _;
        let sized = |fill: u8, index: usize| {
            let file = tempfile::NamedTempFile::new().unwrap();
            let mut bytes = vec![0u8; 200];
            bytes[index] = fill;
            file.as_file().write_all(&bytes).unwrap();
            file
        };
        let head_a = sized(0xAA, 0);
        let head_b = sized(0xBB, 0);
        let a = compute_file(head_a.path()).unwrap();
        let b = compute_file(head_b.path()).unwrap();
        assert_ne!(a.head, b.head);
        assert!(!a.matches(&b));

        let tail_a = sized(0xCC, 199);
        let tail_b = sized(0xDD, 199);
        let a = compute_file(tail_a.path()).unwrap();
        let b = compute_file(tail_b.path()).unwrap();
        assert_ne!(a.tail, b.tail);
        assert!(!a.matches(&b));
    }

    #[test]
    fn different_sizes_never_match() {
        use std::io::Write as _;
        let small = tempfile::NamedTempFile::new().unwrap();
        let big = tempfile::NamedTempFile::new().unwrap();
        small.as_file().write_all(&vec![7u8; 10]).unwrap();
        big.as_file().write_all(&vec![7u8; 20]).unwrap();
        let small_fp = compute_file(small.path()).unwrap();
        let big_fp = compute_file(big.path()).unwrap();
        assert_ne!(small_fp.size, big_fp.size);
        assert!(!small_fp.matches(&big_fp));
    }

    #[test]
    fn missing_files_do_not_fingerprint() {
        let error = compute_file(Path::new("/nowhere/definitely-not-here.mov"));
        assert!(error.is_err());
    }
}
