use std::error;
use std::fmt;

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
