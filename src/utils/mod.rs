pub mod from_str;

use std::error::Error;
use std::fmt::Display;
use rand::{Rng, thread_rng};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

// Util function to convert error to string
pub fn to_string<T>(err: T) -> String where T: Display {
    err.to_string()
}

pub fn create_client_id() -> String {
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    const ID_LEN: usize = 10;
    let mut rng =  thread_rng();
    (0..ID_LEN)
        .map(|_| {
            let idx = rng.gen_range(0, CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
