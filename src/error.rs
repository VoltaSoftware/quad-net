#[derive(Debug)]
pub enum Error {
    #[cfg(not(target_arch = "wasm32"))]
    TungsteniteError(tungstenite::error::Error),
    IOError(std::io::Error),
}

#[cfg(not(target_arch = "wasm32"))]
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
            #[cfg(not(target_arch = "wasm32"))]
            Error::TungsteniteError(e) => std::io::Error::new(std::io::ErrorKind::Other, e),
        }
    }
}
