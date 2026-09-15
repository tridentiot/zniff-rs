// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Reading a window of frames out of a trace, without decoding the rest.
//!
//! This is what makes a very large trace usable: seeking to a time costs a
//! handful of small reads (about 17 for 300 MB), and only the frames the
//! caller asks for are decoded.
use std::io;

use crate::pti;
use crate::resync::{
    self,
    DEFAULT_WINDOW,
    RecordHeader,
};
use crate::source::TraceSource;
use crate::trace::{
    Definitions,
    TraceFrame,
    build_frame,
};
use crate::zlf::ZLF_HEADER_SIZE;
use crate::zlf::types::ApiType;

/// Rough bytes per record, for guessing how far back to step when reading
/// backwards. Only an estimate: the search widens when it falls short.
const ESTIMATED_RECORD_BYTES: u64 = 64;

/// How far a stepped seek has narrowed.
#[derive(Debug, Clone, Copy)]
pub enum SeekState {
    /// Halving the file to bracket the target.
    Searching { lo: u64, hi: u64 },
    /// Walking records to the first one at or after the target.
    Scanning { at: u64 },
}

impl SeekState {
    /// The byte range the next step will read.
    pub fn probe(&self) -> (u64, u64) {
        match *self {
            SeekState::Searching { lo, hi } => (lo + (hi - lo) / 2, DEFAULT_WINDOW),
            // Scanning steps record by record through a span the previous
            // probe already covered, so ask only for what one record needs.
            SeekState::Scanning { at } => (at, MAX_RECORD),
        }
    }
}

/// The outcome of one [`TraceCursor::seek_step`].
#[derive(Debug, Clone, Copy)]
pub enum SeekStep {
    /// Keep going from this state.
    More(SeekState),
    /// Bracketing is done; scan from here.
    Scan(SeekState),
    /// The record offset, or None if the time is past the end.
    Done(Option<u64>),
}

/// Span the search narrows to before walking records one by one.
pub const SEEK_SPAN: u64 = 64 * 1024;

/// Enough to cover one record plus the resync scan that finds it.
const MAX_RECORD: u64 = 8 * 1024;

/// Where a window of frames begins, and where the next one continues from.
#[derive(Debug, Clone)]
pub struct FrameWindow {
    /// Record offset the window started at.
    pub offset: u64,
    /// The decoded frames.
    pub frames: Vec<TraceFrame>,
    /// For each frame, the offset of the record it came from and its
    /// position within that record. This is how a row is addressed later,
    /// since there is no global frame number without reading the whole file.
    pub origins: Vec<(u64, usize)>,
    /// Record offset to pass back in to continue, or None at end of file.
    pub next_offset: Option<u64>,
    /// Offset just past the last record consumed.
    ///
    /// Unlike `next_offset` this is set even at end of file, which is what
    /// lets a still-growing capture resume exactly where it stopped instead
    /// of re-reading the last window.
    pub end_offset: u64,
}

/// Reads frames from a trace on demand.
pub struct TraceCursor<S: TraceSource> {
    source: S,
    definitions: Definitions,
}

impl<S: TraceSource> TraceCursor<S> {
    /// Open a trace. Validates the ZLF file header but reads nothing else.
    pub fn new(source: S, definitions: Definitions) -> io::Result<Self> {
        let mut cursor = Self { source, definitions };
        cursor.validate_header()?;
        Ok(cursor)
    }

