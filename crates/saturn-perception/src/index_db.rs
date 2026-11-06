// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Semantic media index sidecar (Packet 2.2).
//!
//! A SQLite database next to the project file — `<project>.saturn-index.sqlite`
//! — holding per-frame SigLIP vectors: `frames(media_id, tick, vector)` keyed
//! by media id plus the revision of the embedding model that produced them.
//!
//! Deliberate simplicity: retrieval is a plain cosine scan over stored blobs.
//! For the scale a project index reaches (hundreds of vectors), that scan is
//! microseconds; if the index ever outgrows it, a vector extension can be
//! swapped in behind the same API. Nothing here invents scores — the returned
//! `similarity` is exactly the cosine between the query and stored vectors.

use crate::embeddings::cosine_similarity;
use rusqlite::{params, Connection};
use std::path::{Path, PathBuf};

/// Sidecar file next to `project_path`:
/// `<project dir>/<file stem>.saturn-index.sqlite`.
pub fn sidecar_path(project_path: &Path) -> Result<PathBuf, String> {
    let parent = project_path
        .parent()
        .map(|p| if p.as_os_str().is_empty() { Path::new(".") } else { p })
        .ok_or_else(|| "Project path has no parent directory".to_string())?;
    let stem = project_path
        .file_stem()
        .ok_or_else(|| "Project path has no file name".to_string())?;
    Ok(parent.join(format!(
        "{}.saturn-index.sqlite",
        stem.to_string_lossy()
    )))
}

/// One search hit: where a stored vector lives and how similar it was.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub media_id: String,
    pub tick_ticks: i64,
    /// Cosine similarity between the query vector and this frame's vector.
    pub similarity: f32,
}

/// Open (creating if needed) the semantic index sidecar for a project.
pub struct SemanticIndex {
    conn: Connection,
    path: PathBuf,
}

