use std::fmt::Display;

use super::frame_decoder::{DecodedChunk, DisplayType, FieldType};

/// Protocols are used to identify the source or type of the captured frame
/// This i used to select the top-level frame decoder
#[derive(Debug)]
pub enum MACProtocol {
    ZWave,
    IEEE802_15_4,
    LoRa,
}

/// Represents a captured frame with associated metadata and decoded information
/// 
#[derive(Debug)]
pub struct MACFrame {
    pub id: u32,
    pub timestamp: u32,
    pub protocol: MACProtocol,
    pub data: Vec<u8>,
    pub top: DecodedChunk,
}

/// Display implementation for Protocol enum for user-friendly output
impl Display for MACProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MACProtocol::ZWave => write!(f, "Z-Wave"),
            MACProtocol::IEEE802_15_4 => write!(f, "IEEE 802.15.4"),
            MACProtocol::LoRa => write!(f, "LoRa"),
        }
    }
}

impl Display for MACFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Frame ID: {}, Timestamp: {}, Protocol: {}",
            self.id, self.timestamp, self.protocol
        )?;
        self.top.fmt(&self.data, 0, f)?;
        Ok(())
    }
}
