use std::{error::Error as StdError, fmt, io};
use warp::reject;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Minification(&'static str),
    Syntect(syntect::LoadingError),
    JsonSerialization(serde_json::error::Error),
    ThemeNotFound,
}

impl reject::Reject for Error {}

impl StdError for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            Io(err) => err.fmt(f),
            Minification(err) => write!(f, "{}", err),
            Syntect(err) => err.fmt(f),
            JsonSerialization(err) => err.fmt(f),
            ThemeNotFound => write!(f, "Theme not found"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<syntect::LoadingError> for Error {
    fn from(err: syntect::LoadingError) -> Error {
        Error::Syntect(err)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Error {
        Error::JsonSerialization(err)
    }
}
