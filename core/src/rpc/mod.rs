mod client;
pub mod server;

// Fix name conflict between our crate `core` and std's `core`
#[allow(unused_extern_crates)]
extern crate std;

tonic::include_proto!("ruft");
