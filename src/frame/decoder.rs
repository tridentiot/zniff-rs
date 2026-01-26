use std::{
    collections::HashMap,
    fmt::{Display, write},
    ops::Range,
};

use quick_xml::de;

use crate::frame::decoder;
use super::frame::Frame;

/// Type of a frame field.
#[derive(Debug)]
pub enum FieldType {
    UInt8,
    UInt16,
    UInt16BE,
    UInt32,
    Uint32BE,
    Int8,
    Int16,
    Int16BE,
    Int32,
    Int32BE,
    Bytes,
    SubFrame(DecodedChunk),
}

/// How to display the field value.
#[derive(Debug)]
pub enum DisplayType {
    Hex,
    Decimal,
    Binary,
    Ascii,
}

pub type Segment = Range<usize>;

/// A field within a frame.
/// Fields can be simple types like integers or complex types like sub-frames.
#[derive(Debug)]
pub struct DecodedField {
    /// Short name used in filters
    ///
    pub short_name: String,
    /// Field name shown in the UI.
    pub name: String,
    pub segment: Segment,
    /// Data type of the field, this is also used for complex types like sub-frames.
    pub field_type: FieldType,

    /// How to display the field value.
    pub display_type: DisplayType,
}

/// A part of a frame, consisting of multiple fields
#[derive(Debug)]
pub struct DecodedChunk {
    pub fields: Vec<DecodedField>,
}

// Macro to create DecodedField instances more compactly
macro_rules! field {
    // Full range variant (for variable-size fields like Bytes or when you need custom ranges)
    ($short:expr, $name:expr, $start:expr, $end:expr, $field_type:expr, $display:expr) => {
        DecodedField {
            short_name: $short.to_string(),
            name: $name.to_string(),
            segment: $start..$end,
            field_type: $field_type,
            display_type: $display,
        }
    };
    // Auto-size variant - determines end position from FieldType
    ($short:expr, $name:expr, $start:expr, $field_type:expr, $display:expr) => {
        field!(
            $short,
            $name,
            $start,
            $start + match $field_type {
                FieldType::UInt8 | FieldType::Int8 => 1,
                FieldType::UInt16 | FieldType::UInt16BE | FieldType::Int16 | FieldType::Int16BE => 2,
                FieldType::UInt32 | FieldType::Uint32BE | FieldType::Int32 | FieldType::Int32BE => 4,
                _ => panic!("Cannot auto-determine size for field type {:?}. Use explicit range variant.", $field_type)
            },
            $field_type,
            $display
        )
    };
}

// Export the macro for use in other modules
pub(crate) use field;

impl FieldType {
    // Extract the value from the data slice and format it according to the field type.
    pub fn fmt_value(&self, data: &[u8], f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldType::UInt8 => Ok(u8::from_le_bytes(data[0..1].try_into().unwrap()).fmt(f)?),
            FieldType::UInt16 => Ok(u16::from_le_bytes(data[0..2].try_into().unwrap()).fmt(f)?),
            FieldType::UInt16BE => Ok(u16::from_be_bytes(data[0..2].try_into().unwrap()).fmt(f)?),
            FieldType::UInt32 => Ok(u32::from_le_bytes(data[0..4].try_into().unwrap()).fmt(f)?),
            FieldType::Uint32BE => Ok(u32::from_be_bytes(data[0..4].try_into().unwrap()).fmt(f)?),
            FieldType::Int8 => Ok(i8::from_le_bytes(data[0..1].try_into().unwrap()).fmt(f)?),
            FieldType::Int16 => Ok(i16::from_le_bytes(data[0..2].try_into().unwrap()).fmt(f)?),
            FieldType::Int16BE => Ok(i16::from_be_bytes(data[0..2].try_into().unwrap()).fmt(f)?),
            FieldType::Int32 => Ok(i32::from_le_bytes(data[0..4].try_into().unwrap()).fmt(f)?),
            FieldType::Int32BE => Ok(i32::from_be_bytes(data[0..4].try_into().unwrap()).fmt(f)?),
            FieldType::Bytes => Ok(write!(f, "{:?}", data)?),
            FieldType::SubFrame(_) => Ok(()), // Handled in DecodedChunk
        }
    }
}

impl DecodedField {
    pub fn fmt(&self, data: &[u8], level: usize, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for _ in 0..level {
            write!(f, "  ")?; // Indentation for sub-fields
        }
        write!(f, "{:<20} : ", self.name)?;
        let d = &data[self.segment.clone()];
        match &self.display_type {
            DisplayType::Hex => write!(f, "{}", hex::encode_upper(d))?,
            DisplayType::Decimal => self.field_type.fmt_value(d, f)?,
            DisplayType::Binary => todo!(),
            DisplayType::Ascii => String::from_utf8_lossy(d).fmt(f)?,
        }
        writeln!(f)?;
        Ok(())
    }
}

impl DecodedChunk {
    pub fn new(fields: Vec<DecodedField>) -> Self {
        DecodedChunk { fields }
    }

    pub fn fmt(
        &self,
        data: &[u8],
        level: usize,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        for field in &self.fields {
            field.fmt(data, level, f)?;
            if let FieldType::SubFrame(sub_chunk) = &field.field_type {
                sub_chunk.fmt(data, level + 1, f)?;
            }
        }
        Ok(())
    }
}
pub trait FrameDecoder 
{
    fn decode_frame(&self,decoder : &Decoder, frame: &Frame, segment :Segment  ) -> Option<DecodedChunk>;
    fn decoder_name(&self) -> &str;
  }

pub struct Decoder {
    // Decoder implementation

    decoders: HashMap<String,Box<dyn FrameDecoder>>,
}

impl Decoder {
    pub fn new() -> Self {
        Decoder {
            decoders: HashMap::new(),
        }
    }

    fn register_decoder(&mut self, decoder: Box<dyn FrameDecoder>) {
        self.decoders.insert(decoder.decoder_name().to_string(), decoder);
    }

    fn decode(&self,decoder_name: &str, frame: &Frame, segment: Segment) -> Option<DecodedChunk> {
        self.decoders.get(decoder_name)?.decode_frame(self,frame, segment)
    }
}