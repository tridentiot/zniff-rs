// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use std::io::{
    self,
    Read,
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

/// Size of the ZLF file header, in bytes.
pub const ZLF_HEADER_SIZE: usize = 2048;

/// Offset of the header CRC within the header.
const HEADER_CRC_OFFSET: usize = 2046;

/// .NET `DateTime` ticks (100 ns units) between 0001-01-01 and the Unix epoch.
const TICKS_UNIX_EPOCH: i64 = 621_355_968_000_000_000;

/// Ticks per millisecond in .NET `DateTime`.
const TICKS_PER_MILLISECOND: i64 = 10_000;

/// A timestamp as stored in a ZLF record.
///
/// ZLF stores .NET `DateTime.ToBinary()`: the low 62 bits are a tick count
/// (100 ns units since 0001-01-01) and the top 2 bits are the `DateTimeKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timestamp {
    /// Raw `DateTime.ToBinary()` value, exactly as stored.
    pub raw: i64,
}

impl Timestamp {
    pub fn from_binary(raw: i64) -> Self {
        Self { raw }
    }

    /// Tick count with the `DateTimeKind` bits masked off.
    pub fn ticks(self) -> i64 {
        self.raw & 0x3FFF_FFFF_FFFF_FFFF
    }

    /// Milliseconds since the Unix epoch, for use with JS `Date`.
    pub fn unix_millis(self) -> i64 {
        (self.ticks() - TICKS_UNIX_EPOCH) / TICKS_PER_MILLISECOND
    }

    /// Build a timestamp from milliseconds since the Unix epoch.
    ///
    /// The `DateTimeKind` bits are left at zero (`Unspecified`), matching what
    /// the C# writer stores for captured records.
    pub fn from_unix_millis(millis: i64) -> Self {
        Self { raw: millis * TICKS_PER_MILLISECOND + TICKS_UNIX_EPOCH }
    }
}

/// One record as stored in a ZLF file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZlfRecord {
    /// Wall-clock time the record was captured.
    pub timestamp: Timestamp,
    /// True when the record was transmitted by the host rather than received.
    pub is_outcome: bool,
    /// Capture session the record belongs to.
    pub session_id: u8,
    /// What the payload contains.
    pub api_type: ApiType,
    /// Raw payload bytes. Not frame-aligned: may hold part of a frame,
    /// one frame, or several.
    pub payload: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum ZlfError {
    #[error("Invalid ZLF version: {0}")]
    InvalidZlfVersion(u32),
    #[error("Invalid ZLF header checksum")]
    InvalidHeaderChecksum,
    #[error("Invalid payload length: {0}")]
    InvalidPayloadLength(i32),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("unexpected EOF while parsing a record")]
    Eof,
}

/// Reads records from a ZLF trace.
///
/// Generic over any [`Read`], so it works equally on a file or on an in-memory
/// buffer via [`std::io::Cursor`] — the latter is what the WebAssembly build uses.
#[derive(Debug)]
pub struct ZlfReader<R: Read> {
    r: R,
    record_counter: usize,
}

impl<R: Read> ZlfReader<R> {
    /// Read and validate the 2048-byte file header.
    pub fn new(mut r: R) -> Result<Self, ZlfError> {
        let mut header = [0u8; ZLF_HEADER_SIZE];
        r.read_exact(&mut header)?;

        let file_checksum =
            u16::from_le_bytes([header[HEADER_CRC_OFFSET], header[HEADER_CRC_OFFSET + 1]]);
        if crc16::State::<crc16::AUG_CCITT>::calculate(&header[..HEADER_CRC_OFFSET])
            != file_checksum
        {
            return Err(ZlfError::InvalidHeaderChecksum);
        }

        let version = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
        if version != ZLF_VERSION {
            return Err(ZlfError::InvalidZlfVersion(version));
        }

        // header[4..8] is the text encoding and header[8..520] a UTF-16LE
        // comment; neither is needed to decode records.

        Ok(Self { r, record_counter: 0 })
    }

    /// Number of records read so far.
    pub fn record_count(&self) -> usize {
        self.record_counter
    }

    /// Invoke `callback` for every remaining record.
    pub fn read_records<F>(&mut self, mut callback: F) -> Result<(), ZlfError>
    where
        F: FnMut(ZlfRecord),
    {
        while let Some(record) = self.next_record()? {
            callback(record);
        }
        Ok(())
    }

    /// Read the next record, or `Ok(None)` at end of file.
    pub fn next_record(&mut self) -> Result<Option<ZlfRecord>, ZlfError> {
        let mut timestamp = [0u8; 8];
        match self.r.read_exact(&mut timestamp) {
            Ok(()) => {},
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e.into()),
        }
        let timestamp = Timestamp::from_binary(i64::from_le_bytes(timestamp));

