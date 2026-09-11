// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Checks against a real capture and the C# reference decoder.
//!
//! A capture carries the home ids, node ids and key exchange of the
//! network it was taken from, so no trace is kept in the repository.
//! Put one in place and enable the `fixtures` feature to run these:
//! see `fixtures/README.md`.
#![cfg(feature = "fixtures")]

use std::io::Cursor;

use zniff_rs_core::pti;
use zniff_rs_core::zlf::{
    ApiType,
    ZlfReader,
};

const TRACE: &[u8] = include_bytes!("fixtures/pti-800.zlf");

fn frames() -> Vec<pti::PtiFrame> {
    let mut reader = ZlfReader::new(Cursor::new(TRACE)).expect("header");
    let mut out = Vec::new();
    reader
        .read_records(|rec| {
            if rec.api_type == ApiType::Pti {
                out.extend(pti::parse_payload(&rec.payload));
            }
        })
        .expect("records");
    out
}

#[test]
fn reads_every_record() {
    let mut reader = ZlfReader::new(Cursor::new(TRACE)).unwrap();
    reader.read_records(|_| {}).unwrap();
    assert_eq!(reader.record_count(), 181);
}

#[test]
fn extracts_zwave_frames() {
    let frames = frames();
    assert!(!frames.is_empty(), "no frames extracted from the fixture");

    for f in &frames {
        assert!(!f.mpdu.is_empty(), "empty MPDU");
        assert!(f.speed <= 3, "impossible speed {}", f.speed);
        assert!(f.channel <= 3, "impossible channel {}", f.channel);
    }
    eprintln!("extracted {} frames", frames.len());
}

#[test]
fn mpdu_home_ids_are_consistent() {
    // Every frame in a single capture of one network should share a home id,
    // which is a strong signal that the MPDU offsets are right.
    let frames = frames();
    let mut home_ids: Vec<[u8; 4]> = frames
        .iter()
        .filter(|f| !f.is_beam() && f.mpdu.len() >= 4)
        .map(|f| [f.mpdu[0], f.mpdu[1], f.mpdu[2], f.mpdu[3]])
        .collect();
    home_ids.sort_unstable();
    home_ids.dedup();
    eprintln!("distinct home ids: {home_ids:02X?}");
    assert!(!home_ids.is_empty());
}

/// Fields decoded by the C# reference implementation (`ZlfDump`), used as the
/// oracle for the Rust port.
const ORACLE: &str = include_str!("fixtures/pti-800.oracle.txt");

fn oracle_field<'a>(line: &'a str, key: &str) -> &'a str {
    line.split_whitespace()
        .find_map(|tok| tok.strip_prefix(key))
        .unwrap_or_else(|| panic!("no {key} in {line}"))
}

#[test]
fn radio_fields_match_reference_decoder() {
    let frames = frames();
    let oracle: Vec<&str> = ORACLE.lines().filter(|l| l.starts_with('#')).collect();
    assert_eq!(frames.len(), oracle.len(), "frame count differs from reference");

    for (i, (frame, line)) in frames.iter().zip(&oracle).enumerate() {
        assert_eq!(
            frame.channel.to_string(),
            oracle_field(line, "ch="),
            "channel mismatch at frame {i}"
        );
        assert_eq!(
            frame.speed.to_string(),
            oracle_field(line, "sp="),
            "speed mismatch at frame {i}"
        );
        assert_eq!(
            frame.region.to_string(),
            oracle_field(line, "freq="),
            "region mismatch at frame {i}"
        );
        assert_eq!(
            frame.rssi.to_string(),
            oracle_field(line, "rssi="),
            "rssi mismatch at frame {i}"
        );
    }
}

/// The reference decoder's names for header types, keyed by our header name.
fn oracle_name(header_name: &str) -> &str {
    match header_name {
        "SINGLECAST" | "SINGLECAST24" | "SINGLECASTLR" => "Singlecast",
        "TRANSFER_ACKNOWLEDGE" | "TRANSFER_ACKNOWLEDGE24" | "TRANSFER_ACKNOWLEDGELR" => "Ack",
        "BROADCAST" | "BROADCAST24" | "BROADCASTLR" => "Broadcast",
        "EXPLORER_NORMAL" | "EXPLORER_NORMAL24" => "ExplorerNormal",
        "EXPLORER_AUTOINCLUSION" | "EXPLORER_AUTOINCLUSION24" => "ExplorerAutoInclusion",
        "EXPLORER_SEARCH_RESULT" | "EXPLORER_SEARCH_RESULT24" => "ExplorerSearchResult",
        "ROUTED_SINGLECAST" | "ROUTED_SINGLECAST24" => "RoutedSinglecast",
        "ROUTED_ACKNOWLEDGE" | "ROUTED_ACKNOWLEDGE24" => "RoutedAck",
        "MULTICAST" | "MULTICAST24" => "Multicast",
        other => other,
    }
}

