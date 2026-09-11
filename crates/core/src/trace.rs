// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! End-to-end decoding of a ZLF trace, from file bytes to displayable frames.
use std::io::Cursor;

use serde::{
    Deserialize,
    Serialize,
};

use crate::decoder::command_class::{
    Decoded,
    ZwClasses,
};
use crate::decoder::frame_definition::{
    DecodedHeader,
    FrameDefinition,
};
use crate::pti::{
    self,
    Direction,
};
use crate::zlf::{
    ApiType,
    ZlfError,
    ZlfReader,
};
use crate::zniffer_parser;

/// One frame of a trace, decoded as far as it can be.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceFrame {
    /// One-based position within the batch this frame was decoded in: the
    /// whole file for [`Trace`], or one window for [`crate::cursor`].
    ///
    /// Not a frame number for the trace as a whole. Numbering absolutely
    /// would mean counting every preceding frame, which for a trace read
    /// over the network means fetching all of it.
    pub index: usize,
    /// Capture time in milliseconds since the Unix epoch.
    pub time_ms: i64,
    /// Milliseconds since the previous frame.
    pub delta_ms: i64,
    pub direction: Direction,
    pub channel: u8,
    pub speed: u8,
    pub region: u8,
    pub rssi: i8,
    /// Repetitions, when the frame is a wake-up beam.
    pub beam_count: Option<u16>,
    /// The Z-Wave MPDU.
    pub mpdu: Vec<u8>,
    /// Decoded MPDU header, absent when no header type matched.
    pub header: Option<DecodedHeader>,
    /// Decoded application payload, absent when the frame carries none.
    pub application: Option<Decoded>,
}

impl TraceFrame {
    /// Home id, as shown in the frame list.
    pub fn home_id(&self) -> Option<u32> {
        self.header.as_ref()?.value("HomeID").map(|v| v as u32)
    }

    pub fn source(&self) -> Option<u64> {
        self.header.as_ref()?.value("SourceNodeID")
    }

    pub fn destination(&self) -> Option<u64> {
        self.header.as_ref()?.value("DestinationNodeID")
    }

    /// Frame type name, e.g. `Singlecast`.
    pub fn header_text(&self) -> &str {
        self.header.as_ref().map_or("Unknown", |h| h.header_text.as_str())
    }

    /// Sequence number from the MPDU header, when the layout carries one.
    ///
    /// A 4-bit field, so it wraps every 15 frames: the value alone does
    /// not identify a transmission.
    pub fn sequence_number(&self) -> Option<u64> {
        self.header.as_ref()?.value("Properties2.SequenceNumber")
    }

    /// True when this frame acknowledges another.
    pub fn is_ack(&self) -> bool {
        self.header.as_ref().is_some_and(|h| h.is_ack)
    }

    /// Header key of the decoded MPDU header, if one matched.
    pub fn header_key(&self) -> Option<u8> {
        self.header.as_ref().map(|h| h.header_key)
    }

    /// True when this frame is a singlecast, routed or not.
    ///
    /// Only a singlecast is acknowledged, and so only a singlecast is
    /// retransmitted: G.9959 8.1.5.1.4.2 has the ACK request subfield
    /// ignored on every other MPDU type. The keys are the SINGLECAST,
    /// SINGLECAST24, SINGLECASTLR, ROUTED_SINGLECAST and
    /// ROUTED_SINGLECAST24 headers of FrameDefinition.xml.
    pub fn is_singlecast(&self) -> bool {
        matches!(self.header_key(), Some(13 | 14 | 70 | 22 | 23))
    }

    /// True when this frame is a wake-up beam.
    pub fn is_beam(&self) -> bool {
        matches!(self.header_key(), Some(60..=63)) || self.beam_count.is_some()
    }

