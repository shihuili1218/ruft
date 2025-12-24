use crate::endpoint::Endpoint;

pub enum State {
    Electing {
        nodes: Vec<Endpoint>,
    },
    Leading {
        term: usize,
        followers: Vec<Endpoint>,
        learners: Vec<Endpoint>,
    },
    Following {
        term: usize,
        leader: Endpoint,
    },
    Learning {
        term: usize,
        leader: Endpoint,
    },
}
