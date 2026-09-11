// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! WebAssembly bindings for reading and decoding Z-Wave ZLF traces in a browser.
//!
//! The trace stays in WebAssembly memory; JavaScript pulls the rows it needs to
//! render and the detail of the selected frame, so a large trace never has to
//! cross the boundary in one piece.
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::prelude::*;
use zniff_rs_core::cursor::{
    FrameWindow,
    SEEK_SPAN,
    SeekStep,
    TraceCursor,
};
use zniff_rs_core::retransmit::{
    Retransmission,
    RetransmitTracker,
};
use zniff_rs_core::source::{
    SliceSource,
    TraceSource,
};
use zniff_rs_core::trace::{
    Definitions,
    TraceFrame,
};

use range::RangeSource;

mod color;
mod filter;
mod range;

use color::frame_colors;
use filter::Filter;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

/// One row of the frame list.
#[derive(Serialize)]
struct Row {
    /// Offset of the record this frame came from, and its position within
    /// that record. Together these address the row for a detail lookup.
    ///
    /// There is no frame number: numbering a frame absolutely would mean
    /// counting every frame before it, which for a linked trace means
    /// fetching the whole file. Frames are identified by capture time.
    record_offset: f64,
    frame_in_record: usize,
    time_ms: i64,
    delta_ms: i64,
    speed: &'static str,
    rssi: i8,
    channel: u8,
    direction: &'static str,
    source: Option<u64>,
    destination: Option<u64>,
    home_id: Option<String>,
    /// Frame type, e.g. `Singlecast`.
    data: String,
    /// Application summary, e.g. `Binary Switch - Switch Binary Set`.
    application: String,
    hex: String,
    crc_ok: bool,
    /// How many times this MPDU was sent before; 0 for a first attempt.
    retransmission: u32,
    /// The frame this one repeats, addressed as (record offset, position).
    original_offset: Option<f64>,
    original_in_record: Option<usize>,
    /// Text colour, from the desktop Zniffer's frame-type scheme.
    fg: &'static str,
    /// Row background, from the desktop Zniffer's speed scheme.
    bg: &'static str,
}

impl Row {
    fn new(frame: &TraceFrame, origin: (u64, usize), seen: Retransmission) -> Self {
        let colors = frame_colors(frame);
        Self {
            retransmission: seen.attempt,
            original_offset: seen.original.map(|(o, _)| o as f64),
            original_in_record: seen.original.map(|(_, i)| i),
            record_offset: origin.0 as f64,
            frame_in_record: origin.1,
            time_ms: frame.time_ms,
            delta_ms: frame.delta_ms,
            speed: frame.speed_text(),
            rssi: frame.rssi,
            channel: frame.channel,
            direction: match frame.direction {
                zniff_rs_core::pti::Direction::Rx => "Rx",
                zniff_rs_core::pti::Direction::Tx => "Tx",
            },
            source: frame.source(),
            destination: frame.destination(),
            home_id: frame.home_id().map(|h| format!("{h:08X}")),
            data: frame.header_text().to_string(),
            application: frame.application_summary(),
            hex: hex::encode_upper(&frame.mpdu),
            crc_ok: frame.crc_ok(),
            fg: colors.0,
            bg: colors.1,
        }
    }
}

/// A field in the detail tree.
#[derive(Serialize)]
struct DetailField {
    name: String,
    value: String,
    /// Byte range within the MPDU, for hex highlighting.
    start: usize,
    end: usize,
    children: Vec<DetailField>,
}

/// What a trace is, gathered from its header and a sample of its frames.
#[derive(Serialize)]
struct TraceInfo {
    /// ZLF format version.
    version: u32,
    /// Comment stored in the file header, when the capture tool wrote one.
    comment: String,
    size: f64,
    start_time_ms: f64,
    end_time_ms: f64,
    /// Frame count, exact only when the whole trace was read.
    frames: f64,
    frames_exact: bool,
    /// Frames the summary below was drawn from.
    sampled: usize,
    /// Regions seen, as `908.4 MHz (US)`.
    regions: Vec<String>,
    /// Bit rates seen, as `100 kbps`.
    speeds: Vec<String>,
    channels: Vec<u8>,
    /// Distinct home ids seen, formatted as hex.
    home_ids: Vec<String>,
}

