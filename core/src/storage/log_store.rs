use prost::Message;
use rocksdb::{DB, Direction, IteratorMode, Options, WriteBatch};
use std::io;
use std::path::PathBuf;

use crate::RuftError;
pub use crate::rpc::LogEntry;

/// RocksDB-backed log storage for Raft
///
/// Design principles:
/// 1. Index is key (u64 big-endian) - maintains sort order
/// 2. LogEntry is value (protobuf) - stable serialization
/// 3. No special cases - just KV operations
///
/// Why RocksDB:
/// - Battle-tested in production (etcd, TiKV, CockroachDB)
/// - Optimized for write-heavy workloads (LSM-tree)
/// - Extensive tuning options for different scenarios
/// - 10+ years of optimization by Facebook/Meta
pub struct LogStore {
    db: DB,
}

impl LogStore {
    pub fn open(path: String) -> crate::Result<Self> {
        let path = format!("{}/data", path);
        let path = PathBuf::from(&path);
        Self::_open(path)
    }
    /// Open or create a log store at the given path
    fn _open(path: PathBuf) -> crate::Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        // Optimize for Raft workload
        opts.set_write_buffer_size(64 * 1024 * 1024); // 64MB write buffer
        opts.set_max_write_buffer_number(3);
        opts.set_target_file_size_base(64 * 1024 * 1024);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        let db = DB::open(&opts, path).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(Self { db })
    }

    /// Append a log entry
    ///
    /// Note: Caller must ensure index is monotonically increasing
    pub fn append(&self, entry: &LogEntry) -> crate::Result<()> {
        let key = Self::index_to_key(entry.index);
        let value = entry.encode_to_vec();

        self.db.put(key, value).map_err(|e| RuftError::Io(io::Error::new(io::ErrorKind::Other, e)))
    }

    /// Append multiple log entries in a batch
    ///
    /// Atomic operation - either all succeed or all fail
    pub fn append_batch(&self, entries: &[LogEntry]) -> crate::Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut batch = WriteBatch::default();
        for entry in entries {
            let key = Self::index_to_key(entry.index);
            let value = entry.encode_to_vec();
            batch.put(key, value);
        }

        self.db.write(batch).map_err(|e| RuftError::Io(io::Error::new(io::ErrorKind::Other, e)))
    }

    /// Get a log entry by index
    pub fn get(&self, index: u64) -> crate::Result<Option<LogEntry>> {
        let key = Self::index_to_key(index);

        match self.db.get(key) {
            Ok(Some(bytes)) => {
                let entry = LogEntry::decode(&bytes[..]).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
                Ok(Some(entry))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(RuftError::Io(io::Error::new(io::ErrorKind::Other, e))),
        }
    }

    /// Get a range of log entries [start_index, end_index)
    ///
    /// Returns empty vec if no entries found in range
    pub fn range(&self, start_index: u64, end_index: u64) -> crate::Result<Vec<LogEntry>> {
        if start_index >= end_index {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        let start_key = Self::index_to_key(start_index);
        let end_key = Self::index_to_key(end_index);

        let iter = self.db.iterator(IteratorMode::From(&start_key, Direction::Forward));

        for item in iter {
            let (key, value) = item.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            // Stop if we've reached the end of range
            if key.as_ref() >= end_key.as_slice() {
                break;
            }

            let entry = LogEntry::decode(&value[..]).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            entries.push(entry);
        }

        Ok(entries)
    }

    /// Get the last log entry's (index, term)
    ///
    /// Returns None if log is empty
    pub fn last(&self) -> crate::Result<Option<(u64, u64)>> {
        let mut iter = self.db.iterator(IteratorMode::End);

        match iter.next() {
            Some(result) => {
                let (_key, value) = result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

                let entry = LogEntry::decode(&value[..]).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

                Ok(Some((entry.index, entry.term)))
            }
            None => Ok(None),
        }
    }

    /// Delete all entries with index >= from_index
    ///
    /// Used when handling conflicting entries from leader
    pub fn truncate_suffix(&self, from_index: u64) -> crate::Result<()> {
        let start_key = Self::index_to_key(from_index);

        // Collect keys to delete
        let mut keys_to_delete = Vec::new();
        let iter = self.db.iterator(IteratorMode::From(&start_key, Direction::Forward));

        for item in iter {
            let (key, _) = item.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            keys_to_delete.push(key.to_vec());
        }

        // Delete in batch
        if !keys_to_delete.is_empty() {
            let mut batch = WriteBatch::default();
            for key in keys_to_delete {
                batch.delete(key);
            }
            self.db.write(batch).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }

        Ok(())
    }

    /// Convert index to big-endian key bytes
    ///
    /// Big-endian ensures lexicographic order = numeric order
    #[inline]
    fn index_to_key(index: u64) -> [u8; 8] {
        index.to_be_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn test_path(name: &str) -> PathBuf {
        PathBuf::from(format!("/tmp/ruft_test/{}", name))
    }

    fn cleanup(path: &Path) {
        if path.exists() {
            std::fs::remove_dir_all(path).ok();
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
    }

    #[test]
    fn test_append_and_get() {
        let path = test_path("append_get");
        cleanup(&path);

        let store = LogStore::_open(path).unwrap();

        let entry = LogEntry {
            index: 1,
            term: 1,
            command: vec![1, 2, 3],
        };

        store.append(&entry).unwrap();

        let retrieved = store.get(1).unwrap().unwrap();
        assert_eq!(retrieved.index, 1);
        assert_eq!(retrieved.term, 1);
        assert_eq!(retrieved.command, vec![1, 2, 3]);

        assert!(store.get(2).unwrap().is_none());
    }

    #[test]
    fn test_range() {
        let path = test_path("range");
        cleanup(&path);

        let store = LogStore::_open(path).unwrap();

        for i in 1..=10 {
            let entry = LogEntry {
                index: i,
                term: i / 3,
                command: vec![i as u8],
            };
            store.append(&entry).unwrap();
        }

        let entries = store.range(3, 7).unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].index, 3);
        assert_eq!(entries[3].index, 6);

        let empty = store.range(100, 200).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_last() {
        let path = test_path("last");
        cleanup(&path);

        let store = LogStore::_open(path).unwrap();

        assert!(store.last().unwrap().is_none());

        store.append(&LogEntry { index: 1, term: 1, command: vec![] }).unwrap();
        assert_eq!(store.last().unwrap(), Some((1, 1)));

        store.append(&LogEntry { index: 5, term: 2, command: vec![] }).unwrap();
        assert_eq!(store.last().unwrap(), Some((5, 2)));
    }

    #[test]
    fn test_truncate_suffix() {
        let path = test_path("truncate");
        cleanup(&path);

        let store = LogStore::_open(path).unwrap();

        for i in 1..=10 {
            store.append(&LogEntry { index: i, term: 1, command: vec![] }).unwrap();
        }

        store.truncate_suffix(6).unwrap();

        assert!(store.get(5).unwrap().is_some());
        assert!(store.get(6).unwrap().is_none());
        assert!(store.get(10).unwrap().is_none());

        assert_eq!(store.last().unwrap(), Some((5, 1)));
    }

    #[test]
    fn test_append_batch() {
        let path = test_path("batch");
        cleanup(&path);

        let store = LogStore::_open(path).unwrap();

        let entries: Vec<LogEntry> = (1..=100)
            .map(|i| LogEntry {
                index: i,
                term: i / 10,
                command: vec![i as u8],
            })
            .collect();

        store.append_batch(&entries).unwrap();

        assert_eq!(store.last().unwrap(), Some((100, 10)));
        assert_eq!(store.range(50, 60).unwrap().len(), 10);
    }

    #[test]
    fn test_persistence() {
        let path = test_path("persistence");
        cleanup(&path);

        // Write data
        {
            let path = test_path("persistence");
            let store = LogStore::_open(path).unwrap();
            for i in 1..=5 {
                store
                    .append(&LogEntry {
                        index: i,
                        term: 1,
                        command: vec![i as u8],
                    })
                    .unwrap();
            }
        }

        // Reopen and verify
        {
            let path = test_path("persistence");
            let store = LogStore::_open(path).unwrap();
            assert_eq!(store.last().unwrap(), Some((5, 1)));
            let entry = store.get(3).unwrap().unwrap();
            assert_eq!(entry.command, vec![3]);
        }
    }
}
