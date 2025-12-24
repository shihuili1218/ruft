use crate::endpoint::Endpoint;

pub struct Learner {
    endpoint: Endpoint,
    pub leader: Endpoint,
}

impl Learner {
    pub fn new(endpoint: Endpoint, leader: Endpoint) -> Self {
        Learner { endpoint, leader }
    }
}