/// The command class and command a frame carries.
#[derive(Serialize)]
struct AppHeading {
    /// Class name with its key, e.g. `Command Class Security 2 (0x9F)`.
    class: String,
    /// Highest class version that defines this command.
    version: u8,
    /// Command name with its key, e.g. `S2 Message Encapsulation (0x03)`.
    command: String,
}

/// Everything shown for the selected frame.
#[derive(Serialize)]
struct Detail {
    /// Capture time, which is how a frame is identified.
    time_ms: i64,
    hex: String,
    header_name: String,
    crc_ok: bool,
    /// MPDU header fields.
    header: Vec<DetailField>,
    /// Application payload fields.
    application: Vec<DetailField>,
    /// Which command class and command, when one was decoded.
    application_heading: Option<AppHeading>,
    /// One-line description, for frames with no decoded command.
    application_summary: String,
    /// Set when this frame repeats an earlier one.
    retransmission: Option<RetransmissionInfo>,
}

/// What a retransmitted frame repeats.
#[derive(Serialize)]
struct RetransmissionInfo {
    /// Which attempt this is, counting the first send as 1.
    attempt: u32,
    /// The frame it repeats, for the caller to navigate to.
    original_offset: f64,
    original_in_record: usize,
}

/// A window of frames, with where to continue from.
#[derive(Serialize)]
struct Window {
    /// Record offset this window started at.
    offset: f64,
    /// Record offset of the next window. Always present, null at the end
    /// of the trace, so a caller cannot mistake absent for "not finished".
    next_offset: Option<f64>,
    /// Offset just past the last record read. Set even at end of file, so a
    /// growing capture can resume from exactly where it stopped.
    end_offset: f64,
    rows: Vec<Row>,
}

/// What is known about a trace before any frames are read.
#[derive(Serialize)]
struct Overview {
    /// Total bytes.
    size: f64,
    /// Capture time of the first and last frame.
    start_time_ms: f64,
    end_time_ms: f64,
    /// Offset of the first record.
    first_offset: f64,
    /// Frame count. Exact when the whole trace fits in one sample,
    /// otherwise extrapolated from the mean record size — counting for
    /// real would mean reading the whole trace.
    estimated_frames: f64,
    /// Whether `estimated_frames` is exact.
    frames_exact: bool,
}

/// A ZLF trace, read on demand.
///
/// The trace is never decoded in full. Frames are addressed by the byte
/// offset of the record holding them, so a window can be read from anywhere
/// without knowing what precedes it.
#[wasm_bindgen]
pub struct Trace {
    cursor: TraceCursor<Backing>,
}

/// Where a trace's bytes come from.
enum Backing {
    Memory(Vec<u8>),
    Http(RangeSource),
}

impl TraceSource for Backing {
    fn len(&self) -> u64 {
        match self {
            Backing::Memory(b) => b.len() as u64,
            Backing::Http(r) => r.len(),
        }
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Backing::Memory(b) => SliceSource::new(b).read_at(offset, buf),
            Backing::Http(r) => r.read_at(offset, buf),
        }
    }
}

