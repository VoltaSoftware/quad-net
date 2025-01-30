#[derive(Debug)]
pub enum Error {
    IOError(std::io::Error),
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
        }
    }
}