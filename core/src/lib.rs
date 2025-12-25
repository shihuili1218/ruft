pub mod command;
mod endpoint;
mod meta;
pub mod node;
mod repeat_timer;
mod role;
mod rpc;

pub use node::ruft::Ruft;
pub use node::Config;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
