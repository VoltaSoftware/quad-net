use crate::error::Error;
use crate::quad_socket::client::{IncomingSocketMessage, OutgoingSocketMessage};
use log::error;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use std::io::ErrorKind;
use std::net::TcpStream;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;
use tungstenite::HandshakeError::{Failure, Interrupted};
use tungstenite::{connect, Bytes, Connector, Message};

pub struct WebSocket {
    rx: Receiver<IncomingSocketMessage>,
    tx: Sender<OutgoingSocketMessage>,
}

impl WebSocket {
    pub fn send(&mut self, data: &[u8]) {
        let _ = self.tx.send(OutgoingSocketMessage::Send(data.to_vec()));
    }

    pub fn try_recv(&mut self) -> Option<IncomingSocketMessage> {
        self.rx.try_recv().ok()
    }

    pub fn close(&mut self) {
        let _ = self.tx.send(OutgoingSocketMessage::Close);
    }
}

impl WebSocket {
    pub fn connect(addr: impl Into<String>, disable_cert_verification: bool) -> WebSocket {
        let (incoming_sock_msg_tx, incoming_sock_msg_rx) = std::sync::mpsc::channel();
        let (outgoing_sock_msg_tx, outgoing_sock_msg_rx) = std::sync::mpsc::channel();

        let addr = addr.into();
        std::thread::spawn(move || {
            // Create a connector that disables certificate verification if requested
            let socket = if disable_cert_verification {
                // Create a connector with certificate verification disabled
                let config = rustls::ClientConfig::builder()
                    .dangerous()
                    .with_custom_certificate_verifier(Arc::new(NoCertificateVerification {}))
                    .with_no_client_auth();

                let connector = Connector::Rustls(Arc::new(config));

                let ip_port = addr
                    .strip_prefix("wss://")
                    .expect("Address must start with 'wss://'");

                match TcpStream::connect(ip_port) {
                    Ok(stream) => {
                        let client = tungstenite::client_tls_with_config(
                            addr,
                            stream,
                            None,
                            Some(connector),
                        );

                        match client {
                            Ok(client) => Ok(client),
                            Err(e) => match e {
                                Interrupted(_) => Err(tungstenite::Error::Io(std::io::Error::new(
                                    ErrorKind::ConnectionAborted,
                                    "TlsHandshake interrupted: ".to_string(),
                                ))),
                                Failure(error) => Err(error),
                            },
                        }
                    }
                    Err(e) => Err(tungstenite::Error::Io(e)),
                }
            } else {
                // Use standard connect with certificate verification
                connect(addr)
            };

            if let Err(e) = socket {
                incoming_sock_msg_tx
                    .send(IncomingSocketMessage::Error(Error::from(e)))
                    .expect("Socket tx closed");
                return;
            }

            let (mut websocket_out, response) = socket.unwrap();

            match websocket_out.get_mut() {
                tungstenite::stream::MaybeTlsStream::Plain(stream) => stream.set_nonblocking(true),
                tungstenite::stream::MaybeTlsStream::Rustls(stream) => {
                    stream.get_mut().set_nonblocking(true)
                }
                e => unimplemented!("Unsupported stream type {:?}", e),
            }
            .expect("Failed to set nonblocking");

            incoming_sock_msg_tx
                .send(IncomingSocketMessage::Connected)
                .unwrap();

            'outer: loop {
                while let Some(msg) = outgoing_sock_msg_rx.try_recv().ok() {
                    match msg {
                        OutgoingSocketMessage::Close => {
                            let _ = incoming_sock_msg_tx.send(IncomingSocketMessage::Closed);

                            let _ = websocket_out
                                .close(None)
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
                            break 'outer;
                        }
                        OutgoingSocketMessage::Send(data) => {
                            let result = websocket_out
                                .send(Message::Binary(data.into()))
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

                            if let Err(e) = result {
                                let _ = incoming_sock_msg_tx
                                    .send(IncomingSocketMessage::Error(Error::from(e)));
                            }
                        }
                    }
                }

                if let Ok(msg) = websocket_out.read() {
                    if let Message::Binary(data) = msg {
                        if let Err(err) = incoming_sock_msg_tx
                            .send(IncomingSocketMessage::PacketReceived(data.into()))
                        {
                            error!("Failed to send incoming message: {:?}", err);
                            break;
                        }
                    }
                } else {
                    let fps = 30;
                    let frame_time = Duration::from_millis(1000 / fps);
                    std::thread::sleep(frame_time);
                }

                if !websocket_out.can_read() || !websocket_out.can_write() {
                    let _ = incoming_sock_msg_tx.send(IncomingSocketMessage::Closed);
                    break 'outer;
                }
            }
        });

        //socket.get_mut().set_nodelay(true).map_err(|e| Error::from(e))?;

        WebSocket {
            rx: incoming_sock_msg_rx,
            tx: outgoing_sock_msg_tx,
        }
    }
}

// Add this struct for rustls certificate verification disabling
#[derive(Debug)]
struct NoCertificateVerification;

impl rustls::client::danger::ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // For this example, we will skip all verification
        // In a real application, you should implement proper certificate verification
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // For TLS 1.2, we can skip the verification
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // For TLS 1.3, we can skip the verification
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
        ]
    }
}
