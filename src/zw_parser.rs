use crate::frame_definition::{FrameDefinition};
use crate::xml::{
    ZwClasses,
    CmdClassCmdChild,
};

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
                            let sub_value = &value[0] >> bit_offset & ((1 << n) - 1);
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
}