#[wasm_bindgen]
impl Trace {
    /// Open a trace already in memory.
    pub fn open_bytes(bytes: Vec<u8>) -> Result<Trace, JsError> {
        let cursor = TraceCursor::new(Backing::Memory(bytes), Definitions::load())
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Trace { cursor })
    }

    /// Open a trace over HTTP, fetching only the ranges that are read.
    pub async fn open_url(url: String) -> Result<Trace, JsError> {
        let mut source = RangeSource::open(url)
            .await
            .map_err(|e| JsError::new(&format!("{e:?}")))?;
        // The cursor validates the file header synchronously, so it has to
        // be in the cache before the cursor is built.
        source
            .prefetch(0, PROBE)
            .await
            .map_err(|e| JsError::new(&format!("{e:?}")))?;
        let cursor = TraceCursor::new(Backing::Http(source), Definitions::load())
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Trace { cursor })
    }

    /// Size, time span and a rough frame count.
    pub async fn overview(&mut self) -> Result<JsValue, JsError> {
        let size = self.cursor.source().len();
        self.prefetch(0, PROBE).await?;
        self.prefetch(size.saturating_sub(PROBE), PROBE).await?;

        let (start, end) = self
            .cursor
            .time_span()
            .map_err(io_err)?
            .unwrap_or((0, 0));
        let first_offset = self.cursor.first_record().map_err(io_err)?.unwrap_or(0);

        // Sample the start rather than reading the whole trace. If the
        // sample runs off the end, the trace is small enough that the
        // sample *is* the trace and the count is exact.
        let sample = 512;
        let window = self.cursor.frames_at(first_offset, sample).map_err(io_err)?;
        let counted = window.frames.len();
        let (frames, exact) = match window.next_offset {
            None => (counted as f64, true),
            Some(next) => {
                let spanned = (next - first_offset).max(1) as f64;
                let per_frame = spanned / (counted.max(1) as f64);
                let rest = (size - first_offset) as f64 / per_frame;
                (rest.max(counted as f64).round(), false)
            }
        };

        let overview = Overview {
            size: size as f64,
            start_time_ms: start as f64,
            end_time_ms: end as f64,
            first_offset: first_offset as f64,
            estimated_frames: frames,
            frames_exact: exact,
        };
        to_js(&overview)
    }

    /// What the trace is: header fields, plus what a sample of its frames
    /// says about regions, bit rates, channels and networks.
    ///
    /// A large trace is never read in full, so the summary is drawn from
    /// the first `sample` frames and says how many it saw.
    pub async fn info(&mut self, sample: usize) -> Result<JsValue, JsError> {
        let size = self.cursor.source().len();
        self.prefetch(0, PROBE).await?;
        self.prefetch(size.saturating_sub(PROBE), PROBE).await?;

        let (version, comment) = self.cursor.file_header().map_err(io_err)?;
        let (start, end) = self.cursor.time_span().map_err(io_err)?.unwrap_or((0, 0));
        let first = self.cursor.first_record().map_err(io_err)?.unwrap_or(0);

        self.prefetch(first, span_for(sample)).await?;
        let window = self.cursor.frames_at(first, sample).map_err(io_err)?;

        let fd = &self.cursor.definitions().frame_definition;
        let mut regions: Vec<String> = Vec::new();
        let mut speeds: Vec<String> = Vec::new();
        let mut channels: Vec<u8> = Vec::new();
        let mut home_ids: Vec<String> = Vec::new();

        for frame in &window.frames {
            let region = fd
                .region_text(frame.region)
                .map(str::to_string)
                .unwrap_or_else(|| format!("region {}", frame.region));
            if !regions.contains(&region) {
                regions.push(region);
            }
            let speed = frame.speed_text().to_string();
            if !speeds.contains(&speed) {
                speeds.push(speed);
            }
            if !channels.contains(&frame.channel) {
                channels.push(frame.channel);
            }
            if let Some(home) = frame.home_id() {
                let text = format!("{home:08X}");
                if !home_ids.contains(&text) {
                    home_ids.push(text);
                }
            }
        }
        channels.sort_unstable();
        home_ids.sort();

        // The sample is the whole trace only if it ran off the end.
        let exact = window.next_offset.is_none();
        let frames = if exact {
            window.frames.len() as f64
        } else {
            let spanned = (window.next_offset.unwrap_or(first) - first).max(1) as f64;
            let per_frame = spanned / (window.frames.len().max(1) as f64);
            ((size - first) as f64 / per_frame).round()
        };

        let info = TraceInfo {
            version,
            comment,
            size: size as f64,
            start_time_ms: start as f64,
            end_time_ms: end as f64,
            frames,
            frames_exact: exact,
            sampled: window.frames.len(),
            regions,
            speeds,
            channels,
            home_ids,
        };
        to_js(&info)
    }

    /// Read `count` frames starting at a record offset.
    pub async fn rows_at(&mut self, offset: f64, count: usize) -> Result<JsValue, JsError> {
        let offset = offset.max(0.0) as u64;
        // Enough to cover the window plus the resync scan.
        self.prefetch(offset, span_for(count)).await?;
        let window = self.cursor.frames_at(offset, count).map_err(io_err)?;
        self.window_to_js(window)
    }

    /// Read `count` frames starting at the first one at or after `time_ms`.
    ///
    /// This is the operation a large trace is navigated by: seeking costs a
    /// binary search over the file, not a scan.
    pub async fn rows_from_time(
        &mut self,
        time_ms: f64,
        count: usize,
    ) -> Result<JsValue, JsError> {
        let target = time_ms as i64;
        // Seeking probes across the whole file, so fetch as it goes.
        let offset = self.seek_prefetching(target).await?;
        match offset {
            Some(at) => self.rows_at(at as f64, count).await,
            None => self.window_to_js(FrameWindow {
                offset: 0,
                frames: Vec::new(),
                origins: Vec::new(),
                next_offset: None,
                end_offset: 0,
            }),
        }
    }

    /// Full detail of one frame within the record at `offset`.
    ///
    /// `retransmission` is what the list already worked out for this row,
    /// since recognising a retry needs the frames that came before it and
    /// the detail view reads only one record.
    pub async fn detail(
        &mut self,
        offset: f64,
        frame_in_record: usize,
        retransmission: u32,
        original_offset: Option<f64>,
        original_in_record: Option<usize>,
    ) -> Result<JsValue, JsError> {
        let offset = offset.max(0.0) as u64;
        self.prefetch(offset, span_for(1)).await?;
        let window = self.cursor.frames_at(offset, 1).map_err(io_err)?;
        let frame = window
            .frames
            .get(frame_in_record)
            .ok_or_else(|| JsError::new("no such frame in that record"))?;

        let header = frame
            .header
            .as_ref()
            .map(|h| h.params.iter().map(header_field).collect())
            .unwrap_or_default();

        // Command class fields are relative to the application payload,
        // but the hex view shows the whole MPDU.
        let payload_at = frame.header.as_ref().map_or(0, |h| h.payload_offset);
        let application = match &frame.application {
            Some(zniff_rs_core::decoder::command_class::Decoded::Command(c)) => {
                c.params.iter().map(|p| app_field(p, payload_at)).collect()
            }
            _ => Vec::new(),
        };

        let detail = Detail {
            time_ms: frame.time_ms,
            hex: hex::encode_upper(&frame.mpdu),
            header_name: frame.header_text().to_string(),
            crc_ok: frame.crc_ok(),
            header,
            application,
            application_heading: match &frame.application {
                Some(zniff_rs_core::decoder::command_class::Decoded::Command(c)) => {
                    Some(AppHeading {
                        class: format!("{} (0x{:02X})", c.class_help, c.class_key),
                        version: c.class_version,
                        command: format!("{} (0x{:02X})", c.cmd_help, c.cmd_key),
                    })
                }
                _ => None,
            },
            application_summary: frame.application_summary(),
            retransmission: match (retransmission, original_offset, original_in_record) {
                (attempt, Some(o), Some(i)) if attempt > 0 => Some(RetransmissionInfo {
                    // The first send is attempt 1, so a first retry is 2.
                    attempt: attempt + 1,
                    original_offset: o,
                    original_in_record: i,
                }),
                _ => None,
            },
        };
        to_js(&detail)
    }

    /// Scan forward from `offset` for frames matching `query`, for at most
    /// `limit` records, returning the matches found and where to resume.
    ///
    /// Filtering a large trace means reading it, so this is deliberately
    /// incremental: the caller loops and reports progress.
    pub async fn filter_step(
        &mut self,
        query: &str,
        offset: f64,
        limit: usize,
    ) -> Result<JsValue, JsError> {
        let filter = Filter::parse(query);
        let offset = offset.max(0.0) as u64;
        self.prefetch(offset, span_for(limit)).await?;

        let window = self.cursor.frames_at(offset, limit).map_err(io_err)?;
        // Retransmissions are judged against the whole window, so that a
        // filtered-out first attempt still marks the repeat that follows.
        let all = rows_of(&window);
        let rows: Vec<Row> = all
            .into_iter()
            .zip(window.frames.iter())
            .filter(|(row, f)| filter.matches(f, row.retransmission))
            .map(|(row, _)| row)
            .collect();

        let result = Window {
            offset: window.offset as f64,
            next_offset: window.next_offset.map(|o| o as f64),
            end_offset: window.end_offset as f64,
            rows,
        };
        to_js(&result)
    }
}

