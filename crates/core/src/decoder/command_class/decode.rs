// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Walks the command class definitions over an application payload.
use serde::{
    Deserialize,
    Serialize,
};

use super::schema::{
    Cmd,
    CmdItem,
    Param,
    ParamItem,
    ParamType,
    ZwClasses,
};

/// A decoded command class parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodedParam {
    pub name: String,
    /// Byte range within the application payload.
    pub range: std::ops::Range<usize>,
    /// Numeric value, for params of four bytes or fewer.
    pub value: Option<u64>,
    /// Raw bytes of the field.
    pub bytes: Vec<u8>,
    /// Rendered value, e.g. `On`, `true`, `0x1F`, or an ASCII string.
    pub display: String,
    /// Bit flags and bit fields packed into this byte.
    pub children: Vec<DecodedParam>,
}

/// A decoded command class command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodedCommand {
    pub class_key: u8,
    pub class_name: String,
    /// Friendly class name, e.g. `Binary Switch`.
    pub class_help: String,
    pub class_version: u8,
    pub cmd_key: u8,
    pub cmd_name: String,
    /// Friendly command name, e.g. `Switch Binary Set`.
    pub cmd_help: String,
    pub params: Vec<DecodedParam>,
    /// Bytes not consumed by the definition.
    pub remainder: Vec<u8>,
}

impl DecodedCommand {
    /// One-line summary for the Application column.
    ///
    /// The command name alone, as the desktop Zniffer shows it: its
    /// converter assigns the class text and then overwrites it with the
    /// command text when one matches (`BaseApplicationCellConverter`).
    /// Command names already carry enough of the class to read well —
    /// "S2 Nonce Get", "Switch Binary Set".
    pub fn summary(&self) -> String {
        self.cmd_help.clone()
    }

    /// Command class and command, for the detail pane, where there is room
    /// to say which class a command belongs to.
    pub fn full_name(&self) -> String {
        format!("{} · {}", self.class_help, self.cmd_help)
    }
}

/// Why a payload could not be decoded.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DecodeFailure {
    /// Payload was empty or held only a class byte.
    Empty,
    /// The command class is not in the definitions.
    UnknownClass { class_key: u8 },
    /// The class is known but the command is not.
    UnknownCommand { class_key: u8, class_name: String, cmd_key: u8 },
}

/// Result of decoding an application payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Decoded {
    Command(Box<DecodedCommand>),
    /// Undecodable payloads keep their bytes so nothing is lost.
    Raw { failure: DecodeFailure, bytes: Vec<u8> },
}

impl Decoded {
    /// One-line summary for the frame list.
    pub fn summary(&self) -> String {
        match self {
            Decoded::Command(c) => c.summary(),
            Decoded::Raw { failure, bytes } => match failure {
                DecodeFailure::Empty => String::new(),
                DecodeFailure::UnknownClass { class_key } => {
                    format!("Command Class 0x{class_key:02X} not found [{}]", hex::encode_upper(bytes))
                }
                // The class text is the fallback when no command matched,
                // which is what the desktop tool falls back to as well.
                DecodeFailure::UnknownCommand { class_name, cmd_key, .. } => {
                    format!("{class_name} - Command 0x{cmd_key:02X} not found")
                }
            },
        }
    }
}

/// Bytes of command class and command that precede a command's params.
const CMD_HEADER: usize = 2;

/// Move decoded ranges from body-relative to payload-relative.
fn shift_ranges(params: &mut [DecodedParam], by: usize) {
    for param in params {
        param.range.start += by;
        param.range.end += by;
        shift_ranges(&mut param.children, by);
    }
}

/// Read a big-endian unsigned integer of `len` bytes.
fn read_uint(data: &[u8], len: usize) -> Option<u64> {
    if len == 0 || len > 8 || data.len() < len {
        return None;
    }
    Some(data[..len].iter().fold(0u64, |acc, &b| (acc << 8) | b as u64))
}

/// Decode the bit flags and bit fields packed into a single byte.
fn decode_bits(param: &Param, byte: u8, at: usize) -> Vec<DecodedParam> {
    let mut out = Vec::new();
    for item in &param.items {
        match item {
            ParamItem::BitFlag(f) => {
                let set = byte & f.flag_mask != 0;
                out.push(DecodedParam {
                    name: f.flag_name.clone(),
                    range: at..at + 1,
                    value: Some(set as u64),
                    bytes: Vec::new(),
                    display: set.to_string(),
                    children: Vec::new(),
                });
            }
            ParamItem::BitField(f) => {
                let shift = f.shifter.unwrap_or(0);
                let value = ((byte & f.field_mask) >> shift) as u64;
                out.push(DecodedParam {
                    name: f.field_name.clone(),
                    range: at..at + 1,
                    value: Some(value),
                    bytes: Vec::new(),
                    display: value.to_string(),
                    children: Vec::new(),
                });
            }
            ParamItem::FieldEnum(f) => {
                let shift = f.shifter.unwrap_or(0);
                let value = ((byte & f.field_mask) >> shift) as u64;
                out.push(DecodedParam {
                    name: f.field_name.clone(),
                    range: at..at + 1,
                    value: Some(value),
                    bytes: Vec::new(),
                    display: value.to_string(),
                    children: Vec::new(),
                });
            }
            _ => {}
        }
    }
    out
}

