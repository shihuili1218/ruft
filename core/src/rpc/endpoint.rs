use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Endpoint {
    id: u8,
    host: String,
    port: u16,
    is_voting: bool,
}

impl Endpoint {
    pub fn new_voter(id: u8, host: String, port: u16) -> Self {
        Endpoint { id, host, port, is_voting: true }
    }

    pub fn id(&self) -> u8 {
        self.id
    }

    pub fn host(&self) -> &String {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    pub fn is_voting(&self) -> bool {
        self.is_voting
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]:[", self.id)?;
        write!(f, "{}:{}", self.host, self.port)?;
        write!(f, "]")
    }
}