/// Bytes read when probing for the time span.
const PROBE: u64 = 1 << 20;

/// Roughly how many bytes `count` frames span, with room for the resync scan.
fn span_for(count: usize) -> u64 {
    (count as u64).saturating_mul(2048).max(1 << 16) + (1 << 16)
}

/// Build the rows of a window, marking retransmissions.
///
/// A retransmission can only be recognised from the frames that came
/// before it, so the tracker runs over the window in order.
fn rows_of(window: &FrameWindow) -> Vec<Row> {
    let mut tracker = RetransmitTracker::default();
    window
        .frames
        .iter()
        .zip(&window.origins)
        .map(|(frame, &origin)| {
            let seen = tracker.observe(frame, origin);
            Row::new(frame, origin, seen)
        })
        .collect()
}

/// Serialise with `None` as `null` rather than `undefined`, so the caller
/// can distinguish "no next window" from a field that was left out.
fn to_js<T: Serialize>(value: &T) -> Result<JsValue, JsError> {
    value
        .serialize(&Serializer::new().serialize_missing_as_null(true))
        .map_err(Into::into)
}

fn io_err(e: std::io::Error) -> JsError {
    JsError::new(&e.to_string())
}

impl Trace {
    /// Make sure a range is available before the synchronous reader wants it.
    async fn prefetch(&mut self, offset: u64, len: u64) -> Result<(), JsError> {
        if let Backing::Http(source) = self.cursor.source_mut() {
            source
                .prefetch(offset, len)
                .await
                .map_err(|e| JsError::new(&format!("{e:?}")))?;
        }
        Ok(())
    }

