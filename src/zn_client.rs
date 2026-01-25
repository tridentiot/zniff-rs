/**
 * This module defines the client functionality for connecting to a device,
 * generator, server, or proxy using the specified protocol.
 *
 * It includes the `Client` struct and its associated methods for establishing
 * connections and handling communication.
 */
use crate::types::Frame;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use std::net::Ipv4Addr;

#[derive(Debug, Clone)]
enum ZnProtocol {
    Pti,
    ZniffRs,
}

impl From<&str> for ZnProtocol {
    fn from(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "pti" => ZnProtocol::Pti,
            "zniff-rs" => ZnProtocol::ZniffRs,
            _ => panic!("Unknown protocol"),
        }
    }
}

struct Endpoint {
    ip: Ipv4Addr,
    port: u16,
    protocol: ZnProtocol,
}

impl TryFrom<&str> for Endpoint {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let parts = value.split(',').collect::<Vec<&str>>();
        let mut ip: Option<Ipv4Addr> = None;
        let mut port: Option<u16> = None;
        let mut protocol: Option<ZnProtocol> = None;

        if parts.len() > 3 {
            return Err(());
        }

        for part in parts.iter() {
            let (k, v) = match part.split_once('=') {
                Some(kv) => kv,
                None => return Err(()),
            };
            //println!("Key: {}, Value: {}", k, v);
            match k.to_ascii_lowercase().as_str() {
                "ip" => {
                    ip = match v.parse() {
                        Ok(addr) => Some(addr),
                        Err(_) => return Err(()),
                    };
                }
                "port" => {
                    port = match v.parse() {
                        Ok(p) => Some(p),
                        Err(_) => return Err(()),
                    };
                }
                "protocol" => {
                    protocol = match v.to_ascii_lowercase().as_str() {
                        "pti" => Some(ZnProtocol::Pti),
                        "zniff-rs" => Some(ZnProtocol::ZniffRs),
                        _ => return Err(()),
                    };
                }
                _ => return Err(()),
            }

            if port.is_none() {
                port = Some(4905); // Default port
            }
            if protocol.is_none() {
                protocol = Some(ZnProtocol::Pti); // Default protocol
            }
        }
        Ok(Endpoint { ip: ip.ok_or(())?, port: port.ok_or(())?, protocol: protocol.ok_or(())? })
    }

}

#[derive(Debug)]
pub enum ZnClientError {
    InvalidEndpointFormat,
    ConnectionFailed,
}

pub struct ZnClient {
    serial: Vec<String>,
    endpoints: Vec<Endpoint>,
    tx: broadcast::Sender<Frame>,
}

impl ZnClient {
    pub fn try_new(serial: &Vec<String>, endpoints: &Vec<String>) -> Result<Self, ZnClientError> {
        for endpoint in endpoints {
            match Endpoint::try_from(endpoint.as_str()) {
                Ok(_ep) => {
                    //println!("Parsed endpoint: ip={}, port={}, protocol={:?}", ep.ip, ep.port, ep.protocol);
                },
                Err(_) => {
                    return Err(ZnClientError::InvalidEndpointFormat);
                }
            }
        }

        let (tx, _rx) = broadcast::channel(100);
        Ok(ZnClient {
            serial: serial.clone(),
            endpoints: endpoints.iter().filter_map(|e| Endpoint::try_from(e.as_str()).ok()).collect(),
            tx,
        })
    }

    async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        /*
        let addr = format!("{}:9000", self.address);
        let mut stream = TcpStream::connect(&addr).await?;
        println!("Connected to {addr}");
 */
        // Further implementation for handling communication goes here

        Ok(())
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("Starting ZnClient with the following endpoints:");
        for endpoint in &self.endpoints {
            let tx_clone = self.tx.clone();
            tokio::spawn(async move {

            });
            println!("Connecting to {}:{} using {:?}", endpoint.ip, endpoint.port, endpoint.protocol);
        }
        self.connect().await
    }
}
