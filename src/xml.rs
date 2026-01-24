use serde::{Deserialize, Serialize};

/*
 * The following structs are initialliy generated from zwave.xml via
 * https://thomblin.github.io/xml_schema_generator/.
 *
 * Some modifications are made to support cases where elements are not consecutively present.
 * For instance: <param/><variant_group/><param/>.
 */

#[derive(Serialize, Deserialize, Debug)]
pub struct ZwClasses {
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub bas_dev: Vec<BasDev>,
    pub gen_dev: Vec<GenDev>,
    pub cmd_class: Vec<CmdClass>,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
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
    #[serde(rename = "$value", default)]
    pub children: Option<Vec<CmdClassCmdChild>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CmdClassCmdChild {
    Param(CmdClassCmdParam),
    VariantGroup(CmdVariantGroup),
}

#[derive(Serialize, Deserialize, Debug)]
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
    //#[serde(default)]
    //pub fieldenum: Option<Vec<CmdClassCmdParamFieldenum>>,
    pub bitmask: Option<CmdParamBitmask>,
    #[serde(rename = "const")]
    pub param_const: Option<Vec<CmdParamConst>>,
    pub arrayattrib: Option<Arrayattrib>,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdParamBitflag {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@flagname")]
    pub flagname: String,
    #[serde(rename = "@flagmask")]
    pub flagmask: String,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdParamFieldenumFieldenum {
    #[serde(rename = "@value")]
    pub value: String,
    #[serde(rename = "@key")]
    pub key: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdParamConst {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@flagname")]
    pub flagname: String,
    #[serde(rename = "@flagmask")]
    pub flagmask: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Arrayattrib {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@len")]
    pub len: String,
    #[serde(rename = "@is_ascii")]
    pub is_ascii: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
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
    #[serde(rename = "param")]
    pub param: Option<Vec<CmdVariantGroupParam>>,
    pub variant_group: Option<VariantGroupVariantGroup>,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct VariantGroupParamVariant {
    #[serde(rename = "@paramoffs")]
    pub paramoffs: String,
    #[serde(rename = "@sizemask")]
    pub sizemask: String,
    #[serde(rename = "@sizechange")]
    pub sizechange: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VariantGroupParamConst {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@flagname")]
    pub flagname: String,
    #[serde(rename = "@flagmask")]
    pub flagmask: String,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct VariantGroupParamBitflag {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@flagname")]
    pub flagname: String,
    #[serde(rename = "@flagmask")]
    pub flagmask: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MultiArray {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    pub paramdescloc: Option<Paramdescloc>,
    pub bitflag: Option<Vec<ParamMultiArrayBitflag>>,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct ParamMultiArrayBitflag {
    #[serde(rename = "@key")]
    pub key: String,
    #[serde(rename = "@flagname")]
    pub flagname: String,
    #[serde(rename = "@flagmask")]
    pub flagmask: String,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct VariantGroupParamFieldenumFieldenum {
    #[serde(rename = "@value")]
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
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
    #[serde(rename = "param")]
    pub param: Vec<VariantGroupVariantGroupParam>,
}

#[derive(Serialize, Deserialize, Debug)]
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

    /* This section can be used to get detailed error paths during deserialization.
    let xd = &mut quick_xml::de::Deserializer::from_str(xml_data);

    let result: Result<ZwClasses, _> = serde_path_to_error::deserialize(xd);

    match result {
        Ok(_) => panic!("expected a type error"),
        Err(err) => {
            println!("Deserialization error: {}", err);
            let path = err.path().to_string();
            assert_eq!(path, "dependencies.serde.version");
        }
    }
    */

    let zw_classes: ZwClasses = match quick_xml::de::from_str(xml_data) {
        Ok(data) => data,
        Err(e) => {
            panic!("Failed to parse XML: {}", e);
        }
    };
    println!("Parsed XML to struct: {:?}", zw_classes.cmd_class.len());
    zw_classes
}
