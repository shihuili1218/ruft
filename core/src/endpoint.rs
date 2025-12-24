use std::net::{Ipv4Addr, Ipv6Addr};

pub struct Endpoint {
    id: usize,
     address: Address,
}

impl Endpoint {
    pub fn fmt(&self) ->String{
        let addr = match &self.address {
            Address::Ipv4 { v4 } => String::from( v4.to_string()),
            Address::Ipv6 { v6 } => String::from( v6.to_string()),
            Address::Connect { host, port } => format!("{}:{}", host, port)
        };
        format!("[{}]:[{}]", self.id, addr)
    }
}

enum Address {
    Ipv4 { v4: Ipv4Addr },
    Ipv6 { v6: Ipv6Addr },
    Connect { host: String, port: u16 },
}
