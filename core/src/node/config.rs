use crate::endpoint::Endpoint;

pub struct Config {
    endpoints: Vec<Endpoint>,
}

impl Config {
    pub fn new(endpoints: Vec<Endpoint>) -> Self {
        Config{
            endpoints
        }
    }
}
