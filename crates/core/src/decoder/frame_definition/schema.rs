// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Deserialisation of `FrameDefinition.xml`, the declarative description of
//! Z-Wave MPDU layouts used by the Zniffer tooling.
use serde::{
    Deserialize,
    Serialize,
};

/// Root of `FrameDefinition.xml`.
#[derive(Debug, Deserialize)]
pub struct FrameDefinition {
    #[serde(rename = "RadioFrequency", default)]
    pub radio_frequencies: Vec<RadioFrequency>,
    #[serde(rename = "BaseHeader", default)]
    pub base_headers: Vec<BaseHeader>,
    #[serde(rename = "Header", default)]
    pub headers: Vec<Header>,
    #[serde(rename = "DefineSet", default)]
    pub define_sets: Vec<DefineSet>,
}

/// Maps a region code to the base header layout it uses.
#[derive(Debug, Deserialize)]
pub struct RadioFrequency {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Code")]
    pub code: u8,
    #[serde(rename = "@BaseHeader")]
    pub base_header: u8,
    #[serde(rename = "@Text")]
    pub text: String,
}

/// Fields common to every frame on a given PHY.
#[derive(Debug, Deserialize)]
pub struct BaseHeader {
    #[serde(rename = "@Key")]
    pub key: u8,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Text")]
    pub text: String,
    #[serde(rename = "Param", default)]
    pub params: Vec<Param>,
    #[serde(rename = "HomeId")]
    pub home_id: Option<Ref>,
    #[serde(rename = "Source")]
    pub source: Option<Ref>,
    #[serde(rename = "HeaderType")]
    pub header_type: Option<Ref>,
    #[serde(rename = "IsLTX")]
    pub is_ltx: Option<Ref>,
    #[serde(rename = "SequenceNumber")]
    pub sequence_number: Option<Ref>,
}

/// Fields specific to one frame type, appended after the base header.
#[derive(Debug, Deserialize)]
pub struct Header {
    #[serde(rename = "@Key")]
    pub key: u8,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Text")]
    pub text: String,
    /// Which base header this frame type extends. Absent for beams.
    #[serde(rename = "@BaseHeaderKey")]
    pub base_header_key: Option<u8>,
    #[serde(rename = "@IsAck", default)]
    pub is_ack: bool,
    #[serde(rename = "@IsError", default)]
    pub is_error: bool,
    #[serde(rename = "@IsMulticast", default)]
    pub is_multicast: bool,
    #[serde(rename = "@IsRouted", default)]
    pub is_routed: bool,
    #[serde(rename = "Param", default)]
    pub params: Vec<Param>,
    /// All conditions must hold for this header to apply.
    #[serde(rename = "Validation", default)]
    pub validations: Vec<Validation>,
}

/// One field, possibly subdivided into bit fields.
#[derive(Debug, Deserialize)]
pub struct Param {
    #[serde(rename = "@Order")]
    pub order: u16,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Text")]
    pub text: String,
    #[serde(rename = "@Type")]
    pub param_type: ParamType,
    #[serde(rename = "@Bits")]
    pub bits: u8,
    /// Names the `DefineSet` giving symbolic values for this field.
    #[serde(rename = "@Defines")]
    pub defines: Option<String>,
    /// The field is present only when the referenced flag is set.
    #[serde(rename = "@OptRef")]
    pub opt_ref: Option<String>,
    /// The field's length in bytes is given by the referenced field.
    #[serde(rename = "@SizeRef")]
    pub size_ref: Option<String>,
    #[serde(rename = "Param", default)]
    pub params: Vec<Param>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ParamType {
    #[serde(rename = "HEX")]
    Hex,
    #[serde(rename = "NUMBER")]
    Number,
    #[serde(rename = "NUMBER_SIGNED")]
    NumberSigned,
    #[serde(rename = "NODE_NUMBER")]
    NodeNumber,
    #[serde(rename = "BOOLEAN")]
    Boolean,
    #[serde(rename = "BITMASK")]
    Bitmask,
}

/// A condition on an already-decoded field, used to pick the header type.
#[derive(Debug, Deserialize)]
pub struct Validation {
    /// Dotted path, e.g. `Properties1.HeaderType`.
    #[serde(rename = "@ParamName")]
    pub param_name: String,
    #[serde(rename = "@ParamHexValue", deserialize_with = "hex_u32")]
    pub param_hex_value: u32,
}

/// A reference to a field by name.
#[derive(Debug, Deserialize)]
pub struct Ref {
    #[serde(rename = "@Ref")]
    pub reference: String,
}

/// Symbolic names for the values of a field.
#[derive(Debug, Deserialize)]
pub struct DefineSet {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "Define", default)]
    pub defines: Vec<Define>,
}

