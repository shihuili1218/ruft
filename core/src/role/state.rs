use crate::role::follower::Follower;
use crate::role::leader::Leader;
use crate::role::learner::Learner;

pub enum State {
    Electing,
    Leading { term: usize, leader: Leader },
    Following { term: usize, follower: Follower },
    Learning { term: usize, learner: Learner },
}
