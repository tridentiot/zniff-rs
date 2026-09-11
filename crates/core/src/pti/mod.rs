// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Extraction of Z-Wave frames from Silicon Labs PTI (Packet Trace Interface)
//! records, as stored in ZLF records of type [`ApiType::Pti`].
//!
//! Ported from `ZnifferApplication/Parsers/PtiFrameParser.cs` and
//! `ZnifferApplication/SnifferPtiFrameClient.cs` in z-wave-tools-core.
//!
//! [`ApiType::Pti`]: crate::zlf::ApiType::Pti
mod speed;

use serde::{
    Deserialize,
    Serialize,
};

pub use speed::speed_for;

/// Bracket preamble introducing a PTI record.
const PREAMBLE: u8 = 0x5B; // '['
/// Bracket postamble terminating a PTI record.
const POSTAMBLE: u8 = 0x5D; // ']'

/// Length of the DCH header for each supported version.
const DCH_LENGTH_VER2: usize = 11;
const DCH_LENGTH_VER3: usize = 18;

/// Hardware start/stop tags surrounding the radio payload.
const HW_RX_START: u8 = 0xF8;
const HW_TX_START: u8 = 0xFC;
const HW_RX_SUCCESS: u8 = 0xF9;
const HW_TX_SUCCESS: u8 = 0xFD;

/// Marker byte repeated throughout a wake-up beam.
const BEAM_MARKER: u8 = 0x55;

/// Z-Wave protocol id in the PTI appendix.
const ZWAVE_PROTOCOL: u8 = 0x06;

const REGION_MASK: u8 = 0x0F;
const CHANNEL_MASK: u8 = 0x0F;
const PROTOCOL_MASK: u8 = 0x0F;

/// Offset of the MPDU length field within a Z-Wave header.
const LENGTH_INDEX_IN_HEADER: usize = 7;

/// Direction a PTI frame was observed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    Rx,
    Tx,
}

/// A Z-Wave frame recovered from a PTI record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PtiFrame {
    pub direction: Direction,
    /// Region id, as encoded by PTI (not [`crate::types::Region`]).
    pub region: u8,
    pub channel: u8,
    /// 0 = 9.6k, 1 = 40k, 2 = 100k, 3 = Long Range.
    pub speed: u8,
    /// Raw RSSI byte; zero for transmitted frames, which carry no RSSI.
    pub rssi: i8,
    /// Number of beam repetitions, when this is a wake-up beam.
    pub beam_count: Option<u16>,
    /// The Z-Wave MPDU.
    pub mpdu: Vec<u8>,
}

impl PtiFrame {
    /// True when the frame is a wake-up beam rather than a regular MPDU.
    pub fn is_beam(&self) -> bool {
        self.beam_count.is_some()
    }
}

/// Split a ZLF PTI payload into its bracket-delimited records.
///
/// A single payload may hold several records. Iteration stops at the first
/// malformed record, mirroring the reference implementation.
pub fn split_records(payload: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    if payload.len() <= 4 {
        return out;
    }
    let mut index = 1usize;
    while index > 0 && index < payload.len() {
        let len = payload[index] as usize;
        if index + len >= payload.len()
            || payload[index - 1] != PREAMBLE
            || payload[index + len] != POSTAMBLE
        {
            break;
        }
        // The record body starts two bytes past the length and excludes the
        // length and postamble bytes.
        if len >= 2 && index + 2 + (len - 2) <= payload.len() {
            out.push(&payload[index + 2..index + 2 + (len - 2)]);
        }
        index += len + 2;
    }
    out
}

