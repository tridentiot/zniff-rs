// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use hex::FromHexError;
use zniff_rs_core::types::Region;
use crate::frame_definition::{FrameDefinition};
use crate::xml::{
    ZwClasses,
    CmdClassCmdChild,
};

#[derive(Debug)]
pub enum ZwParserError {
    InvalidHexString { c: char, index: usize },
    OddLength,
    FrameTooShort,
    FrameTooLong,
    UnknownHeaderType,
    UnknownCommandClass,
    UnknownCommand,
}

impl core::fmt::Display for ZwParserError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ZwParserError::InvalidHexString { c, index } => write!(f, "Invalid hex string: {:?} at position {}", c, index),
            ZwParserError::OddLength => write!(f, "Hex string has an odd length"),
            ZwParserError::FrameTooShort => write!(f, "Frame too short"),
            ZwParserError::FrameTooLong => write!(f, "Frame too long"),
            ZwParserError::UnknownHeaderType => write!(f, "Unknown header type"),
            ZwParserError::UnknownCommandClass => write!(f, "Unknown command class"),
            ZwParserError::UnknownCommand => write!(f, "Unknown command"),
        }
    }
}

impl From<FromHexError> for ZwParserError {
    fn from(err: FromHexError) -> Self {
        match err {
            FromHexError::InvalidHexCharacter { c, index } => {
                ZwParserError::InvalidHexString { c, index }
            },
            FromHexError::OddLength => {
                ZwParserError::OddLength
            },
            FromHexError::InvalidStringLength => {
                ZwParserError::InvalidHexString { c: ' ', index: 0 }
            },
        }
    }
}

#[derive(Debug)]
pub struct ZwParser<'a> {
    fd: &'a FrameDefinition,
    zwc: &'a ZwClasses,
}

impl<'a> ZwParser<'a> {
    pub fn new(fd: &'a FrameDefinition, zwc: &'a ZwClasses) -> Self {
        ZwParser {
            fd,
            zwc,
        }
    }

