use serde::{Deserialize, Serialize};

/// generate the unified response data
#[derive(Debug, Serialize, Deserialize)]
pub struct Response<T> {
    pub code: usize,
    pub data: T,
    pub msg: String,
}

impl<T> Response<T>
where
    T: serde::ser::Serialize,
{
    pub fn new(code: usize, data: T, msg: &str) -> Self {
        Response {
            code,
            data,
            msg: msg.to_string(),
        }
    }
}