    /// Seek to a time, fetching each probe before it is read.
    ///
    /// The cursor drives the search one step at a time and says which range
    /// it will touch next, so the fetching and the searching cannot take
    /// different paths through the file.
    async fn seek_prefetching(&mut self, target: i64) -> Result<Option<u64>, JsError> {
        self.prefetch(0, PROBE).await?;
        let mut state = self.cursor.seek_start().map_err(io_err)?;
        // Bounded so a malformed trace cannot spin here.
        for _ in 0..4096 {
            let (offset, len) = state.probe();
            self.prefetch(offset, len).await?;
            match self.cursor.seek_step(target, state).map_err(io_err)? {
                SeekStep::Scan(next) => {
                    // The scan walks a narrow span; fetch it in one go
                    // rather than a block per record.
                    let (from, _) = next.probe();
                    self.prefetch(from, SEEK_SPAN).await?;
                    state = next;
                }
                SeekStep::More(next) => state = next,
                SeekStep::Done(found) => return Ok(found),
            }
        }
        Ok(None)
    }

    fn window_to_js(&self, window: FrameWindow) -> Result<JsValue, JsError> {
        let result = Window {
            offset: window.offset as f64,
            next_offset: window.next_offset.map(|o| o as f64),
            end_offset: window.end_offset as f64,
            rows: rows_of(&window),
        };
        to_js(&result)
    }
}

fn header_field(param: &zniff_rs_core::decoder::frame_definition::DecodedParam) -> DetailField {
    let range = param.span.byte_range();
    DetailField {
        name: param.text.clone(),
        value: param.display_value(),
        start: range.start,
        end: range.end,
        children: param.children.iter().map(header_field).collect(),
    }
}

/// Convert a command class field, moving its range from payload-relative to
/// MPDU-relative so it lines up with the hex view.
fn app_field(
    param: &zniff_rs_core::decoder::command_class::DecodedParam,
    payload_at: usize,
) -> DetailField {
    DetailField {
        name: param.name.clone(),
        value: param.display.clone(),
        start: param.range.start + payload_at,
        end: param.range.end + payload_at,
        children: param.children.iter().map(|p| app_field(p, payload_at)).collect(),
    }
}
