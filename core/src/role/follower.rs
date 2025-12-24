pub struct Follower {}

impl Follower {
    pub fn new() -> Self {
        Follower {}
    }

    pub fn on_pre_vote(&self) {}

    pub fn on_vote(&self) {}

    pub fn on_append_entry(&self) {}
}
