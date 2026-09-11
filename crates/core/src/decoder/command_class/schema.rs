// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Deserialisation of `zwave.xml`, the Z-Wave Alliance command class definitions.
use serde::Deserialize;

/// Root of `zwave.xml`.
#[derive(Debug, Deserialize)]
pub struct ZwClasses {
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "cmd_class", default)]
    pub cmd_classes: Vec<CmdClass>,
}

#[derive(Debug, Deserialize)]
pub struct CmdClass {
    #[serde(rename = "@key", deserialize_with = "hex_u8")]
    pub key: u8,
    #[serde(rename = "@version", deserialize_with = "dec_u8")]
    pub version: u8,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@help")]
    pub help: String,
    #[serde(rename = "cmd", default)]
    pub cmds: Vec<Cmd>,
}

#[derive(Debug, Deserialize)]
pub struct Cmd {
    #[serde(rename = "@key", deserialize_with = "hex_u8")]
    pub key: u8,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@help")]
    pub help: String,
    #[serde(rename = "$value", default)]
    pub items: Vec<CmdItem>,
}

/// A command's body is an ordered mix of plain params and repeating groups.
#[derive(Debug, Deserialize)]
pub enum CmdItem {
    #[serde(rename = "param")]
    Param(Param),
    #[serde(rename = "variant_group")]
    VariantGroup(VariantGroup),
}

/// A group of params repeated a variable number of times.
#[derive(Debug, Deserialize)]
pub struct VariantGroup {
    #[serde(rename = "@key", deserialize_with = "hex_u8")]
    pub key: u8,
    #[serde(rename = "@name")]
    pub name: String,
    /// Index of the param holding the repeat count, or 0xFF for "until the end".
    #[serde(rename = "@paramOffs", deserialize_with = "hex_u8")]
    pub param_offs: u8,
    #[serde(rename = "@sizemask", deserialize_with = "hex_u8")]
    pub size_mask: u8,
    #[serde(rename = "@sizeoffs", deserialize_with = "hex_u8")]
    pub size_offs: u8,
    #[serde(rename = "param", default)]
    pub params: Vec<Param>,
}

#[derive(Debug, Deserialize)]
pub struct Param {
    #[serde(rename = "@key", deserialize_with = "hex_u8")]
    pub key: u8,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub param_type: ParamType,
    /// Index of the byte holding the flag that makes this param present.
    #[serde(rename = "@optionaloffs", default, deserialize_with = "opt_hex_u8")]
    pub optional_offs: Option<u8>,
    #[serde(rename = "@optionalmask", default, deserialize_with = "opt_hex_u8")]
    pub optional_mask: Option<u8>,
    #[serde(rename = "@encaptype")]
    pub encap_type: Option<String>,
    /// Child elements in document order; bit flags and bit fields interleave
    /// by bit position, so the order carries meaning.
    #[serde(rename = "$value", default)]
    pub items: Vec<ParamItem>,
}

/// One child element of a `param`.
#[derive(Debug, Deserialize)]
pub enum ParamItem {
    #[serde(rename = "variant")]
    Variant(Variant),
    #[serde(rename = "arrayattrib")]
    ArrayAttrib(ArrayAttrib),
    #[serde(rename = "bitflag")]
    BitFlag(BitFlag),
    #[serde(rename = "bitfield")]
    BitField(BitField),
    #[serde(rename = "const")]
    Const(Const),
    #[serde(rename = "bitmask")]
    BitMask(BitMask),
    #[serde(rename = "fieldenum")]
    FieldEnum(FieldEnum),
    #[serde(rename = "multi_array")]
    MultiArray(serde::de::IgnoredAny),
}

impl Param {
    /// Sizing rule, for variable-length params.
    pub fn variant(&self) -> Option<&Variant> {
        self.items.iter().find_map(|i| match i {
            ParamItem::Variant(v) => Some(v),
            _ => None,
        })
    }

    /// Fixed array length, for `ARRAY` params.
    pub fn array_attrib(&self) -> Option<&ArrayAttrib> {
        self.items.iter().find_map(|i| match i {
            ParamItem::ArrayAttrib(a) => Some(a),
            _ => None,
        })
    }

    /// Sizing rule, for `BITMASK` params.
    pub fn bitmask(&self) -> Option<&BitMask> {
        self.items.iter().find_map(|i| match i {
            ParamItem::BitMask(b) => Some(b),
            _ => None,
        })
    }

