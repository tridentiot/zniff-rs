// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use crate::types::{
  Frame,
  Region,
};

const SOF_COMMAND: u8 = 0x23;
const SOF_FRAME: u8 = 0x21;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum ParserResult {
  ValidCommand {
    id: u8,
    payload: Vec<u8>,
  },
  ValidFrame {
    frame: Frame,
  },
  IncompleteFrame,
  InvalidFrame,
}

#[derive(Debug)]
#[derive(PartialEq)]
enum ParserState {
  AwaitStartOfFrame,
  AwaitCommandID,
  AwaitLength,
  AwaitPayload,
  AwaitType,
  AwaitTimestamp,
  AwaitChannelAndSpeed,
  AwaitRegion,
  AwaitRssi,
  AwaitStartofDataOne,
  AwaitStartofDataTwo,
}

#[derive(Debug)]
pub struct Parser {
  state: ParserState,
  command_id: u8,
  length: u8,
  payload_count: u8,
  frame_type: u8,
  timestamp_state: bool,
  rssi: u8,
  frame: Frame,
}

impl Parser {
  pub fn new() -> Self {
    Parser {
      state: ParserState::AwaitStartOfFrame,
      command_id: 0,
      length: 0,
      payload_count: 0,
      frame_type: 0,
      timestamp_state: false,
      rssi: 0,
      frame: Frame::default()
    }
  }

  fn reset(&mut self) {
    self.state = ParserState::AwaitStartOfFrame;
    self.command_id = 0;
    self.length = 0;
    self.payload_count = 0;
    self.frame_type = 0;
    self.timestamp_state = false;
    self.rssi = 0;
    self.frame = Frame::default();
  }

  pub fn parse_bytes(&mut self, input: Vec<u8>) -> ParserResult {
    for value in input {
      let result = self.parse(value);
      if result != ParserResult::IncompleteFrame {
        return result;
      }
    }
    ParserResult::IncompleteFrame
  }

  pub fn parse(&mut self, value: u8) -> ParserResult {
    //println!("{:?}", self);
    //for value in input {
      //print!("0x{:02X} ", value);
      match self.state {
        ParserState::AwaitStartOfFrame => {
          if value == SOF_COMMAND {
            self.state = ParserState::AwaitCommandID;
          } else if value == SOF_FRAME {
              self.state = ParserState::AwaitType;
          } else {
              // Do nothing since this is the idle state.
          }
        },
        ParserState::AwaitCommandID => {
          self.command_id = value;
          if matches!(self.command_id, 1 | 2 | 3 | 19 | 4 | 5 | 14) {
            self.state = ParserState::AwaitLength;
          } else {
            // Unsupported command. Reset.
            self.reset();
            return ParserResult::InvalidFrame;
          }
        },
        ParserState::AwaitType => {
          if matches!(value, 1 | 2 | 4 | 5) {
            self.frame_type = value;
            self.state = ParserState::AwaitTimestamp;
          } else {
            self.reset();
            return ParserResult::InvalidFrame;
          }
        },
        ParserState::AwaitTimestamp => {
          if self.timestamp_state == false {
            self.frame.timestamp = (value as u16) << 8;
            self.timestamp_state = true;
          } else {
              self.frame.timestamp |= value as u16;
              self.timestamp_state = false;
              self.state = ParserState::AwaitChannelAndSpeed;
          }
        },
        ParserState::AwaitChannelAndSpeed => {
          self.frame.channel = value >> 5;
          self.frame.speed = value & 0x1F;
          self.state = ParserState::AwaitRegion;
        },
        ParserState::AwaitRegion => {
          self.frame.region = match Region::try_from(value) {
            Ok(region) => {
              self.state = ParserState::AwaitRssi;
              region
            },
            Err(_) => {
              self.reset();
              return ParserResult::InvalidFrame;
            },
          };
        },
        ParserState::AwaitRssi => {
          self.frame.rssi = value;
          self.state = ParserState::AwaitStartofDataOne;
        },
        ParserState::AwaitStartofDataOne => {
          if value == 0x21 {
            self.state = ParserState::AwaitStartofDataTwo;
          } else {
              self.reset();
              return ParserResult::InvalidFrame;
          }
        },
        ParserState::AwaitStartofDataTwo => {
          if value == 0x03 {
            self.state = ParserState::AwaitLength;
          } else {
              self.reset();
              return ParserResult::InvalidFrame;
          }
        },
        ParserState::AwaitLength => {
          self.length = value;
          self.payload_count = self.length;
          if self.length == 0 {
            // No payload, so we can return the frame immediately.
            let result: ParserResult;
            // TODO: Check for command vs. frame. Assuming command for now.
            result = ParserResult::ValidCommand { id: self.command_id, payload: vec![] };
            self.reset();
            return result;
          } else {
            self.state = ParserState::AwaitPayload
          }
        },
        ParserState::AwaitPayload => {
          self.frame.payload.push(value);
          self.payload_count -= 1;
          if self.payload_count < 1 {
            let result: ParserResult;
            if matches!(self.frame_type, 1 | 2 | 4 | 5) {
              // Return a clone of the valid frame because this function
              // will continue parsing and overwrite self.frame.
              result = ParserResult::ValidFrame { frame: self.frame.clone() };
            } else {
              result = ParserResult::ValidCommand { id: self.command_id, payload: self.frame.payload.clone() };
            }
            self.reset();
            return result;
          }
        },
      } // match
    //}
    ParserResult::IncompleteFrame
  }

