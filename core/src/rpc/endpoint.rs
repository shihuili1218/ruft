use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Endpoint {
    id: u8,
    host: String,
    port: u16,
}

impl Endpoint {
    pub fn new(id: u8, host: String, port: u16) -> Self {
        Endpoint { id, host, port }
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
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]:[", self.id)?;
        write!(f, "{}:{}", self.host, self.port)?;
        write!(f, "]")
    }
}