    /// Parse from text (hex), using the already-loaded config.
    pub fn parse_str(&self, region: &Region, s: &str) -> Result<(), ZwParserError> {
        let frame: Vec<u8> = match hex::decode(s) {
            Ok(f) => f,
            Err(err) => {
                return Err(ZwParserError::from(err));
            },
        };

        println!("Frame: {}", hex::encode_upper(&frame));

        fn get_header_type_name(fd: &FrameDefinition, header_type_id: u8) -> String {
            for ds in &fd.define_set {
                if ds.name != "HeaderType" {
                    continue;
                }
                for define in &ds.define {
                    let key_stripped: &str = define.key.trim_start_matches("0x");
                    let key = match u8::from_str_radix(&key_stripped, 16) {
                        Ok(k) => k,
                        Err(_) => {
                            println!("Failed to parse key: {:?}", define.key);
                            panic!("Failed to parse key");
                            //return Err(ZwParserError::UnknownHeaderType);
                        },
                    };
                    //println!("Key: {:?}, Header Type ID: {:?}", key, header_type_id);

                    if key == header_type_id {
                        return define.name.clone();
                    }
                }
            }
            "Unknown".to_string()
        }

        let key: &str = match region {
            Region::USLR | Region::EULR => "2", // Z-Wave LR frame header
            _ => "0", // Default to classic Z-Wave for other regions
        };
        //let key = "0"; // Classic Z-Wave frame header

        let mut header_type = 0u8;

        let mut byte_counter = 0;

        for base_header in &self.fd.base_header {
            if base_header.key != key {
                continue;
            }
            // Found the classic Z-Wave header.
            println!("Header type: {:?}", base_header.name);

            for param in &base_header.param {
                let n: usize = param.bits.parse::<usize>().unwrap();
                let start = byte_counter;
                let end = byte_counter + n/8;

                let value = &frame[start..end];

                println!("{} ({:?}): {:?}", param.name, param.bits, value);

                match &param.param {
                    None => {
                        // No sub-parameters. Do nothing.
                    },
                    Some(sub_params) => {
                        let mut bit_offset = 0;

                        // Convert value bytes to a u64 for bit manipulation
                        // Supports up to 64 bits (8 bytes)
                        let mut combined_value: u64 = 0;
                        for (i, &byte) in value.iter().enumerate() {
                            if i < 8 {
                                combined_value |= (byte as u64) << (i * 8);
                            }
                        }

                        for sub_param in sub_params {
                            let n: usize = sub_param.bits.parse::<usize>().unwrap();

                            let sub_value = (combined_value >> bit_offset) & ((1u64 << n) - 1);
                            println!("{} ({:?}): 0x{:X}", sub_param.name, sub_param.bits, sub_value);
                            bit_offset += n;

                            // Save header type
                            if sub_param.name == "HeaderType" {
                                header_type = sub_value as u8;
                            }
                        }
                    }
                };
                byte_counter += n / 8;
            }

            let header_type_name = get_header_type_name(&self.fd, header_type);
            println!("Header type: {}", header_type_name);

            // Process fields from header
            for header in &self.fd.header {
                if header.name == header_type_name.clone().to_uppercase() {
                    //println!("Header: {:?}", header.name);
                    for param in &header.param {
                        let n: usize = param.bits.parse::<usize>().unwrap();
                        let start = byte_counter;
                        let end = byte_counter + n/8;

                        println!("{}: {:?}", param.param_text, &frame[start..end]);
                        byte_counter += n / 8;
                    }
                }
            }
        }

        println!("Byte counter: {:?}", byte_counter);

        let frame = frame[byte_counter..].to_vec(); // Skip header bytes already processed.

        println!("Updated frame: {}", hex::encode_upper(&frame));

        for class in &self.zwc.cmd_class {
            let key_stripped: &str = class.key.trim_start_matches("0x");
            let cc: u8 = match u8::from_str_radix(key_stripped, 16) {
                Ok(k) => k,
                Err(_) => {
                    println!("Failed to parse key: {:?}", class.key);
                    return Err(ZwParserError::UnknownCommandClass);
                },
            };
            if cc == frame[0] {
                println!("CC: {} (Version {})", class.help, class.version);

                match &class.cmd {
                    None => {
                        println!("No commands found for this class.");
                    },
                    Some(cmds) => {
                        for cmd in cmds {
                            let cmd_key_stripped: &str = cmd.key.trim_start_matches("0x");
                            let cmd_id: u8 = match u8::from_str_radix(cmd_key_stripped, 16) {
                                Ok(k) => k,
                                Err(_) => {
                                    println!("Failed to parse cmd key: {:?}", cmd.key);
                                    return Err(ZwParserError::UnknownCommand);
                                },
                            };
                            if cmd_id == frame[1] {
                                println!("CMD: {}", cmd.help);

                                match &cmd.children {
                                    None => {
                                        println!(" No parameters found for this command.");
                                    },
                                    Some(children) => {
                                        let mut byte_counter = 2; // Start after CC and CMD
                                        for p in children {
                                            if byte_counter >= frame.len() {
                                                //println!("{:?} ({:?}): <not supported>", p.name, p.param_type);
                                                println!("Parameter not supported: {:?}", p);
                                            } else {
                                                match p {
                                                    CmdClassCmdChild::Param(p) => {
                                                        print!("{} ({}): ", p.name, p.param_type);
                                                        match p.param_type.as_str() {
                                                            "BYTE" => {
                                                                let value: u8 = frame[byte_counter];
                                                                byte_counter += 1;
                                                                println!("0x{:02X}", value);
                                                            },
                                                            _ => {
                                                                println!("Unsupported parameter type: {:?}", p.param_type);
                                                            }
                                                        }
                                                    },
                                                    CmdClassCmdChild::VariantGroup(vg) => {
                                                        println!(" Variant Group: {:?}", vg.name);
                                                    },
                                                };
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                };
            }
        }
        Ok(())
    }
}

// This section contains the unit tests.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame_definition;
    use crate::xml;

    #[test]
    fn test_parse_str() {
        let fd = frame_definition::parse_xml();
        let zwc = xml::parse_xml();
        let zw_parser: ZwParser = ZwParser::new(&fd, &zwc);

        // Validate successful parsing
        let region: Region = Region::US;
        let input = "E62E1DB102030F0B0170A1";
        let result = zw_parser.parse_str(&region, input);
        assert!(result.is_ok(), "Parsing failed: {:?}", result.err());

        // Validate odd length error
        let region: Region = Region::US;
        let input = "E62E1DB102030F0B0170A12";
        let result = zw_parser.parse_str(&region, input);
        assert!(result.is_err(), "Parsing should have failed for odd length input");
        assert!(matches!(result.as_ref().err(), Some(ZwParserError::OddLength)), "Expected OddLength error, got: {:?}", result.err());

        // Validate invalid character error
        let region: Region = Region::US;
        let input = "E62E1DB102030F0B0170AG";
        let result = zw_parser.parse_str(&region, input);
        assert!(result.is_err(), "Parsing should have failed for invalid character input");
        assert!(matches!(result.as_ref().err(), Some(ZwParserError::InvalidHexString { c: 'G', index: 21 })), "Expected InvalidHexString error with character 'G' at index 22, got: {:?}", result.err());

        // Validate a LR frame parsing
        let region: Region = Region::USLR;
        let input = "EDDFB4690011000F031297FAF0946B";
        let result = zw_parser.parse_str(&region, input);
        assert!(result.is_ok(), "Parsing failed for LR frame: {:?}", result.err());
    }
}
