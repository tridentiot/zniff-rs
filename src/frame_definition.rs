// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct FrameDefinition {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "RadioFrequency")]
    pub radio_frequency: Vec<RadioFrequency>,
    #[serde(rename = "BaseHeader")]
    pub base_header: Vec<BaseHeader>,
    #[serde(rename = "Header")]
    pub header: Vec<Header>,
    #[serde(rename = "DefineSet")]
    pub define_set: Vec<DefineSet>,
    #[serde(rename = "HeaderFilter")]
    pub header_filter: Vec<HeaderFilter>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RadioFrequency {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Code")]
    pub code: String,
    #[serde(rename = "@BaseHeader")]
    pub base_header: String,
    #[serde(rename = "@Text")]
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BaseHeader {
    #[serde(rename = "@Key")]
    pub key: String,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Text")]
    pub base_header_text: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "Param")]
    pub param: Vec<FrameDefinitionBaseHeaderParam>,
    #[serde(rename = "HomeId")]
    pub home_id: HomeId,
    #[serde(rename = "Source")]
    pub source: Source,
    #[serde(rename = "HeaderType")]
    pub header_type: HeaderType,
    #[serde(rename = "IsLTX")]
    pub is_ltx: IsLtx,
    #[serde(rename = "SequenceNumber")]
    pub sequence_number: SequenceNumber,
    #[serde(rename = "Destination")]
    pub destination: Option<BaseHeaderDestination>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FrameDefinitionBaseHeaderParam {
    #[serde(rename = "@Order")]
    pub order: String,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Text")]
    pub param_text: String,
    #[serde(rename = "@Type")]
    pub param_type: String,
    #[serde(rename = "@Bits")]
    pub bits: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "Param")]
    pub param: Option<Vec<BaseHeaderParamParam>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BaseHeaderParamParam {
    #[serde(rename = "@Order")]
    pub order: String,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Text")]
    pub text: String,
    #[serde(rename = "@Type")]
    pub param_type: String,
    #[serde(rename = "@Bits")]
    pub bits: String,
    #[serde(rename = "@Defines")]
    pub defines: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HomeId {
    #[serde(rename = "@Ref")]
    pub home_id_ref: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Source {
    #[serde(rename = "@Ref")]
    pub source_ref: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HeaderType {
    #[serde(rename = "@Ref")]
    pub header_type_ref: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IsLtx {
    #[serde(rename = "@Ref")]
    pub is_ltx_ref: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SequenceNumber {
    #[serde(rename = "@Ref")]
    pub sequence_number_ref: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BaseHeaderDestination {
    #[serde(rename = "@Ref")]
    pub destination_ref: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Header {
    #[serde(rename = "@Key")]
    pub key: String,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Text")]
    pub header_text: String,
    #[serde(rename = "@BaseHeaderKey")]
    pub base_header_key: Option<String>,
    #[serde(rename = "@IsAck")]
    pub is_ack: String,
    #[serde(rename = "@IsError")]
    pub is_error: String,
    #[serde(rename = "@IsMulticast")]
    pub is_multicast: String,
    #[serde(rename = "@IsRouted")]
    pub is_routed: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "Repeaters")]
    pub repeaters: Option<Repeaters>,
    #[serde(rename = "Hops")]
    pub hops: Option<Hops>,
    #[serde(rename = "Destination")]
    pub destination: Option<HeaderDestination>,
    #[serde(rename = "Param")]
    pub param: Vec<FrameDefinitionHeaderParam>,
    #[serde(rename = "Validation")]
    pub validation: Option<Vec<Validation>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Repeaters {
    #[serde(rename = "@Ref")]
    pub repeaters_ref: String,
    #[serde(rename = "@SizeRef")]
    pub size_ref: String,
    #[serde(rename = "@IsBitmask")]
    pub is_bitmask: String,
    #[serde(rename = "@SizeCorrection")]
    pub size_correction: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Hops {
    #[serde(rename = "@Ref")]
    pub hops_ref: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HeaderDestination {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "Destinations")]
    pub destinations: Option<Destinations>,
    #[serde(rename = "Destination")]
    pub destination: Option<Vec<DestinationDestination>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Destinations {
    #[serde(rename = "@Ref")]
    pub destinations_ref: String,
    #[serde(rename = "@SizeRef")]
    pub size_ref: String,
    #[serde(rename = "@IsBitmask")]
    pub is_bitmask: String,
    #[serde(rename = "@SizeCorrection")]
    pub size_correction: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DestinationDestination {
    #[serde(rename = "@Ref")]
    pub destination_ref: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FrameDefinitionHeaderParam {
    #[serde(rename = "@Order")]
    pub order: String,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Text")]
    pub param_text: String,
    #[serde(rename = "@Type")]
    pub param_type: String,
    #[serde(rename = "@Bits")]
    pub bits: String,
    #[serde(rename = "@OptRef")]
    pub opt_ref: Option<String>,
    #[serde(rename = "@SizeRef")]
    pub size_ref: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "Param")]
    pub param: Option<Vec<HeaderParamParam>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HeaderParamParam {
    #[serde(rename = "@Order")]
    pub order: String,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Text")]
    pub text: String,
    #[serde(rename = "@Type")]
    pub param_type: String,
    #[serde(rename = "@Bits")]
    pub bits: String,
    #[serde(rename = "@Defines")]
    pub defines: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Validation {
    #[serde(rename = "@ParamName")]
    pub param_name: String,
    #[serde(rename = "@ParamHexValue")]
    pub param_hex_value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DefineSet {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "Define")]
    pub define: Vec<Define>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Define {
    #[serde(rename = "@Key")]
    pub key: String,
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Text")]
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HeaderFilter {
    #[serde(rename = "@Order")]
    pub order: String,
    #[serde(rename = "@Text")]
    pub header_filter_text: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "HeaderCodes")]
    pub header_codes: HeaderCodes,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HeaderCodes {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "HeaderCode")]
    pub header_code: Vec<String>,
}

pub fn parse_xml() -> FrameDefinition {
    let xml_data = include_str!("./FrameDefinition.xml");

    /* This section can be used to get detailed error paths during deserialization.
    let xd = &mut quick_xml::de::Deserializer::from_str(xml_data);

    let result: Result<FrameDefinition, _> = serde_path_to_error::deserialize(xd);

    match result {
        Ok(_) => panic!("expected a type error"),
        Err(err) => {
            println!("Deserialization error: {}", err);
            let path = err.path().to_string();
            assert_eq!(path, "dependencies.serde.version");
        }
    }
    */

    let fd: FrameDefinition = match quick_xml::de::from_str(xml_data) {
        Ok(data) => data,
        Err(e) => {
            panic!("Failed to parse XML: {}", e);
        }
    };
    fd
}
