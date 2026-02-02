// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT

//! Silabs PTI (Packet Trace Interface) parser
//! 
//! This module implements parsing for Silabs PTI format frames, which can contain
//! Z-Wave packet data wrapped in DCH (Debug Channel) frames.
//! 
//! Reference implementations:
//! - https://github.com/SiliconLabs/z-wave-ts-silabs/blob/main/z_wave_ts_silabs/parsers.py
//! - https://github.com/Z-Wave-Alliance/z-wave-tools-core/blob/85e0d6ba2ec7f05c355b1f6e76d85ae8fea288c7/ZnifferApplication/Parsers/PtiFrameParser.cs

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
    // Minimum DCH frame size check
    if data.len() < 15 {
        return PtiParseResult::IncompleteFrame;
    }
    
    // Check start symbol
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
}
