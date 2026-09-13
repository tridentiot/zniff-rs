// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use std::io::Write;

use crate::zlf::reader::{
    Timestamp,
    ZLF_HEADER_SIZE,
    ZlfError,
};
use crate::zlf::types::{
    ApiType,
    ZLF_VERSION,
};

/// Offset of the header CRC within the header.
const HEADER_CRC_OFFSET: usize = 2046;

/// Offset of the UTF-16LE comment within the header.
const COMMENT_OFFSET: usize = 8;

/// Bytes reserved for the comment.
const COMMENT_BYTES: usize = 512;

/// Text encoding marker written at `header[4..8]`.
///
/// The C# writer stores `StorageHeader.TextEncoding`, which is left at its
/// default of 0 for every trace the reader has been verified against.
const TEXT_ENCODING: u32 = 0;

/// Writes a ZLF trace.
///
/// The counterpart to [`ZlfReader`](crate::zlf::ZlfReader). Only [`Write`] is
/// required, so records can be streamed to a file as they are captured without
/// seeking back — which is what lets the trace be served while it grows.
#[derive(Debug)]
pub struct ZlfWriter<W: Write> {
    w: W,
    record_counter: usize,
    bytes: u64,
}

impl<W: Write> ZlfWriter<W> {
    /// Write the 2048-byte file header and return a writer positioned for
    /// records.
    ///
    /// `comment` is stored as UTF-16LE and is what the viewer shows as the
    /// trace's origin; it is truncated to fit the reserved 512 bytes.
    pub fn new(mut w: W, comment: &str) -> Result<Self, ZlfError> {
        w.write_all(&Self::header(comment))?;
        Ok(Self { w, record_counter: 0, bytes: ZLF_HEADER_SIZE as u64 })
    }

    /// Build the file header, with the CRC the reader validates.
    fn header(comment: &str) -> [u8; ZLF_HEADER_SIZE] {
        let mut header = [0u8; ZLF_HEADER_SIZE];
        header[..4].copy_from_slice(&ZLF_VERSION.to_le_bytes());
        header[4..8].copy_from_slice(&TEXT_ENCODING.to_le_bytes());

        // UTF-16LE, truncated on a code-unit boundary so the text stays valid.
        let mut at = COMMENT_OFFSET;
        for unit in comment.encode_utf16() {
            if at + 2 > COMMENT_OFFSET + COMMENT_BYTES {
                break;
            }
            header[at..at + 2].copy_from_slice(&unit.to_le_bytes());
            at += 2;
        }

        let crc =
            crc16::State::<crc16::AUG_CCITT>::calculate(&header[..HEADER_CRC_OFFSET]);
        header[HEADER_CRC_OFFSET..].copy_from_slice(&crc.to_le_bytes());
        header
    }

    /// Append one record.
    ///
    /// `payload` is stored verbatim: a ZLF record holds raw transport bytes,
    /// not decoded frames, so a capture never has to understand what it logs.
    pub fn write_record(
        &mut self,
        timestamp: Timestamp,
        is_outcome: bool,
        session_id: u8,
        api_type: ApiType,
        payload: &[u8],
    ) -> Result<(), ZlfError> {
        let length = i32::try_from(payload.len())
            .map_err(|_| ZlfError::InvalidPayloadLength(i32::MAX))?;

        self.w.write_all(&timestamp.raw.to_le_bytes())?;
        // Bit 7 marks an outgoing record; the low 7 bits are the session id.
        self.w
            .write_all(&[(session_id & 0x7F) | if is_outcome { 0x80 } else { 0 }])?;
        self.w.write_all(&length.to_le_bytes())?;
        self.w.write_all(payload)?;
        self.w.write_all(&[api_type.to_eod()])?;

        self.record_counter += 1;
        // ticks(8) + props(1) + len(4) + payload + eod(1)
        self.bytes += 14 + payload.len() as u64;
        Ok(())
    }

    /// Bytes written so far, header included.
    ///
    /// Serving a growing trace needs the exact on-disk length, and tracking it
    /// here avoids a `stat` race against a partially flushed record.
    pub fn bytes_written(&self) -> u64 {
        self.bytes
    }

    /// Number of records written so far.
    pub fn record_count(&self) -> usize {
        self.record_counter
    }

    pub fn flush(&mut self) -> Result<(), ZlfError> {
        self.w.flush()?;
        Ok(())
    }

    /// Consume the writer and return the underlying sink.
    pub fn into_inner(self) -> W {
        self.w
    }
}

