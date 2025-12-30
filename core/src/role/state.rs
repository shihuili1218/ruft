use crate::endpoint::Endpoint;
use std::collections::HashMap;
use std::fmt::Display;

#[derive(Clone)]
pub enum State {
    Electing {
        term: u64,
        votes_received: u64,
    },
    Leading {
        term: u64,
        next_index: HashMap<Endpoint, u64>,
        match_index: HashMap<Endpoint, u64>,
    },
    Following {
        term: u64,
        leader: Endpoint,
    },
    Learning {
        term: u64,
        leader: Endpoint,
    },
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Electing { .. } => write!(f, "Electing"),
            State::Leading { term, .. } => write!(f, "Leading[{}]", term),
            State::Following { term, .. } => write!(f, "Following[{}]", term),
            State::Learning { term, .. } => write!(f, "Learning[{}]", term),
        }
    }
}
