use crate::frame::*;

pub struct ZWaveFrameDecoder;

impl FrameDecoder for ZWaveFrameDecoder {
    fn decode_frame(
        &self,
        _library: &DecoderLibrary,
        _frame: &MACFrame,
        segment: Segment,
    ) -> Option<DecodedChunk> {
        Some(DecodedChunk {
            fields: vec![field!(
                "data",
                "TODO: Data",
                segment.start,
                segment.end,
                FieldType::Bytes,
                DisplayType::Hex
            )],
        })
    }

    fn decoder_name(&self) -> &str {
        "ZWaveFrameDecoder"
    }
}
