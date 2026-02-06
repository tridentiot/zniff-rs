// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
  pub region: Region,
  pub channel: u8,
  pub speed: u8,
  pub timestamp: u16,
  pub rssi: u8,
  pub payload: Vec<u8>,
}

#[repr(u8)]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub enum Region {
    #[default]
    EU = 0,
    US = 1,
    ANZ = 2,
    HK = 3,
    IN = 5,
    IL = 6,
    RU = 7,
    CN = 8,
    USLR = 9,
    EULR = 11,
    JP = 32,
    KR = 33,
}

impl FromStr for Region {
    type Err = ParseRegionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "eu" => Ok(Region::EU),
            "us" => Ok(Region::US),
            "anz" => Ok(Region::ANZ),
            "hk" => Ok(Region::HK),
            "in" => Ok(Region::IN),
            "il" => Ok(Region::IL),
            "ru" => Ok(Region::RU),
            "cn" => Ok(Region::CN),
            "uslr" => Ok(Region::USLR),
            "eulr" => Ok(Region::EULR),
            "jp" => Ok(Region::JP),
            "kr" => Ok(Region::KR),
            _ => Err(ParseRegionError),
        }
    }
}

impl TryFrom<u8> for Region {
    type Error = ParseRegionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Region::EU),
            0x01 => Ok(Region::US),
            0x02 => Ok(Region::ANZ),
            0x03 => Ok(Region::HK),
            0x05 => Ok(Region::IN),
            0x06 => Ok(Region::IL),
            0x07 => Ok(Region::RU),
            0x08 => Ok(Region::CN),
            0x09 => Ok(Region::USLR),
            0x0B => Ok(Region::EULR),
            0x20 => Ok(Region::JP),
            0x21 => Ok(Region::KR),
            _ => Err(ParseRegionError),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParseRegionError;

impl fmt::Display for ParseRegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid region")
    }
}

impl std::error::Error for ParseRegionError {}

#[repr(u8)]
/// Region identifiers as encoded in PTI (protocol trace / capture) data.
///
/// This enum represents the region values used by the PTI tooling / capture
/// format, which differ from the Z-Wave `Region` enum discriminants above.
/// When converting between `Region` and `PtiRegion`, an explicit mapping is
/// required rather than relying on the underlying numeric values.
pub enum PtiRegion {
    EU = 1,
    US = 2,
    ANZ = 3,
    HK = 4,
    IN = 5,
    IL = 9,
    RU = 8,
    CN = 11,
    USLR = 12,
    #[allow(dead_code)]
    // `USLRBACK` is a legacy/back-channel PTI region code that appears in PTI traces
    // but is not a user-selectable Z-Wave region in this crate. It is intentionally
    // not represented in `Region` and therefore cannot be produced by `From<Region>`.
    USLRBACK = 13,
    EULR = 15,
    JP = 7,
    KR = 10,
    #[allow(dead_code)]
    // This PTI region value is only used by the PTI protocol for end devices and
    // has no corresponding variant in the public `Region` enum, so it cannot be
    // produced by the `From<Region>` conversion and remains intentionally unused.
    USLRENDDEVICE = 14,
}

impl From<Region> for PtiRegion {
    fn from(region: Region) -> Self {
        match region {
            Region::EU => PtiRegion::EU,
            Region::US => PtiRegion::US,
            Region::ANZ => PtiRegion::ANZ,
            Region::HK => PtiRegion::HK,
            Region::IN => PtiRegion::IN,
            Region::IL => PtiRegion::IL,
            Region::RU => PtiRegion::RU,
            Region::CN => PtiRegion::CN,
            Region::USLR => PtiRegion::USLR,
            Region::EULR => PtiRegion::EULR,
            Region::JP => PtiRegion::JP,
            Region::KR => PtiRegion::KR,
        }
    }
}

//     SnifferPtiFrameClient.cs and PtiFrameParser.cs
// if DCH version == 2 => dch length == 11 . if DCH version == 3 => dch length == 18
// For simplicity this "PTI generator" will use version 2. therefore a frame would be:
// zniffer data:  data[0]                                       data[BeforeDataLength - 1]
//                                                                   BeforeDataLength - 1        data.Length - AfterDataLength,    RSSI  RegionIdOffset   ChannelOffset   ProtocolOffset
//      [ 5b, XX, 00, 02, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX,           F8,            ... ,              F9,                   --,       --,               --,             06,              51, 5d]
// indx:               0                                                    11                                -6                                                                                 -1

// BEAMS TAGS:
//      [ 5b, XX, 00, 02, XX, XX, XX, XX, XX, XX, XX, XX, XX, XX, F8, 55 ... , F9, XX, XX, XX, XX, 51, 5d]
// indx:               0                                          11, 12       -6                  -1

// ZWaveProtocol=0x06

impl Frame {
    pub fn to_pti_vector(&mut self) -> Result<Vec<u8>, bool> {

      let mut buffer: Vec<u8> = vec![0; 15+self.payload.len()+7];
      buffer[0] = 0x5b;
      buffer[1] = (15+self.payload.len()+7-2) as u8;
      buffer[2] = 0x00;
      buffer[3] = 0x02;
      buffer[4] = 0x00;
      buffer[5] = 0x52;
      buffer[6] = 0x6e;
      buffer[7] = 0x7d;
      buffer[8] = 0x50;
      buffer[9] = 0x12;
      buffer[10] = 0x00;
      buffer[11] = 0x2a;
      buffer[12] = 0x00;
      buffer[13] = 0x87;
      buffer[14] = 0xf8;
      for i in 0..self.payload.len(){buffer[i+15] = self.payload[i] }
      let after_data_index: usize  = 14 + self.payload.len() + 1;
      buffer[after_data_index] = 0xF9;
      buffer[after_data_index+1] = self.rssi;
      let pti_region: PtiRegion = self.region.into();
      buffer[after_data_index+2] = pti_region as u8;
      buffer[after_data_index+3] = self.channel;
      buffer[after_data_index+4] = 0x06;
      buffer[after_data_index+5] = 0x51;
      buffer[after_data_index+6] = 0x5d;


      // let buffer: Vec<u8> = vec![, , , , , , (end=5)
      // , , , , , ,   0xcc, 0x14, 0xd2, 0xf3, 0x02, 0x03,
      // 0x0f, 0x0b, 0x01, 0xf2, 0xc9, 0xf9, 0x16, 0x01, 0x00, 0x06, 0x51, 0x5d];

      // let buffer: Vec<u8> = vec![0x5b, 0x1f, 0x00, 0x02, 0x00, 0x52,
      // 0x6e, 0x7d, 0x50, 0x12, 0x00, 0x2a, 0x00, 0x87, 0xf8, 0xcc, 0x14, 0xd2, 0xf3, 0x02, 0x03,
      // 0x0f, 0x0b, 0x01, 0xf2, 0xc9, 0xf9, 0x16, 0x01, 0x00, 0x06, 0x51, 0x5d];
      Ok(buffer)
    }
}
