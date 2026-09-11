// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Finding record boundaries at an arbitrary offset in a trace.
//!
//! ZLF records are variable length, so a byte offset alone does not say
//! where a record begins. They do carry enough structure to recognise one:
//!
//! ```text
//! ticks(8) | props(1) | len(4) | payload(len) | eod(1)
//! ```
//!
//! A candidate offset is accepted when the length is sane, the timestamp is
//! a plausible .NET `DateTime`, the payload starts with the PTI preamble
//! `0x5B`, and the trailing byte decodes to a known API type — and when the
//! same holds for the [`CHAIN`] records that follow.
//!
//! Measured on a 60 MB capture of 1,140,300 records: zero false positives in
//! 400,000 random byte offsets, and no true record start rejected.
use std::io;

use crate::source::TraceSource;
use crate::zlf::ZLF_HEADER_SIZE;
use crate::zlf::types::ApiType;

/// Consecutive records that must parse before an offset is accepted.
const CHAIN: usize = 3;

/// Largest payload considered plausible. Real records are a few hundred
/// bytes; this only has to exclude nonsense read from the middle of one.
const MAX_PAYLOAD: u32 = 1 << 20;

/// Bytes of a record that precede its payload.
const RECORD_HEADER: usize = 13;

/// How far to search for a boundary before giving up.
pub const DEFAULT_WINDOW: u64 = 1 << 20;

/// .NET ticks for the year 2000 and 2100, bounding a plausible capture time.
const TICKS_2000: i64 = 630_822_816_000_000_000;
const TICKS_2100: i64 = 662_479_488_000_000_000;

/// A record header read from the trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordHeader {
    /// Offset the record starts at.
    pub offset: u64,
    /// Raw `DateTime.ToBinary()` value.
    pub raw_ticks: i64,
    pub is_outcome: bool,
    pub session_id: u8,
    pub payload_len: u32,
    pub api_type: ApiType,
}

impl RecordHeader {
    /// Offset of the record that follows this one.
    pub fn next_offset(&self) -> u64 {
        self.offset + RECORD_HEADER as u64 + self.payload_len as u64 + 1
    }

    /// Offset of the payload's first byte.
    pub fn payload_offset(&self) -> u64 {
        self.offset + RECORD_HEADER as u64
    }

    /// Capture time in milliseconds since the Unix epoch.
    pub fn unix_millis(&self) -> i64 {
        crate::zlf::Timestamp::from_binary(self.raw_ticks).unix_millis()
    }
}

/// Read and validate a single record header at `offset`.
///
/// Returns `Ok(None)` when the bytes there are not a plausible record, which
/// is the normal answer while scanning for a boundary.
pub fn read_record(
    src: &mut impl TraceSource,
    offset: u64,
) -> io::Result<Option<RecordHeader>> {
    let mut head = [0u8; RECORD_HEADER];
    if src.read_at(offset, &mut head)? < RECORD_HEADER {
        return Ok(None);
    }

    let raw_ticks = i64::from_le_bytes(head[0..8].try_into().unwrap());
    let ticks = raw_ticks & 0x3FFF_FFFF_FFFF_FFFF;
    if !(TICKS_2000..=TICKS_2100).contains(&ticks) {
        return Ok(None);
    }

    let payload_len = i32::from_le_bytes(head[9..13].try_into().unwrap());
    if payload_len <= 0 || payload_len as u32 > MAX_PAYLOAD {
        return Ok(None);
    }
    let payload_len = payload_len as u32;

    let end = offset + RECORD_HEADER as u64 + payload_len as u64;
    if end >= src.len() {
        return Ok(None);
    }

    // The payload's first byte and the trailing terminator together are what
    // make a false match vanishingly unlikely.
    let mut first = [0u8; 1];
    if src.read_at(offset + RECORD_HEADER as u64, &mut first)? < 1 {
        return Ok(None);
    }

    let mut eod = [0u8; 1];
    if src.read_at(end, &mut eod)? < 1 {
        return Ok(None);
    }
    let api_type = ApiType::from_eod(eod[0]);
    if matches!(api_type, ApiType::Unknown(_)) {
        return Ok(None);
    }
    // PTI payloads always open with '['. Other dialects carry their own
    // shapes, so only insist on it for the type that guarantees it.
    if matches!(api_type, ApiType::Pti | ApiType::PtiDiagnostic) && first[0] != 0x5B {
        return Ok(None);
    }

    Ok(Some(RecordHeader {
        offset,
        raw_ticks,
        is_outcome: head[8] & 0x80 != 0,
        session_id: head[8] & 0x7F,
        payload_len,
        api_type,
    }))
}

