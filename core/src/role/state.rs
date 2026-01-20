
/// Marker trait for valid Raft node states.
/// This enables the typestate pattern: RaftNode<S: RaftState>
pub trait Role: Sized {
    fn term(&self) -> u64;
    fn state_name() -> &'static str;
}