  pub fn timeout(&mut self) {
    self.reset();
  }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_parser() {
      let mut parser = Parser::new();

      // Start of Frame
      let result: ParserResult = parser.parse_bytes(vec![SOF_COMMAND]);
      assert_eq!(result, ParserResult::IncompleteFrame);

      parser.timeout();
      assert_eq!(parser.state, ParserState::AwaitStartOfFrame);

      let result = parser.parse_bytes(vec![SOF_FRAME]);
      assert_eq!(result, ParserResult::IncompleteFrame);
      assert_eq!(parser.state, ParserState::AwaitType);
      parser.timeout();

      let result = parser.parse_bytes(vec![
        0x21, // FRAME SOF
        0x01, // FRAME TYPE
        0x6D, 0xCE, // TIMESTAMP
        0x20, // Channel and speed
        0x00, // Region
        0x9D, // RSSI
        0x21, 0x03, // Start of data
        0x12, // Length
        0xE2, 0xEA, 0x36, 0xC3, 0x01, 0x81,
        0x0D, 0x12, 0x20, 0x0B, 0x10, 0x02,
        0x41, 0x7F, 0x7F, 0x7F, 0x7F, 0xE5,
      ]);
      assert_eq!(result, ParserResult::ValidFrame {
        frame: Frame {
          region: Region::EU,
          channel: 0x01,
          speed: 0x00,
          timestamp: 0x6DCE,
          rssi: 0x9D,
          payload: vec![
            0xE2, 0xEA, 0x36, 0xC3, 0x01, 0x81,
            0x0D, 0x12, 0x20, 0x0B, 0x10, 0x02,
            0x41, 0x7F, 0x7F, 0x7F, 0x7F, 0xE5,
          ]
        }
       });

      let result = parser.parse_bytes(vec![
        0x21, 0x01, 0x63, 0xEF, 0x02, 0x00, 0xC8, 0x21, 0x03
      ]);
      assert_eq!(result, ParserResult::IncompleteFrame);
      parser.timeout();

      println!("New frame");

      let result = parser.parse_bytes(vec![
        0x21,
        0x01,
        0x44, 0x9E,
        0x02,
        0x00,
        0xC5,
        0x21, 0x03,
        0x15,
        0xE5, 0x07, 0x76, 0x83, 0x01, 0x41, 0x0C,
        0x15, 0x31, 0x98, 0x80, 0x3F, 0xF0, 0x2A,
        0xE0, 0x8C, 0x27, 0x72, 0x3D, 0xF1, 0x14,
      ]);
      assert_eq!(result, ParserResult::ValidFrame {
        frame: Frame {
          region: Region::EU,
          channel: 0x00,
          speed: 0x02,
          timestamp: 0x449E,
          rssi: 0xC5,
          payload: vec![
            0xE5, 0x07, 0x76, 0x83, 0x01, 0x41, 0x0C,
            0x15, 0x31, 0x98, 0x80, 0x3F, 0xF0, 0x2A,
            0xE0, 0x8C, 0x27, 0x72, 0x3D, 0xF1, 0x14,
          ]
        }
       });
    }
}
