// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Reading backwards, which is how the viewer scrolls back to frames it
//! trimmed out of the list.

use zniff_rs_core::cursor::TraceCursor;
use zniff_rs_core::source::SliceSource;
use zniff_rs_core::trace::Definitions;

const FIXTURE: &[u8] = include_bytes!("fixtures/pti-800.zlf");

/// Every frame of the trace, in order, as (record offset, index) pairs.
fn all_origins() -> Vec<(u64, usize)> {
    let mut cursor =
        TraceCursor::new(SliceSource::new(FIXTURE), Definitions::load()).unwrap();
    let first = cursor.first_record().unwrap().unwrap();
    cursor.frames_at(first, 1_000_000).unwrap().origins
}

/// Reading back from a point must return the frames immediately before it,
/// in order, with none repeated or skipped.
#[test]
fn reads_the_frames_immediately_before_an_offset() {
    let origins = all_origins();
    let mut cursor =
        TraceCursor::new(SliceSource::new(FIXTURE), Definitions::load()).unwrap();

    // Page forward to somewhere in the middle, the way the viewer does.
    let first = cursor.first_record().unwrap().unwrap();
    let forward = cursor.frames_at(first, 60).unwrap();
    let boundary = forward.next_offset.expect("more trace after 60 frames");
    let seen = forward.frames.len();

    let back = cursor.frames_before(boundary, 25).unwrap();
    assert_eq!(back.frames.len(), 25, "asked for 25 frames");
    assert_eq!(back.next_offset, Some(boundary), "must join up to where it stopped");

    // The frames returned are exactly the 25 that precede the boundary.
    let expected = &origins[seen - 25..seen];
    assert_eq!(back.origins, expected, "wrong frames, or wrong order");
}

/// Asking for more than exists must stop at the start rather than inventing
/// frames or looping.
#[test]
fn stops_at_the_start_of_the_trace() {
    let origins = all_origins();
    let mut cursor =
        TraceCursor::new(SliceSource::new(FIXTURE), Definitions::load()).unwrap();
    let first = cursor.first_record().unwrap().unwrap();

    let forward = cursor.frames_at(first, 10).unwrap();
    let boundary = forward.next_offset.unwrap();
    let seen = forward.frames.len();

    // Far more than came before.
    let back = cursor.frames_before(boundary, 500).unwrap();
    assert_eq!(back.frames.len(), seen, "only what exists before the boundary");
    assert_eq!(back.origins, origins[..seen], "and it is the start of the trace");
}

/// At the very beginning there is nothing earlier.
#[test]
fn returns_nothing_before_the_first_record() {
    let mut cursor =
        TraceCursor::new(SliceSource::new(FIXTURE), Definitions::load()).unwrap();
    let first = cursor.first_record().unwrap().unwrap();
    let back = cursor.frames_before(first, 50).unwrap();
    assert!(back.frames.is_empty(), "nothing precedes the first record");
}

/// Walking the whole trace backwards must visit every frame exactly once,
/// in the same order as reading it forwards.
#[test]
fn walking_back_covers_the_trace_exactly() {
    let origins = all_origins();
    let mut cursor =
        TraceCursor::new(SliceSource::new(FIXTURE), Definitions::load()).unwrap();
    let first = cursor.first_record().unwrap().unwrap();

    // Start at the end.
    let whole = cursor.frames_at(first, 1_000_000).unwrap();
    let mut at = whole.end_offset;

    let mut collected: Vec<(u64, usize)> = Vec::new();
    for _ in 0..100 {
        let back = cursor.frames_before(at, 17).unwrap();
        if back.frames.is_empty() {
            break;
        }
        let mut batch = back.origins.clone();
        batch.extend(collected);
        collected = batch;
        at = back.offset;
        if at <= first {
            break;
        }
    }

    assert_eq!(collected, origins, "backward walk must reproduce the trace");
}