/// Decode one PTI record body into a Z-Wave frame.
///
/// Returns `None` when the record is not a complete Z-Wave frame — a
/// diagnostic record, an aborted transmission, or another protocol.
pub fn parse_record(data: &[u8]) -> Option<PtiFrame> {
    let before = match data.first()? {
        2 => DCH_LENGTH_VER2 + 1,
        3 => DCH_LENGTH_VER3 + 1,
        _ => return None,
    };
    if data.len() <= before {
        return None;
    }

    // The byte before the payload says whether this is a receive or transmit,
    // which in turn fixes the size and layout of the trailing appendix.
    let (direction, after, rssi_offset) = match data[before - 1] {
        HW_RX_START => (Direction::Rx, 6usize, Some(1usize)),
        HW_TX_START => (Direction::Tx, 5usize, None),
        _ => return None,
    };
    if data.len() <= before + after {
        return None;
    }

    // Only a successful transfer carries a valid frame.
    let end_tag = data[data.len() - after];
    let ok = match direction {
        Direction::Rx => end_tag == HW_RX_SUCCESS,
        Direction::Tx => end_tag == HW_TX_SUCCESS,
    };
    let is_beam = data[before] == BEAM_MARKER;
    if !ok && !is_beam {
        return None;
    }

    let appendix = data.len() - after;
    let base = rssi_offset.unwrap_or(0);
    let protocol = data[appendix + base + 3] & PROTOCOL_MASK;
    if protocol != ZWAVE_PROTOCOL {
        return None;
    }
    let region = data[appendix + base + 1] & REGION_MASK;
    let channel = data[appendix + base + 2] & CHANNEL_MASK;
    let rssi = rssi_offset.map_or(0, |o| data[appendix + o] as i8);
    let speed = speed_for(channel, region);

    let packet_length = data.len() - before - after;
    if is_beam {
        let frame_length = if speed == 3 { 4 } else { 3 };
        return Some(PtiFrame {
            direction,
            region,
            channel,
            speed,
            rssi,
            beam_count: Some(beam_count(frame_length, &data[before..before + packet_length])),
            mpdu: data[before..before + packet_length].to_vec(),
        });
    }

    // Prefer the length declared in the Z-Wave header; fall back to the
    // whole payload when it is absent or implausible.
    let mpdu_len = data
        .get(before + LENGTH_INDEX_IN_HEADER)
        .map(|&l| l as usize)
        .filter(|&l| LENGTH_INDEX_IN_HEADER < packet_length && l <= packet_length)
        .unwrap_or(packet_length);

    Some(PtiFrame {
        direction,
        region,
        channel,
        speed,
        rssi,
        beam_count: None,
        mpdu: data[before..before + mpdu_len].to_vec(),
    })
}

/// Extract every Z-Wave frame from a ZLF PTI payload.
pub fn parse_payload(payload: &[u8]) -> Vec<PtiFrame> {
    split_records(payload).into_iter().filter_map(parse_record).collect()
}

/// Count how many times a beam repeats within `data`.
///
/// A beam is the same short pattern sent over and over. Rather than divide
/// the length, each candidate repetition is compared against the first and
/// the scan realigns on the `0x55` marker when one does not match, which is
/// what `ParseWakeUpBeamsCounter` in z-wave-tools-core does.
fn beam_count(frame_length: usize, data: &[u8]) -> u16 {
    if frame_length == 0 || data.is_empty() {
        return 1;
    }
    // A truncated beam, missing its home id hash, is still one beam.
    let frame_length = frame_length.min(data.len());

    let mut count = 1u16;
    let mut offset = frame_length;
    while offset + frame_length <= data.len() {
        if data[offset] != BEAM_MARKER {
            offset += 1;
            continue;
        }
        if data[offset..offset + frame_length] == data[..frame_length] {
            count = count.saturating_add(1);
            offset += frame_length;
        } else {
            offset += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    /// First record of `crates/core/tests/fixtures/pti-800.zlf`.
    const REAL_RECORD: &[u8] = &[
        0x00, 0x02, 0x00, 0x52, 0x6E, 0x7D, 0x50, 0x12, 0x00, 0x2A, 0x00, 0x87, 0xF8,
    ];

    #[test]
    fn ignores_short_payloads() {
        assert!(split_records(&[0x5B, 0x02]).is_empty());
        assert!(parse_record(&[]).is_none());
        assert!(parse_record(&[0x09]).is_none());
    }

    #[test]
    fn counts_repeated_beams() {
        // Three copies of a 3-byte beam.
        let beam = [0x55u8, 0xAB, 0xCD];
        let data: Vec<u8> = beam.iter().chain(&beam).chain(&beam).copied().collect();
        assert_eq!(beam_count(3, &data), 3);

        // One beam, and a truncated one, are both a single beam.
        assert_eq!(beam_count(3, &beam), 1);
        assert_eq!(beam_count(3, &beam[..2]), 1);
        assert_eq!(beam_count(3, &[]), 1);
    }

    #[test]
    fn does_not_count_a_different_pattern_as_a_repeat() {
        let beam = [0x55u8, 0xAB, 0xCD];
        let other = [0x55u8, 0x11, 0x22];
        let data: Vec<u8> = beam.iter().chain(&other).copied().collect();
        // The second block starts with the marker but is not the same beam.
        assert_eq!(beam_count(3, &data), 1);
    }

    #[test]
    fn realigns_past_padding_between_beams() {
        let beam = [0x55u8, 0xAB, 0xCD];
        let mut data: Vec<u8> = beam.to_vec();
        data.push(0x00); // a stray byte between repetitions
        data.extend_from_slice(&beam);
        assert_eq!(beam_count(3, &data), 2);
    }

    #[test]
    fn rejects_unknown_dch_version() {
        let mut data = REAL_RECORD.to_vec();
        data[0] = 0x09;
        assert!(parse_record(&data).is_none());
    }
}