/// Length of a variable-length param, resolved against already-decoded params.
fn variable_len(param: &Param, decoded: &[DecodedParam], remaining: usize) -> usize {
    // A BITMASK may declare a fixed length outright.
    if let Some(bm) = param.bitmask()
        && let Some(len) = bm.len
    {
        return (len as usize).min(remaining);
    }

    let Some(variant) = param.variant() else {
        return remaining;
    };
    // 255 means "everything left in the message".
    if variant.param_offs == 255 || variant.size_mask == 0 {
        return remaining;
    }
    // Otherwise an earlier param carries the length, masked and shifted.
    let Some(source) = decoded.get(variant.param_offs as usize) else {
        return remaining;
    };
    let raw = source.value.unwrap_or(0) as u8;
    let shift = variant.size_offs.unwrap_or(0);
    let len = ((raw & variant.size_mask) >> shift) as usize;
    len.min(remaining)
}

/// True when an optional param's controlling flag is set.
fn is_present(param: &Param, decoded: &[DecodedParam]) -> bool {
    let (Some(offs), Some(mask)) = (param.optional_offs, param.optional_mask) else {
        return true;
    };
    decoded
        .get(offs as usize)
        .and_then(|p| p.value)
        .is_some_and(|v| (v as u8) & mask != 0)
}

/// Decode one param at `at`, returning it and the number of bytes consumed.
fn decode_param(
    param: &Param,
    data: &[u8],
    at: usize,
    decoded: &[DecodedParam],
) -> Option<(DecodedParam, usize)> {
    if at >= data.len() || !is_present(param, decoded) {
        return None;
    }
    let remaining = data.len() - at;

    let len = match param.param_type {
        ParamType::Array => param
            .array_attrib()
            .map_or(remaining, |a| (a.len as usize).min(remaining)),
        ParamType::Variant | ParamType::Bitmask | ParamType::MultiArray => {
            variable_len(param, decoded, remaining)
        }
        other => other.fixed_len().unwrap_or(remaining).min(remaining),
    };
    if len == 0 {
        return None;
    }

    let bytes = data[at..at + len].to_vec();
    let value = read_uint(&bytes, len);

    let is_ascii = param.array_attrib().is_some_and(|a| a.is_ascii)
        || param.variant().is_some_and(|v| v.is_ascii);

    let display = if is_ascii {
        String::from_utf8_lossy(&bytes).trim_end_matches('\0').to_string()
    } else if param.param_type == ParamType::Const
        && let Some(name) = value.and_then(|v| param.const_name(v as u8))
    {
        name.to_string()
    } else if len <= 4 {
        format!("0x{}", hex::encode_upper(&bytes))
    } else {
        hex::encode_upper(&bytes)
    };

    let children = if len == 1 { decode_bits(param, bytes[0], at) } else { Vec::new() };

    Some((
        DecodedParam {
            name: param.name.clone(),
            range: at..at + len,
            value,
            bytes,
            display,
            children,
        },
        len,
    ))
}

/// Decode the params of a command, including repeating groups.
fn decode_params(cmd: &Cmd, data: &[u8]) -> (Vec<DecodedParam>, usize) {
    let mut decoded: Vec<DecodedParam> = Vec::new();
    let mut at = 0usize;

    for item in &cmd.items {
        match item {
            CmdItem::Param(param) => {
                let Some((d, len)) = decode_param(param, data, at, &decoded) else {
                    continue;
                };
                at += len;
                decoded.push(d);
            }
            CmdItem::VariantGroup(group) => {
                // The repeat count comes from an earlier param, or the group
                // repeats until the payload is exhausted.
                let repeats = if group.param_offs == 0xFF || group.size_mask == 0 {
                    usize::MAX
                } else {
                    decoded
                        .get(group.param_offs as usize)
                        .and_then(|p| p.value)
                        .map(|v| ((v as u8 & group.size_mask) >> group.size_offs) as usize)
                        .unwrap_or(0)
                };

                let mut done = 0usize;
                while done < repeats && at < data.len() {
                    let before = at;
                    let mut members = Vec::new();
                    for param in &group.params {
                        let Some((d, len)) = decode_param(param, data, at, &members) else {
                            break;
                        };
                        at += len;
                        members.push(d);
                    }
                    // Guard against a group that consumes nothing.
                    if at == before {
                        break;
                    }
                    decoded.push(DecodedParam {
                        name: format!("{} [{}]", group.name, done),
                        range: before..at,
                        value: None,
                        bytes: data[before..at].to_vec(),
                        display: String::new(),
                        children: members,
                    });
                    done += 1;
                }
            }
        }
    }
    (decoded, at)
}

