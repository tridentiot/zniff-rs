// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! The zniffer's serial command protocol, independent of how bytes move.
//!
//! Shared by the capture bridge, which talks to a serial port, and the
//! browser, which talks to the same device over Web Serial. Only the I/O
//! differs; the framing and parsing here are the same on both sides.

/// Start of a command or command response.
pub const SOF_COMMAND: u8 = 0x23;

pub const CMD_GET_VERSION: u8 = 0x01;

/// Select the capture region.
///
/// Alone among the commands, this one sends **no reply**: in the zniffer
/// firmware (`apps/zniffer/zniffer_app.c`, `ZNIFFER_CMD_SET_REGION`) the
/// handler changes the radio region and returns, while every other case
/// calls `zniffer_reply_data` or `zniffer_reply_no_data`.
///
/// Waiting for a response to it therefore waits forever. Confirm the change
/// by reading [`CMD_GET_FREQUENCIES`] back instead.
pub const CMD_SET_FREQUENCY: u8 = 0x02;
pub const CMD_GET_FREQUENCIES: u8 = 0x03;
pub const CMD_START: u8 = 0x04;
pub const CMD_STOP: u8 = 0x05;
pub const CMD_GET_FREQUENCY_STR: u8 = 0x13;

/// Baud rates to try, fastest first.
///
/// Measured on a sniffer v0.11: it answers at 230400 and is completely silent
/// at 115200, so probing in this order finds it on the first attempt.
pub const BAUD_RATES: [u32; 2] = [230_400, 115_200];

/// Firmware and chip identification, from `GetVersion`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub chip_type: u8,
    pub chip_version: u8,
    pub sniffer_version: u8,
    pub sniffer_revision: u8,
}

impl Version {
    /// Decode a `GetVersion` payload, which carries four bytes.
    pub fn parse(payload: &[u8]) -> Option<Self> {
        if payload.len() < 4 {
            return None;
        }
        Some(Self {
            chip_type: payload[0],
            chip_version: payload[1],
            sniffer_version: payload[2],
            sniffer_revision: payload[3],
        })
    }
}

impl core::fmt::Display for Version {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "sniffer {}.{} (chip {:#04x} rev {})",
            self.sniffer_version, self.sniffer_revision, self.chip_type, self.chip_version
        )
    }
}

/// Frame a command for transmission: `0x23 | command | length | payload`.
///
/// The C# builds this as `0x23` prepended to `[command] ++ parameters`, where
/// the caller's first parameter byte *is* the length. Stop therefore goes out
/// as `23 05 00` — three bytes, with no payload — which is what the hardware
/// was observed to answer.
pub fn command(cmd: u8, parameters: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + parameters.len());
    out.push(SOF_COMMAND);
    out.push(cmd);
    out.extend_from_slice(parameters);
    out
}

/// Find a response to `cmd` in `buffer` and return its payload.
///
/// Returns `None` while the response is still arriving, so a caller can keep
/// reading rather than treating a short buffer as a failure.
pub fn response_payload(buffer: &[u8], cmd: u8) -> Option<Vec<u8>> {
    found_response(buffer, cmd).map(|(payload, _)| payload)
}

/// As [`response_payload`], but also reporting where the response ended.
///
/// Anything after that point is not part of the reply — captured frames,
/// most likely — so a caller can keep it instead of discarding it.
pub fn found_response(buffer: &[u8], cmd: u8) -> Option<(Vec<u8>, usize)> {
    let mut i = 0;
    while i + 3 <= buffer.len() {
        if buffer[i] != SOF_COMMAND {
            i += 1;
            continue;
        }
        let length = buffer[i + 2] as usize;
        let end = i + 3 + length;
        if end > buffer.len() {
            return None; // Response still arriving.
        }
        if buffer[i + 1] == cmd {
            return Some((buffer[i + 3..end].to_vec(), end));
        }
        i = end;
    }
    None
}