    /// Symbolic name for a value, from this param's `const` entries.
    pub fn const_name(&self, value: u8) -> Option<&str> {
        self.items.iter().find_map(|i| match i {
            ParamItem::Const(c) if c.flag_mask == value => Some(c.flag_name.as_str()),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum ParamType {
    #[serde(rename = "BYTE")]
    Byte,
    #[serde(rename = "WORD")]
    Word,
    #[serde(rename = "DWORD")]
    Dword,
    #[serde(rename = "BIT_24")]
    Bit24,
    #[serde(rename = "STRUCT_BYTE")]
    StructByte,
    #[serde(rename = "VARIANT")]
    Variant,
    #[serde(rename = "CONST")]
    Const,
    #[serde(rename = "BITMASK")]
    Bitmask,
    #[serde(rename = "ARRAY")]
    Array,
    #[serde(rename = "MARKER")]
    Marker,
    #[serde(rename = "MULTI_ARRAY")]
    MultiArray,
}

impl ParamType {
    /// Fixed width in bytes, for the types that have one.
    pub fn fixed_len(self) -> Option<usize> {
        match self {
            ParamType::Byte
            | ParamType::StructByte
            | ParamType::Const
            | ParamType::Marker => Some(1),
            ParamType::Word => Some(2),
            ParamType::Bit24 => Some(3),
            ParamType::Dword => Some(4),
            _ => None,
        }
    }
}

/// Sizing rule for a variable-length param.
#[derive(Debug, Deserialize)]
pub struct Variant {
    /// Index of the param carrying the length, or 255 for "rest of message".
    #[serde(rename = "@paramoffs", deserialize_with = "dec_u8")]
    pub param_offs: u8,
    #[serde(rename = "@sizemask", deserialize_with = "hex_u8")]
    pub size_mask: u8,
    #[serde(rename = "@sizeoffs", default, deserialize_with = "opt_dec_u8")]
    pub size_offs: Option<u8>,
    #[serde(rename = "@is_ascii", default)]
    pub is_ascii: bool,
}

#[derive(Debug, Deserialize)]
pub struct ArrayAttrib {
    #[serde(rename = "@len", deserialize_with = "dec_u8")]
    pub len: u8,
    #[serde(rename = "@is_ascii", default)]
    pub is_ascii: bool,
}

#[derive(Debug, Deserialize)]
pub struct BitFlag {
    #[serde(rename = "@flagname")]
    pub flag_name: String,
    #[serde(rename = "@flagmask", deserialize_with = "hex_u8")]
    pub flag_mask: u8,
}

#[derive(Debug, Deserialize)]
pub struct BitField {
    #[serde(rename = "@fieldname")]
    pub field_name: String,
    #[serde(rename = "@fieldmask", deserialize_with = "hex_u8")]
    pub field_mask: u8,
    #[serde(rename = "@shifter", default, deserialize_with = "opt_dec_u8")]
    pub shifter: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct Const {
    #[serde(rename = "@flagname")]
    pub flag_name: String,
    #[serde(rename = "@flagmask", deserialize_with = "hex_u8")]
    pub flag_mask: u8,
}

#[derive(Debug, Deserialize)]
pub struct BitMask {
    #[serde(rename = "@paramoffs", deserialize_with = "dec_u8")]
    pub param_offs: u8,
    #[serde(rename = "@lenmask", deserialize_with = "hex_u8")]
    pub len_mask: u8,
    #[serde(rename = "@len", default, deserialize_with = "opt_dec_u8")]
    pub len: Option<u8>,
    #[serde(rename = "@lenoffs", default, deserialize_with = "opt_dec_u8")]
    pub len_offs: Option<u8>,
}

#[derive(Debug, Deserialize)]
pub struct FieldEnum {
    #[serde(rename = "@fieldname")]
    pub field_name: String,
    #[serde(rename = "@fieldmask", deserialize_with = "hex_u8")]
    pub field_mask: u8,
    #[serde(rename = "@shifter", default, deserialize_with = "opt_dec_u8")]
    pub shifter: Option<u8>,
}

fn parse_int(s: &str) -> Result<u32, std::num::ParseIntError> {
    let t = s.trim();
    match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => t.parse(),
    }
}

fn hex_u8<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u8, D::Error> {
    let s = String::deserialize(d)?;
    parse_int(&s).map(|v| v as u8).map_err(serde::de::Error::custom)
}

fn dec_u8<'de, D: serde::Deserializer<'de>>(d: D) -> Result<u8, D::Error> {
    let s = String::deserialize(d)?;
    parse_int(&s).map(|v| v as u8).map_err(serde::de::Error::custom)
}

fn opt_hex_u8<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u8>, D::Error> {
    let s = Option::<String>::deserialize(d)?;
    Ok(s.and_then(|s| parse_int(&s).ok()).map(|v| v as u8))
}

fn opt_dec_u8<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<u8>, D::Error> {
    opt_hex_u8(d)
}

impl ZwClasses {
    /// Parse the vendored `zwave.xml`.
    pub fn load() -> Result<Self, quick_xml::DeError> {
        quick_xml::de::from_str(include_str!("zwave.xml"))
    }

    /// Highest-versioned definition of a command class.
    pub fn cmd_class(&self, key: u8) -> Option<&CmdClass> {
        self.cmd_classes.iter().filter(|c| c.key == key).max_by_key(|c| c.version)
    }

    /// A command, resolved against the highest command class version that
    /// defines it.
    pub fn cmd(&self, class_key: u8, cmd_key: u8) -> Option<(&CmdClass, &Cmd)> {
        self.cmd_classes
            .iter()
            .filter(|c| c.key == class_key)
            .filter_map(|c| c.cmds.iter().find(|m| m.key == cmd_key).map(|m| (c, m)))
            .max_by_key(|(c, _)| c.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bundled_definitions() {
        let zw = ZwClasses::load().expect("zwave.xml should parse");
        assert_eq!(zw.version, "2.18.1");
        assert_eq!(zw.cmd_classes.len(), 259);

        // Binary Switch Set carries a single value byte.
        let (class, cmd) = zw.cmd(0x25, 0x01).expect("Binary Switch Set");
        assert_eq!(class.name, "COMMAND_CLASS_SWITCH_BINARY");
        assert_eq!(cmd.name, "SWITCH_BINARY_SET");
        assert!(!cmd.items.is_empty());
    }

    #[test]
    fn picks_the_highest_class_version() {
        let zw = ZwClasses::load().unwrap();
        let cc = zw.cmd_class(0x25).unwrap();
        let max = zw
            .cmd_classes
            .iter()
            .filter(|c| c.key == 0x25)
            .map(|c| c.version)
            .max()
            .unwrap();
        assert_eq!(cc.version, max);
    }
}
