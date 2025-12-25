mod config;
pub mod ruft;

use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;
pub use crate::node::config::Config;
use crate::node::ruft::Ruft;
use std::sync::Arc;

pub struct Node {
    ruft: Arc<Ruft>,
}

impl Node {
    pub fn new(config: Config) -> Self {
        Node {
            ruft: Arc::new(Ruft::new(config)),
        }
    }

    pub fn start(&self) {
        let rpc_server_handle = self.ruft.clone().start_rpc_server();
    }

    pub fn update_member(&self, endpoints: Vec<Endpoint>) {
        self.ruft.update_member(endpoints);
    }

    pub fn emit(&self, cmd: CmdReq) -> CmdResp {
        self.ruft.emit(cmd)
    }
}