impl SemanticIndex {
    pub fn open_for_project(project_path: &Path) -> Result<Self, String> {
        let path = sidecar_path(project_path)?;
        let conn = Connection::open(&path)
            .map_err(|error| format!("Could not open the semantic index at {}: {error}", path.display()))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS frames (
                 media_id  TEXT    NOT NULL,
                 tick      INTEGER NOT NULL,
                 revision  TEXT    NOT NULL,
                 vector    BLOB    NOT NULL,
                 PRIMARY KEY (media_id, tick)
             );
             CREATE TABLE IF NOT EXISTS media (
                 media_id TEXT PRIMARY KEY,
                 path     TEXT NOT NULL,
                 revision TEXT NOT NULL
             );",
        )
        .map_err(|error| format!("Could not initialize the semantic index schema: {error}"))?;
        Ok(Self { conn, path })
    }

    /// Register/refresh a media's indexed location and embedding revision,
    /// dropping vectors from other revisions (stale embeddings).
    pub fn register_media(
        &self,
        media_id: &str,
        media_path: &Path,
        revision: &str,
    ) -> Result<(), String> {
        let stale = self
            .current_revision(media_id)
            .map(|was| was.as_deref() != Some(revision))
            .unwrap_or(true);
        if stale {
            self.drop_media(media_id)?;
        }
        self.conn
            .execute(
                "INSERT INTO media (media_id, path, revision) VALUES (?1, ?2, ?3)
                 ON CONFLICT(media_id) DO UPDATE SET path = ?2, revision = ?3",
                params![media_id, media_path.to_string_lossy(), revision],
            )
            .map_err(|error| format!("Could not register media in the index: {error}"))?;
        Ok(())
    }

    fn current_revision(&self, media_id: &str) -> Result<Option<String>, String> {
        self.conn
            .query_row(
                "SELECT revision FROM media WHERE media_id = ?1",
                params![media_id],
                |row| row.get(0),
            )
            .map(Some)
            .or_else(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => Ok(None),
                other => Err(format!("Index revision lookup failed: {other}")),
            })
    }

    /// Replace (or add) the vector for one sampled frame.
    pub fn store_frame_vector(
        &self,
        media_id: &str,
        tick: saturn_core::Tick,
        revision: &str,
        vector: &[f32],
    ) -> Result<(), String> {
        let bytes = f32_vec_to_bytes(vector);
        self.conn
            .execute(
                "INSERT INTO frames (media_id, tick, revision, vector) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(media_id, tick) DO UPDATE SET vector = ?4, revision = ?3",
                params![media_id, tick.0, revision, bytes],
            )
            .map_err(|error| format!("Could not store the frame vector: {error}"))?;
        Ok(())
    }

    /// Remove a media's vectors and registration (call when media is deleted).
    pub fn drop_media(&self, media_id: &str) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM frames WHERE media_id = ?1", params![media_id])
            .and_then(|_| {
                self.conn
                    .execute("DELETE FROM media WHERE media_id = ?1", params![media_id])
            })
            .map_err(|error| format!("Could not drop media from the index: {error}"))?;
        Ok(())
    }

    /// Cosine-scan the index for the `limit` most similar stored frames.
    /// `revision` filters to vectors from the same embedding model (vectors
    /// from a different model would be incomparable).
    pub fn query(
        &self,
        query_vector: &[f32],
        revision: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, String> {
        let mut statement = self
            .conn
            .prepare("SELECT media_id, tick, vector FROM frames WHERE revision = ?1")
            .map_err(|error| format!("Could not prepare the index query: {error}"))?;
        let rows = statement
            .query_map(params![revision], |row| {
                let media_id: String = row.get(0)?;
                let tick: i64 = row.get(1)?;
                let bytes: Vec<u8> = row.get(2)?;
                Ok((media_id, tick, bytes))
            })
            .map_err(|error| format!("Index scan failed: {error}"))?;
        let mut hits: Vec<SearchHit> = rows
            .filter_map(|row| {
                let (media_id, tick, bytes) = row.ok()?;
                let vector = f32_bytes_to_vec(&bytes)?;
                Some(SearchHit {
                    media_id,
                    tick_ticks: tick,
                    similarity: cosine_similarity(query_vector, &vector),
                })
            })
            .collect();
        hits.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(limit);
        Ok(hits)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Number of stored frame vectors across all revisions — used to tell
    /// "index exists but empty" (an honest not-yet-indexed state) apart from
    /// populated content before running queries.
    pub fn stored_vector_count(&self) -> Result<u64, String> {
        self.conn
            .query_row("SELECT COUNT(*) FROM frames", [], |row| row.get(0))
            .map_err(|error| format!("Could not count the indexed vectors: {error}"))
    }
}

fn f32_vec_to_bytes(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn f32_bytes_to_vec(bytes: &[u8]) -> Option<Vec<f32>> {
    if bytes.len() % 4 != 0 {
        return None;
    }
    Some(
        bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_project() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "saturn-index-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("test-project.eep")
    }

    fn unit(dim: usize, hot: usize) -> Vec<f32> {
        let mut v = vec![0.0; dim];
        v[hot] = 1.0;
        v
    }

    #[test]
    fn sidecar_path_sits_next_to_the_project() {
        let p = sidecar_path(Path::new("/media/project.eep")).unwrap();
        assert_eq!(p, PathBuf::from("/media/project.saturn-index.sqlite"));
        assert!(!sidecar_path(Path::new("project.eep")).is_err());
    }

    #[test]
    fn store_query_and_revision_stale_drop() {
        let project = temp_project();
        let index = SemanticIndex::open_for_project(&project).unwrap();
        index.register_media("m1", Path::new("/videos/a.mp4"), "rev1").unwrap();
        index
            .store_frame_vector("m1", saturn_core::Tick(0), "rev1", &unit(4, 0))
            .unwrap();
        index
            .store_frame_vector("m1", saturn_core::Tick(1), "rev1", &unit(4, 1))
            .unwrap();
        let hits = index.query(&unit(4, 1), "rev1", 10).unwrap();
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].tick_ticks, 1);
        assert!(hits[0].similarity > 0.99);
        assert!(hits[1].similarity < 0.1);

        // Same media re-indexed under a new revision drops the old vectors.
        index.register_media("m1", Path::new("/videos/a.mp4"), "rev2").unwrap();
        assert!(index.query(&unit(4, 0), "rev1", 10).unwrap().is_empty());
        index
            .store_frame_vector("m1", saturn_core::Tick(0), "rev2", &unit(4, 2))
            .unwrap();
        let hits = index.query(&unit(4, 2), "rev2", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].media_id, "m1");

        // drop_media clears both tables.
        index.drop_media("m1").unwrap();
        assert!(index.query(&unit(4, 2), "rev2", 10).unwrap().is_empty());
        let _ = std::fs::remove_file(index.path());
        let _ = std::fs::remove_dir(project.parent().unwrap());
    }

    #[test]
    fn blob_roundtrip() {
        let vector = vec![1.5_f32, -2.25, 0.0, f32::MIN_POSITIVE];
        let bytes = f32_vec_to_bytes(&vector);
        assert_eq!(f32_bytes_to_vec(&bytes).unwrap(), vector);
        assert!(f32_bytes_to_vec(&bytes[..5]).is_none());
    }
}
