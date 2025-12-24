use crate::command::Command;
use crate::node::meta::Meta;
use crate::node::state::State;
use crate::role::follower::Follower;
use crate::role::leader::Leader;
use crate::role::learner::Learner;
use std::path::PathBuf;
use crate::response::Response;

struct Node {
    meta: Meta,
    state: State,
    leader: Leader,
    follower: Follower,
    learner: Learner,
}

impl Node {
    pub fn new(config_path: String) -> Self {
        Node {
            meta: Meta::new(PathBuf::new()),
            state: State::Electing { nodes: Vec::new() },
            leader: Leader::new(),
            follower: Follower::new(),
            learner: Learner::new(),
        }
    }

    pub fn emit(&self, command: Command) -> Response {
        match &self.state {
            State::Electing { .. } => Response::Failure { message: String::from("Electing") },
            State::Leading { .. } => self.leader.append_entry(command),
            State::Following { term, leader } => Response::Failure { message: format!("Following, leader[{}]: {}", term, leader.fmt()) },
            State::Learning { term, leader } => Response::Failure { message: format!("Learning, leader[{}]: {}", term, leader.fmt()) }
        }
    }
}

