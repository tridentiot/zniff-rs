use std::io::{self, Read, Seek, SeekFrom};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::zlf::types::{
    ApiType,
    ZLF_VERSION,
};

/// Zniffer frame kinds (from Silicon Labs Zniffer API docs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FrameType {
    Cmd = 0x00,       // CMD_FRAME
    Data = 0x01,      // DATA_FRAME
    Beam = 0x02,      // BEAM_FRAME
    BeamStart = 0x04, // BEAM_START
    BeamStop = 0x05,  // BEAM_STOP
    Unknown(u8),
}

/// Raw frame as read from ZLF after header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawFrame {
    pub sof: u8,         // SOF '#' or SODF '!'
    pub typ: FrameType,  // parsed type
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
    InvalidZlfVersion(u8),
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
}

impl<R: Read + Seek> ZlfReader<R> {
    /// Construct reader and skip the static 2048-byte header.
    pub fn new(mut r: R) -> Result<Self, ZlfError> {
        // Community reverse engineering confirms a 2048-byte header. [1](https://github.com/knutkj/battery-service/blob/main/README.md)

        // Check for ZLF version at index 0.
        // TODO: Is 32 bit.
        let mut zlf_version = [0u8; 1];
        r.read_exact(&mut zlf_version)?;
        if zlf_version[0] != ZLF_VERSION {
            return Err(ZlfError::InvalidZlfVersion(zlf_version[0]));
        }

        // TODO: Read 4 bytes / 32 bit of text encoding at index 4.
        let mut encoding = [0u8; 4];
        r.read_exact(&mut encoding)?;

        // TODO: Read comment at index 8. 512 bytes.

        // Jump to start pattern
        r.seek(SeekFrom::Current(2041))?;

        // Check for 0x2312 pattern.
        // TODO: This is a CRC value of the file header.
        let mut pattern = [0u8; 2];
        r.read_exact(&mut pattern)?;
        //println!("Start pattern: {:?}", pattern);
        if pattern[0] != 0x23 || pattern[1] != 0x12 {
            return Err(ZlfError::InvalidStartPattern);
        }

        Ok(Self { r })
    }

    /// Read the next frame. Returns Ok(None) at EOF.
    pub fn next(&mut self) -> Result<Option<ZlfRecord>, ZlfError> {
        // Read a timestamp of 8 bytes
        let mut timestamp = [0u8; 8];
        let n = self.r.read(&mut timestamp)?;
        if n == 0 {
            return Ok(None); // EOF
        }
        println!("Timestamp: {:?}", u64::from_le_bytes(timestamp));

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

        Ok(Some(ZlfRecord::Other(RawFrame { sof: 0, typ: FrameType::Unknown(0), payload })))

/*
        // Read marker (SOF or SODF). In the Zniffer runtime these are '#' and '!' respectively. [2](https://docs.silabs.com/z-wave/latest/zwave-api/zniffer)
        let mut marker = [0u8; 1];
        match self.r.read(&mut marker)? {
            0 => return Ok(None), // EOF
            1 => {},
            _ => unreachable!(),
        }
        let sof = marker[0];
        if sof != b'#' && sof != b'!' {
            // If frames are concatenated, noisy bytes can appear. You might want to resync by scanning forward
            // until you find '#'/ '!'. Here we fail fast.
            return Err(ZlfError::BadMarker(sof));
        }

        // Read type
        let mut typ_b = [0u8; 1];
        if self.r.read(&mut typ_b)? != 1 {
            return Err(ZlfError::Eof);
        }
        let typ = match typ_b[0] {
            0x00 => FrameType::Cmd,
            0x01 => FrameType::Data,
            0x02 => FrameType::Beam,
            0x04 => FrameType::BeamStart,
            0x05 => FrameType::BeamStop,
            other => FrameType::Unknown(other),
        };

        // Read payload length (uint8_t in device API). [3](https://tridentiot.github.io/tridentiot-sdk/z-wave/group__Zniffer.html)
        let mut len_b = [0u8; 1];
        if self.r.read(&mut len_b)? != 1 {
            return Err(ZlfError::Eof);
        }
        let payload_len = len_b[0] as usize;

        // Read payload bytes
        let mut payload = vec![0u8; payload_len];
        let mut read = 0usize;
        while read < payload_len {
            let n = self.r.read(&mut payload[read..])?;
            if n == 0 {
                return Err(ZlfError::Eof);
            }
            read += n;
        }

        // If it's a data frame, decode according to the Zniffer API layout. [3](https://tridentiot.github.io/tridentiot-sdk/z-wave/group__Zniffer.html)
        if let FrameType::Data = typ {
            if payload.len() < 6 {
                return Err(ZlfError::ShortDataPayload);
            }
            let ts = u16::from_le_bytes([payload[0], payload[1]]);
            let ch_speed = payload[2];
            let region = payload[3];
            let rssi = payload[4] as i8;
            let mpdu_len = payload[5] as usize;
            if 6 + mpdu_len > payload.len() {
                return Err(ZlfError::ShortDataPayload);
            }
            let mpdu = payload[6..(6 + mpdu_len)].to_vec();
            Ok(Some(ZlfRecord::Data(DataFrame {
                timestamp: ts,
                ch_and_speed: ch_speed,
                region,
                rssi,
                mpdu,
            })))
        } else {
            Ok(Some(ZlfRecord::Other(RawFrame { sof, typ, payload })))
        }
*/
    }
}
