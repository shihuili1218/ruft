use memmap2::MmapMut;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io;
use std::mem::align_of;
use std::path::PathBuf;

/// Marker trait for types safe to use with direct memory mapping.
///
/// # Safety
/// Only implement this for types that are:
/// - `Copy` (no Drop logic)
/// - `repr(C)` or `repr(transparent)` (known layout)
/// - No padding or all padding is initialized
/// - No pointers or references
///
/// Violating these rules will cause undefined behavior!
pub unsafe trait Pod: Copy {}

// Example safe implementations for primitive types
unsafe impl Pod for u8 {}
unsafe impl Pod for u16 {}
unsafe impl Pod for u32 {}
unsafe impl Pod for u64 {}
unsafe impl Pod for i8 {}
unsafe impl Pod for i16 {}
unsafe impl Pod for i32 {}
unsafe impl Pod for i64 {}
unsafe impl Pod for f32 {}
unsafe impl Pod for f64 {}

pub struct MmapStorage {
    mmap: MmapMut,
}

impl MmapStorage {
    fn create(path: PathBuf, size: u64) -> std::io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).create(true).open(path)?;
        file.set_len(size)?;
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        Ok(Self { mmap })
    }

    fn open(path: &std::path::Path) -> std::io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let mmap = unsafe { MmapMut::map_mut(&file)? };
        Ok(Self { mmap })
    }

    pub fn open_or_create(path: PathBuf, size: u64) -> std::io::Result<Self> {
        if path.exists() {
            Self::open(&path)
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            Self::create(path, size)
        }
    }

    /// Write serializable data to storage
    pub fn write_serialized<T: Serialize>(&mut self, data: &T) -> io::Result<()> {
        let bytes = bincode::serialize(data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if bytes.len() > self.mmap.len() {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "data too large for mmap"));
        }

        self.mmap[..bytes.len()].copy_from_slice(&bytes);
        self.mmap.flush()
    }

    /// Read serializable data from storage
    pub fn read_serialized<T: for<'de> Deserialize<'de>>(&self) -> io::Result<T> {
        bincode::deserialize(&self.mmap).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// Direct memory access for POD types
    ///
    /// # Safety
    /// T must implement Pod trait, which guarantees:
    /// - Copy semantics
    /// - repr(C) layout
    /// - No padding issues
    /// - No pointers
    ///
    /// The compiler will reject non-Pod types at compile time!
    pub fn with_mut<T: Pod>(&mut self, f: impl FnOnce(&mut T)) -> io::Result<()> {
        // Safety checks
        if std::mem::size_of::<T>() > self.mmap.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "type too large"));
        }
        if (self.mmap.as_ptr() as usize) % align_of::<T>() != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "misaligned pointer"));
        }

        unsafe {
            let ptr = self.mmap.as_mut_ptr() as *mut T;
            f(&mut *ptr);
        }
        self.mmap.flush()
    }

    /// Direct memory access for POD types (read-only)
    pub fn with_ref<T: Pod, R>(&self, f: impl FnOnce(&T) -> R) -> io::Result<R> {
        if std::mem::size_of::<T>() > self.mmap.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "type too large"));
        }
        if (self.mmap.as_ptr() as usize) % align_of::<T>() != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "misaligned pointer"));
        }

        unsafe {
            let ptr = self.mmap.as_ptr() as *const T;
            Ok(f(&*ptr))
        }
    }

    pub fn flush(&self) -> std::io::Result<()> {
        self.mmap.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestMeta {
        term: u64,
        log_id: u64,
        committed_index: u64,
    }

    #[test]
    fn test_serialization() {
        let path = PathBuf::from("/tmp/raft/meta_test.bin");
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }

        let mut store = MmapStorage::open_or_create(path.clone(), 4096).unwrap();

        // Write data
        let meta = TestMeta {
            term: 1,
            log_id: 100,
            committed_index: 50,
        };
        store.write_serialized(&meta).unwrap();

        // Read data back
        let loaded: TestMeta = store.read_serialized().unwrap();
        assert_eq!(loaded, meta);

        // Update data
        let meta2 = TestMeta {
            term: 2,
            log_id: 200,
            committed_index: 150,
        };
        store.write_serialized(&meta2).unwrap();

        // Reopen and read
        let store2 = MmapStorage::open_or_create(path, 4096).unwrap();
        let loaded2: TestMeta = store2.read_serialized().unwrap();
        assert_eq!(loaded2, meta2);
    }

    // Test that Pod trait prevents unsafe usage
    #[test]
    fn test_pod_safety() {
        let path = PathBuf::from("/tmp/raft/pod_test.bin");
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }

        let mut store = MmapStorage::open_or_create(path, 4096).unwrap();

        // This compiles - u64 implements Pod
        store.with_mut(|val: &mut u64| *val = 42).unwrap();

        // This would NOT compile - String doesn't implement Pod:
        // store.with_mut(|val: &mut String| *val = "hello".into()).unwrap();
        //                         ^^^^^^ the trait `Pod` is not implemented for `String`
    }
}