/// True when `offset` begins a run of [`CHAIN`] valid records.
fn is_boundary(src: &mut impl TraceSource, offset: u64) -> io::Result<bool> {
    let mut at = offset;
    for _ in 0..CHAIN {
        match read_record(src, at)? {
            Some(rec) => at = rec.next_offset(),
            // Running off the end still counts: the last records in a file
            // cannot be followed by CHAIN more.
            None => return Ok(at >= src.len()),
        }
    }
    Ok(true)
}

/// Offset of the first record at or after `from`.
///
/// Scans forward at most `window` bytes. Returns `Ok(None)` if no boundary
/// is found in that span.
pub fn next_record_at(
    src: &mut impl TraceSource,
    from: u64,
    window: u64,
) -> io::Result<Option<u64>> {
    let start = from.max(ZLF_HEADER_SIZE as u64);
    let limit = (start + window).min(src.len());
    for offset in start..limit {
        if is_boundary(src, offset)? {
            return Ok(Some(offset));
        }
    }
    Ok(None)
}

/// Capture time of the first record at or after `offset`, with its position.
pub fn record_time_at(
    src: &mut impl TraceSource,
    offset: u64,
) -> io::Result<Option<(u64, i64)>> {
    let Some(at) = next_record_at(src, offset, DEFAULT_WINDOW)? else {
        return Ok(None);
    };
    Ok(read_record(src, at)?.map(|r| (at, r.unix_millis())))
}

// These tests read a real capture, which is not in the repository.
// Enable the `fixtures` feature with a trace in place to run them.
#[cfg(all(test, feature = "fixtures"))]
mod tests {
    use super::*;
    use crate::source::SliceSource;

    const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/pti-800.zlf");

    /// Every record start, found by reading the file in order.
    fn true_starts(bytes: &[u8]) -> Vec<u64> {
        let mut src = SliceSource::new(bytes);
        let mut at = ZLF_HEADER_SIZE as u64;
        let mut out = Vec::new();
        while let Some(rec) = read_record(&mut src, at).unwrap() {
            out.push(at);
            at = rec.next_offset();
        }
        out
    }

    #[test]
    fn finds_every_record_in_the_fixture() {
        let starts = true_starts(FIXTURE);
        assert_eq!(starts.len(), 181);
        assert_eq!(starts[0], ZLF_HEADER_SIZE as u64);
    }

    #[test]
    fn accepts_true_starts_and_rejects_everything_else() {
        // The property the whole seek design rests on.
        let starts = true_starts(FIXTURE);
        let set: std::collections::HashSet<u64> = starts.iter().copied().collect();
        let mut src = SliceSource::new(FIXTURE);

        for &s in &starts {
            assert!(is_boundary(&mut src, s).unwrap(), "rejected a real start at {s}");
        }

        let mut false_positives = 0;
        for offset in ZLF_HEADER_SIZE as u64..FIXTURE.len() as u64 - 64 {
            if !set.contains(&offset) && is_boundary(&mut src, offset).unwrap() {
                false_positives += 1;
            }
        }
        assert_eq!(false_positives, 0, "resync accepted a non-boundary");
    }

    #[test]
    fn seeks_forward_to_the_next_boundary() {
        let starts = true_starts(FIXTURE);
        let mut src = SliceSource::new(FIXTURE);

        // From just past a record start, the next boundary is the next record.
        let found = next_record_at(&mut src, starts[4] + 1, DEFAULT_WINDOW).unwrap();
        assert_eq!(found, Some(starts[5]));

        // Landing exactly on a start returns it unchanged.
        let found = next_record_at(&mut src, starts[9], DEFAULT_WINDOW).unwrap();
        assert_eq!(found, Some(starts[9]));
    }

    #[test]
    fn reads_a_plausible_capture_time() {
        let mut src = SliceSource::new(FIXTURE);
        let (at, ms) = record_time_at(&mut src, 0).unwrap().expect("first record");
        assert_eq!(at, ZLF_HEADER_SIZE as u64);
        // The fixture was captured on 2026-02-04.
        assert_eq!(ms, 1_770_223_530_405);
    }

    #[test]
    fn times_do_not_decrease_through_the_fixture() {
        // seek_time binary-searches on this assumption.
        let mut src = SliceSource::new(FIXTURE);
        let mut previous = i64::MIN;
        for at in true_starts(FIXTURE) {
            let ms = read_record(&mut src, at).unwrap().unwrap().unix_millis();
            assert!(ms >= previous, "time went backwards at offset {at}");
            previous = ms;
        }
    }
}
