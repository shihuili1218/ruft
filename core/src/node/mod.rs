pub mod meta;
pub mod node;

use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;
use crate::node::node::Node;
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
        if let Ok(rt) = tokio::runtime::Runtime::new() {
            rt.block_on(async {
                let _rpc_server_handle = run_server(&self.inner).await;
            });
        }
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

pub struct Config {
    origin_endpoint: Vec<Endpoint>,
    data_dir: String,
}

impl Config {
    pub fn new(endpoints: Vec<Endpoint>) -> Self {
        Config {
            origin_endpoint: endpoints,
            data_dir: String::from("/tmp/ruft"),
        }
    }
}
