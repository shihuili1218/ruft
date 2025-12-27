use std::fmt::{Display, Formatter};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Endpoint {
    id: usize,
    address: Address,
    pub url: String,
}

impl Endpoint {
    pub fn new(id: usize, address: Address) -> Self {
        let url = format!("http://{}:{}", address.host, address.port);
        Endpoint {
            id,
            address,
            url,
        }
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]:[", self.id)?;
        write!(f, "{}:{}", self.address.host, self.address.port)?;
        write!(f, "]")
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Address {
    host: String,
    port: u16,
}

impl Address {
    pub fn new(host: String, port: u16) -> Self {
        Address { host, port }
    }
}
