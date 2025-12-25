use crate::command::{CmdReq, CmdResp};
use crate::endpoint::Endpoint;
use crate::meta::Meta;
use crate::node::Config;
use crate::role::state::State;
use crate::rpc::server::run_server;
use std::path::PathBuf;
use std::sync::Arc;

pub struct Ruft {
    meta: Meta,
    pub state: State,
}

impl Ruft {
    pub fn new(config: Config) -> Self {
        Ruft {
            meta: Meta::new(PathBuf::new()),
            state: State::Electing,
        }
    }

    pub fn start_rpc_server(
        self: Arc<Self>,
    ) -> tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>> {
        tokio::spawn(async move { run_server(self).await })
    }

    pub fn update_member(&self,endpoints: Vec<Endpoint>) {}

    pub fn emit(&self, cmd: CmdReq) -> CmdResp {
        match &self.state {
            State::Electing => CmdResp::Failure {
                message: String::from("Electing"),
            },
            State::Leading { term, leader } => leader.append_entry(cmd),
            State::Following { term, follower } => CmdResp::Failure {
                message: format!("Following, leader[{}]: {}", term, follower.leader),
            },
            State::Learning { term, learner } => CmdResp::Failure {
                message: format!("Learning, leader[{}]: {}", term, learner.leader),
            },
        }
    }
}
