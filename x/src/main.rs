fn main() {
    println!("Starting x");
}

use tracing_subscriber::EnvFilter;

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("raft=info".parse().unwrap()))
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();
}
