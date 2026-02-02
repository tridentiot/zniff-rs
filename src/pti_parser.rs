// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT

//! Silabs PTI (Packet Trace Interface) parser
//! 
//! This module implements parsing for Silabs PTI format frames, which can contain
//! Z-Wave packet data wrapped in DCH (Debug Channel) frames.
//! 
//! ## Overview
//! 
//! PTI (Packet Trace Interface) is Silicon Labs' protocol for transmitting packet trace data.
//! It wraps radio packets with metadata including RSSI, channel, region, and protocol information.
//! 
//! ## Frame Structure
//! 
//! ### DCH (Debug Channel) Frame
//! 
//! DCH frames encapsulate PTI data for transmission over TCP or other transports:
//! 
//! ```text
//! DCH v2: [START][LEN(2)][VER(2)][TS(6)][TYPE(2)][SEQ(1)][PTI_PAYLOAD][END]
//! DCH v3: [START][LEN(2)][VER(2)][TS(8)][TYPE(2)][FLAGS(4)][SEQ(2)][PTI_PAYLOAD][END]
//! ```
//! 
//! ### PTI Frame
//! 
//! PTI frames contain the actual radio packet data:
//! 
//! ```text
//! [HW_START][OTA_DATA...][HW_END][APPENDED_INFO]
//! ```
//! 
//! Where APPENDED_INFO (parsed backward) is:
//! ```text
//! [RSSI (Rx only)][RADIO_CONFIG][RADIO_INFO][STATUS_0][APPENDED_INFO_CFG]
//! ```
//! 
//! ## Usage
//! 
//! ### Parsing a complete DCH frame
//! 
//! ```rust
//! # use zniff_rs::pti_parser::{parse_dch_frame, PtiParseResult};
//! # fn example() {
//! let dch_data: Vec<u8> = vec![/* ... */];
//! match parse_dch_frame(&dch_data) {
//!     PtiParseResult::ValidFrame { frame } => {
//!         println!("Received Z-Wave frame: {:?}", frame);
//!     },
//!     PtiParseResult::IncompleteFrame => {
//!         // Need more data
//!     },
//!     PtiParseResult::InvalidFrame => {
//!         // Not a valid PTI frame
//!     },
//! }
//! # }
//! ```
//! 
//! ### Parsing PTI frame directly (without DCH wrapper)
//! 
//! ```rust
//! # use zniff_rs::pti_parser::{parse_pti_frame, PtiParseResult};
//! # fn example() {
//! let pti_data: Vec<u8> = vec![/* ... */];
//! match parse_pti_frame(&pti_data) {
//!     PtiParseResult::ValidFrame { frame } => {
//!         println!("Region: {:?}, Channel: {}, RSSI: {}", 
//!                  frame.region, frame.channel, frame.rssi);
//!     },
//!     _ => {},
//! }
//! # }
//! ```
//! 
//! ## Reference implementations
//! 
//! - [Python implementation](https://github.com/SiliconLabs/z-wave-ts-silabs/blob/main/z_wave_ts_silabs/parsers.py)
//! - [C# implementation](https://github.com/Z-Wave-Alliance/z-wave-tools-core/blob/85e0d6ba2ec7f05c355b1f6e76d85ae8fea288c7/ZnifferApplication/Parsers/PtiFrameParser.cs)

use crate::types::{Frame, Region};

/// DCH frame symbols
const DCH_START_SYMBOL: u8 = 0x5B; // '['
const DCH_END_SYMBOL: u8 = 0x5D;   // ']'

/// PTI hardware symbols
const HW_RX_START: u8 = 0xF8;
const HW_TX_START: u8 = 0xFC;
const HW_RX_SUCCESS: u8 = 0xF9;
const HW_TX_SUCCESS: u8 = 0xFD;

/// PTI protocol ID for Z-Wave
const PTI_PROTOCOL_ZWAVE: u8 = 0x06;

