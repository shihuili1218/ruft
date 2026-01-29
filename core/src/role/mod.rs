mod candidate;
mod follower;
mod leader;
mod learner;
mod replication;
mod state;

pub(crate) use crate::role::candidate::{Candidate, VoteResult};
pub(crate) use crate::role::follower::Follower;
pub(crate) use crate::role::leader::Leader;
pub(crate) use crate::role::learner::Learner;
pub(crate) use crate::role::state::{Common, Role};
