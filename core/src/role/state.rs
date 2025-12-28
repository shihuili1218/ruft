use crate::role::candidate::Candidate;
use crate::role::follower::Follower;
use crate::role::leader::Leader;
use crate::role::learner::Learner;
use std::fmt::Display;

pub enum State {
    Electing { candidate: Candidate },
    Leading { term: usize, leader: Leader },
    Following { term: usize, follower: Follower },
    Learning { term: usize, learner: Learner },
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
