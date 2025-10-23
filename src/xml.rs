use quick_xml::events::Event;
use serde::{Deserialize, Serialize};

// Following structs are generated from zwave.xml and https://thomblin.github.io/xml_schema_generator/ 
#[derive(Serialize, Deserialize)]
pub struct ZwClasses {
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub bas_dev: Vec<BasDev>,
    pub gen_dev: Vec<GenDev>,
    pub cmd_class: Vec<CmdClass>,
}

#[derive(Serialize, Deserialize)]
pub struct BasDev {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@help")]
    pub help: String,
    #[serde(rename = "@comment")]
    pub comment: String,
}

#[derive(Serialize, Deserialize)]
pub struct GenDev {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@help")]
    pub help: String,
    #[serde(rename = "@comment")]
    pub comment: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub spec_dev: Vec<SpecDev>,
}

#[derive(Serialize, Deserialize)]
pub struct SpecDev {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@help")]
    pub help: String,
    #[serde(rename = "@comment")]
    pub comment: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CmdClass {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@help")]
    pub help: String,
    #[serde(rename = "@comment")]
    pub comment: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub cmd: Option<Vec<Cmd>>,
}

#[derive(Serialize, Deserialize)]
pub struct Cmd {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@help")]
    pub help: String,
    #[serde(rename = "@support_mode")]
    pub support_mode: Option<String>,
    #[serde(rename = "@comment")]
    pub comment: Option<String>,
    #[serde(rename = "@cmd_mask")]
    pub cmd_mask: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "cmd_param")]
    pub param: Option<Vec<CmdClassCmdParam>>,
    pub variant_group: Option<Vec<CmdVariantGroup>>,
}

#[derive(Serialize, Deserialize)]
pub struct CmdClassCmdParam {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub param_type: String,
    #[serde(rename = "@encaptype")]
    pub encaptype: Option<String>,
    #[serde(rename = "@comment")]
    pub comment: Option<String>,
    #[serde(rename = "@optionalmask")]
    pub optionalmask: Option<String>,
    #[serde(rename = "@optionaloffs")]
    pub optionaloffs: Option<String>,
    #[serde(rename = "@cmd_mask")]
    pub cmd_mask: Option<String>,
    #[serde(rename = "@skipfield")]
    pub skipfield: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub variant: Option<CmdParamVariant>,
    #[serde(rename = "multi_array")]
    pub bitflag: Option<Vec<CmdParamBitflag>>,
    #[serde(rename = "multi_array2")]
    pub bitfield: Option<Vec<CmdParamBitfield>>,
    pub fieldenum: Option<Vec<CmdClassCmdParamFieldenum>>,
    pub bitmask: Option<CmdParamBitmask>,
    #[serde(rename = "const")]
    pub param_const: Option<Vec<CmdParamConst>>,
    pub arrayattrib: Option<Arrayattrib>,
}

