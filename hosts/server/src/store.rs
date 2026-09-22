//! File-backed blob store (docs/hosts/server.md "Storage"): `data/<id>/pet.bin` plus
//! `meta.json`, written to a temp file and renamed so a crash never leaves a half-written
//! blob. One user, one blob per pet: no database (ADR 0013).

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Meta {
    /// Server sequence number, incremented on every accepted PUT. Breaks `sim_now` ties
    /// (docs/SYNC.md: "ties break on server sequence number").
    pub seq: u64,
    /// The blob's own `sim_now`, as the client declared and the server verified on PUT.
    pub sim_now: u32,
    /// RFC 3339, server clock; informational only, never used for conflict resolution.
    pub saved_at: String,
    pub content_hash: u32,
}

#[derive(Debug, Clone)]
pub struct Stored {
    pub meta: Meta,
    pub blob: Vec<u8>,
}

pub struct Store {
    root: PathBuf,
    /// One async mutex per id so concurrent PUTs to the same pet serialise (read-compare-write
    /// must be atomic per id); different ids never block each other.
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

/// Client-chosen ids are path components; keep them boring (docs/SYNC.md: "`default` is fine
/// for one pet; a UUID if you want several").
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

impl Store {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Store {
            root: root.into(),
            locks: Mutex::new(HashMap::new()),
        }
    }

    pub async fn lock(&self, id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().await;
        locks
            .entry(id.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    fn dir(&self, id: &str) -> PathBuf {
        self.root.join(id)
    }

    pub async fn get(&self, id: &str) -> io::Result<Option<Stored>> {
        let dir = self.dir(id);
        let meta_bytes = match tokio::fs::read(dir.join("meta.json")).await {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let meta: Meta = serde_json::from_slice(&meta_bytes).map_err(io::Error::other)?;
        let blob = tokio::fs::read(dir.join("pet.bin")).await?;
        Ok(Some(Stored { meta, blob }))
    }

    /// Writes `blob` as the new current save, with `seq` one past the previous (or 1).
    /// Callers hold `lock(id)` across their read-compare-write.
    pub async fn put(
        &self,
        id: &str,
        blob: &[u8],
        sim_now: u32,
        content_hash: u32,
    ) -> io::Result<Meta> {
        let dir = self.dir(id);
        tokio::fs::create_dir_all(&dir).await?;
        let prev_seq = self.get(id).await?.map(|s| s.meta.seq).unwrap_or(0);
        let meta = Meta {
            seq: prev_seq + 1,
            sim_now,
            saved_at: now_rfc3339(),
            content_hash,
        };
        write_atomic(&dir.join("pet.bin"), blob).await?;
        let meta_json = serde_json::to_vec_pretty(&meta).map_err(io::Error::other)?;
        write_atomic(&dir.join("meta.json"), &meta_json).await?;
        Ok(meta)
    }

    pub async fn delete(&self, id: &str) -> io::Result<()> {
        match tokio::fs::remove_dir_all(self.dir(id)).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

async fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, path).await
}

fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_boring_path_components() {
        assert!(valid_id("default"));
        assert!(valid_id("3f2a-UUID_like"));
        assert!(!valid_id(""));
        assert!(!valid_id("../etc"));
        assert!(!valid_id("a/b"));
        assert!(!valid_id(&"x".repeat(65)));
    }

    #[tokio::test]
    async fn put_get_delete_round_trip_and_seq_increments() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        assert!(store.get("default").await.unwrap().is_none());

        let m1 = store.put("default", b"one", 10, 7).await.unwrap();
        assert_eq!(m1.seq, 1);
        let s = store.get("default").await.unwrap().unwrap();
        assert_eq!(s.blob, b"one");
        assert_eq!(s.meta.sim_now, 10);
        assert_eq!(s.meta.content_hash, 7);

        let m2 = store.put("default", b"two", 11, 7).await.unwrap();
        assert_eq!(m2.seq, 2);
        assert_eq!(store.get("default").await.unwrap().unwrap().blob, b"two");
        assert!(!dir.path().join("default/pet.bin.tmp").exists()); // renamed away

        store.delete("default").await.unwrap();
        assert!(store.get("default").await.unwrap().is_none());
        store.delete("default").await.unwrap(); // idempotent
    }
}
