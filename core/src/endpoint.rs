use std::fmt::{Display, Formatter};
use std::net::{Ipv4Addr, Ipv6Addr};

pub struct Endpoint {
    id: usize,
    address: Address,
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}]:[", self.id)?;

        match &self.address {
            Address::Ipv4 { v4 } => write!(f, "{}", v4)?,
            Address::Ipv6 { v6 } => write!(f, "{}", v6)?,
            Address::Connect { host, port } => write!(f, "{}:{}", host, port)?,
        }

        write!(f, "]")
    }
}

enum Address {
    Ipv4 { v4: Ipv4Addr },
    Ipv6 { v6: Ipv6Addr },
    Connect { host: String, port: u16 },
}
