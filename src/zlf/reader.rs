// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use std::io::{
    self,
    Read,
    Seek,
};
use serde::{
    Deserialize,
    Serialize,
};
use thiserror::Error;
use crate::zlf::types::{
    ApiType,
    ZLF_VERSION,
};

/// Zniffer frame kinds (from Silicon Labs Zniffer API docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FrameType {
    Command = 0x00,   // CMD_FRAME
    Data = 0x01,      // DATA_FRAME
    Beam = 0x02,      // BEAM_FRAME
    BeamStart = 0x04, // BEAM_START
    BeamStop = 0x05,  // BEAM_STOP
}

impl TryFrom<u8> for FrameType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(FrameType::Command),
            0x01 => Ok(FrameType::Data),
            0x02 => Ok(FrameType::Beam),
            0x04 => Ok(FrameType::BeamStart),
            0x05 => Ok(FrameType::BeamStop),
            _ => Err(()),
        }
    }
}

/// Raw frame as read from ZLF after header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFrame {
    pub timestamp: u64, // file timestamp
    pub sof: u8,         // SOF '#' or SODF '!'
    pub frame_type: FrameType,  // parsed type
    pub payload: Vec<u8> // raw payload bytes
}

/// Decoded DATA_FRAME fields (payload layout mirrors device->host Zniffer API).
/// payload := [ts_lo, ts_hi, ch_speed, region, rssi (i8), mpdu_len, mpdu...]
#[derive(Debug, Clone)]
pub struct DataFrame {
    pub timestamp: u16,       // wraps; device ticks
    pub ch_and_speed: u8,     // channel & bitrate
    pub region: u8,           // region code
    pub rssi: i8,             // signed RSSI
    pub mpdu: Vec<u8>,        // Z-Wave MPDU bytes
}

/// Either a decoded DATA_FRAME or a raw frame for other types.
#[derive(Debug, Clone)]
pub enum ZlfRecord {
    Data(DataFrame),
    Other(RawFrame),
}

#[derive(Error, Debug)]
pub enum ZlfError {
    #[error("Invalid ZLF version: {0}")]
    InvalidZlfVersion(u32),
    #[error("Invalid start pattern")]
    InvalidStartPattern,
    #[error("Invalid properties field: {0}")]
    InvalidPropertiesField(u8),
    #[error("Invalid API type field: {0}")]
    InvalidApiTypeField(u8),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("unexpected EOF while parsing a frame")]
    Eof,
    #[error("invalid frame marker: {0:#04x}")]
    BadMarker(u8),
    #[error("payload too short for data frame")]
    ShortDataPayload,
}

pub struct ZlfReader<R: Read + Seek> {
    r: R,
    // If you discover version markers in the 2048-byte header, store them here
    // to adjust parsing per version.
    frame_counter: usize,
}

impl<R: Read + Seek> ZlfReader<R> {
    pub fn new(mut r: R) -> Result<Self, ZlfError> {
        // Read the 2048-byte header into a buffer.
        let mut header: [u8; 2048] = [0u8; 2048];
        r.read_exact(&mut header)?;

        // Check header checksum
        let file_checksum = u16::from_le_bytes([header[2046], header[2047]]);
        use crc16::*;
        if State::<AUG_CCITT>::calculate(&header[..2046]) != file_checksum {
            return Err(ZlfError::InvalidStartPattern);
        }

        // Check for ZLF version at index 0.
        let version: u32 = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        if version != ZLF_VERSION {
            return Err(ZlfError::InvalidZlfVersion(version));
        }

        // Read 4 bytes / 32 bit of text encoding at index 4.
        let _encoding: u32 = u32::from_le_bytes([header[4], header[5], header[6], header[7]]);

        // TODO: Read comment at index 8. 512 bytes.

        Ok(Self { r, frame_counter: 0 })
    }

    pub fn frame_count(&self) -> usize {
        self.frame_counter
    }

    /// Read the next frame. Returns Ok(None) at EOF.
    pub fn next(&mut self) -> Result<Option<ZlfRecord>, ZlfError> {
        // Read a timestamp of 8 bytes
        let mut timestamp = [0u8; 8];
        match self.r.read_exact(&mut timestamp) {
            Ok(()) => {},
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        }

        let mut properties = [0u8; 1];
        self.r.read_exact(&mut properties)?;
//        if properties[0] != 0 && properties[0] != 0x81 {
//            return Err(ZlfError::InvalidPropertiesField(properties[0]));
//        }

        let mut payload_length = [0u8; 4];
        self.r.read_exact(&mut payload_length)?;
        let payload_length: u32 = u32::from_le_bytes(payload_length);
        println!("Payload length: {:?}", payload_length);

        let mut payload: Vec<u8> = vec![0u8; payload_length as usize];
        let mut read = 0usize;
        while read < payload_length as usize {
            let n = self.r.read(&mut payload[read..])?;
            if n == 0 {
                return Err(ZlfError::Eof);
            }
            read += n;
        }

        for byte in &payload {
            print!("{:02X} ", byte);
        }
        println!();

        let mut api_type = [0u8; 1];
        self.r.read_exact(&mut api_type)?;
        match ApiType::from(api_type[0]) {
            ApiType::Unknown(_) => { return Err(ZlfError::InvalidApiTypeField(api_type[0])); },
            _ => { },
        }

        // TODO: Do we need the frame type?
        let frame_type = FrameType::Data;

        self.frame_counter += 1;

        Ok(Some(ZlfRecord::Other(RawFrame { timestamp: 0, sof: 0, frame_type, payload })))
    }
}
