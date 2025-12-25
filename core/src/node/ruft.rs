use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;
use crate::node::{Config, Node};
use crate::rpc::server::run_server;
use std::sync::Arc;

pub struct Ruft {
    inner: Arc<Node>,
}

impl Ruft {
    pub fn new(config: Config) -> Self {
        Ruft {
            inner: Arc::new(Node::new(config)),
        }
    }

    pub fn start(&self) {
        let _rpc_server_handle = run_server(&self.inner);
        self.inner.start();
    }

    pub fn update_member(&self, endpoints: Vec<Endpoint>) {
        self.inner.update_member(endpoints);
    }

    pub fn emit(&self, cmd: CmdReq) -> CmdResp {
        self.inner.emit(cmd)
    }
}

impl Clone for Ruft {
    fn clone(&self) -> Self {
        Ruft {
            inner: self.inner.clone(),
        }
    }
}
