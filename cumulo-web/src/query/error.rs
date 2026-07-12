#[derive(Debug)]
pub struct Error(String);

impl Error {
    pub fn unsupported<T>(what: &str) -> Result<T, Self> {
        Err(Error(format!("query format cannot serialize {what}")))
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl serde::de::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}

impl serde::ser::Error for Error {
    fn custom<T: std::fmt::Display>(msg: T) -> Self {
        Error(msg.to_string())
    }
}