    /// Text for the Application column.
    ///
    /// Follows the desktop Zniffer's converter: beams report their
    /// repetition count, routed acks and errors name themselves, plain
    /// acks and anything above the explorer range say nothing, and the
    /// rest show the decoded command.
    pub fn application_summary(&self) -> String {
        match self.header_key() {
            // Beam, and Long Range beam, carry a repetition count. The
            // start and stop markers have nothing to say.
            Some(60) | Some(63) => {
                format!("WakeUp Beam({})", self.beam_count.unwrap_or(1))
            }
            Some(61) | Some(62) => String::new(),
            // Routed acknowledge, in its three header variants.
            Some(25..=27) => "Routed Ack".to_string(),
            // Routed error.
            Some(28..=30) => "Routed Error".to_string(),
            // Plain acks carry no payload, and neither does anything past
            // the explorer headers.
            Some(19..=21) => String::new(),
            Some(key) if key > 38 => String::new(),
            _ => self.application.as_ref().map(|a| a.summary()).unwrap_or_default(),
        }
    }

    /// Speed as shown in the UI.
    pub fn speed_text(&self) -> &'static str {
        match self.speed {
            0 => "9.6 kbps",
            1 => "40 kbps",
            2 => "100 kbps",
            3 => "LR 100 kbps",
            _ => "unknown",
        }
    }

    /// True when the frame's checksum verified.
    pub fn crc_ok(&self) -> bool {
        self.header.as_ref().is_some_and(|h| h.crc_ok)
    }
}

/// A decoded trace.
pub struct Trace {
    pub frames: Vec<TraceFrame>,
}

impl Trace {
    /// Decode a ZLF trace held in memory.
    pub fn parse(bytes: &[u8]) -> Result<Self, ZlfError> {
        let definitions = Definitions::load();
        Self::parse_with(bytes, &definitions)
    }

