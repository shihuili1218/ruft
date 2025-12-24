use crate::endpoint::Endpoint;

pub struct Follower {
    endpoint: Endpoint,
    pub leader: Endpoint,
}

impl Follower {
    pub fn new(endpoint: Endpoint, leader: Endpoint) -> Self {
        Follower { endpoint, leader }
    }

    pub fn on_pre_vote(&self) {}

    pub fn on_vote(&self) {}

    pub fn on_append_entry(&self) {}
}