impl ZlfWriter<Vec<u8>> {
    /// The bytes written so far.
    ///
    /// Only for an in-memory trace, which is how the browser records: it
    /// needs to read the trace back while still appending to it.
    pub fn buffer(&self) -> &[u8] {
        &self.w
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::zlf::ZlfReader;

    /// Expected file length for a set of records.
    fn buffer_len_of<T>(records: &[(T, bool, u8, ApiType, Vec<u8>)]) -> u64 {
        ZLF_HEADER_SIZE as u64
            + records.iter().map(|(_, _, _, _, p)| 14 + p.len() as u64).sum::<u64>()
    }

    #[test]
    fn header_round_trips_through_the_reader() {
        let mut buffer = Vec::new();
        ZlfWriter::new(&mut buffer, "capture").unwrap();
        assert_eq!(buffer.len(), ZLF_HEADER_SIZE);
        // Constructing a reader validates both the version and the CRC.
        ZlfReader::new(Cursor::new(buffer)).unwrap();
    }

    #[test]
    fn records_round_trip() {
        let written = [
            (Timestamp::from_binary(0x08DE_0000_0000_0001), false, 0, ApiType::Pti, vec![
                0x5B, 0x01, 0x02,
            ]),
            (Timestamp::from_binary(0x08DE_0000_0000_0002), true, 3, ApiType::Zniffer, vec![
                0x23, 0x04, 0x00,
            ]),
            // An empty payload is legal and must survive.
            (Timestamp::from_binary(0x08DE_0000_0000_0003), false, 0, ApiType::Text, vec![]),
            // Unknown API types must round-trip rather than being normalised.
            (
                Timestamp::from_binary(0x08DE_0000_0000_0004),
                false,
                127,
                ApiType::Unknown(0x42),
                vec![0xFF; 300],
            ),
        ];

        let mut buffer = Vec::new();
        let mut writer = ZlfWriter::new(&mut buffer, "round trip").unwrap();
        for (ts, out, session, api, payload) in &written {
            writer.write_record(*ts, *out, *session, *api, payload).unwrap();
        }
        writer.flush().unwrap();
        assert_eq!(writer.record_count(), written.len());
        assert_eq!(writer.bytes_written(), buffer_len_of(&written));

        let mut reader = ZlfReader::new(Cursor::new(buffer)).unwrap();
        for (ts, out, session, api, payload) in &written {
            let record = reader.next_record().unwrap().expect("record");
            assert_eq!(record.timestamp, *ts);
            assert_eq!(record.is_outcome, *out);
            assert_eq!(record.session_id, *session);
            assert_eq!(record.api_type, *api);
            assert_eq!(&record.payload, payload);
        }
        assert!(reader.next_record().unwrap().is_none());
    }

    /// A trace built the way the browser builds one — appending records to
    /// an in-memory buffer and reading it back while still writing — must
    /// parse at every step.
    #[test]
    fn an_in_memory_trace_is_readable_while_it_grows() {
        let mut writer = ZlfWriter::new(Vec::new(), "live").unwrap();
        assert_eq!(writer.buffer().len(), ZLF_HEADER_SIZE, "header first");
        ZlfReader::new(Cursor::new(writer.buffer().to_vec())).unwrap();

        for i in 0..5 {
            writer
                .write_record(
                    Timestamp::from_unix_millis(1_700_000_000_000 + i),
                    false,
                    0,
                    ApiType::Zniffer,
                    &[0x23, 0x04, 0x00],
                )
                .unwrap();

            // Read back everything written so far, as the viewer would.
            let mut reader = ZlfReader::new(Cursor::new(writer.buffer().to_vec())).unwrap();
            let mut seen = 0;
            while reader.next_record().unwrap().is_some() {
                seen += 1;
            }
            assert_eq!(seen, i + 1, "every record written is readable");
            assert_eq!(writer.buffer().len() as u64, writer.bytes_written());
        }
    }

    #[test]
    fn comment_is_truncated_without_splitting_a_code_unit() {
        // Two bytes per UTF-16 code unit, so 512 bytes holds 256 of them.
        let comment = "\u{4e2d}".repeat(400);
        let mut buffer = Vec::new();
        ZlfWriter::new(&mut buffer, &comment).unwrap();
        ZlfReader::new(Cursor::new(buffer.clone())).unwrap();

        let stored = &buffer[COMMENT_OFFSET..COMMENT_OFFSET + COMMENT_BYTES];
        let units: Vec<u16> =
            stored.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        assert!(units.iter().all(|u| *u == 0x4e2d));
        assert_eq!(units.len(), COMMENT_BYTES / 2);
    }
}
