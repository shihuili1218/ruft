use bytes::Bytes;
use core::command::CmdReq;
use core::node::{Config, Node};
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() {
    init_tracing();

    info!("Starting x");
    let config = Config::new(vec![]);
    let node = Node::new(config);
    node.start();
    let req = CmdReq {
        id: "cmd_789".to_string(),
        data: Bytes::from(b"network_data".to_vec()),
    };
    node.emit(req);
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("raft=info".parse().unwrap()))
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();
}