/// Decode an application payload, where `payload[0]` is the command class and
/// `payload[1]` the command.
pub fn decode(zw: &ZwClasses, payload: &[u8]) -> Decoded {
    if payload.len() < 2 {
        return Decoded::Raw { failure: DecodeFailure::Empty, bytes: payload.to_vec() };
    }
    let class_key = payload[0];
    let cmd_key = payload[1];

    let Some((class, cmd)) = zw.cmd(class_key, cmd_key) else {
        let failure = match zw.cmd_class(class_key) {
            Some(class) => DecodeFailure::UnknownCommand {
                class_key,
                class_name: class.help.clone(),
                cmd_key,
            },
            None => DecodeFailure::UnknownClass { class_key },
        };
        return Decoded::Raw { failure, bytes: payload.to_vec() };
    };

    // Params are decoded against the body, after the class and command
    // bytes, but their ranges are documented as payload-relative so a
    // caller can map them onto the frame.
    let body = &payload[2..];
    let (mut params, consumed) = decode_params(cmd, body);
    shift_ranges(&mut params, CMD_HEADER);

    Decoded::Command(Box::new(DecodedCommand {
        class_key,
        class_name: class.name.clone(),
        class_help: class.help.clone(),
        class_version: class.version,
        cmd_key,
        cmd_name: cmd.name.clone(),
        cmd_help: cmd.help.clone(),
        params,
        remainder: body[consumed.min(body.len())..].to_vec(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes() -> ZwClasses {
        ZwClasses::load().unwrap()
    }

    #[test]
    fn decodes_binary_switch_set() {
        let zw = classes();
        let Decoded::Command(c) = decode(&zw, &[0x25, 0x01, 0xFF]) else {
            panic!("should decode");
        };
        assert_eq!(c.class_key, 0x25);
        assert_eq!(c.cmd_name, "SWITCH_BINARY_SET");
        assert_eq!(c.params[0].value, Some(0xFF));
        // The Application column shows the command, not the class, as the
        // desktop Zniffer does.
        assert_eq!(c.summary(), "Switch Binary Set");
        assert_eq!(c.class_help, "Command Class Binary Switch");
    }

    #[test]
    fn decodes_basic_set() {
        let zw = classes();
        let Decoded::Command(c) = decode(&zw, &[0x20, 0x01, 0x00]) else {
            panic!("should decode");
        };
        assert_eq!(c.cmd_name, "BASIC_SET");
        assert_eq!(c.params[0].value, Some(0));
    }

    #[test]
    fn reports_unknown_class_without_losing_bytes() {
        let zw = classes();
        let payload = [0xFEu8, 0x01, 0xAA];
        let Decoded::Raw { failure, bytes } = decode(&zw, &payload) else {
            panic!("0xFE is not a command class");
        };
        assert_eq!(failure, DecodeFailure::UnknownClass { class_key: 0xFE });
        assert_eq!(bytes, payload);
    }

    #[test]
    fn reports_unknown_command() {
        let zw = classes();
        let Decoded::Raw { failure, .. } = decode(&zw, &[0x25, 0xEE]) else {
            panic!("0xEE is not a Binary Switch command");
        };
        assert!(matches!(failure, DecodeFailure::UnknownCommand { cmd_key: 0xEE, .. }));
    }

    #[test]
    fn handles_empty_payload() {
        let zw = classes();
        assert!(matches!(
            decode(&zw, &[]),
            Decoded::Raw { failure: DecodeFailure::Empty, .. }
        ));
    }

    #[test]
    fn ranges_index_the_payload_not_the_body() {
        // A range must select its own bytes when used on the payload it was
        // decoded from; off by the two class and command bytes is the easy
        // mistake, and it puts every highlight on the wrong byte.
        let zw = classes();
        let payload = [0x25u8, 0x01, 0xFF];
        let Decoded::Command(c) = decode(&zw, &payload) else {
            panic!("Binary Switch Set should decode");
        };
        let value = &c.params[0];
        assert_eq!(value.range, 2..3, "the value byte is at index 2 of the payload");
        assert_eq!(payload[value.range.clone()], [0xFF]);
        assert_eq!(value.bytes, payload[value.range.clone()]);
    }

    #[test]
    fn nested_ranges_are_payload_relative_too() {
        let zw = classes();
        // Meter Report: a STRUCT_BYTE whose bit fields carry the parent range.
        let payload = [0x32u8, 0x02, 0x21, 0x34, 0x00, 0x00];
        let Decoded::Command(c) = decode(&zw, &payload) else {
            panic!("should decode");
        };
        for param in &c.params {
            assert!(param.range.end <= payload.len(), "{} runs past the payload", param.name);
            for child in &param.children {
                assert_eq!(
                    child.range, param.range,
                    "a bit field should span its parent byte"
                );
            }
        }
    }

    #[test]
    fn decodes_bit_fields_within_a_byte() {
        let zw = classes();
        // Meter Report carries a STRUCT_BYTE of scale/rate/type bits.
        let Decoded::Command(c) = decode(&zw, &[0x32, 0x02, 0x21, 0x34, 0x00, 0x00]) else {
            panic!("should decode");
        };
        assert!(
            c.params.iter().any(|p| !p.children.is_empty()),
            "expected at least one param with bit fields"
        );
    }
}
