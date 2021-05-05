pub mod from_str;

use prost::Message;
use std::fmt::Display;

// Util function to convert error to string
pub fn to_string<T>(err: T) -> String
where
    T: Display,
{
    err.to_string()
}

pub fn prost_serialize<T: Message>(msg: &T) -> Result<Vec<u8>, prost::EncodeError> {
    let mut buf = Vec::new();
    msg.encode(&mut buf)?;

    Ok(buf)
}
