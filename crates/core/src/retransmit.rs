// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Spotting frames that are retransmissions of an earlier one.
//!
//! ITU-T G.9959 (01/2015) clause 8.1.3.3.7 requires that "the MAC layer
//! shall use the same sequence number for the initial transmission and for
//! all retransmissions of a given MPDU", and that a sender advances the
//! value for each new MPDU, wrapping 0xf back to 0x1.
//!
//! So a sender repeating the sequence number it last used is retrying,
//! and a sender that has moved on has sent something new. Only the last
//! value each sender used matters, which makes this a small amount of
//! state and independent of timing.
use std::collections::HashMap;

use crate::trace::TraceFrame;

/// Where a frame sits in a trace.
///
/// Frames have no absolute number — establishing one would mean counting
/// every frame from the start of the file — so they are addressed by the
/// record holding them and their position within it.
pub type FrameId = (u64, usize);

/// What a frame is, relative to earlier ones from the same sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Retransmission {
    /// How many times this MPDU has been seen before. Zero for the first.
    pub attempt: u32,
    /// The first transmission this one repeats, when it is a retry.
    pub original: Option<FrameId>,
}

impl Retransmission {
    pub fn is_retransmission(&self) -> bool {
        self.attempt > 0
    }
}

/// A sender, identified by its network and node id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Sender {
    home_id: u32,
    source: u16,
}

/// The last sequence number a sender used, and how many times.
#[derive(Debug, Clone, Copy)]
struct LastSent {
    sequence: u8,
    attempt: u32,
    /// The first frame that carried this sequence number.
    original: FrameId,
}

/// Tracks the sequence number each sender last used.
///
/// Frames must be offered in capture order.
#[derive(Debug, Default)]
pub struct RetransmitTracker {
    last: HashMap<Sender, LastSent>,
}

