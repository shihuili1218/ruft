use crate::command::Command;
use crate::endpoint::Endpoint;
use crate::response::Response;

pub struct Leader {
    endpoint: Endpoint,
    followers: Vec<Endpoint>,
    learners: Vec<Endpoint>,
}

impl Leader {
    pub fn new(endpoint: Endpoint, followers: Vec<Endpoint>, learners: Vec<Endpoint>) -> Self {
        Leader {
            endpoint,
            followers,
            learners,
        }
    }

    pub(crate) fn append_entry(&self, command: Command) -> Response {
        Response::Failure {
            message: String::new(),
        }
    }
}
