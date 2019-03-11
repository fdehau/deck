use std::error::Error as StdError;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Minification(&'static str),
    SyntaxHightlighting(syntect::LoadingError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            Io(err) => err.fmt(f),
            Minification(err) => write!(f, "{}", err),
            SyntaxHightlighting(err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        match self {
            Error::Io(err) => err.description(),
            Error::Minification(err) => err,
            Error::SyntaxHightlighting(err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&StdError> {
        None
    }
}
