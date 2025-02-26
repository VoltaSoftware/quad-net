#[derive(Debug)]
pub enum Error {
    TungsteniteError(tungstenite::error::Error),
    IOError(std::io::Error),
}

impl From<tungstenite::error::Error> for Error {
    fn from(error: tungstenite::error::Error) -> Error {
        Error::TungsteniteError(error)
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Error {
        Error::IOError(error)
    }
}

impl From<Error> for std::io::Error {
    fn from(error: Error) -> std::io::Error {
        match error {
            Error::IOError(error) => error,
            Error::TungsteniteError(e) => std::io::Error::new(std::io::ErrorKind::Other, e),
        }
    }
}
