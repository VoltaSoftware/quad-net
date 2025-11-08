use crate::error::Error;
use crate::quad_socket::client::{IncomingSocketMessage, OutgoingSocketMessage};
use futures::{SinkExt, StreamExt};
use log::error;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use std::io::ErrorKind;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::runtime::Runtime;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, connect_async_tls_with_config, Connector};
use ureq::run;

pub struct WebSocket {
    _runtime: Runtime, // Need this here to keep it alive for the duration of the socket otherwise it kills all tasks
    rx: UnboundedReceiver<IncomingSocketMessage>,
    tx: UnboundedSender<OutgoingSocketMessage>,
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
        let (mut incoming_sock_msg_tx, mut incoming_sock_msg_rx) = unbounded_channel();
        let (mut outgoing_sock_msg_tx, mut outgoing_sock_msg_rx) = unbounded_channel();

        let addr = addr.into();
        // Make new tokio runbtime on new thread
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.spawn(async move {
            // Create a connector that disables certificate verification if requested
            let socket = if disable_cert_verification {
                // Create a TLS connector that disables certificate verification
                let tls_config = {
                    let config = rustls::ClientConfig::builder()
                        .dangerous()
                        .with_custom_certificate_verifier(Arc::new(NoCertificateVerification {}))
                        .with_no_client_auth();
                    Arc::new(config)
                };
                let connector = Connector::Rustls(tls_config);
                // Connect with the custom connector
                connect_async_tls_with_config(addr, None, true, Some(connector)).await
            } else {
                connect_async(addr).await
            };

            if let Err(e) = socket {
                incoming_sock_msg_tx
                    .send(IncomingSocketMessage::Error(Error::from(e)))
                    .expect("Socket tx closed");
                return;
            }

            let (mut websocket_out, response) = socket.unwrap();

            match websocket_out.get_mut() {
                tokio_tungstenite::MaybeTlsStream::Plain(stream) => {
                    //let _ = stream.set_nonblocking(true);
                    let _ = stream.set_nodelay(true);
                }
                tokio_tungstenite::MaybeTlsStream::Rustls(stream) => {
                    //let _ = stream.get_mut().0.set_nonblocking(true);
                    let _ = stream.get_mut().0.set_nodelay(true);
                }
                e => unimplemented!("Unsupported stream type {:?}", e),
            };

            let (write_half, read_half) = websocket_out.split();

            incoming_sock_msg_tx
                .send(IncomingSocketMessage::Connected)
                .unwrap();

            // Read half
            let incoming_sock_msg_tx_clone = incoming_sock_msg_tx.clone();
            tokio::spawn(async move {
                let mut read_half = read_half;
                while let Some(msg) = read_half.next().await {
                    match msg {
                        Ok(Message::Binary(data)) => {
                            let read_at = current_time_millis();
                            if let Err(err) = incoming_sock_msg_tx_clone
                                .send(IncomingSocketMessage::PacketReceived(data.into(), read_at))
                            {
                                error!("Failed to send incoming message: {:?}", err);
                                break;
                            }
                        }
                        Ok(_) => {}
                        Err(e) => {
                            let _ = incoming_sock_msg_tx_clone.send(IncomingSocketMessage::Error(
                                Error::from(std::io::Error::new(ErrorKind::Other, e)),
                            ));
                            break;
                        }
                    }
                }
            });

            // Write half
            let incoming_sock_msg_tx = incoming_sock_msg_tx.clone();
            tokio::spawn(async move {
                let mut write_half = write_half;
                while let Some(msg) = outgoing_sock_msg_rx.recv().await {
                    match msg {
                        OutgoingSocketMessage::Close => {
                            let _ = incoming_sock_msg_tx.send(IncomingSocketMessage::Closed);
                            let _ = write_half
                                .send(Message::Close(None))
                                .await
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));
                            break;
                        }
                        OutgoingSocketMessage::Send(data) => {
                            let result = write_half
                                .send(Message::Binary(data.into()))
                                .await
                                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

                            if let Err(e) = result {
                                let _ = incoming_sock_msg_tx
                                    .send(IncomingSocketMessage::Error(Error::from(e)));
                            }
                        }
                    }
                }
            });
        });

        WebSocket {
            _runtime: runtime,
            rx: incoming_sock_msg_rx,
            tx: outgoing_sock_msg_tx,
        }
    }
}

// Add this struct for rustls certificate verification disabling
#[derive(Debug)]
struct NoCertificateVerification;

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        // For this example, we will skip all verification
        // In a real application, you should implement proper certificate verification
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        // For TLS 1.2, we can skip the verification
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
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

pub fn current_time_millis() -> u64 {
    let duration_since_epoch = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    duration_since_epoch.as_millis() as u64
}
