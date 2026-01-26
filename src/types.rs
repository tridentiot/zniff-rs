// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Frame {
  pub region: u8,
  pub channel: u8,
  pub speed: u8,
  pub timestamp: u16,
  pub rssi: u8,
  pub payload: Vec<u8>,
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

      let zpal_to_pti_region = HashMap::from([
        (0x00, 0x01), //EU
        (0x01, 0x02), //US
        (0x02, 0x03), //ANZ
        (0x03, 0x04), //HK
        (0x05, 0x05), //IN
        (0x06, 0x09), //IL
        (0x07, 0x08), //RU
        (0x08, 0x0B), //CN
        (0x09, 0x0C), //US_LR
        (0x0A, 0x0D), //US_LR_BACK
        (0x0B, 0x0F), //EU_LR
        (0x20, 0x07), //JP
        (0x21, 0x0A), //KR
        (0x30, 0x0E), //US_LR_END_DEVICE
        (0xFF, 0x01), //DEFAULT
      ]);

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
      buffer[after_data_index+2] = *zpal_to_pti_region.get(&self.region).unwrap() as u8;
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
