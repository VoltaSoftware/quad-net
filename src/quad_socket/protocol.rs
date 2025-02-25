use log::{debug, error};
use std::io::ErrorKind;

#[derive(Debug)]
pub enum MessageReader {
    Empty,
    Amount(usize),
}

impl MessageReader {
    pub fn new() -> MessageReader {
        MessageReader::Empty
    }

    pub fn next(&mut self, mut stream: impl std::io::Read) -> Result<Option<Vec<u8>>, ()> {
        match self {
            MessageReader::Empty => {
                let mut size = [0u8; 4];
                match stream.read_exact(&mut size) {
                    Ok(_) => {
                        //debug!("We found a length of {:?} {:?}", u32::from_be_bytes(size) as usize, size);
                        *self = MessageReader::Amount(u32::from_be_bytes(size) as usize);
                        Ok(None)
                    }
                    Err(err) if err.kind() == ErrorKind::WouldBlock => {
                        //debug!("Would block but didnt");
                        Ok(None)
                    }
                    Err(err) => {
                        //error!("Error reading empty message: {:?}", err);
                        Err(())
                    }
                }
            }
            MessageReader::Amount(len) => {
                //debug!("Reading message of length {}", len);
                if *len == u32::MAX as usize {
                    error!("Message too large to read {:?}", len);
                    return Err(());
                }
                let mut buf = vec![0u8; *len];
                match stream.read_exact(&mut buf) {
                    Ok(_) => {
                        //debug!("Finished Read message with length={len}");
                        *self = MessageReader::Empty;
                        Ok(Some(buf))
                    }
                    Err(err) if err.kind() == ErrorKind::WouldBlock => {
                        //debug!("Would block but didnt with length={len}");
                        Ok(None)
                    }
                    Err(err) => {
                        //error!("Error reading message with length={len}: {:?}", err);
                        Err(())
                    }
                }
            }
        }
    }
}
