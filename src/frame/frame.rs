use std::fmt::Display;

use super::decoder::{DecodedChunk, DisplayType, FieldType};

#[derive(Debug)]
pub enum Protocol {
    ZWave,
}

#[derive(Debug)]
pub struct Frame {
    pub id: u32,
    pub timestamp: u32,
    pub protocol: Protocol,
    pub data: Vec<u8>,
    pub top: DecodedChunk,
}

impl Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Protocol::ZWave => write!(f, "Z-Wave"),
        }
    }
}

impl Display for Frame {
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