        // Bit 7 marks an outgoing record; the low 7 bits are the session id.
        let mut properties = [0u8; 1];
        self.r.read_exact(&mut properties)?;
        let is_outcome = properties[0] & 0x80 != 0;
        let session_id = properties[0] & 0x7F;

        let mut payload_length = [0u8; 4];
        self.r.read_exact(&mut payload_length)?;
        let payload_length = i32::from_le_bytes(payload_length);
        if payload_length < 0 {
            return Err(ZlfError::InvalidPayloadLength(payload_length));
        }

        let mut payload = vec![0u8; payload_length as usize];
        self.r.read_exact(&mut payload).map_err(|e| {
            if e.kind() == io::ErrorKind::UnexpectedEof { ZlfError::Eof } else { e.into() }
        })?;

        // The trailing byte terminates the record and encodes its API type.
        let mut eod = [0u8; 1];
        self.r.read_exact(&mut eod).map_err(|e| {
            if e.kind() == io::ErrorKind::UnexpectedEof { ZlfError::Eof } else { e.into() }
        })?;

        self.record_counter += 1;
        Ok(Some(ZlfRecord {
            timestamp,
            is_outcome,
            session_id,
            api_type: ApiType::from_eod(eod[0]),
            payload,
        }))
    }
}

impl<R: Read> Iterator for ZlfReader<R> {
    type Item = Result<ZlfRecord, ZlfError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_record().transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a ZLF header with a valid checksum.
    fn header(version: u32) -> Vec<u8> {
        let mut h = vec![0u8; ZLF_HEADER_SIZE];
        h[..4].copy_from_slice(&version.to_le_bytes());
        let crc = crc16::State::<crc16::AUG_CCITT>::calculate(&h[..HEADER_CRC_OFFSET]);
        h[HEADER_CRC_OFFSET..].copy_from_slice(&crc.to_le_bytes());
        h
    }

    fn record(raw_ts: i64, properties: u8, payload: &[u8], eod: u8) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend_from_slice(&raw_ts.to_le_bytes());
        r.push(properties);
        r.extend_from_slice(&(payload.len() as i32).to_le_bytes());
        r.extend_from_slice(payload);
        r.push(eod);
        r
    }

    #[test]
    fn rejects_bad_version() {
        let err = ZlfReader::new(io::Cursor::new(header(103))).unwrap_err();
        assert!(matches!(err, ZlfError::InvalidZlfVersion(103)));
    }

    #[test]
    fn rejects_bad_checksum() {
        let mut h = header(ZLF_VERSION);
        h[HEADER_CRC_OFFSET] ^= 0xFF;
        let err = ZlfReader::new(io::Cursor::new(h)).unwrap_err();
        assert!(matches!(err, ZlfError::InvalidHeaderChecksum));
    }

    #[test]
    fn reads_records_and_keeps_unknown_api_types() {
        // A value taken from a real trace: 2026-02-04T16:45:30 UTC-ish.
        let raw_ts = -8_584_313_833_550_718_250i64;
        let mut data = header(ZLF_VERSION);
        data.extend(record(raw_ts, 0x00, &[0x5B, 0x41], 0xF5)); // Pti
        data.extend(record(raw_ts, 0x81, &[0x01], 0xFD)); // Basic, outgoing, session 1
        data.extend(record(raw_ts, 0x00, &[0x02], 0x00)); // unknown, must not abort

        let mut reader = ZlfReader::new(io::Cursor::new(data)).unwrap();
        let records: Vec<_> =
            std::iter::from_fn(|| reader.next_record().transpose()).collect::<Result<_, _>>().unwrap();

        assert_eq!(records.len(), 3);
        assert_eq!(records[0].api_type, ApiType::Pti);
        assert!(!records[0].is_outcome);
        assert_eq!(records[0].payload, vec![0x5B, 0x41]);

        assert_eq!(records[1].api_type, ApiType::Basic);
        assert!(records[1].is_outcome);
        assert_eq!(records[1].session_id, 1);

        // An unrecognised API type is reported, not treated as a fatal error.
        assert_eq!(records[2].api_type, ApiType::Unknown(0xFE));
        assert_eq!(reader.record_count(), 3);
    }

    #[test]
    fn decodes_dotnet_timestamp() {
        // Same value as above; DateTimeKind is 2 (UTC) in the top bits.
        let ts = Timestamp::from_binary(-8_584_313_833_550_718_250i64);
        assert_eq!((ts.raw >> 62) & 3, 2);
        // 2026-02-04T16:45:30.405Z
        assert_eq!(ts.unix_millis(), 1_770_223_530_405);
    }
}
