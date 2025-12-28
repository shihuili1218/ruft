pub mod meta;
pub mod node;

use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;
use crate::node::node::Node;
use std::sync::Arc;

pub struct Ruft {
    inner: Arc<Node>,
}

impl Ruft {
    pub fn new(me: Endpoint, config: Config) -> Self {
        Ruft {
            inner: Arc::new(Node::new(me, config)),
        }
    }

    pub async fn start(&self) {
        self.inner.start().await;
    }

    pub async fn update_member(&self, endpoints: Vec<Endpoint>) {
        self.inner.update_member(endpoints);
    }

    pub async fn emit(&self, cmd: CmdReq) -> CmdResp {
        self.inner.emit(cmd).await
    }
}

impl Clone for Ruft {
    fn clone(&self) -> Self {
        Ruft { inner: self.inner.clone() }
    }
}

pub struct Config {
    origin_endpoint: Vec<Endpoint>,
    data_dir: String,
    heartbeat_interval_millis: u64,
}

impl Config {
    pub fn new(endpoints: Vec<Endpoint>) -> Self {
        Config {
            origin_endpoint: endpoints,
            data_dir: String::from("/tmp/ruft"),
            heartbeat_interval_millis: 3000,
        }
    }
}
