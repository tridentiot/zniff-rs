// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use serde::{
    Deserialize,
    Serialize,
};

pub const ZLF_VERSION: u32 = 104;

/// Payload type of a ZLF record.
///
/// On disk each record ends with an "EOD" terminator byte, and the API type is
/// recovered as `0xFE - eod` (see `DataChunk.ApiType` in z-wave-tools-core).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiType {
    Zniffer,
    Basic,
    Programmer,
    Zip,
    Attachment,
    Text,
    XModem,
    Pti,
    PtiDiagnostic,
    Uic,
    Unknown(u8),
}

impl ApiType {
    /// Decode from the record's trailing EOD byte.
    pub fn from_eod(eod: u8) -> Self {
        Self::from(0xFEu8.wrapping_sub(eod))
    }

    /// The EOD byte that encodes this API type.
    pub fn to_eod(self) -> u8 {
        0xFEu8.wrapping_sub(u8::from(self))
    }
}

impl From<u8> for ApiType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => ApiType::Zniffer,
            0x01 => ApiType::Basic,
            0x02 => ApiType::Programmer,
            0x03 => ApiType::Zip,
            0x06 => ApiType::Attachment,
            0x07 => ApiType::Text,
            0x08 => ApiType::XModem,
            0x09 => ApiType::Pti,
            0x0A => ApiType::PtiDiagnostic,
            0x0B => ApiType::Uic,
            other => ApiType::Unknown(other),
        }
    }
}

impl From<ApiType> for u8 {
    fn from(value: ApiType) -> Self {
        match value {
            ApiType::Zniffer => 0x00,
            ApiType::Basic => 0x01,
            ApiType::Programmer => 0x02,
            ApiType::Zip => 0x03,
            ApiType::Attachment => 0x06,
            ApiType::Text => 0x07,
            ApiType::XModem => 0x08,
            ApiType::Pti => 0x09,
            ApiType::PtiDiagnostic => 0x0A,
            ApiType::Uic => 0x0B,
            ApiType::Unknown(other) => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eod_round_trip() {
        // Values observed in real traces / documented in z-wave-tools-core.
        assert_eq!(ApiType::from_eod(0xFE), ApiType::Zniffer);
        assert_eq!(ApiType::from_eod(0xF8), ApiType::Attachment);
        assert_eq!(ApiType::from_eod(0xF5), ApiType::Pti);
        assert_eq!(ApiType::Pti.to_eod(), 0xF5);
        assert_eq!(ApiType::from_eod(0x00), ApiType::Unknown(0xFE));
    }
}
