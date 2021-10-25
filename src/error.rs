use std::error;
use std::fmt;
use redis::RedisError;
use serde_json;
use std::io;

#[derive(Debug)]
pub struct Error {
    description: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl Error {
    pub fn new<S: ToString>(description: S) -> Self {
        Self {
            description: description.to_string()
        }
    }

    pub fn from<E: error::Error>(error: E) -> Self {
        Self {
            description: format!("{}", error)
        }
    }
}

impl From<RedisError> for Error {
    fn from(err: RedisError) -> Self {
        Self {
            description: format!("Redis error: {}", err)
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self {
            description: format!("Serde error: {}", err)
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self {
            description: format!("IO error: {}", err)
        }
    }
}
