use crate::command::{CmdReq, CmdResp};
use crate::config::Config;
use crate::node::state::State;
use std::path::PathBuf;
use crate::meta::Meta;

mod state;

struct Node {
    meta: Meta,
    state: State,
}

impl Node {
    pub fn start(config: Config) -> Self {
        Node {
            meta: Meta::new(PathBuf::new()),
            state: State::Electing,
        }
    }

    pub fn emit(&self, command: CmdReq) -> CmdResp {
        match &self.state {
            State::Electing => CmdResp::Failure {
                message: String::from("Electing"),
            },
            State::Leading { term, leader } => leader.append_entry(command),
            State::Following { term, follower } => CmdResp::Failure {
                message: format!("Following, leader[{}]: {}", term, follower.leader),
            },
            State::Learning { term, learner } => CmdResp::Failure {
                message: format!("Learning, leader[{}]: {}", term, learner.leader),
            },
        }
    }
}
