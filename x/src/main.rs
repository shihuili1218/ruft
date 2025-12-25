use bytes::Bytes;
use core::command::CmdReq;
use core::node::Config;
use core::node::Ruft;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() {
    init_tracing();
    info!("Starting x");

    let config = Config::new(vec![]);
    let ruft = Ruft::new(config);
    ruft.start();
    let req = CmdReq {
        id: "cmd_789".to_string(),
        data: Bytes::from(b"network_data".to_vec()),
    };
    ruft.emit(req);
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("raft=info".parse().unwrap()))
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();
}