    /// Decode a trace using already-loaded definitions.
    pub fn parse_with(bytes: &[u8], definitions: &Definitions) -> Result<Self, ZlfError> {
        let mut reader = ZlfReader::new(Cursor::new(bytes))?;
        let mut frames = Vec::new();
        let mut parser = zniffer_parser::Parser::new();
        let mut previous: Option<i64> = None;

        reader.read_records(|record| {
            let time_ms = record.timestamp.unix_millis();
            match record.api_type {
                ApiType::Pti | ApiType::PtiDiagnostic => {
                    for frame in pti::parse_payload(&record.payload) {
                        push(&mut frames, &mut previous, definitions, time_ms, frame);
                    }
                }
                ApiType::Zniffer => {
                    // The legacy dialect is a byte stream; feed it through the
                    // state machine and take whatever frames complete.
                    for byte in &record.payload {
                        if let zniffer_parser::ParserResult::ValidFrame { frame } =
                            parser.parse(*byte)
                        {
                            let direction = if record.is_outcome {
                                Direction::Tx
                            } else {
                                Direction::Rx
                            };
                            push(
                                &mut frames,
                                &mut previous,
                                definitions,
                                time_ms,
                                pti::PtiFrame {
                                    direction,
                                    region: frame.region as u8,
                                    channel: frame.channel,
                                    speed: frame.speed,
                                    rssi: frame.rssi as i8,
                                    beam_count: None,
                                    mpdu: frame.payload,
                                },
                            );
                        }
                    }
                }
                // Attachments and other record types carry no radio frames.
                _ => {}
            }
        })?;

        Ok(Self { frames })
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}

/// Decode one PTI frame into a [`TraceFrame`].
///
/// Shared by the whole-file decoder and the on-demand cursor so the two
/// cannot drift.
pub(crate) fn build_frame(
    definitions: &Definitions,
    index: usize,
    time_ms: i64,
    delta_ms: i64,
    frame: pti::PtiFrame,
) -> TraceFrame {
    let header = crate::decoder::frame_definition::decode_header(
        &definitions.frame_definition,
        &frame.mpdu,
        frame.region,
        frame.speed,
    )
    .ok();

    // The application payload sits between the header and the trailing CRC.
    let application = header.as_ref().and_then(|h| {
        let crc = if frame.speed > 1 { 2 } else { 1 };
        let end = frame.mpdu.len().checked_sub(crc)?;
        if h.payload_offset >= end {
            return None;
        }
        Some(crate::decoder::command_class::decode(
            &definitions.command_classes,
            &frame.mpdu[h.payload_offset..end],
        ))
    });

    TraceFrame {
        index,
        time_ms,
        delta_ms,
        direction: frame.direction,
        channel: frame.channel,
        speed: frame.speed,
        region: frame.region,
        rssi: frame.rssi,
        beam_count: frame.beam_count,
        mpdu: frame.mpdu,
        header,
        application,
    }
}

/// Decode one PTI frame and append it to the trace.
fn push(
    frames: &mut Vec<TraceFrame>,
    previous: &mut Option<i64>,
    definitions: &Definitions,
    time_ms: i64,
    frame: pti::PtiFrame,
) {
    let delta_ms = previous.map_or(0, |p| time_ms - p);
    *previous = Some(time_ms);
    let index = frames.len() + 1;
    frames.push(build_frame(definitions, index, time_ms, delta_ms, frame));
}

/// The XML definitions, parsed once and reused across traces.
pub struct Definitions {
    pub frame_definition: FrameDefinition,
    pub command_classes: ZwClasses,
}

impl Definitions {
    /// Parse the vendored definitions.
    ///
    /// # Panics
    /// Panics if the bundled XML is malformed, which would be a build error.
    pub fn load() -> Self {
        Self {
            frame_definition: FrameDefinition::load().expect("bundled FrameDefinition.xml"),
            command_classes: ZwClasses::load().expect("bundled zwave.xml"),
        }
    }
}

impl Default for Definitions {
    fn default() -> Self {
        Self::load()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame with only the fields the Application column consults.
    fn frame_with(header_key: Option<u8>, beam_count: Option<u16>) -> TraceFrame {
        use crate::decoder::frame_definition::DecodedHeader;
        TraceFrame {
            index: 1,
            time_ms: 0,
            delta_ms: 0,
            direction: Direction::Rx,
            channel: 0,
            speed: 0,
            region: 0,
            rssi: 0,
            beam_count,
            mpdu: Vec::new(),
            header: header_key.map(|key| DecodedHeader {
                header_key: key,
                header_name: String::new(),
                header_text: String::new(),
                is_ack: false,
                is_routed: false,
                is_multicast: false,
                is_error: false,
                params: Vec::new(),
                payload_offset: 0,
                crc_ok: true,
            }),
            application: None,
        }
    }

    #[test]
    fn beams_report_their_repetition_count() {
        assert_eq!(frame_with(Some(60), Some(7)).application_summary(), "WakeUp Beam(7)");
        // Long Range beams too.
        assert_eq!(frame_with(Some(63), Some(2)).application_summary(), "WakeUp Beam(2)");
        // A beam with no counter is still one beam.
        assert_eq!(frame_with(Some(60), None).application_summary(), "WakeUp Beam(1)");
        // Start and stop markers say nothing.
        assert_eq!(frame_with(Some(61), Some(3)).application_summary(), "");
        assert_eq!(frame_with(Some(62), Some(3)).application_summary(), "");
    }

    #[test]
    fn acks_and_routed_frames_name_themselves() {
        for key in 25..=27 {
            assert_eq!(frame_with(Some(key), None).application_summary(), "Routed Ack");
        }
        for key in 28..=30 {
            assert_eq!(frame_with(Some(key), None).application_summary(), "Routed Error");
        }
        // A plain ack carries no payload.
        for key in 19..=21 {
            assert_eq!(frame_with(Some(key), None).application_summary(), "");
        }
        // Nor does anything past the explorer headers.
        assert_eq!(frame_with(Some(70), None).application_summary(), "");
    }

    #[test]
    fn recognises_a_beam() {
        assert!(frame_with(Some(60), None).is_beam());
        assert!(frame_with(Some(63), None).is_beam());
        assert!(!frame_with(Some(13), None).is_beam());
    }

    #[test]
    fn rejects_a_non_zlf_file() {
        assert!(Trace::parse(&[0u8; 16]).is_err());
    }
}
