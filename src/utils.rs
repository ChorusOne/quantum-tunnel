use std::error::Error;
use std::fmt::Display;

// Util function to convert error to string
pub fn to_string<T>(err: T) -> String where T: Display {
    err.to_string()
}