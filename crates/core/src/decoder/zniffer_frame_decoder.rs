use crate::decoder::{
    DecodedChunk, DecodedField, DecoderLibrary, DisplayType, FieldType, MACFrame, FrameDecoder,
    Segment, field, zwave_frame_decoder,
};

/// Decoder for Zniffer frames
pub struct ZnifferFrameDecoder;

impl FrameDecoder for ZnifferFrameDecoder {
    fn decode_frame(
        &self,
        decoder: &DecoderLibrary,
        frame: &MACFrame,
        segment: Segment,
    ) -> Option<DecodedChunk> {
        let zwave_frame_decoder =
            decoder.decode("ZWaveFrameDecoder", frame, 6..frame.data[5] as usize + 6)?;
        Some(DecodedChunk {
            fields: vec![
                field!(
                    "channel",
                    "Channel and speed",
                    0,
                    FieldType::UInt8,
                    DisplayType::Decimal
                ),
                field!(
                    "region",
                    "Region",
                    1,
                    FieldType::UInt8,
                    DisplayType::Decimal
                ),
                field!("rssi", "RSSI", 2, FieldType::Int8, DisplayType::Decimal),
                field!(
                    "start_of_data",
                    "Start of Data",
                    3,
                    FieldType::UInt16,
                    DisplayType::Hex
                ),
                field!(
                    "length",
                    "Length",
                    5,
                    FieldType::UInt8,
                    DisplayType::Decimal
                ),
                field!(
                    "zwave_frame",
                    "Z-WaveFrame",
                    6,
                    frame.data[5] as usize + 6,
                    FieldType::SubFrame(zwave_frame_decoder),
                    DisplayType::None
                ),
            ],
        })
    }

    fn decoder_name(&self) -> &str {
        "ZnifferFrameDecoder"
    }
}

