use crate::command::Command;
use crate::config::Config;
use crate::node::meta::Meta;
use crate::node::state::State;
use crate::response::Response;
use std::path::PathBuf;

mod meta;
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

    pub fn emit(&self, command: Command) -> Response {
        match &self.state {
            State::Electing => Response::Failure {
                message: String::from("Electing"),
            },
            State::Leading { term, leader } => leader.append_entry(command),
            State::Following { term, follower } => Response::Failure {
                message: format!("Following, leader[{}]: {}", term, follower.leader),
            },
            State::Learning { term, learner } => Response::Failure {
                message: format!("Learning, leader[{}]: {}", term, learner.leader),
            },
        }
    }
}