/// PTI region codes (different from internal Region enum)
/// Maps PTI region values to internal Region enum
fn pti_region_to_region(pti_region: u8) -> Option<Region> {
    match pti_region {
        0x01 => Some(Region::EU),
        0x02 => Some(Region::US),
        0x03 => Some(Region::ANZ),
        0x04 => Some(Region::HK),
        0x05 => Some(Region::IN),
        0x09 => Some(Region::IL),
        0x08 => Some(Region::RU),
        0x0B => Some(Region::CN),
        0x0C => Some(Region::USLR),
        0x0F => Some(Region::EULR),
        0x07 => Some(Region::JP),
        0x0A => Some(Region::KR),
        _ => None,
    }
}

/// Determine speed/baud rate from channel and region combination
fn get_speed(channel: u8, region: u8) -> u8 {
    let key = ((channel as u16) << 8) | (region as u16);
    
    // Long Range speeds
    let lr_speeds = [
        0x0303, // CH3 + US_LR1 (0x0C)
        0x0303, // CH3 + US_LR2 (0x0D)
        0x0003, // CH0 + US_LR3 (0x0E)
        0x0103, // CH1 + US_LR3 (0x0E)
        0x0303, // CH3 + EU_LR1 (0x0F)
        0x0303, // CH3 + EU_LR2 (0x10)
        0x0003, // CH0 + EU_LR3 (0x11)
        0x0103, // CH1 + EU_LR3 (0x11)
    ];
    
    // 9600 baud (speed = 0)
    let baud_9600 = [
        0x0201, 0x0202, 0x0203, 0x0204, 0x0205, 0x0206,
        0x0208, 0x0209, 0x020B, 0x020C, 0x020D, 0x020F, 0x0210,
    ];
    
    // 40K baud (speed = 1)
    let baud_40k = [
        0x0101, 0x0102, 0x0103, 0x0104, 0x0105, 0x0106,
        0x0108, 0x0109, 0x010B, 0x010C, 0x010D, 0x010F, 0x0110,
    ];
    
    // 100K baud (speed = 2)
    let baud_100k = [
        0x0001, 0x0002, 0x0003, 0x0004, 0x0005, 0x0006,
        0x0007, 0x0107, 0x0207, 0x0008, 0x0009, 0x000A,
        0x010A, 0x020A, 0x000B, 0x000C, 0x000D, 0x000F, 0x0010,
    ];
    
    if baud_9600.contains(&key) {
        0
    } else if baud_40k.contains(&key) {
        1
    } else if baud_100k.contains(&key) {
        2
    } else if lr_speeds.contains(&key) {
        3
    } else {
        0 // default
    }
}

/// Result of parsing a PTI packet
#[derive(Debug, Clone, PartialEq)]
pub enum PtiParseResult {
    /// Successfully parsed a Z-Wave frame
    ValidFrame { frame: Frame },
    /// Not a valid PTI frame or not Z-Wave
    InvalidFrame,
    /// Incomplete data, need more bytes
    IncompleteFrame,
}

/// Parse a DCH frame containing PTI data
/// 
/// DCH frames have the format:
/// [START_SYMBOL][LENGTH(2)][VERSION(2)][TIMESTAMP(4-8)][DCH_TYPE(2)][FLAGS(4)][SEQ(2)][PAYLOAD][END_SYMBOL]
pub fn parse_dch_frame(data: &[u8]) -> PtiParseResult {
    // Check for minimum possible DCH frame (need at least start, length, version, end)
    if data.len() < 6 {
        return PtiParseResult::IncompleteFrame;
    }
    
    // Check start symbol first
    if data[0] != DCH_START_SYMBOL {
        return PtiParseResult::InvalidFrame;
    }
    
    // Parse length (2 bytes, little-endian)
    let length = u16::from_le_bytes([data[1], data[2]]) as usize;
    
    // Check if we have the complete frame (+2 for start/end symbols)
    if data.len() < length + 2 {
        return PtiParseResult::IncompleteFrame;
    }
    
    // Check end symbol
    let end_symbol_index = length + 1;
    if data[end_symbol_index] != DCH_END_SYMBOL {
        return PtiParseResult::InvalidFrame;
    }
    
    // Parse version (2 bytes, little-endian)
    let version = u16::from_le_bytes([data[3], data[4]]);
    
    // Determine header size based on version
    let (header_size, _timestamp_size) = match version {
        2 => (13, 6), // DCHv2: 13 bytes header, 6 bytes timestamp
        3 => (20, 8), // DCHv3: 20 bytes header, 8 bytes timestamp
        _ => return PtiParseResult::InvalidFrame,
    };
    
    // Check if there's payload
    if length <= header_size {
        return PtiParseResult::InvalidFrame;
    }
    
    // Extract PTI payload (between header and end symbol)
    let pti_start = if version == 2 { 14 } else { 21 };
    let pti_payload = &data[pti_start..end_symbol_index];
    
    // Parse the PTI frame within the DCH payload
    parse_pti_frame(pti_payload)
}