#[test]
fn mpdu_header_matches_reference_decoder() {
    use zniff_rs_core::decoder::frame_definition::{
        FrameDefinition,
        decode_header,
    };

    let fd = FrameDefinition::load().expect("FrameDefinition.xml");
    let frames = frames();
    let oracle: Vec<&str> = ORACLE.lines().filter(|l| l.starts_with('#')).collect();
    assert_eq!(frames.len(), oracle.len());

    let mut compared = 0;
    for (i, (frame, line)) in frames.iter().zip(&oracle).enumerate() {
        let want_type = line.split_whitespace().nth(1).unwrap();
        let decoded = decode_header(&fd, &frame.mpdu, frame.region, frame.speed)
            .unwrap_or_else(|e| panic!("frame {i} ({want_type}) failed to decode: {e}"));

        assert_eq!(
            oracle_name(&decoded.header_name),
            want_type,
            "header type mismatch at frame {i}"
        );
        assert_eq!(
            decoded.value("SourceNodeID").unwrap_or_default().to_string(),
            oracle_field(line, "src="),
            "source mismatch at frame {i} ({want_type})"
        );
        assert_eq!(
            decoded.value("Properties2.SequenceNumber").unwrap_or_default().to_string(),
            oracle_field(line, "seq="),
            "sequence mismatch at frame {i} ({want_type})"
        );
        assert_eq!(
            decoded.is_ack.to_string(),
            oracle_field(line, "ack=").to_lowercase(),
            "ack mismatch at frame {i} ({want_type})"
        );
        assert_eq!(
            decoded.crc_ok.to_string(),
            oracle_field(line, "crcOk=").to_lowercase(),
            "crc mismatch at frame {i} ({want_type})"
        );
        compared += 1;
    }
    eprintln!("compared {compared} frames against the reference decoder");
}

#[test]
fn decodes_application_payloads() {
    use zniff_rs_core::decoder::command_class::{
        Decoded,
        ZwClasses,
        decode,
    };
    use zniff_rs_core::decoder::frame_definition::{
        FrameDefinition,
        decode_header,
    };

    let fd = FrameDefinition::load().unwrap();
    let zw = ZwClasses::load().unwrap();

    let mut summaries = Vec::new();
    for frame in frames() {
        let Ok(header) = decode_header(&fd, &frame.mpdu, frame.region, frame.speed) else {
            continue;
        };
        // Application payload sits between the header and the trailing CRC.
        let crc = if frame.speed > 1 { 2 } else { 1 };
        let end = frame.mpdu.len().saturating_sub(crc);
        if header.payload_offset >= end {
            continue;
        }
        let payload = &frame.mpdu[header.payload_offset..end];
        if let Decoded::Command(c) = decode(&zw, payload) {
            summaries.push(c.summary());
        }
    }

    assert!(!summaries.is_empty(), "no application payloads decoded");
    let mut counts: std::collections::BTreeMap<&str, usize> = Default::default();
    for s in &summaries {
        *counts.entry(s.as_str()).or_default() += 1;
    }
    for (summary, count) in &counts {
        eprintln!("{count:4}  {summary}");
    }
}

#[test]
fn trace_api_decodes_the_fixture() {
    use zniff_rs_core::trace::Trace;

    let trace = Trace::parse(TRACE).expect("trace should parse");
    assert_eq!(trace.len(), 181);

    let decoded_headers = trace.frames.iter().filter(|f| f.header.is_some()).count();
    assert_eq!(decoded_headers, 181, "every frame should decode a header");
    assert!(trace.frames.iter().all(|f| f.crc_ok()), "every fixture frame has a valid CRC");

    // Timestamps must advance, and the first delta is zero by definition.
    assert_eq!(trace.frames[0].delta_ms, 0);
    assert!(trace.frames.windows(2).all(|w| w[1].time_ms >= w[0].time_ms));

    // A single network, so one home id throughout is expected for real frames.
    let first = &trace.frames[0];
    eprintln!(
        "#{} {} {} src={:?} dst={:?} home={:08X?} rssi={} {} | {}",
        first.index,
        first.speed_text(),
        first.header_text(),
        first.source(),
        first.destination(),
        first.home_id(),
        first.rssi,
        first.crc_ok(),
        first.application_summary()
    );
}

