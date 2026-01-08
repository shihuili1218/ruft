use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Endpoint {
    id: u8,
    address: Address,
}

impl Endpoint {
    pub fn new(id: u8, address: Address) -> Self {
        Endpoint { id, address }
    }

    pub fn id(&self) -> u8 {
        self.id
    }

    pub fn address(&self) -> &Address {
        &self.address
    }

    pub fn url(&self) -> String {
        format!("http://{}:{}", self.address.host, self.address.port)
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]:[", self.id)?;
        write!(f, "{}:{}", self.address.host, self.address.port)?;
        write!(f, "]")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address {
    host: String,
    port: u16,
}

impl Address {
    pub fn new(host: String, port: u16) -> Self {
        Address { host, port }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
