// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Reading a trace that is still being written.

use std::io::Cursor;

use zniff_rs_core::cursor::TraceCursor;
use zniff_rs_core::trace::Definitions;
use zniff_rs_core::source::SliceSource;
use zniff_rs_core::zlf::{
    ApiType,
    Timestamp,
    ZlfReader,
    ZlfWriter,
};

/// Build a trace holding `count` real PTI records from the fixture.
fn trace_of(count: usize) -> Vec<u8> {
    const FIXTURE: &[u8] = include_bytes!("fixtures/pti-800.zlf");
    let mut reader = ZlfReader::new(Cursor::new(FIXTURE)).unwrap();
    let mut payloads = Vec::new();
    while let Some(r) = reader.next_record().unwrap() {
        if r.api_type == ApiType::Pti && !r.payload.is_empty() {
            payloads.push(r.payload);
        }
    }

    let mut out = Vec::new();
    let mut writer = ZlfWriter::new(&mut out, "growing").unwrap();
    for i in 0..count {
        writer
            .write_record(
                Timestamp::from_unix_millis(1_700_000_000_000 + i as i64 * 10),
                false,
                0,
                ApiType::Pti,
                &payloads[i % payloads.len()],
            )
            .unwrap();
    }
    writer.flush().unwrap();
    out
}

/// Reading to the end, then growing, must continue from where it stopped and
/// never repeat or skip a frame.
#[test]
fn resuming_after_growth_neither_repeats_nor_skips() {
    let short = trace_of(20);
    let mut cursor = TraceCursor::new(SliceSource::new(&short), Definitions::load()).unwrap();
    let first = cursor.first_record().unwrap().unwrap();

    // Read the whole of the short trace.
    let window = cursor.frames_at(first, 10_000).unwrap();
    assert!(window.next_offset.is_none(), "should reach the end");
    let before = window.frames.len();
    // Even at EOF the resume point is known.
    let resume = window.end_offset;
    assert!(resume > first);

    // The file grows; everything already read stays byte-identical.
    let long = trace_of(40);
    assert_eq!(&long[..short.len()], &short[..], "growth only appends");

    let mut cursor = TraceCursor::new(SliceSource::new(&long), Definitions::load()).unwrap();
    let rest = cursor.frames_at(resume, 10_000).unwrap();

    // The continuation plus what was already shown is the whole trace, with
    // nothing counted twice.
    let whole = cursor.frames_at(first, 10_000).unwrap();
    assert_eq!(
        before + rest.frames.len(),
        whole.frames.len(),
        "resuming at end_offset must not repeat or skip frames",
    );
}

/// A trace truncated mid-record must not yield a partial frame.
#[test]
fn a_half_written_record_is_not_decoded() {
    let full = trace_of(12);
    let complete =
        TraceCursor::new(SliceSource::new(&full), Definitions::load())
            .unwrap()
            .frames_at(2048, 10_000)
            .unwrap()
            .frames
            .len();

    // Cut the last record in half, the way an unflushed write would look.
    let cut = full.len() - 20;
    let partial = &full[..cut];
    let frames = TraceCursor::new(SliceSource::new(partial), Definitions::load())
        .unwrap()
        .frames_at(2048, 10_000)
        .unwrap()
        .frames
        .len();

    assert!(
        frames < complete,
        "a truncated record must not decode as a frame ({frames} vs {complete})",
    );
}
