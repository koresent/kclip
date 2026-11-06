use crate::error::StorageError;
use content::compute_sha256;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct BlobStore {
    root_dir: PathBuf,
}

impl BlobStore {
    pub fn new(root_dir: impl AsRef<Path>) -> Result<Self, StorageError> {
        let root = root_dir.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|e| StorageError::Io {
            path: root.clone(),
            source: e,
        })?;
        Ok(Self { root_dir: root })
    }

    pub fn write_blob(&self, bytes: &[u8]) -> Result<String, StorageError> {
        let hash = compute_sha256(bytes);
        let path = self.blob_path(&hash);

        if path.exists() {
            return Ok(hash);
        }

        let tmp_path = self.root_dir.join(format!("{}.tmp", hash));
        fs::write(&tmp_path, bytes).map_err(|e| StorageError::Io {
            path: tmp_path.clone(),
            source: e,
        })?;

        fs::rename(&tmp_path, &path).map_err(|e| StorageError::Io { path, source: e })?;

        Ok(hash)
    }

    pub fn read_blob(&self, hash: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let path = self.blob_path(hash);
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::read(&path).map_err(|e| StorageError::Io {
            path: path.clone(),
            source: e,
        })?;

        let actual_hash = compute_sha256(&bytes);
        if actual_hash != hash {
            return Err(StorageError::HashMismatch {
                hash: hash.to_string(),
            });
        }

        Ok(Some(bytes))
    }

    pub fn delete_blob(&self, hash: &str) -> Result<bool, StorageError> {
        let path = self.blob_path(hash);
        if path.exists() {
            fs::remove_file(&path).map_err(|e| StorageError::Io { path, source: e })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn blob_path(&self, hash: &str) -> PathBuf {
        self.root_dir.join(hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_and_read_blob() {
        let dir = tempdir().unwrap();
        let store = BlobStore::new(dir.path()).unwrap();

        let data = b"large image binary payload";
        let hash = store.write_blob(data).unwrap();
        assert_eq!(hash, compute_sha256(data));

        let retrieved = store.read_blob(&hash).unwrap().expect("blob must exist");
        assert_eq!(retrieved, data);

        assert!(store.delete_blob(&hash).unwrap());
        assert!(store.read_blob(&hash).unwrap().is_none());
    }
}