/// Parse a PTI frame to extract Z-Wave data
/// 
/// PTI frame format:
/// [HW_START][OTA_DATA...][HW_END][APPENDED_INFO]
/// 
/// APPENDED_INFO format (parsed backward):
/// [APPENDED_INFO_CFG][STATUS_0][RADIO_INFO][RADIO_CONFIG][RSSI (Rx only)]
fn parse_pti_frame(data: &[u8]) -> PtiParseResult {
    // Minimum PTI frame size: HW_START(1) + HW_END(1) + APPENDED_INFO(4) = 6
    if data.len() < 6 {
        return PtiParseResult::InvalidFrame;
    }
    
    let hw_start = data[0];
    
    // Check for valid HW_START
    if hw_start != HW_RX_START && hw_start != HW_TX_START {
        return PtiParseResult::InvalidFrame;
    }
    
    // Parse APPENDED_INFO backward from the end
    let mut idx = data.len() - 1;
    
    // APPENDED_INFO_CFG (1 byte)
    let appended_info_cfg = data[idx];
    let is_rx = (appended_info_cfg & 0b01000000) >> 6;
    let appended_info_length = ((appended_info_cfg & 0b00111000) >> 3) + 3;
    idx -= 1;
    
    // STATUS_0 (1 byte)
    let status_0 = data[idx];
    let protocol_id = status_0 & 0x0F;
    idx -= 1;
    
    // Only process Z-Wave frames
    if protocol_id != PTI_PROTOCOL_ZWAVE {
        return PtiParseResult::InvalidFrame;
    }
    
    // RADIO_INFO (1 byte)
    let radio_info = data[idx];
    let channel = radio_info & 0b00111111;
    idx -= 1;
    
    // RADIO_CONFIG (1 byte for Z-Wave)
    let radio_config = data[idx];
    let pti_region = radio_config & 0b00011111;
    idx -= 1;
    
    // Convert PTI region to internal Region enum
    let region = match pti_region_to_region(pti_region) {
        Some(r) => r,
        None => return PtiParseResult::InvalidFrame,
    };
    
    // RSSI (1 byte, only for Rx)
    let rssi = if is_rx == 1 {
        let rssi_raw = data[idx] as i8;
        // PTI version 1+ requires RSSI offset
        let appended_info_version = appended_info_cfg & 0b00000111;
        let rssi_value = if appended_info_version >= 1 {
            rssi_raw.saturating_sub(0x32)
        } else {
            rssi_raw
        };
        // Convert to u8, taking absolute value for negative RSSI
        rssi_value.unsigned_abs()
    } else {
        0 // Tx frames don't have RSSI
    };
    
    // HW_END position (it comes just before the appended info)
    let hw_end_pos = data.len() - 1 - appended_info_length as usize;
    let hw_end = data[hw_end_pos];
    
    // Check for valid HW_END
    let expected_hw_end = if is_rx == 1 { HW_RX_SUCCESS } else { HW_TX_SUCCESS };
    if hw_end != expected_hw_end {
        return PtiParseResult::InvalidFrame;
    }
    
    // Extract OTA (Over-The-Air) packet data
    let ota_data = &data[1..hw_end_pos];
    
    // Skip wake-up beams (start with 0x55)
    if ota_data.is_empty() || ota_data[0] == 0x55 {
        return PtiParseResult::InvalidFrame;
    }
    
    // Determine speed from channel and region
    let speed = get_speed(channel, pti_region);
    
    // Create Z-Wave frame
    let frame = Frame {
        region,
        channel,
        speed,
        timestamp: 0, // PTI doesn't include timestamp in the same way
        rssi,
        payload: ota_data.to_vec(),
    };
    
    PtiParseResult::ValidFrame { frame }
}

