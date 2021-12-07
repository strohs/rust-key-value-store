use serde::{Deserialize, Serialize};

/// These are the request "commands" that can be made to a key/value store
#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    /// get a value from the store
    Get {
        /// the key to search for
        key: String
    },
    /// set a key/value in the store
    Set {
        /// the key to set
        key: String,
        /// the value to set
        value: String
    },
    /// remove a key/value from the store
    Remove {
        /// the key to remove
        key: String
    },
}

/// The response Types that can be returned for any KVS Request
#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    /// this variant is returned when a request was successful
    Ok(Option<String>),
    /// this variant is returned if an Error occurs while processing the request
    Err(String),
}

// /// The Response type for a GET request
// #[derive(Debug, Serialize, Deserialize)]
// pub enum GetResponse {
//     Ok(Option<String>),
//     Err(String),
// }
//
// /// The Response type for a SET request
// #[derive(Debug, Serialize, Deserialize)]
// pub enum SetResponse {
//     Ok(()),
//     Err(String),
// }
//
// /// The Response type for a REMOVE response
// #[derive(Debug, Serialize, Deserialize)]
// pub enum RemoveResponse {
//     Ok(()),
//     Err(String),
// }

