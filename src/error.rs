use std::error::Error as StdError;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Minification(&'static str),
    Syntect(syntect::LoadingError),
    ThemeNotFound,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            Io(err) => err.fmt(f),
            Minification(err) => write!(f, "{}", err),
            Syntect(err) => err.fmt(f),
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

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::Io(err) => err.description(),
            Error::Minification(err) => err,
            Error::Syntect(err) => err.description(),
            Error::ThemeNotFound => "Theme not found",
        }
    }

    fn cause(&self) -> Option<&StdError> {
        None
    }
}