/// Stateful PTI parser for handling streaming data
/// 
/// This parser maintains an internal buffer and can process data incrementally,
/// making it suitable for TCP streams where DCH frames may arrive in chunks.
pub struct PtiParser {
    buffer: Vec<u8>,
}

impl PtiParser {
    /// Create a new PTI parser
    pub fn new() -> Self {
        PtiParser {
            buffer: Vec::new(),
        }
    }
    
    /// Parse incoming data, returning any complete frames found
    /// 
    /// This function appends new data to the internal buffer and attempts to parse
    /// complete DCH frames. It returns all successfully parsed frames and removes
    /// them from the buffer.
    /// 
    /// # Arguments
    /// * `data` - New data to parse
    /// 
    /// # Returns
    /// A vector of successfully parsed frames. The vector may be empty if no complete
    /// frames were found in the data.
    pub fn parse(&mut self, data: &[u8]) -> Vec<Frame> {
        self.buffer.extend_from_slice(data);
        
        let mut frames = Vec::new();
        let mut consumed = 0;
        
        // Try to parse frames from the buffer
        while consumed < self.buffer.len() {
            let remaining = &self.buffer[consumed..];
            
            // Need at least 6 bytes to determine if this is a valid DCH frame
            if remaining.len() < 6 {
                break;
            }
            
            // Check for DCH start symbol
            if remaining[0] != DCH_START_SYMBOL {
                // Skip this byte and continue
                consumed += 1;
                continue;
            }
            
            // Parse length to see if we have a complete frame
            let length = u16::from_le_bytes([remaining[1], remaining[2]]) as usize;
            let frame_size = length + 2; // +2 for start and end symbols
            
            if remaining.len() < frame_size {
                // Not enough data for complete frame yet
                break;
            }
            
            // Try to parse this frame
            match parse_dch_frame(&remaining[..frame_size]) {
                PtiParseResult::ValidFrame { frame } => {
                    frames.push(frame);
                    consumed += frame_size;
                },
                PtiParseResult::IncompleteFrame => {
                    // This shouldn't happen since we checked the length
                    break;
                },
                PtiParseResult::InvalidFrame => {
                    // Skip this start symbol and try again
                    consumed += 1;
                },
            }
        }
        
        // Remove consumed data from buffer
        if consumed > 0 {
            self.buffer.drain(..consumed);
        }
        
        frames
    }
    
    /// Reset the parser, clearing the internal buffer
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
    
    /// Get the current buffer size
    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }
}

impl Default for PtiParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_pti_frame_rx() {
        // Sample PTI frame with Rx data
        // HW_START(0xF8) + OTA_DATA + HW_END(0xF9) + APPENDED_INFO
        let pti_data = vec![
            0xF8, // HW_RX_START
            // OTA data (Z-Wave packet)
            0x01, 0x02, 0x03, 0x04, 0x05,
            0xF9, // HW_RX_SUCCESS
            // APPENDED_INFO (5 bytes for Rx)
            0x9D,       // RSSI (signed)
            0x01,       // RADIO_CONFIG: EU region (0x01)
            0x01,       // RADIO_INFO: channel 1
            0x06,       // STATUS_0: Z-Wave protocol
            0x52,       // APPENDED_INFO_CFG: is_rx=1, length=2 (for 5 total bytes), version=2
        ];
        
