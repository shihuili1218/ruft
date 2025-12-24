pub enum Response {
    Success,
    Failure { message: String },
    Unknown { log_id: usize },
}
