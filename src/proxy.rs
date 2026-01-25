use std::str::FromStr;
use std::fmt;

use tokio::{
    io::{
        AsyncWriteExt,
        AsyncReadExt,
    },
    net::{
        TcpListener,
        TcpStream,
    },
    sync::broadcast::{
        Sender,
        Receiver,
        self,
    }
};

use crate::types::Frame;
use crate::client_handler::handle_client;

#[derive(Debug, Clone)]
pub enum ProxyProtocol {
    Pti,
    ZniffRs,
}

impl fmt::Display for ProxyProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProxyProtocol::Pti => "pti",
            ProxyProtocol::ZniffRs => "zniff-rs",
        };
        f.write_str(s)
    }
}

impl FromStr for ProxyProtocol {
    type Err = ParseProxyProtocolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "pti" => Ok(ProxyProtocol::Pti),
            "zniff-rs" => Ok(ProxyProtocol::ZniffRs),
            _ => Err(ParseProxyProtocolError),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseProxyProtocolError;

impl fmt::Display for ParseProxyProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid proxy protocol")
    }
}

impl std::error::Error for ParseProxyProtocolError {}


pub struct Proxy {
    address: String,
    protocol: ProxyProtocol,
    tx: broadcast::Sender<Frame>,
}

impl Proxy {
    pub fn new(address: String, protocol: ProxyProtocol) -> Self {
        let (tx, _rx) = broadcast::channel::<Frame>(100);
        Proxy { address, protocol, tx }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let tx_clone = self.tx.clone();
/*
        // Spawn the task that receives from the server and sends frames to the client.
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
            loop {
                interval.tick().await;
                let frame = Frame::new(); // Placeholder for actual frame creation
                if let Err(e) = tx_clone.send(frame) {
                    eprintln!("Failed to send frame: {}", e);
                }
            }
        });
 */
        // PC Zniffer PTI default port is 4905
        // TODO: Make this configurable
        let listener = TcpListener::bind("0.0.0.0:4905").await.unwrap();
        println!("Server listening for {:?} clients on port 4905", self.protocol);

        loop {
            let (stream, addr) = listener.accept().await.unwrap();
            println!("Client connected: {addr}");

            let mut rx = self.tx.subscribe();

            tokio::spawn(async move {
                handle_client(stream, &mut rx).await;
            });
        }

        // Add proxy logic here
        Ok(())
    }
}