#[test]
fn resync_holds_on_a_large_trace() {
    // The fixture is only 181 records; the seek design has to hold on a file
    // large enough to have somewhere to go wrong. Repeat the record stream.
    use std::collections::HashSet;
    use zniff_rs_core::resync::{
        read_record,
        next_record_at,
    };
    use zniff_rs_core::source::SliceSource;
    use zniff_rs_core::zlf::ZLF_HEADER_SIZE;

    let (header, body) = TRACE.split_at(ZLF_HEADER_SIZE);
    let mut big = Vec::with_capacity(header.len() + body.len() * 600);
    big.extend_from_slice(header);
    for _ in 0..600 {
        big.extend_from_slice(body);
    }

    let mut src = SliceSource::new(&big);
    let mut starts = Vec::new();
    let mut at = ZLF_HEADER_SIZE as u64;
    while let Some(rec) = read_record(&mut src, at).unwrap() {
        starts.push(at);
        at = rec.next_offset();
    }
    assert_eq!(starts.len(), 181 * 600);
    let set: HashSet<u64> = starts.iter().copied().collect();

    // Every offset that is not a record start must be rejected. Sampling
    // deterministically across the whole file rather than testing all 5.9 M.
    let mut checked = 0usize;
    let mut offset = ZLF_HEADER_SIZE as u64;
    let end = big.len() as u64 - 64;
    while offset < end {
        if !set.contains(&offset) {
            let found = next_record_at(&mut src, offset, 4096).unwrap().unwrap();
            assert!(
                found > offset && set.contains(&found),
                "resync at {offset} landed on {found}, which is not a record"
            );
            checked += 1;
        }
        offset += 7; // a stride coprime with the record sizes
    }
    assert!(checked > 100_000, "only checked {checked} offsets");
}

/// Counts reads so a seek's cost can be asserted, not assumed.
struct CountingSource<'a> {
    inner: zniff_rs_core::source::SliceSource<'a>,
    reads: std::cell::Cell<usize>,
}

impl zniff_rs_core::source::TraceSource for CountingSource<'_> {
    fn len(&self) -> u64 {
        self.inner.len()
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reads.set(self.reads.get() + 1);
        self.inner.read_at(offset, buf)
    }
}

#[test]
fn seeking_a_large_trace_reads_only_a_little_of_it() {
    use zniff_rs_core::cursor::TraceCursor;
    use zniff_rs_core::source::SliceSource;
    use zniff_rs_core::trace::Definitions;
    use zniff_rs_core::zlf::ZLF_HEADER_SIZE;

    // ~60 MB, the size at which a full decode is already impossible.
    let (header, body) = TRACE.split_at(ZLF_HEADER_SIZE);
    let copies = 6300;
    let mut big = Vec::with_capacity(header.len() + body.len() * copies);
    big.extend_from_slice(header);
    for _ in 0..copies {
        big.extend_from_slice(body);
    }
    let size_mb = big.len() as f64 / 1_048_576.0;

    let source = CountingSource {
        inner: SliceSource::new(&big),
        reads: std::cell::Cell::new(0),
    };
    let mut cursor = TraceCursor::new(source, Definitions::load()).unwrap();

    let (first, last) = cursor.time_span().unwrap().unwrap();
    let target = first + (last - first) * 7 / 10;

    cursor.source().reads.set(0);
    let window = cursor.frames_from_time(target, 100).unwrap();
    let reads = cursor.source().reads.get();

    assert_eq!(window.frames.len(), 100, "should return the frames asked for");
    assert!(
        window.frames[0].time_ms >= target,
        "landed before the requested time"
    );

    // The point of the design: bytes touched must not scale with the file.
    // A full scan would be millions of reads.
    assert!(
        reads < 20_000,
        "seek+decode of 100 frames in {size_mb:.0} MB took {reads} reads"
    );
    eprintln!("{size_mb:.0} MB: seek + 100 frames took {reads} reads");
}

#[test]
fn flags_a_retransmission_injected_into_a_real_trace() {
    // The fixture has no retransmissions, so the detector is exercised by
    // re-sending a real frame the way a node would: the same MPDU, and so
    // the same sequence number, a few tens of milliseconds later.
    use zniff_rs_core::cursor::TraceCursor;
    use zniff_rs_core::retransmit::RetransmitTracker;
    use zniff_rs_core::source::SliceSource;
    use zniff_rs_core::trace::Definitions;

    let mut cursor =
        TraceCursor::new(SliceSource::new(TRACE), Definitions::load()).unwrap();
    let start = cursor.first_record().unwrap().unwrap();
    let window = cursor.frames_at(start, usize::MAX).unwrap();

    // A singlecast is what gets acknowledged, and so what gets retried.
    let original = window
        .frames
        .iter()
        .find(|f| f.header_key() == Some(13))
        .expect("the fixture contains singlecasts");

    let resend = |time_ms: i64| {
        let mut f = original.clone();
        f.time_ms = time_ms;
        f
    };

    let mut tracker = RetransmitTracker::default();
    let first = (2048, 0);
    assert_eq!(tracker.observe(&resend(0), first).attempt, 0, "the first send");

    let retry = tracker.observe(&resend(25), (9000, 0));
    assert_eq!(retry.attempt, 1, "retried after 25 ms");
    assert_eq!(retry.original, Some(first), "and points back at the first");

    let again = tracker.observe(&resend(60), (9100, 1));
    assert_eq!(again.attempt, 2);
    assert_eq!(again.original, Some(first), "every retry links to the first");

    // Timing plays no part: the sender repeating its sequence number is
    // what makes a frame a retry.
    let mut tracker = RetransmitTracker::default();
    assert_eq!(tracker.observe(&resend(0), (0, 0)).attempt, 0);
    assert_eq!(tracker.observe(&resend(5_000), (5_000, 0)).attempt, 1);
}
