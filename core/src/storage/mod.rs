use memmap2::MmapMut;
use std::fs::OpenOptions;
use std::path::PathBuf;

pub struct MmapStorage {
    mmap: MmapMut,
}

impl MmapStorage {
    fn create(path: PathBuf, size: u64) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
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

    pub fn with_mut<T, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        unsafe {
            let ptr = self.mmap.as_mut_ptr() as *mut T;
            f(&mut *ptr)
        }
    }

    pub fn with_ref<T, R>(&self, f: impl FnOnce(&T) -> R) -> R {
        unsafe {
            // debug_assert_eq!((self.mmap.as_ptr() as usize) % align_of::<T>(), 0);
            let ptr = self.mmap.as_ptr() as *const T;
            f(&*ptr)
        }
    }

    pub fn flush(&self) -> std::io::Result<()> {
        self.mmap.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Meta {
        unfamiliar: bool,
        term: u64,
        log_id: u64,
        committed_index: u64,
    }
    #[test]
    fn it_works() {
        let path = PathBuf::from("/tmp/raft/meta.bin");
        if path.exists() {
            std::fs::remove_file(&path).unwrap();
        }
        let mut store = MmapStorage::open_or_create(path, 1024).unwrap();

        let log_id = store.with_mut::<Meta, _>(|meta| {
            meta.term += 1;
            meta.log_id += 1;
            meta.log_id
        });
        assert_eq!(log_id, 1);

        let term = store.with_mut::<Meta, _>(|meta| meta.term);
        assert_eq!(term, 1);

        let unfamiliar = store.with_ref::<Meta, _>(|meta| meta.unfamiliar);
        println!("{:?}", unfamiliar);
        store.flush().unwrap();
    }
}
