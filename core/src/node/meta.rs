use crate::Config;
use crate::endpoint::Endpoint;
use crate::storage::MmapStorage;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

#[repr(C)]
struct Meta {
    unfamiliar: bool,
    term: u64,
    log_id: u64,
    committed_index: u64,
    members: Vec<Endpoint>,
}
pub(super) struct MetaHolder {
    storage: MmapStorage,
}
impl MetaHolder {
    pub fn new(config: &Config) -> Self {
        let path = format!("{}/meta.bin", config.data_dir);
        let meta_path = PathBuf::from(&path);
        let mut meta_store = MmapStorage::open_or_create(meta_path, 1024).expect(format!("open meta file fail, {}", path).as_str());
        meta_store.with_mut::<Meta, _>(|meta| {
            if meta.unfamiliar {
                meta.unfamiliar = true;
                meta.term = 0;
                meta.log_id = 0;
                meta.committed_index = 0;
                meta.members = config.origin_endpoint.clone()
            }
        });
        let _result = meta_store.flush();
        MetaHolder { storage: meta_store }
    }

    pub fn next_log_id(&mut self) -> u64 {
        self.storage.with_mut::<Meta, _>(|meta| {
            meta.log_id += 1;
            meta.log_id
        })
    }

    pub fn next_term(&mut self) -> u64 {
        self.storage.with_mut::<Meta, _>(|meta| {
            meta.term += 1;
            meta.term
        })
    }

    pub fn term(&self) -> u64 {
        self.storage.with_ref::<Meta, _>(|meta| meta.term)
    }

    pub fn log_id(&self) -> u64 {
        self.storage.with_ref::<Meta, _>(|meta| meta.log_id)
    }

    pub fn committed_index(&self) -> u64 {
        self.storage.with_ref::<Meta, _>(|meta| meta.committed_index)
    }

    pub fn members(&self) -> &[Endpoint] {
        self.storage.with_ref(|meta| &meta.members)
    }
}

impl Display for MetaHolder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]:[id:{}]", self.term(), self.log_id())
    }
}
