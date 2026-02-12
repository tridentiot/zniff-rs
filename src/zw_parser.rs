// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use crate::frame_definition::{FrameDefinition};
use crate::xml::{
    ZwClasses,
    CmdClassCmdChild,
};
use crate::xml_output::ParsedFrame;

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
    pub fn parse_str(&self, s: &str) {
        let frame: Vec<u8> = match hex::decode(s) {
            Ok(f) => f,
            Err(_) => panic!("Failed to decode hex string"),
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

        let key = "0"; // Classic Z-Wave frame header

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
                        for sub_param in sub_params {
                            let n: usize = sub_param.bits.parse::<usize>().unwrap();
                            let sub_value = (value[0] >> bit_offset) & ((1 << n) - 1);
                            println!("{} ({:?}): {:02X}", sub_param.name, sub_param.bits, sub_value);
                            bit_offset += n;

                            // Save header type
                            if sub_param.name == "HeaderType" {
                                header_type = sub_value;
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
                    panic!("Failed to parse key");
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
                                    panic!("Failed to parse cmd key");
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
    }

    /// Parse from bytes, returning a ParsedFrame structure suitable for XML output.
    pub fn parse_bytes(&self, frame: &[u8]) -> ParsedFrame {
        let mut parsed_frame = ParsedFrame::new();

        // Add raw frame data
        parsed_frame.add_field(
            "RawFrame".to_string(),
            hex::encode_upper(&frame),
            Some("hex".to_string())
        );

        fn get_header_type_name(fd: &FrameDefinition, header_type_id: u8) -> String {
            for ds in &fd.define_set {
                if ds.name != "HeaderType" {
                    continue;
                }
                for define in &ds.define {
                    let key_stripped: &str = define.key.trim_start_matches("0x");
                    let key = match u8::from_str_radix(&key_stripped, 16) {
                        Ok(k) => k,
                        Err(_) => return "Unknown".to_string(),
                    };
                    if key == header_type_id {
                        return define.name.clone();
                    }
                }
            }
            "Unknown".to_string()
        }

        let key = "0"; // Classic Z-Wave frame header
        let mut header_type = 0u8;
        let mut byte_counter = 0;

        // Parse base header
        for base_header in &self.fd.base_header {
            if base_header.key != key {
                continue;
            }

            parsed_frame.add_field(
                "HeaderName".to_string(),
                base_header.name.clone(),
                Some("string".to_string())
            );

            for param in &base_header.param {
                let n: usize = param.bits.parse::<usize>().unwrap_or(0);
                if n == 0 {
                    continue;
                }
                let start = byte_counter;
                let end = byte_counter + n/8;

                if end > frame.len() {
                    break;
                }

                let value = &frame[start..end];

                match &param.param {
                    None => {
                        parsed_frame.add_field(
                            param.name.clone(),
                            hex::encode_upper(value),
                            Some(format!("{} bits", param.bits))
                        );
                    },
                    Some(sub_params) => {
                        let mut bit_offset = 0;
                        for sub_param in sub_params {
                            let n: usize = sub_param.bits.parse::<usize>().unwrap_or(0);
                            if n == 0 {
                                continue;
                            }
                            let sub_value = &value[0] >> bit_offset & ((1 << n) - 1);
                            
                            parsed_frame.add_field(
                                sub_param.name.clone(),
                                format!("0x{:02X}", sub_value),
                                Some(format!("{} bits", sub_param.bits))
                            );
                            
                            bit_offset += n;

                            if sub_param.name == "HeaderType" {
                                header_type = sub_value;
                            }
                        }
                    }
                };
                byte_counter += n / 8;
            }

            let header_type_name = get_header_type_name(&self.fd, header_type);
            parsed_frame.add_field(
                "HeaderTypeName".to_string(),
                header_type_name.to_uppercase(),
                Some("string".to_string())
            );

            // Process fields from header
            for header in &self.fd.header {
                if header.name == header_type_name.clone().to_uppercase() {
                    for param in &header.param {
                        let n: usize = param.bits.parse::<usize>().unwrap_or(0);
                        if n == 0 {
                            continue;
                        }
                        let start = byte_counter;
                        let end = byte_counter + n/8;

                        if end > frame.len() {
                            break;
                        }

                        parsed_frame.add_field(
                            param.param_text.clone(),
                            hex::encode_upper(&frame[start..end]),
                            Some(format!("{} bits", param.bits))
                        );
                        byte_counter += n / 8;
                    }
                }
            }
        }

        if byte_counter >= frame.len() {
            return parsed_frame;
        }

        let payload = frame[byte_counter..].to_vec();

        // Parse command class and command
        if payload.is_empty() {
            return parsed_frame;
        }

        for class in &self.zwc.cmd_class {
            let key_stripped: &str = class.key.trim_start_matches("0x");
            let cc: u8 = match u8::from_str_radix(key_stripped, 16) {
                Ok(k) => k,
                Err(_) => continue,
            };
            if cc == payload[0] {
                parsed_frame.add_field(
                    "CommandClass".to_string(),
                    class.help.clone(),
                    Some(format!("0x{:02X}", cc))
                );
                parsed_frame.add_field(
                    "CommandClassVersion".to_string(),
                    class.version.clone(),
                    Some("string".to_string())
                );

                if payload.len() < 2 {
                    break;
                }

                match &class.cmd {
                    None => {},
                    Some(cmds) => {
                        for cmd in cmds {
                            let cmd_key_stripped: &str = cmd.key.trim_start_matches("0x");
                            let cmd_id: u8 = match u8::from_str_radix(cmd_key_stripped, 16) {
                                Ok(k) => k,
                                Err(_) => continue,
                            };
                            if cmd_id == payload[1] {
                                parsed_frame.add_field(
                                    "Command".to_string(),
                                    cmd.help.clone(),
                                    Some(format!("0x{:02X}", cmd_id))
                                );

                                match &cmd.children {
                                    None => {},
                                    Some(children) => {
                                        let mut byte_counter = 2;
                                        for p in children {
                                            if byte_counter >= payload.len() {
                                                break;
                                            }
                                            match p {
                                                CmdClassCmdChild::Param(p) => {
                                                    match p.param_type.as_str() {
                                                        "BYTE" => {
                                                            let value: u8 = payload[byte_counter];
                                                            byte_counter += 1;
                                                            parsed_frame.add_field(
                                                                p.name.clone(),
                                                                format!("0x{:02X}", value),
                                                                Some("BYTE".to_string())
                                                            );
                                                        },
                                                        _ => {
                                                            // Unsupported parameter type, skip
                                                        }
                                                    }
                                                },
                                                CmdClassCmdChild::VariantGroup(vg) => {
                                                    parsed_frame.add_field(
                                                        format!("VariantGroup_{}", vg.name),
                                                        "present".to_string(),
                                                        Some("group".to_string())
                                                    );
                                                },
                                            };
                                        }
                                    }
                                }
                            }
                        }
                    },
                };
                break;
            }
        }

        parsed_frame
    }
}