#[derive(Debug, Deserialize)]
pub struct Define {
    #[serde(rename = "@Key", deserialize_with = "hex_u32")]
    pub key: u32,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Text")]
    pub text: String,
}

/// Parse the `0x`-prefixed integers used throughout the schema.
fn hex_u32<'de, D>(d: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(d)?;
    let t = s.trim();
    let parsed = match t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => t.parse(),
    };
    parsed.map_err(serde::de::Error::custom)
}

impl FrameDefinition {
    /// Parse the vendored `FrameDefinition.xml`.
    pub fn load() -> Result<Self, quick_xml::DeError> {
        quick_xml::de::from_str(include_str!("FrameDefinition.xml"))
    }

    pub fn base_header(&self, key: u8) -> Option<&BaseHeader> {
        self.base_headers.iter().find(|b| b.key == key)
    }

    pub fn header(&self, key: u8) -> Option<&Header> {
        self.headers.iter().find(|h| h.key == key)
    }

    pub fn define_set(&self, name: &str) -> Option<&DefineSet> {
        self.define_sets.iter().find(|d| d.name == name)
    }

    /// Human-readable name of a region code, e.g. `908.4 MHz (US)`.
    pub fn region_text(&self, region: u8) -> Option<&str> {
        self.radio_frequencies
            .iter()
            .find(|f| f.code == region)
            .map(|f| f.text.as_str())
    }

    /// Short name of a region code, e.g. `US`.
    pub fn region_name(&self, region: u8) -> Option<&str> {
        self.radio_frequencies
            .iter()
            .find(|f| f.code == region)
            .map(|f| f.name.as_str())
    }

    /// Base header layout used by a region code.
    pub fn base_header_key_for_region(&self, region: u8) -> u8 {
        self.radio_frequencies
            .iter()
            .find(|f| f.code == region)
            .map_or(0, |f| f.base_header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bundled_definition() {
        let fd = FrameDefinition::load().expect("FrameDefinition.xml should parse");
        assert_eq!(fd.headers.len(), 29);
        assert_eq!(fd.base_headers.len(), 3);

        let singlecast = fd.header(13).expect("SINGLECAST");
        assert_eq!(singlecast.name, "SINGLECAST");
        assert_eq!(singlecast.base_header_key, Some(0));
        assert_eq!(singlecast.validations.len(), 2);
        assert_eq!(singlecast.validations[0].param_name, "Properties1.HeaderType");
        assert_eq!(singlecast.validations[0].param_hex_value, 1);

        // Long Range uses its own base header.
        assert_eq!(fd.header(70).expect("SINGLECASTLR").base_header_key, Some(2));

        let basic = fd.base_header(0).expect("BASIC");
        assert_eq!(basic.params[0].name, "HomeID");
        assert_eq!(basic.params[0].bits, 32);
        // Properties1 is subdivided into bit fields.
        assert!(basic.params[2].params.iter().any(|p| p.name == "HeaderType"));
    }

    #[test]
    fn names_regions() {
        let fd = FrameDefinition::load().unwrap();
        assert_eq!(fd.region_name(0), Some("EU"));
        assert_eq!(fd.region_text(1), Some("908.4 MHz (US)"));
        assert_eq!(fd.region_name(28), Some("KR"));
        assert_eq!(fd.region_name(200), None);
    }

    #[test]
    fn resolves_region_base_headers() {
        let fd = FrameDefinition::load().unwrap();
        assert_eq!(fd.base_header_key_for_region(0), 0); // EU
        assert_eq!(fd.base_header_key_for_region(10), 1); // Japan
        assert_eq!(fd.base_header_key_for_region(28), 1); // Korea
    }
}