/// Decode a `GetFrequencyStr` payload.
///
/// The payload is `[region, channels, name...]`, so the text starts at offset
/// 2 — taking it from 0 leaves the region byte glued to the front of the name.
pub fn region_name(payload: &[u8]) -> String {
    if payload.len() <= 2 {
        return String::new();
    }
    String::from_utf8_lossy(&payload[2..])
        .trim_matches(|c: char| c.is_control() || c == '\0')
        .trim()
        .to_string()
}

/// The region the device is currently tuned to, from `GetFrequencies`.
pub fn current_region(payload: &[u8]) -> Option<u8> {
    payload.first().copied()
}

/// The regions the device supports, from `GetFrequencies`.
pub fn supported_regions(payload: &[u8]) -> &[u8] {
    payload.get(1..).unwrap_or(&[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_commands_the_way_the_hardware_expects() {
        // Verified against the device: Stop is exactly 23 05 00.
        assert_eq!(command(CMD_STOP, &[0x00]), vec![0x23, 0x05, 0x00]);
        // Set region EU: 23 02 01 00 — one payload byte, the region code.
        assert_eq!(command(CMD_SET_FREQUENCY, &[0x01, 0x00]), vec![0x23, 0x02, 0x01, 0x00]);
    }

    #[test]
    fn decodes_a_region_name_past_the_header_bytes() {
        // Captured for US_LR_920MHz: region 0x64, 1 channel, then the name.
        let payload = [0x64, 0x01, b'U', b'S', b'_', b'L', b'R'];
        assert_eq!(region_name(&payload), "US_LR");
        assert_eq!(region_name(&[0x00, 0x02]), "");
    }

    #[test]
    fn extracts_a_version_response() {
        // Captured from a sniffer v0.11: 23 01 04 14 00 0b 00.
        let buffer = [0x23, 0x01, 0x04, 0x14, 0x00, 0x0b, 0x00];
        let payload = response_payload(&buffer, CMD_GET_VERSION).unwrap();
        assert_eq!(payload, vec![0x14, 0x00, 0x0b, 0x00]);

        let version = Version::parse(&payload).unwrap();
        assert_eq!(version.sniffer_version, 11);
        assert_eq!(version.chip_type, 0x14);
    }

    #[test]
    fn skips_an_unrelated_response() {
        // A stale Stop reply ahead of the version reply must not be mistaken
        // for it.
        let buffer = [0x23, 0x05, 0x00, 0x23, 0x01, 0x04, 0x14, 0x00, 0x0b, 0x00];
        let payload = response_payload(&buffer, CMD_GET_VERSION).unwrap();
        assert_eq!(payload, vec![0x14, 0x00, 0x0b, 0x00]);
    }

    #[test]
    fn waits_for_a_truncated_response() {
        let buffer = [0x23, 0x01, 0x04, 0x14, 0x00];
        assert!(response_payload(&buffer, CMD_GET_VERSION).is_none());
    }

    #[test]
    fn reports_where_a_response_ended() {
        // A reply followed by capture data: the tail must survive.
        let buffer = [0x23, 0x05, 0x00, 0x21, 0x03, 0xAA, 0xBB];
        let (payload, end) = found_response(&buffer, CMD_STOP).unwrap();
        assert!(payload.is_empty());
        assert_eq!(&buffer[end..], &[0x21, 0x03, 0xAA, 0xBB], "frames kept");
    }

    #[test]
    fn reads_the_region_list() {
        // Captured verbatim from the device with EU selected.
        let buffer = [
            0x23, 0x03, 0x11, 0x00, 0x00, 0x01, 0x02, 0x03, 0x05, 0x06, 0x07, 0x08, 0x09,
            0x64, 0x0b, 0x65, 0x20, 0x21, 0x66, 0x67,
        ];
        let payload = response_payload(&buffer, CMD_GET_FREQUENCIES).unwrap();
        assert_eq!(current_region(&payload), Some(0x00));
        assert_eq!(supported_regions(&payload).len(), 0x10);
        assert!(supported_regions(&payload).contains(&0x09), "US_LR");
    }
}
