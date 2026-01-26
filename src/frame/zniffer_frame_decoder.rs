use quick_xml::se;

use crate::frame::{
    field, DecodedChunk, DecodedField, Decoder, DisplayType, FieldType, Frame, FrameDecoder, Segment,
};

struct SnifferFrameDecoder;

impl FrameDecoder for SnifferFrameDecoder {
    fn decode_frame(
        &self,
        decoder: &Decoder,
        frame: &Frame,
        segment: Segment,
    ) -> Option<DecodedChunk> {
        Some(DecodedChunk {
            fields: vec![
                field!("channel", "Channel and speed", 0, FieldType::UInt8, DisplayType::Decimal),
                field!("region", "Region", 1, FieldType::UInt8, DisplayType::Decimal),
                field!("rssi", "RSSI", 2, FieldType::Int8, DisplayType::Decimal),
                field!("start_of_data", "Start of Data", 3, FieldType::UInt16, DisplayType::Hex),
                field!("length", "Length", 5, FieldType::UInt8, DisplayType::Decimal),
                field!("data", "Data", 6, frame.data[5] as usize + 6, FieldType::Bytes, DisplayType::Hex),
            ],
        })
    }

    fn decoder_name(&self) -> &str {
        "SnifferFrameDecoder"
    }
}

#[cfg(test)]
mod test_zniffer_frame_decoder {
    use crate::frame::Protocol;

    use super::*;

    #[test]
    fn test_frame() {
        let decoder = SnifferFrameDecoder;
        let mut frame = Frame {
            id: 1,
            timestamp: 123456,
            protocol: Protocol::ZWave,
            top: DecodedChunk { fields: vec![] },
            data: vec![
                0x20, // Channel and speed
                0x00, // Region
                0x9D, // RSSI
                0x21, 0x03, // Start of data
                0x12, // Length
                0xE2, 0xEA, 0x36, 0xC3, 0x01, 0x81, 0x0D, 0x12, 0x20, 0x0B, 0x10, 0x02, 0x41, 0x7F,
                0x7F, 0x7F, 0x7F, 0xE5,
            ],
        };
        let segment = 0..frame.data.len();
        let decoded_chunk = decoder
            .decode_frame(&Decoder::new(), &frame, segment)
            .unwrap();

        frame.top = decoded_chunk;
        println!("{}", frame);
    }
}