impl RetransmitTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Classify the next frame, which sits at `id`.
    ///
    /// Frames that carry no sender or sequence number — beams, and
    /// anything whose header did not decode — are never flagged, and do
    /// not disturb what is known about a sender.
    pub fn observe(&mut self, frame: &TraceFrame, id: FrameId) -> Retransmission {
        let (Some(home_id), Some(source), Some(sequence)) =
            (frame.home_id(), frame.source(), frame.sequence_number())
        else {
            return Retransmission::default();
        };

        let sender = Sender { home_id, source: source as u16 };
        let sequence = sequence as u8;

        let (attempt, original) = match self.last.get(&sender) {
            // The sender repeated itself, so this is a retry of whatever
            // first carried the sequence number.
            Some(last) if last.sequence == sequence => (last.attempt + 1, last.original),
            // A new sequence number: a new MPDU, and its own original.
            _ => (0, id),
        };
        self.last.insert(sender, LastSent { sequence, attempt, original });

        Retransmission {
            attempt,
            original: (attempt > 0).then_some(original),
        }
    }

    /// Number of senders being tracked.
    pub fn senders(&self) -> usize {
        self.last.len()
    }

    /// Forget every sender.
    pub fn clear(&mut self) {
        self.last.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decoder::frame_definition::{
        DecodedHeader,
        DecodedParam,
    };
    use crate::pti::Direction;

    /// A frame carrying just the fields retransmission detection reads.
    fn frame(
        time_ms: i64,
        home: u32,
        src: u64,
        dst: u64,
        seq: u64,
        is_ack: bool,
    ) -> TraceFrame {
        fn param(name: &str, value: u64) -> DecodedParam {
            use crate::decoder::frame_definition::{
                BitSpan,
                ParamType,
            };
            DecodedParam {
                name: name.to_string(),
                text: name.to_string(),
                span: BitSpan { start_bit: 0, bits: 8 },
                value: Some(value),
                bytes: Vec::new(),
                symbol: None,
                param_type: ParamType::Number,
                children: Vec::new(),
            }
        }

        let mut properties2 = param("Properties2", seq);
        properties2.children.push(param("SequenceNumber", seq));

        TraceFrame {
            index: 1,
            time_ms,
            delta_ms: 0,
            direction: Direction::Rx,
            channel: 0,
            speed: 2,
            region: 1,
            rssi: 0,
            beam_count: None,
            mpdu: Vec::new(),
            header: Some(DecodedHeader {
                header_key: if is_ack { 19 } else { 13 },
                header_name: String::new(),
                header_text: String::new(),
                is_ack,
                is_routed: false,
                is_multicast: false,
                is_error: false,
                params: vec![
                    param("HomeID", home as u64),
                    param("SourceNodeID", src),
                    properties2,
                    param("DestinationNodeID", dst),
                ],
                payload_offset: 0,
                crc_ok: true,
            }),
            application: None,
        }
    }

    #[test]
    fn a_sender_repeating_its_sequence_number_is_retrying() {
        let mut t = RetransmitTracker::new();
        assert_eq!(t.observe(&frame(0, 1, 1, 2, 5, false), (0, 0)).attempt, 0);
        assert_eq!(t.observe(&frame(100, 1, 1, 2, 5, false), (0, 0)).attempt, 1);
        assert_eq!(t.observe(&frame(200, 1, 1, 2, 5, false), (0, 0)).attempt, 2);
    }

    #[test]
    fn a_retry_points_back_at_the_first_transmission() {
        let mut t = RetransmitTracker::new();
        let first = (2048, 0);

        let original = t.observe(&frame(0, 1, 1, 2, 5, false), first);
        assert_eq!(original.attempt, 0);
        assert_eq!(original.original, None, "the first send has nothing to point at");

        let second = t.observe(&frame(30, 1, 1, 2, 5, false), (3000, 0));
        assert_eq!(second.attempt, 1);
        assert_eq!(second.original, Some(first));

        // The third points at the first as well, not at the second, so
        // every retry links to where the exchange began.
        let third = t.observe(&frame(60, 1, 1, 2, 5, false), (4000, 1));
        assert_eq!(third.attempt, 2);
        assert_eq!(third.original, Some(first));
    }

    #[test]
    fn a_new_sequence_number_starts_a_new_original() {
        let mut t = RetransmitTracker::new();
        t.observe(&frame(0, 1, 1, 2, 5, false), (100, 0));
        t.observe(&frame(10, 1, 1, 2, 6, false), (200, 0));
        // A retry of the second frame points at the second, not the first.
        let retry = t.observe(&frame(20, 1, 1, 2, 6, false), (300, 0));
        assert_eq!(retry.original, Some((200, 0)));
    }

    #[test]
    fn timing_does_not_matter() {
        // The rule is about the sequence number, not how long ago.
        let mut t = RetransmitTracker::new();
        t.observe(&frame(0, 1, 1, 2, 5, false), (0, 0));
        assert_eq!(t.observe(&frame(3_600_000, 1, 1, 2, 5, false), (0, 0)).attempt, 1);
    }

    #[test]
    fn a_new_sequence_number_is_a_new_frame() {
        let mut t = RetransmitTracker::new();
        assert_eq!(t.observe(&frame(0, 1, 1, 2, 5, false), (0, 0)).attempt, 0);
        assert_eq!(t.observe(&frame(10, 1, 1, 2, 6, false), (0, 0)).attempt, 0);
        // Returning to an earlier value still counts as new, because it
        // is not the one the sender last used.
        assert_eq!(t.observe(&frame(20, 1, 1, 2, 5, false), (0, 0)).attempt, 0);
    }

    #[test]
    fn the_destination_is_not_part_of_the_rule() {
        // A sender advances its sequence number per MPDU, whoever it is
        // addressed to, so the same value to a different node is a retry.
        let mut t = RetransmitTracker::new();
        assert_eq!(t.observe(&frame(0, 1, 1, 2, 5, false), (0, 0)).attempt, 0);
        assert_eq!(t.observe(&frame(10, 1, 1, 9, 5, false), (0, 0)).attempt, 1);
    }

    #[test]
    fn senders_are_tracked_separately() {
        let mut t = RetransmitTracker::new();
        assert_eq!(t.observe(&frame(0, 1, 1, 2, 5, false), (0, 0)).attempt, 0);
        // A different node in the same network.
        assert_eq!(t.observe(&frame(10, 1, 3, 2, 5, false), (0, 0)).attempt, 0);
        // And the same node id in a different network.
        assert_eq!(t.observe(&frame(20, 2, 1, 2, 5, false), (0, 0)).attempt, 0);
        assert_eq!(t.senders(), 3);
    }

    #[test]
    fn an_ack_from_the_other_node_does_not_disturb_the_sender() {
        let mut t = RetransmitTracker::new();
        t.observe(&frame(0, 1, 1, 2, 5, false), (0, 0));
        // Node 2 acknowledges; that is node 2's own sequence state.
        t.observe(&frame(5, 1, 2, 1, 5, true), (0, 0));
        // Node 1 repeating 5 is still a retry.
        assert_eq!(t.observe(&frame(10, 1, 1, 2, 5, false), (0, 0)).attempt, 1);
    }

    #[test]
    fn frames_without_the_fields_are_never_flagged() {
        let mut t = RetransmitTracker::new();
        let mut bare = frame(0, 1, 1, 2, 5, false);
        bare.header = None;
        assert_eq!(t.observe(&bare, (0, 0)).attempt, 0);
        assert_eq!(t.observe(&bare, (0, 0)).attempt, 0);
        // And they leave no state behind.
        assert_eq!(t.senders(), 0);
    }
}
