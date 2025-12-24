use crate::command::Command;
use crate::response::Response;

pub struct Leader {}

impl Leader {
    pub fn new() -> Self {
        Leader {}
    }

    pub(crate) fn append_entry(&self, command: Command) -> Response {
        Response::Failure {
            message: String::new(),
        }
    }
}