    fn validate_header(&mut self) -> io::Result<()> {
        let mut header = [0u8; ZLF_HEADER_SIZE];
        self.source.read_exact_at(0, &mut header)?;
        let version = u32::from_le_bytes(header[0..4].try_into().unwrap());
        if version != crate::zlf::types::ZLF_VERSION {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported ZLF version {version}"),
            ));
        }
        Ok(())
    }

    pub fn source(&self) -> &S {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }

    /// Capture time of the first record at or after `offset`.
    ///
    /// Exposed so a caller that must fetch bytes before reading them can
    /// follow the same probe pattern as [`Self::seek_time`].
    pub fn record_time_near(&mut self, offset: u64) -> io::Result<Option<i64>> {
        Ok(resync::record_time_at(&mut self.source, offset)?.map(|(_, t)| t))
    }

    pub fn definitions(&self) -> &Definitions {
        &self.definitions
    }

    /// Format version and comment from the file header.
    ///
    /// The comment is the only free-text field ZLF stores, and the capture
    /// tool leaves it empty unless someone types one, so it is often blank.
    pub fn file_header(&mut self) -> io::Result<(u32, String)> {
        let mut header = [0u8; ZLF_HEADER_SIZE];
        self.source.read_exact_at(0, &mut header)?;
        let version = u32::from_le_bytes(header[0..4].try_into().unwrap());

        // 512 bytes of UTF-16LE, zero padded.
        let units: Vec<u16> = header[8..520]
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .take_while(|&c| c != 0)
            .collect();
        Ok((version, String::from_utf16_lossy(&units).trim().to_string()))
    }

    /// Offset of the first record in the file.
    pub fn first_record(&mut self) -> io::Result<Option<u64>> {
        resync::next_record_at(&mut self.source, ZLF_HEADER_SIZE as u64, DEFAULT_WINDOW)
    }

    /// Capture time of the first and last record, for the overall span.
    pub fn time_span(&mut self) -> io::Result<Option<(i64, i64)>> {
        let Some((_, first)) = resync::record_time_at(&mut self.source, 0)? else {
            return Ok(None);
        };
        // Walk back from the end until a record is found.
        let len = self.source.len();
        let mut back = DEFAULT_WINDOW.min(len);
        let last = loop {
            let from = len.saturating_sub(back);
            let mut latest = None;
            let mut at = from;
            while let Some(found) =
                resync::next_record_at(&mut self.source, at, DEFAULT_WINDOW)?
            {
                let Some(rec) = resync::read_record(&mut self.source, found)? else {
                    break;
                };
                latest = Some(rec.unix_millis());
                at = rec.next_offset();
                if at >= len {
                    break;
                }
            }
            if let Some(t) = latest {
                break t;
            }
            if back >= len {
                break first;
            }
            back = (back * 4).min(len);
        };
        Ok(Some((first, last)))
    }

    /// One step of a time seek, so a caller that must fetch bytes can drive
    /// the search and prefetch each probe as it goes.
    ///
    /// Returns the next range to make available and the state to pass back,
    /// or the answer once the search has narrowed.
    pub fn seek_step(&mut self, time_ms: i64, state: SeekState) -> io::Result<SeekStep> {
        match state {
            SeekState::Searching { mut lo, mut hi } => {
                if hi - lo <= SEEK_SPAN {
                    return Ok(SeekStep::Scan(SeekState::Scanning { at: lo }));
                }
                let mid = lo + (hi - lo) / 2;
                match resync::record_time_at(&mut self.source, mid)? {
                    Some((found, t)) if t < time_ms => {
                        let Some(rec) = resync::read_record(&mut self.source, found)? else {
                            return Ok(SeekStep::Scan(SeekState::Scanning { at: lo }));
                        };
                        let next = rec.next_offset();
                        if next <= lo {
                            return Ok(SeekStep::Scan(SeekState::Scanning { at: lo }));
                        }
                        lo = next;
                    }
                    _ => hi = mid,
                }
                Ok(SeekStep::More(SeekState::Searching { lo, hi }))
            }
            SeekState::Scanning { at } => {
                if at >= self.source.len() {
                    return Ok(SeekStep::Done(None));
                }
                let Some(found) =
                    resync::next_record_at(&mut self.source, at, DEFAULT_WINDOW)?
                else {
                    return Ok(SeekStep::Done(None));
                };
                let Some(rec) = resync::read_record(&mut self.source, found)? else {
                    return Ok(SeekStep::Done(None));
                };
                if rec.unix_millis() >= time_ms {
                    return Ok(SeekStep::Done(Some(found)));
                }
                Ok(SeekStep::More(SeekState::Scanning { at: rec.next_offset() }))
            }
        }
    }

    /// Where a stepped seek should start.
    pub fn seek_start(&mut self) -> io::Result<SeekState> {
        let first = self.first_record()?.unwrap_or(ZLF_HEADER_SIZE as u64);
        Ok(SeekState::Searching { lo: first, hi: self.source.len() })
    }

    /// Offset of the first record at or after `time_ms`.
    ///
    /// Binary searches the file itself: each probe resyncs to a record
    /// boundary and reads its timestamp, so no index is needed. Timestamps
    /// are assumed non-decreasing; the result is refined by walking back to
    /// the true first match, which tolerates small local disorder.
    pub fn seek_time(&mut self, time_ms: i64) -> io::Result<Option<u64>> {
        let Some(first) = self.first_record()? else {
            return Ok(None);
        };
        let len = self.source.len();

        let mut lo = first;
        let mut hi = len;
        // Narrow to a small span, then scan it exactly.
        while hi - lo > 64 * 1024 {
            let mid = lo + (hi - lo) / 2;
            match resync::record_time_at(&mut self.source, mid)? {
                Some((_, t)) if t < time_ms => {
                    // Everything at or before mid is too early.
                    let Some(found) =
                        resync::next_record_at(&mut self.source, mid, DEFAULT_WINDOW)?
                    else {
                        break;
                    };
                    let Some(rec) = resync::read_record(&mut self.source, found)? else {
                        break;
                    };
                    let next = rec.next_offset();
                    if next <= lo {
                        break;
                    }
                    lo = next;
                }
                Some(_) => hi = mid,
                // No record in that window; treat the span as exhausted.
                None => hi = mid,
            }
        }

        // Scan forward from lo for the first record at or after the target.
        let mut at = lo;
        while at < len {
            let Some(found) = resync::next_record_at(&mut self.source, at, DEFAULT_WINDOW)?
            else {
                return Ok(None);
            };
            let Some(rec) = resync::read_record(&mut self.source, found)? else {
                return Ok(None);
            };
            if rec.unix_millis() >= time_ms {
                return Ok(Some(found));
            }
            at = rec.next_offset();
        }
        Ok(None)
    }

    /// Decode up to `count` frames starting at the record at `offset`.
    pub fn frames_at(&mut self, offset: u64, count: usize) -> io::Result<FrameWindow> {
        let start = match resync::next_record_at(&mut self.source, offset, DEFAULT_WINDOW)? {
            Some(at) => at,
            None => {
                return Ok(FrameWindow {
                    offset,
                    frames: Vec::new(),
                    origins: Vec::new(),
                    next_offset: None,
                    end_offset: offset,
                });
            }
        };

        let mut frames = Vec::with_capacity(count.min(4096));
        let mut origins = Vec::with_capacity(count.min(4096));
        let mut at = start;
        let mut previous: Option<i64> = None;
        let mut next_offset = None;

        while frames.len() < count {
            let Some(rec) = resync::read_record(&mut self.source, at)? else {
                break;
            };
            let before = frames.len();
            // A record may hold more frames than the window has room for.
            // Decoding only part of it would make the leftovers unreachable,
            // since the window can only resume on a record boundary, so a
            // record is always taken whole and the window may overshoot.
            self.decode_record(&rec, &mut frames, &mut previous)?;
            origins.extend((0..frames.len() - before).map(|i| (rec.offset, i)));
            at = rec.next_offset();
            next_offset = (at < self.source.len()).then_some(at);
            if next_offset.is_none() {
                break;
            }
        }

        Ok(FrameWindow { offset: start, frames, origins, next_offset, end_offset: at })
    }

    /// Decode up to `count` frames ending just before the record at `offset`.
    ///
    /// Records can only be read forwards, so this steps back a byte window,
    /// resyncs to the first boundary there, and reads forward to `offset`,
    /// keeping the last `count` frames. Stepping back further than needed is
    /// harmless: the extra frames are discarded.
    ///
    /// This is what lets a reader scroll back to frames that were trimmed out
    /// of the list after being read once.
    pub fn frames_before(&mut self, offset: u64, count: usize) -> io::Result<FrameWindow> {
        let first = match self.first_record()? {
            Some(first) => first,
            None => {
                return Ok(FrameWindow {
                    offset,
                    frames: Vec::new(),
                    origins: Vec::new(),
                    next_offset: None,
                    end_offset: offset,
                });
            },
        };
        if offset <= first {
            return Ok(FrameWindow {
                offset: first,
                frames: Vec::new(),
                origins: Vec::new(),
                next_offset: Some(offset),
                end_offset: first,
            });
        }

        // Enough bytes to hold `count` frames with room to spare, so one step
        // back is normally enough; widen if it was not.
        let mut back = (count as u64 + 8) * ESTIMATED_RECORD_BYTES;
        loop {
            let from = offset.saturating_sub(back).max(first);
            let start = match resync::next_record_at(&mut self.source, from, DEFAULT_WINDOW)? {
                Some(at) if at < offset => at,
                // No boundary before the target: start from the beginning.
                _ => first,
            };

            let mut frames = Vec::new();
            let mut origins = Vec::new();
            let mut previous: Option<i64> = None;
            let mut at = start;
            while at < offset {
                let Some(rec) = resync::read_record(&mut self.source, at)? else {
                    break;
                };
                let before = frames.len();
                self.decode_record(&rec, &mut frames, &mut previous)?;
                origins.extend((0..frames.len() - before).map(|i| (rec.offset, i)));
                at = rec.next_offset();
            }

            // Keep only the last `count`, unless the trace starts here, in
            // which case there is nothing earlier to miss.
            let enough = frames.len() >= count || start == first;
            if enough {
                let drop = frames.len().saturating_sub(count);
                return Ok(FrameWindow {
                    offset: if drop > 0 { origins[drop].0 } else { start },
                    frames: frames.split_off(drop),
                    origins: origins.split_off(drop),
                    next_offset: Some(offset),
                    end_offset: at,
                });
            }

            // Too few frames: the records were larger than estimated.
            back *= 2;
        }
    }

    /// Decode `count` frames starting from the first one at or after `time_ms`.
    pub fn frames_from_time(
        &mut self,
        time_ms: i64,
        count: usize,
    ) -> io::Result<FrameWindow> {
        match self.seek_time(time_ms)? {
            Some(offset) => self.frames_at(offset, count),
            None => Ok(FrameWindow {
                offset: 0,
                frames: Vec::new(),
                origins: Vec::new(),
                next_offset: None,
                end_offset: 0,
            }),
        }
    }

    /// Decode the frames carried by one record.
    fn decode_record(
        &mut self,
        rec: &RecordHeader,
        frames: &mut Vec<TraceFrame>,
        previous: &mut Option<i64>,
    ) -> io::Result<()> {
        // Only the record types that carry radio frames are worth reading.
        if !matches!(
            rec.api_type,
            ApiType::Pti | ApiType::PtiDiagnostic | ApiType::Zniffer
        ) {
            return Ok(());
        }

        let mut payload = vec![0u8; rec.payload_len as usize];
        self.source.read_exact_at(rec.payload_offset(), &mut payload)?;
        let time_ms = rec.unix_millis();

        match rec.api_type {
            ApiType::Pti | ApiType::PtiDiagnostic => {
                for frame in pti::parse_payload(&payload) {
                    let delta = previous.map_or(0, |p| time_ms - p);
                    *previous = Some(time_ms);
                    let index = frames.len() + 1;
                    frames.push(build_frame(
                        &self.definitions,
                        index,
                        time_ms,
                        delta,
                        frame,
                    ));
                }
            }
            ApiType::Zniffer => {
                // The legacy dialect is a byte stream. A record boundary is
                // not necessarily a frame boundary, so a frame split across
                // records is dropped here; see the note in the module docs.
                let mut parser = crate::zniffer_parser::Parser::new();
                for byte in &payload {
                    if let crate::zniffer_parser::ParserResult::ValidFrame { frame } =
                        parser.parse(*byte)
                    {
                        let delta = previous.map_or(0, |p| time_ms - p);
                        *previous = Some(time_ms);
                        let index = frames.len() + 1;
                        frames.push(build_frame(
                            &self.definitions,
                            index,
                            time_ms,
                            delta,
                            pti::PtiFrame {
                                direction: if rec.is_outcome {
                                    pti::Direction::Tx
                                } else {
                                    pti::Direction::Rx
                                },
                                region: frame.region as u8,
                                channel: frame.channel,
                                speed: frame.speed,
                                rssi: frame.rssi as i8,
                                beam_count: None,
                                mpdu: frame.payload,
                            },
                        ));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SliceSource;

    const FIXTURE: &[u8] = include_bytes!("../tests/fixtures/pti-800.zlf");

    fn cursor() -> TraceCursor<SliceSource<'static>> {
        TraceCursor::new(SliceSource::new(FIXTURE), Definitions::load()).unwrap()
    }

    #[test]
    fn rejects_a_file_that_is_not_a_trace() {
        let junk = vec![0u8; 4096];
        assert!(TraceCursor::new(SliceSource::new(&junk), Definitions::load()).is_err());
    }

    #[test]
    fn reads_the_file_header() {
        let mut c = cursor();
        let (version, comment) = c.file_header().unwrap();
        assert_eq!(version, 104);
        // The capture tool writes a comment only if someone types one, so
        // an empty one is the norm rather than a parse failure.
        assert_eq!(comment, "");
    }

    #[test]
    fn reads_a_window_from_the_start() {
        let mut c = cursor();
        let start = c.first_record().unwrap().unwrap();
        let w = c.frames_at(start, 10).unwrap();
        assert_eq!(w.frames.len(), 10);
        assert_eq!(w.frames[0].header_text(), "Explorer Autoinclusion");
        assert!(w.next_offset.is_some());
    }

    #[test]
    fn reports_the_time_span() {
        let mut c = cursor();
        let (first, last) = c.time_span().unwrap().unwrap();
        assert_eq!(first, 1_770_223_530_405);
        assert!(last > first, "span should advance");
        // The trace runs about 317 seconds.
        assert!((last - first) > 300_000, "span was {} ms", last - first);
    }

    #[test]
    fn seeks_to_a_time_in_the_middle() {
        let mut c = cursor();
        let (first, last) = c.time_span().unwrap().unwrap();
        let target = first + (last - first) / 2;

        let at = c.seek_time(target).unwrap().expect("a record at the midpoint");
        let w = c.frames_at(at, 1).unwrap();
        assert_eq!(w.frames.len(), 1);
        assert!(
            w.frames[0].time_ms >= target,
            "seek landed before the target: {} < {target}",
            w.frames[0].time_ms
        );

        // And it is genuinely mid-trace: frames exist before the target.
        let start = c.first_record().unwrap().unwrap();
        let earlier = c
            .frames_at(start, usize::MAX)
            .unwrap()
            .frames
            .into_iter()
            .filter(|f| f.time_ms < target)
            .count();
        assert!(earlier > 0, "the midpoint should have frames before it");
    }

    #[test]
    fn seeking_before_the_start_returns_the_first_frame() {
        let mut c = cursor();
        let w = c.frames_from_time(0, 3).unwrap();
        assert_eq!(w.frames.len(), 3);
        assert_eq!(w.frames[0].time_ms, 1_770_223_530_405);
    }

    #[test]
    fn seeking_past_the_end_finds_nothing() {
        let mut c = cursor();
        assert!(c.seek_time(i64::MAX / 2).unwrap().is_none());
        assert!(c.frames_from_time(i64::MAX / 2, 10).unwrap().frames.is_empty());
    }

    #[test]
    fn windows_chain_to_cover_the_whole_trace() {
        let mut c = cursor();
        let mut at = c.first_record().unwrap().unwrap();
        let mut total = 0;
        loop {
            let w = c.frames_at(at, 25).unwrap();
            total += w.frames.len();
            match w.next_offset {
                Some(next) if !w.frames.is_empty() => at = next,
                _ => break,
            }
        }
        assert_eq!(total, 181, "paging should visit every frame exactly once");
    }
}
