use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub request1: String,
    pub request2: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub response1: String,
    pub response2: String,
}