        match parse_pti_frame(&pti_data) {
            PtiParseResult::ValidFrame { frame } => {
                assert_eq!(frame.region, Region::EU);
                assert_eq!(frame.channel, 1);
                assert_eq!(frame.payload, vec![0x01, 0x02, 0x03, 0x04, 0x05]);
            }
            _ => panic!("Expected ValidFrame"),
        }
    }
    
    #[test]
    fn test_parse_pti_frame_invalid_protocol() {
        // PTI frame with non-Z-Wave protocol
        let pti_data = vec![
            0xF8, // HW_RX_START
            0x01, 0x02, 0x03,
            0xF9, // HW_RX_SUCCESS
            0x9D,       // RSSI
            0x01,       // RADIO_CONFIG
            0x01,       // RADIO_INFO
            0x05,       // STATUS_0: BLE protocol (not Z-Wave)
            0x52,       // APPENDED_INFO_CFG: is_rx=1, length=2, version=2
        ];
        
        match parse_pti_frame(&pti_data) {
            PtiParseResult::InvalidFrame => {},
            _ => panic!("Expected InvalidFrame for non-Z-Wave protocol"),
        }
    }
    
    #[test]
    fn test_parse_dch_frame() {
        // Sample DCH v2 frame with PTI payload
        let dch_data = vec![
            0x5B,       // DCH_START
            0x19, 0x00, // LENGTH (25 bytes - header + payload, not including start/end symbols)
            0x02, 0x00, // VERSION (2)
            // Timestamp (6 bytes)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, // DCH_TYPE
            0x00,       // SEQUENCE
            // PTI payload starts here (index 14)
            0xF8,       // HW_RX_START
            0x01, 0x02, 0x03, 0x04, 0x05, // OTA data (5 bytes)
            0xF9,       // HW_RX_SUCCESS
            0x9D,       // RSSI
            0x01,       // RADIO_CONFIG: EU
            0x01,       // RADIO_INFO: channel 1
            0x06,       // STATUS_0: Z-Wave
            0x52,       // APPENDED_INFO_CFG: is_rx=1, length=2, version=2
            0x5D,       // DCH_END
        ];
        
        match parse_dch_frame(&dch_data) {
            PtiParseResult::ValidFrame { frame } => {
                assert_eq!(frame.region, Region::EU);
                assert_eq!(frame.payload, vec![0x01, 0x02, 0x03, 0x04, 0x05]);
            }
            _ => panic!("Expected ValidFrame from DCH frame"),
        }
    }
    
    #[test]
    fn test_incomplete_frame() {
        let incomplete_data = vec![0x5B, 0x1C, 0x00];
        match parse_dch_frame(&incomplete_data) {
            PtiParseResult::IncompleteFrame => {},
            _ => panic!("Expected IncompleteFrame"),
        }
    }
    
    #[test]
    fn test_parse_pti_frame_tx() {
        // Sample PTI frame with Tx data (no RSSI)
        let pti_data = vec![
            0xFC, // HW_TX_START
            // OTA data (Z-Wave packet)
            0x01, 0x02, 0x03, 0x04,
            0xFD, // HW_TX_SUCCESS
            // APPENDED_INFO (4 bytes for Tx, no RSSI)
            0x02,       // RADIO_CONFIG: US region (0x02)
            0x02,       // RADIO_INFO: channel 2
            0x06,       // STATUS_0: Z-Wave protocol
            0x08,       // APPENDED_INFO_CFG: is_rx=0, length=1 (for 4 total bytes), version=0
        ];
        
        match parse_pti_frame(&pti_data) {
            PtiParseResult::ValidFrame { frame } => {
                assert_eq!(frame.region, Region::US);
                assert_eq!(frame.channel, 2);
                assert_eq!(frame.rssi, 0); // Tx has no RSSI
                assert_eq!(frame.payload, vec![0x01, 0x02, 0x03, 0x04]);
            }
            _ => panic!("Expected ValidFrame for Tx"),
        }
    }
    
    #[test]
    fn test_parse_pti_frame_different_speed() {
        // Test 100K speed (speed=2) with US region and channel 0
        let pti_data = vec![
            0xF8, // HW_RX_START
            0x01, 0x02, 0x03,
            0xF9, // HW_RX_SUCCESS
            0x80,       // RSSI
            0x02,       // RADIO_CONFIG: US region (0x02)
            0x00,       // RADIO_INFO: channel 0
            0x06,       // STATUS_0: Z-Wave protocol
            0x52,       // APPENDED_INFO_CFG: is_rx=1, length=2, version=2
        ];
        
        match parse_pti_frame(&pti_data) {
            PtiParseResult::ValidFrame { frame } => {
                assert_eq!(frame.region, Region::US);
                assert_eq!(frame.speed, 2); // 100K based on channel 0 + US region
            }
            _ => panic!("Expected ValidFrame"),
        }
    }
    
    #[test]
    fn test_parse_pti_frame_wakeup_beam() {
        // Frame starting with 0x55 should be rejected (wake-up beam)
        let pti_data = vec![
            0xF8, // HW_RX_START
            0x55, 0x55, 0x55, // Wake-up beam pattern
            0xF9, // HW_RX_SUCCESS
            0x80, 0x01, 0x01, 0x06, 0x52,
        ];
        
        match parse_pti_frame(&pti_data) {
            PtiParseResult::InvalidFrame => {},
            _ => panic!("Expected InvalidFrame for wake-up beam"),
        }
    }
    
    #[test]
    fn test_parse_dch_v3_frame() {
        // DCH v3 frame has 8-byte timestamp and additional fields
        let dch_data = vec![
            0x5B,       // DCH_START
            0x1E, 0x00, // LENGTH (30 bytes for v3 header + payload)
            0x03, 0x00, // VERSION (3)
            // Timestamp (8 bytes for v3)
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, // DCH_TYPE
            0x00, 0x00, 0x00, 0x00, // FLAGS (4 bytes)
            0x00, 0x00, // SEQUENCE (2 bytes)
            // PTI payload starts here (index 21)
            0xF8,       // HW_RX_START
            0x01, 0x02, 0x03, // OTA data (3 bytes)
            0xF9,       // HW_RX_SUCCESS
            0x80, 0x01, 0x01, 0x06, 0x52, // APPENDED_INFO (5 bytes)
            0x5D,       // DCH_END
        ];
        
        match parse_dch_frame(&dch_data) {
            PtiParseResult::ValidFrame { frame } => {
                assert_eq!(frame.region, Region::EU);
                assert_eq!(frame.payload, vec![0x01, 0x02, 0x03]);
            }
            _ => panic!("Expected ValidFrame from DCH v3 frame"),
        }
    }
    
    #[test]
    fn test_invalid_dch_start_symbol() {
        let invalid_data = vec![0x5C, 0x10, 0x00, 0x02, 0x00, 0x00]; // 6 bytes, wrong start
        match parse_dch_frame(&invalid_data) {
            PtiParseResult::InvalidFrame => {},
            _ => panic!("Expected InvalidFrame for bad start symbol"),
        }
    }
    
    #[test]
    fn test_invalid_dch_end_symbol() {
        let invalid_data = vec![
            0x5B,       // DCH_START (correct)
            0x05, 0x00, // LENGTH = 5
            0x02, 0x00, // VERSION
            0x00,       // Some data
            0x5C,       // Wrong end symbol (should be 0x5D)
        ];
        match parse_dch_frame(&invalid_data) {
            PtiParseResult::InvalidFrame => {},
            _ => panic!("Expected InvalidFrame for bad end symbol"),
        }
    }
    
    #[test]
    fn test_streaming_parser_single_frame() {
        let mut parser = PtiParser::new();
        
        let dch_data = vec![
            0x5B, 0x19, 0x00, 0x02, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00,
            0xF8, 0x01, 0x02, 0x03, 0x04, 0x05,
            0xF9, 0x9D, 0x01, 0x01, 0x06, 0x52,
            0x5D,
        ];
        
        let frames = parser.parse(&dch_data);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].region, Region::EU);
        assert_eq!(frames[0].payload, vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(parser.buffer_len(), 0);
    }
    
    #[test]
    fn test_streaming_parser_partial_frame() {
        let mut parser = PtiParser::new();
        
        // Send first half of frame
        let part1 = vec![
            0x5B, 0x19, 0x00, 0x02, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        
        let frames = parser.parse(&part1);
        assert_eq!(frames.len(), 0); // No complete frames yet
        assert!(parser.buffer_len() > 0); // Data is buffered
        
        // Send second half
        let part2 = vec![
            0x00, 0xF8, 0x01, 0x02, 0x03, 0x04, 0x05,
            0xF9, 0x9D, 0x01, 0x01, 0x06, 0x52, 0x5D,
        ];
        
        let frames = parser.parse(&part2);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].payload, vec![0x01, 0x02, 0x03, 0x04, 0x05]);
        assert_eq!(parser.buffer_len(), 0);
    }
    
    #[test]
    fn test_streaming_parser_multiple_frames() {
        let mut parser = PtiParser::new();
        
        // Two complete frames in one data block
        // Frame 1: v2 DCH with 3-byte OTA
        let mut data = vec![
            0x5B, 0x17, 0x00, // START + LENGTH(23)
            0x02, 0x00, // VERSION
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // TIMESTAMP
            0x00, 0x00, // TYPE
            0x00, // SEQ
            0xF8, 0x01, 0x02, 0x03, // PTI: START + 3 OTA
            0xF9, 0x9D, 0x01, 0x01, 0x06, 0x52, // END + APPENDED_INFO
            0x5D, // DCH END
        ];
        
        // Add second frame with 3-byte OTA
        data.extend_from_slice(&[
            0x5B, 0x17, 0x00, // START + LENGTH(23)
            0x02, 0x00, // VERSION
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // TIMESTAMP
            0x00, 0x00, // TYPE
            0x00, // SEQ
            0xF8, 0x04, 0x05, 0x06, // PTI: START + 3 OTA
            0xF9, 0x80, 0x02, 0x02, 0x06, 0x52, // END + APPENDED_INFO
            0x5D, // DCH END
        ]);
        
        let frames = parser.parse(&data);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].payload, vec![0x01, 0x02, 0x03]);
        assert_eq!(frames[1].payload, vec![0x04, 0x05, 0x06]);
        assert_eq!(frames[1].region, Region::US);
    }
    
    #[test]
    fn test_streaming_parser_skip_garbage() {
        let mut parser = PtiParser::new();
        
        // Garbage bytes followed by a valid frame
        let mut data = vec![0xFF, 0xFF, 0x00, 0x00, 0xFF]; // Garbage
        data.extend_from_slice(&[
            0x5B, 0x17, 0x00, // START + LENGTH(23)
            0x02, 0x00, // VERSION
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // TIMESTAMP
            0x00, 0x00, // TYPE
            0x00, // SEQ
            0xF8, 0x01, 0x02, 0x03, // PTI: START + 3 OTA
            0xF9, 0x9D, 0x01, 0x01, 0x06, 0x52, // END + APPENDED_INFO
            0x5D, // DCH END
        ]);
        
        let frames = parser.parse(&data);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].payload, vec![0x01, 0x02, 0x03]);
    }
    
    #[test]
    fn test_streaming_parser_reset() {
        let mut parser = PtiParser::new();
        
        let part1 = vec![0x5B, 0x19, 0x00, 0x02, 0x00];
        parser.parse(&part1);
        assert!(parser.buffer_len() > 0);
        
        parser.reset();
        assert_eq!(parser.buffer_len(), 0);
    }
}
