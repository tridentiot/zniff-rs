use crate::frame::{self, DecodedChunk, DecoderLibrary, MACFrame, FrameDecoder};


pub struct Decoder {
    frame_decoder : DecoderLibrary

}


impl Decoder {
    pub fn new() -> Self {
        let mut frame_decoder = DecoderLibrary::new();
        frame_decoder.register_decoder(Box::new( frame::ZnifferFrameDecoder {}));
        frame_decoder.register_decoder(Box::new( frame::ZWaveFrameDecoder {}));
        Decoder { frame_decoder }
    }

    pub fn decode(&self, frame: &MACFrame) -> Option<DecodedChunk> {
        match frame.protocol {
            frame::MACProtocol::ZWave => {
                self.frame_decoder.decode("ZnifferFrameDecoder", frame, 0..frame.data.len())
            },
            ///TODO other protocols
            _ => None,
        }
    }
}



#[cfg(test)]
mod test_decoder {
    use crate::frame::MACProtocol;

    use super::*;

    #[test]
    fn test_frame() {
        let decoder = Decoder::new();
        let mut frame = MACFrame {
            id: 1,
            timestamp: 123456,
            protocol: MACProtocol::ZWave,
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
        let decoded_chunk = decoder
            .decode(&frame)
            .unwrap();

        frame.top = decoded_chunk;
        println!("{}", frame);
    }
}
