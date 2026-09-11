// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Channel/region to bit-rate mapping.
//!
//! PTI records do not carry the bit rate; it is implied by the combination of
//! channel and region. The tables come from ITU-T G.9959 (tables A.1, A.2) via
//! `PtiFrameParser.cs` in z-wave-tools-core. Each entry is `(channel << 8) | region`.

/// 9.6 kbit/s.
const BAUD_9600: &[u16] = &[
    (0x02 << 8) + 0x01, // European Union
    (0x02 << 8) + 0x02, // United States
    (0x02 << 8) + 0x03, // Australia/New Zealand
    (0x02 << 8) + 0x04, // Hong Kong
    (0x02 << 8) + 0x05, // Malaysia
    (0x02 << 8) + 0x06, // India
    (0x02 << 8) + 0x08, // Russian Federation
    (0x02 << 8) + 0x09, // Israel
    (0x02 << 8) + 0x0B, // China
    (0x02 << 8) + 0x0C, // United States, Long Range 1
    (0x02 << 8) + 0x0D, // United States, Long Range 2
    (0x02 << 8) + 0x0F, // European Union, Long Range 1
    (0x02 << 8) + 0x10, // European Union, Long Range 2
];

/// 40 kbit/s.
const BAUD_40K: &[u16] = &[
    (0x01 << 8) + 0x01,
    (0x01 << 8) + 0x02,
    (0x01 << 8) + 0x03,
    (0x01 << 8) + 0x04,
    (0x01 << 8) + 0x05,
    (0x01 << 8) + 0x06,
    (0x01 << 8) + 0x08,
    (0x01 << 8) + 0x09,
    (0x01 << 8) + 0x0B,
    (0x01 << 8) + 0x0C,
    (0x01 << 8) + 0x0D,
    (0x01 << 8) + 0x0F,
    (0x01 << 8) + 0x10,
];

/// 100 kbit/s.
const BAUD_100K: &[u16] = &[
    (0x00 << 8) + 0x01,
    (0x00 << 8) + 0x02,
    (0x00 << 8) + 0x03,
    (0x00 << 8) + 0x04,
    (0x00 << 8) + 0x05,
    (0x00 << 8) + 0x06,
    (0x00 << 8) + 0x07, // Japan
    (0x01 << 8) + 0x07, // Japan
    (0x02 << 8) + 0x07, // Japan
    (0x00 << 8) + 0x08,
    (0x00 << 8) + 0x09,
    (0x00 << 8) + 0x0A, // Korea
    (0x01 << 8) + 0x0A, // Korea
    (0x02 << 8) + 0x0A, // Korea
    (0x00 << 8) + 0x0B,
    (0x00 << 8) + 0x0C,
    (0x00 << 8) + 0x0D,
    (0x00 << 8) + 0x0F,
    (0x00 << 8) + 0x10,
];

/// Long Range.
const BAUD_LR: &[u16] = &[
    (0x03 << 8) + 0x0C, // CH3 + US_LR1
    (0x03 << 8) + 0x0D, // CH3 + US_LR2
    (0x00 << 8) + 0x0E, // CH0 + US_LR3
    (0x01 << 8) + 0x0E, // CH1 + US_LR3
    (0x03 << 8) + 0x0F, // CH3 + EU_LR1
    (0x03 << 8) + 0x10, // CH3 + EU_LR2
    (0x00 << 8) + 0x11, // CH0 + EU_LR3
    (0x01 << 8) + 0x11, // CH1 + EU_LR3
];

/// PTI region ids that use base header 1 (Japan and Korea).
pub const REGIONS_BASE_1: &[u8] = &[0x07, 0x0A];

/// Bit rate implied by a channel/region pair.
///
/// Returns 0 (9.6k) for combinations that appear in no table, matching the
/// reference implementation's default.
pub fn speed_for(channel: u8, region: u8) -> u8 {
    let key = ((channel as u16) << 8) | region as u16;
    if BAUD_9600.contains(&key) {
        0
    } else if BAUD_40K.contains(&key) {
        1
    } else if BAUD_100K.contains(&key) {
        2
    } else if BAUD_LR.contains(&key) {
        3
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_pairs() {
        assert_eq!(speed_for(0x00, 0x01), 2); // EU channel 0 is 100k
        assert_eq!(speed_for(0x01, 0x01), 1); // EU channel 1 is 40k
        assert_eq!(speed_for(0x02, 0x01), 0); // EU channel 2 is 9.6k
        assert_eq!(speed_for(0x03, 0x0C), 3); // US Long Range
        assert_eq!(speed_for(0x02, 0x07), 2); // Japan is 100k on all channels
    }

    #[test]
    fn defaults_to_slowest_for_unknown() {
        assert_eq!(speed_for(0x0F, 0x0F), 0);
    }
}
