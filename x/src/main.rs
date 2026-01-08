use bytes::Bytes;
use core::command::CmdReq;
use core::endpoint::{Address, Endpoint};
use core::{Config, Ruft};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    init_tracing();
    info!("Starting Raft node example");

    // Create config using the new builder pattern
    let config = Config::builder().data_dir("/tmp/ruft/node0").heartbeat_interval(1000).build();

    let endpoint = Endpoint::new(0, Address::new("127.0.0.1".to_string(), 5000));

    match Ruft::new(endpoint, config) {
        Ok(ruft) => {
            info!("Raft node created successfully");

            if let Err(e) = ruft.start().await {
                eprintln!("Failed to start Raft node: {}", e);
                return;
            }

            info!("Raft node started, current state: {}", ruft.state().await);

            // Submit a test command
            let req = CmdReq {
                id: "cmd_1".to_string(),
                data: Bytes::from(b"test_data".to_vec()),
            };

            let resp = ruft.submit(req).await;
            info!("Command response: {:?}", resp);

            // Keep running
            tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
            info!("Shutting down...");
        }
        Err(e) => {
            eprintln!("Failed to create Raft node: {}", e);
        }
    }
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("core=info".parse().unwrap()).add_directive("x=info".parse().unwrap()))
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();
}