#[derive(Serialize, Deserialize)]
pub struct CmdParamVariant {
    #[serde(rename = "@paramoffs")]
    pub paramoffs: String,
    #[serde(rename = "@sizemask")]
    pub sizemask: String,
    #[serde(rename = "@is_ascii")]
    pub is_ascii: Option<String>,
    #[serde(rename = "@sizechange")]
    pub sizechange: Option<String>,
    #[serde(rename = "@sizeoffs")]
    pub sizeoffs: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CmdParamBitflag {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@flagname")]
    pub flagname: String,
    #[serde(rename = "@flagmask")]
    pub flagmask: String,
}

#[derive(Serialize, Deserialize)]
pub struct CmdParamBitfield {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@fieldname")]
    pub fieldname: String,
    #[serde(rename = "@fieldmask")]
    pub fieldmask: String,
    #[serde(rename = "@shifter")]
    pub shifter: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CmdClassCmdParamFieldenum {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@fieldname")]
    pub fieldname: String,
    #[serde(rename = "@fieldmask")]
    pub fieldmask: String,
    #[serde(rename = "@shifter")]
    pub shifter: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub fieldenum: Vec<CmdParamFieldenumFieldenum>,
}

#[derive(Serialize, Deserialize)]
pub struct CmdParamFieldenumFieldenum {
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@key")]
    pub key: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CmdParamBitmask {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@paramoffs")]
    pub paramoffs: String,
    #[serde(rename = "@lenmask")]
    pub lenmask: String,
    #[serde(rename = "@len")]
    pub len: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CmdParamConst {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@flagname")]
    pub flagname: String,
    #[serde(rename = "@flagmask")]
    pub flagmask: String,
}

#[derive(Serialize, Deserialize)]
pub struct Arrayattrib {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@len")]
    pub len: String,
    #[serde(rename = "@is_ascii")]
    pub is_ascii: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CmdVariantGroup {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@paramOffs")]
    pub param_offs: String,
    #[serde(rename = "@sizemask")]
    pub sizemask: String,
    #[serde(rename = "@sizeoffs")]
    pub sizeoffs: String,
    #[serde(rename = "@optionalmask")]
    pub optionalmask: Option<String>,
    #[serde(rename = "@optionaloffs")]
    pub optionaloffs: Option<String>,
    #[serde(rename = "@moretofollowmask")]
    pub moretofollowmask: Option<String>,
    #[serde(rename = "@moretofollowoffs")]
    pub moretofollowoffs: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub param: Vec<CmdVariantGroupParam>,
    pub variant_group: Option<VariantGroupVariantGroup>,
}

#[derive(Serialize, Deserialize)]
pub struct CmdVariantGroupParam {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub param_type: String,
    #[serde(rename = "@encaptype")]
    pub encaptype: Option<String>,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub variant: Option<VariantGroupParamVariant>,
    #[serde(rename = "const")]
    pub param_const: Option<Vec<VariantGroupParamConst>>,
    #[serde(rename = "CmdVariantGroupParam_bitfield")]
    pub bitfield: Option<Vec<VariantGroupParamBitfield>>,
    pub bitflag: Option<Vec<VariantGroupParamBitflag>>,
    pub multi_array: Option<Vec<MultiArray>>,
    pub bitmask: Option<VariantGroupParamBitmask>,
    pub fieldenum: Option<CmdVariantGroupParamFieldenum>,
}

#[derive(Serialize, Deserialize)]
pub struct VariantGroupParamVariant {
    #[serde(rename = "@paramoffs")]
    pub paramoffs: String,
    #[serde(rename = "@sizemask")]
    pub sizemask: String,
    #[serde(rename = "@sizechange")]
    pub sizechange: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct VariantGroupParamConst {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@flagname")]
    pub flagname: String,
    #[serde(rename = "@flagmask")]
    pub flagmask: String,
}

#[derive(Serialize, Deserialize)]
pub struct VariantGroupParamBitfield {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@fieldname")]
    pub fieldname: String,
    #[serde(rename = "@fieldmask")]
    pub fieldmask: String,
    #[serde(rename = "@shifter")]
    pub shifter: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct VariantGroupParamBitflag {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@flagname")]
    pub flagname: String,
    #[serde(rename = "@flagmask")]
    pub flagmask: String,
}

#[derive(Serialize, Deserialize)]
pub struct MultiArray {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub paramdescloc: Option<Paramdescloc>,
    pub bitflag: Option<Vec<ParamMultiArrayBitflag>>,
}

#[derive(Serialize, Deserialize)]
pub struct Paramdescloc {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@param")]
    pub param: String,
    #[serde(rename = "@paramdesc")]
    pub paramdesc: String,
    #[serde(rename = "@paramstart")]
    pub paramstart: String,
}

#[derive(Serialize, Deserialize)]
pub struct ParamMultiArrayBitflag {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@flagname")]
    pub flagname: String,
    #[serde(rename = "@flagmask")]
    pub flagmask: String,
}

#[derive(Serialize, Deserialize)]
pub struct VariantGroupParamBitmask {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@paramoffs")]
    pub paramoffs: String,
    #[serde(rename = "@lenmask")]
    pub lenmask: String,
    #[serde(rename = "@lenoffs")]
    pub lenoffs: String,
}

#[derive(Serialize, Deserialize)]
pub struct CmdVariantGroupParamFieldenum {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@fieldname")]
    pub fieldname: String,
    #[serde(rename = "@fieldmask")]
    pub fieldmask: String,
    #[serde(rename = "@shifter")]
    pub shifter: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub fieldenum: Vec<VariantGroupParamFieldenumFieldenum>,
}

#[derive(Serialize, Deserialize)]
pub struct VariantGroupParamFieldenumFieldenum {
    #[serde(rename = "@value")]
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct VariantGroupVariantGroup {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@paramOffs")]
    pub param_offs: String,
    #[serde(rename = "@sizemask")]
    pub sizemask: String,
    #[serde(rename = "@sizeoffs")]
    pub sizeoffs: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub param: VariantGroupVariantGroupParam,
}

#[derive(Serialize, Deserialize)]
pub struct VariantGroupVariantGroupParam {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@type")]
    pub param_type: String,
}

pub fn parse_xml() -> ZwClasses {
    let xml_data = include_str!("./zwave.xml");
 
    let zw_classes: ZwClasses = quick_xml::de::from_str(xml_data).unwrap();
    println!("Parsed XML to struct: {:?}", zw_classes.cmd_class.len());
    zw_classes
}
