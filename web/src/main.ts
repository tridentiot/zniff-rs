// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
import init, { Trace } from "../pkg/zniff_rs_wasm.js";
import {
  type Capture,
  type Dongle,
  connect,
  serialSupported,
} from "./capture.js";

interface Row {
  record_offset: number;
  frame_in_record: number;
  time_ms: number;
  delta_ms: number;
  speed: string;
  rssi: number;
  channel: number;
  direction: string;
  source: number | null;
  destination: number | null;
  home_id: string | null;
  data: string;
  application: string;
  hex: string;
  crc_ok: boolean;
  retransmission: number;
  original_offset: number | null;
  original_in_record: number | null;
  fg: string;
  bg: string;
}

interface Field {
  name: string;
  value: string;
  start: number;
  end: number;
  children: Field[];
}

interface Window_ {
  offset: number;
  next_offset: number | null;
  end_offset: number;
  rows: Row[];
}

interface Overview {
  size: number;
  start_time_ms: number;
  end_time_ms: number;
  first_offset: number;
  estimated_frames: number;
  frames_exact: boolean;
}

interface TraceInfo {
  version: number;
  comment: string;
  size: number;
  start_time_ms: number;
  end_time_ms: number;
  frames: number;
  frames_exact: boolean;
  sampled: number;
  regions: string[];
  speeds: string[];
  channels: number[];
  home_ids: string[];
}

interface AppHeading {
  class: string;
  version: number;
  command: string;
}

interface Detail {
  time_ms: number;
  hex: string;
  header_name: string;
  crc_ok: boolean;
  header: Field[];
  application: Field[];
  application_heading: AppHeading | null;
  application_summary: string;
  retransmission: RetransmissionInfo | null;
}

interface RetransmissionInfo {
  attempt: number;
  original_offset: number;
  original_in_record: number;
}

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

const els = {
  drop: $<HTMLDivElement>("drop"),
  cols: $<HTMLTableColElement>("cols"),
  head: $<HTMLTableRowElement>("head"),
  file: $<HTMLInputElement>("file"),
  filter: $<HTMLInputElement>("filter"),
  status: $<HTMLSpanElement>("status"),
  viewer: $<HTMLDivElement>("viewer"),
  list: $<HTMLDivElement>("list"),
  rows: $<HTMLTableSectionElement>("rows"),
  detail: $<HTMLElement>("detail"),
  goto: $<HTMLInputElement>("goto"),
  info: $<HTMLDivElement>("info"),
  infoToggle: $<HTMLButtonElement>("info-toggle"),
  serial: $<HTMLParagraphElement>("serial"),
  connect: $<HTMLButtonElement>("connect"),
  captureStop: $<HTMLButtonElement>("capture-stop"),
  captureSave: $<HTMLButtonElement>("capture-save"),
  picker: $<HTMLDivElement>("picker"),
  pickerDevice: $<HTMLParagraphElement>("picker-device"),
  region: $<HTMLSelectElement>("region"),
  pickerStart: $<HTMLButtonElement>("picker-start"),
  pickerCancel: $<HTMLButtonElement>("picker-cancel"),
};

/**
 * Default column widths in pixels, in table order. The order, captions and
 * widths follow the desktop Zniffer (ZWaveZnifferUI/Views/TraceControl.xaml
 * and ZWaveZniffer/Models/TraceSettings.cs), widened slightly where the
 * desktop values are too tight for a proportional web font.
 */
const DEFAULT_WIDTHS: Record<string, number> = {
  line: 55,
  date: 84,
  time: 100,
  speed: 84,
  rssi: 52,
  channel: 40,
  delta: 56,
  src: 44,
  dst: 70,
  home: 82,
  data: 160,
  application: 240,
  hexdata: 400,
};

const WIDTH_KEY = "zniff.columnWidths";
const MIN_WIDTH = 36;

function loadWidths(): Record<string, number> {
  const widths = { ...DEFAULT_WIDTHS };
  try {
    const saved = localStorage.getItem(WIDTH_KEY);
    if (saved) {
      for (const [key, value] of Object.entries(JSON.parse(saved) as Record<string, number>)) {
        if (key in widths && Number.isFinite(value)) {
          widths[key] = Math.max(MIN_WIDTH, value);
        }
      }
    }
  } catch {
    // A blocked or corrupt store just means the defaults are used.
  }
  return widths;
}

const widths = loadWidths();

function saveWidths(): void {
  try {
    localStorage.setItem(WIDTH_KEY, JSON.stringify(widths));
  } catch {
    // Persisting widths is a convenience; ignore quota or privacy errors.
  }
}

/** Build the <col> elements and the drag handles that resize them. */
function setUpColumns(): void {
  const headers = [...els.head.querySelectorAll<HTMLTableCellElement>("th[data-key]")];

  els.cols.replaceChildren(
    ...headers.map((th) => {
      const col = document.createElement("col");
      col.style.width = `${widths[th.dataset.key!]}px`;
      return col;
    }),
  );

  headers.forEach((th, i) => {
    const handle = document.createElement("span");
    handle.className = "resizer";
    // The handle sits inside the header but must not trigger a sort or select.
    handle.addEventListener("mousedown", (e) => startResize(e, i, th.dataset.key!));
    handle.addEventListener("dblclick", () => {
      widths[th.dataset.key!] = DEFAULT_WIDTHS[th.dataset.key!];
      applyWidth(i, widths[th.dataset.key!]);
      saveWidths();
    });
    th.appendChild(handle);
  });
}

function applyWidth(index: number, px: number): void {
  const col = els.cols.children[index] as HTMLElement | undefined;
  if (col) col.style.width = `${px}px`;
}

function startResize(event: MouseEvent, index: number, key: string): void {
  event.preventDefault();
  event.stopPropagation();

  const startX = event.clientX;
  const startWidth = widths[key];
  document.body.classList.add("resizing");

  const onMove = (e: MouseEvent) => {
    const next = Math.max(MIN_WIDTH, Math.round(startWidth + e.clientX - startX));
    widths[key] = next;
    applyWidth(index, next);
  };

  const onUp = () => {
    document.body.classList.remove("resizing");
    document.removeEventListener("mousemove", onMove);
    document.removeEventListener("mouseup", onUp);
    saveWidths();
  };

  document.addEventListener("mousemove", onMove);
  document.addEventListener("mouseup", onUp);
}

let trace: Trace | null = null;
let overview: Overview | null = null;

/** The window of frames currently rendered. */
let rows: Row[] = [];
/** Record offset of the first window currently in the list. */
let windowOffset = 0;

/**
 * Frame number of `rows[0]`, or null when it is not known.
 *
 * Numbering a frame means knowing how many came before it, and there is no
 * index: the only way to know is to have counted them. So the number is
 * tracked while reading forward from the start of the trace, and is null
 * after a jump to a time or a record in the middle, where counting the
 * skipped frames would mean reading the whole file.
 *
 * A blank Line cell is therefore honest rather than missing.
 */
let firstLine: number | null = null;
/** Record offset of the window after the last one loaded, or null at EOF. */
let nextOffset: number | null = null;

/**
 * Live capture state.
 *
 * A trace that is still being captured keeps growing, so reaching its end is
 * not final: `tailOffset` remembers where reading stopped, and polling resumes
 * from there once the file grows. Only set for a `?live=1` session, so an
 * ordinary static trace behaves exactly as before.
 */
let live: { timer: number; size: number } | null = null;

/** Where to resume from after the trace grows, set when EOF is reached. */
let tailOffset: number | null = null;

/** The capture in progress, when one is running. */
let capture: Capture | null = null;
/** Set while a window is loading, so scrolling cannot re-enter. */
let paging = false;

/**
 * The window after the current one, fetched before it is asked for.
 *
 * Reading a window means decoding frames and, for a linked trace, fetching
 * byte ranges. Starting that once the reader is halfway down hides the
 * latency behind the time they spend reading the rest.
 */
let ahead: { offset: number; window: Promise<Window_> } | null = null;
/** Row selected in that window, or -1. */
let selected = -1;

/** Rows fetched per window. Enough to fill a tall screen and overscan. */
const WINDOW = 250;

/**
 * Rows kept in the list before the oldest are dropped.
 *
 * The list grows as it is scrolled, but a trace can hold millions of frames
 * and every row is 13 DOM nodes, so the far end has to be let go of. The
 * scroll position is adjusted when that happens, so the view does not move.
 *
 * 500 is around eight screens on a tall display, which is more scrollback
 * than is useful given that Go to jumps anywhere in the trace.
 */
const MAX_ROWS = 500;

/** Measured at load; a hardcoded value drifts with the font. */
let rowHeight = 23;

function formatDate(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function formatTime(ms: number): string {
  const d = new Date(ms);
  const pad = (n: number, w = 2) => String(n).padStart(w, "0");
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}.${pad(d.getMilliseconds(), 3)}`;
}

/** Render the current window, positioned within the scroller. */
function renderRows(): void {
  const html = rows
    .map((r, pos) => {
      // Absolute striping: the window slides, so :nth-child would flicker.
      const stripe = pos % 2 === 1 ? " stripe" : "";
      const sel = pos === selected ? " selected" : "";
      const retx = r.retransmission > 0 ? " retx" : "";
      const mark =
        r.retransmission > 0
          ? `<span class="tag" title="Retransmission, attempt ${r.retransmission + 1}">RETX</span> `
          : "";
      // Blank when the count is unknown, which is the case after jumping
      // into the middle of a trace.
      const line = firstLine === null ? "" : (firstLine + pos).toLocaleString();
      return `<tr data-pos="${pos}" class="${stripe}${sel}${retx}" style="--frame-fg:${r.fg};--frame-bg:${r.bg}">
        <td class="num line">${line}</td>
        <td>${formatDate(r.time_ms)}</td>
        <td>${formatTime(r.time_ms)}</td>
        <td>${r.speed}</td>
        <td class="num">${r.rssi}</td>
        <td class="num">${r.channel}</td>
        <td class="num">${r.delta_ms}</td>
        <td class="num">${r.source ?? ""}</td>
        <td class="num">${r.destination ?? ""}</td>
        <td>${r.home_id ?? ""}</td>
        <td>${mark}${escape(r.data)}</td>
        <td>${escape(r.application)}</td>
        <td class="hex">${r.hex}</td>
      </tr>`;
    })
    .join("");

  els.rows.innerHTML = html;
  updateStatus();
}

function updateStatus(): void {
  if (!overview) return;

  // A count is exact only when the whole trace was read; otherwise it is
  // extrapolated, and saying so avoids "181 shown of ~175".
  const count = Math.round(overview.estimated_frames).toLocaleString();
  // While capturing, the end of the file is only the end *so far*.
  const capturing = live ? " · capturing" : "";
  if (overview.frames_exact) {
    els.status.textContent =
      `${rows.length.toLocaleString()} frames${capturing}`;
    return;
  }

  // How far the rows loaded so far reach into the file, not where the top
  // of the list is: the list grows downwards as it is scrolled.
  const reached = nextOffset ?? overview.size;
  const pct = overview.size
    ? Math.min(100, Math.round((100 * reached) / overview.size))
    : 0;
  const end = nextOffset === null && !live ? " · end of trace" : "";
  els.status.textContent =
    `${rows.length.toLocaleString()} loaded · ~${count} in trace · ` +
    `${pct}% through${end}${capturing}`;
}

/** Load the window starting at a record offset. */
async function showWindow(offset: number): Promise<void> {
  if (!trace) return;
  // Use the prefetched window when it is the one being asked for.
  const pending = ahead?.offset === offset ? ahead.window : null;
  ahead = null;
  const w = (await (pending ?? trace.rows_at(offset, WINDOW))) as Window_;
  rows = w.rows;
  windowOffset = w.offset;
  nextOffset = w.next_offset ?? null;
  if (nextOffset === null) tailOffset = w.end_offset;
  // Numbering is only known when the window is the start of the trace;
  // anywhere else, the frames before it have not been counted.
  firstLine = overview && w.offset === overview.first_offset ? 1 : null;
  selected = -1;
  renderRows();
  clearDetail();
  els.list.scrollTop = 0;
}

/**
 * Append the next window to the list.
 *
 * Rows accumulate as the trace is scrolled, so the scrollbar reflects what
 * has been read rather than jumping between fixed pages. Once the list
 * reaches [`MAX_ROWS`] the oldest rows are dropped and the scroll position
 * is moved by the same amount, which keeps the view still.
 */
async function appendNext(): Promise<void> {
  if (!trace || paging || nextOffset === null) return;
  paging = true;
  try {
    const offset = nextOffset;
    const pending = ahead?.offset === offset ? ahead.window : null;
    ahead = null;
    const w = (await (pending ?? trace.rows_at(offset, WINDOW))) as Window_;

    rows = rows.concat(w.rows);
    nextOffset = w.next_offset ?? null;
    // Remember where the trace ran out, so a live capture picks up after the
    // last record read rather than re-reading the final window.
    if (nextOffset === null) tailOffset = w.end_offset;

    let dropped = 0;
    if (rows.length > MAX_ROWS) {
      dropped = rows.length - MAX_ROWS;
      rows = rows.slice(dropped);
      // The first window in the list has moved on; remember where from, so
      // `prependPrevious` knows what to read back.
      windowOffset = rows[0]?.record_offset ?? windowOffset;
      if (selected >= 0) selected -= dropped;
      // rows[0] is now a later frame, so its number moves with it. Without
      // this the trimmed rows would be renumbered from the start.
      if (firstLine !== null) firstLine += dropped;
    }

    renderRows();
    if (dropped > 0) {
      // Keep the rows under the cursor where they were.
      els.list.scrollTop -= dropped * rowHeight;
    }
  } finally {
    paging = false;
  }
}

/**
 * Follow a capture that is still running.
 *
 * The viewer learns a trace's length once, when it opens, so a growing file
 * is invisible until the length is re-probed. Polling does that, and only
 * appends when the trace has actually grown, so a quiet radio costs one small
 * request per interval and changes nothing on screen.
 */
function startLiveTail(): void {
  if (live) return;
  const POLL_MS = 1000;
  const timer = window.setInterval(async () => {
    if (!trace || !live || paging) return;
    let size: number;
    try {
      size = await trace.poll_growth();
    } catch {
      // A capture that has finished stops answering; leave the rows in place.
      return;
    }
    if (size <= live.size) return;
    live.size = size;

    // Reaching the end of a growing trace is not final: resume from the
    // record boundary where reading stopped.
    if (nextOffset === null && tailOffset !== null) {
      nextOffset = tailOffset;
      tailOffset = null;
    }
    // Only pull new rows in when the view is already at the bottom, so
    // reading back through the trace is not interrupted.
    const atBottom =
      els.list.scrollTop + els.list.clientHeight >= els.list.scrollHeight - rowHeight * 2;
    if (atBottom) await appendNext();
    updateStatus();
  }, POLL_MS);
  live = { timer, size: 0 };
}

/** Stop following a capture. */
function stopLiveTail(): void {
  if (!live) return;
  window.clearInterval(live.timer);
  live = null;
}

/**
 * Put back the window before the one at the top of the list.
 *
 * Rows trimmed by `appendNext` are gone from the DOM but not from the trace,
 * so scrolling back re-reads them. The scroll position is moved down by the
 * height of what was inserted, which keeps the rows under the cursor still.
 */
async function prependPrevious(): Promise<void> {
  if (!trace || paging || windowOffset <= 0) return;
  // Already at the start; nothing came before.
  if (overview && windowOffset <= overview.first_offset) return;

  paging = true;
  try {
    const w = (await trace.rows_before(windowOffset, WINDOW)) as Window_;
    if (w.rows.length === 0) return;

    rows = w.rows.concat(rows);
    windowOffset = w.offset;
    if (selected >= 0) selected += w.rows.length;
    // The list now starts earlier, so its first frame number does too.
    if (firstLine !== null) firstLine -= w.rows.length;
    // Scrolling back far enough to reach the start makes the count known,
    // even if the trace was entered somewhere in the middle.
    if (firstLine === null && overview && w.offset === overview.first_offset) {
      firstLine = 1;
    }

    let dropped = 0;
    if (rows.length > MAX_ROWS) {
      // Trim from the far end this time, so scrolling back does not grow
      // the list without bound.
      dropped = rows.length - MAX_ROWS;
      rows = rows.slice(0, MAX_ROWS);
      if (selected >= MAX_ROWS) selected = -1;
      nextOffset = rows[rows.length - 1]?.record_offset ?? nextOffset;
    }

    renderRows();
    // Inserting above would otherwise push the view down by that much.
    els.list.scrollTop += w.rows.length * rowHeight;
  } finally {
    paging = false;
  }
}

/**
 * Start reading the next window, if it is not already in flight.
 *
 * Failures are swallowed: this is speculative, and the real attempt in
 * `appendNext` will surface anything that is genuinely wrong.
 */
function prefetchNext(): void {
  if (!trace || ahead !== null || nextOffset === null) return;
  const offset = nextOffset;
  ahead = {
    offset,
    window: (trace.rows_at(offset, WINDOW) as Promise<Window_>).catch(() => {
      if (ahead?.offset === offset) ahead = null;
      throw new Error("prefetch failed");
    }),
  };
  // An unobserved rejection would otherwise reach the console.
  ahead.window.catch(() => {});
}

/** Human-readable byte size. */
function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

/** A duration, as hours and minutes or seconds. */
function formatSpan(ms: number): string {
  const s = Math.round(ms / 1000);
  if (s < 60) return `${s} s`;
  const h = Math.floor(s / 3600);
  const min = Math.floor((s % 3600) / 60);
  return h > 0 ? `${h} h ${min} min` : `${min} min ${s % 60} s`;
}

/**
 * Fill in the trace information panel.
 *
 * ZLF stores almost nothing about a capture — a version, a text encoding
 * and an optional comment — so most of this is read back off the frames
 * themselves, from a sample rather than the whole trace.
 */
async function showInfo(): Promise<void> {
  if (!trace) return;
  const i = (await trace.info(500)) as TraceInfo;

  const list = (label: string, values: string[]) =>
    values.length
      ? `<dt>${label}</dt><dd class="wrap">${values.map(escape).join(", ")}</dd>`
      : "";

  const frames = i.frames_exact
    ? `${i.frames.toLocaleString()}`
    : `<span class="approx">about </span>${Math.round(i.frames).toLocaleString()}`;

  els.info.innerHTML = `
    <dl>
      <dt>Captured</dt><dd>${formatDate(i.start_time_ms)} ${formatTime(i.start_time_ms)}</dd>
      <dt>Duration</dt><dd>${formatSpan(i.end_time_ms - i.start_time_ms)}</dd>
      <dt>Frames</dt><dd>${frames}</dd>
    </dl>
    <dl>
      ${list("Region", i.regions)}
      ${list("Bit rates", i.speeds)}
      ${list("Channels", i.channels.map(String))}
    </dl>
    <dl>
      ${list("Networks", i.home_ids)}
    </dl>
    <dl>
      <dt>File</dt><dd>${formatSize(i.size)} · ZLF version ${i.version}</dd>
      ${i.comment ? `<dt>Comment</dt><dd class="wrap">${escape(i.comment)}</dd>` : ""}
      <dt>Summarised from</dt>
      <dd class="approx">${i.sampled.toLocaleString()} frames${i.frames_exact ? "" : " at the start"}</dd>
    </dl>
  `;
}

/**
 * Show a particular frame, loading the window that holds it if needed.
 *
 * A frame is addressed by the record it sits in, so this works whether
 * the frame is already on screen or somewhere else in the trace.
 */
async function goToFrame(recordOffset: number, inRecord: number): Promise<void> {
  if (!trace) return;
  let pos = rows.findIndex(
    (r) => r.record_offset === recordOffset && r.frame_in_record === inRecord,
  );
  if (pos < 0) {
    // Not in the loaded rows; load the window that starts there.
    await showWindow(recordOffset);
    pos = rows.findIndex(
      (r) => r.record_offset === recordOffset && r.frame_in_record === inRecord,
    );
  }
  if (pos < 0) return;

  await showDetail(pos);
  els.rows.children[pos]?.scrollIntoView({ block: "center" });
}

/** Load the window starting at a capture time. */
async function showTime(timeMs: number): Promise<void> {
  if (!trace) return;
  const w = (await trace.rows_from_time(timeMs, WINDOW)) as Window_;
  if (w.rows.length === 0) {
    els.status.textContent = "No frames at or after that time.";
    return;
  }
  rows = w.rows;
  windowOffset = w.offset;
  nextOffset = w.next_offset ?? null;
  if (nextOffset === null) tailOffset = w.end_offset;
  // A jump by time skips an unknown number of frames.
  firstLine = overview && w.offset === overview.first_offset ? 1 : null;
  ahead = null;
  selected = -1;
  renderRows();
  clearDetail();
  els.list.scrollTop = 0;
}

/**
 * Map the scrollbar onto the file.
 *
 * A trace can hold millions of frames, far more than an element can be tall
 * (browsers cap around 33 M px), so the scrollbar addresses a byte position
 * rather than a row. Fine navigation is by time or the keyboard.
 */
function scrollFraction(): number {
  const max = els.list.scrollHeight - els.list.clientHeight;
  return max > 0 ? els.list.scrollTop / max : 0;
}

function escape(s: string): string {
  return s.replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]!);
}

/** Render the hex dump, highlighting the bytes of the hovered field. */
function renderHex(hex: string, range: [number, number] | null): string {
  const bytes = hex.match(/../g) ?? [];
  return bytes
    .map((b, i) => {
      const on = range && i >= range[0] && i < range[1];
      return on ? `<span class="hl">${b}</span>` : b;
    })
    .join(" ");
}

function renderTree(fields: Field[]): string {
  if (fields.length === 0) return "";
  const item = (f: Field): string =>
    `<li data-start="${f.start}" data-end="${f.end}">
       <span class="k">${escape(f.name)}</span><span class="v">${escape(f.value)}</span>
     </li>${f.children.length ? `<ul>${f.children.map(item).join("")}</ul>` : ""}`;
  return `<ul class="tree">${fields.map(item).join("")}</ul>`;
}

/**
 * A note that this frame repeats an earlier one, with a link back to it.
 */
function renderRetransmission(d: Detail): string {
  const r = d.retransmission;
  if (!r) return "";
  return `<p class="retx-note">
    Retransmission, attempt ${r.attempt} —
    <a href="#" id="go-original"
       data-offset="${r.original_offset}"
       data-in-record="${r.original_in_record}">show the original</a>
  </p>`;
}

/**
 * The Application section: which command class and command, then the
 * decoded parameters. Naming both, with their keys, matches how the
 * desktop Zniffer heads this pane.
 */
function renderApplication(d: Detail): string {
  const h = d.application_heading;
  if (!h && d.application.length === 0) return "";

  const heading = h
    ? `<p class="cc">${escape(h.class)} <span class="ver">ver.${h.version}</span></p>
       <p class="cmd">${escape(h.command)}</p>`
    : "";
  return `<p class="section">Application</p>${heading}${renderTree(d.application)}`;
}

async function showDetail(pos: number): Promise<void> {
  const row = rows[pos];
  if (!trace || !row) return;
  selected = pos;
  for (const tr of els.rows.querySelectorAll("tr")) {
    tr.classList.toggle("selected", Number(tr.dataset.pos) === pos);
  }

  const d = (await trace.detail(
    row.record_offset,
    row.frame_in_record,
    row.retransmission,
    row.original_offset ?? undefined,
    row.original_in_record ?? undefined,
  )) as Detail;
  // The heading and hex dump stay put while only the field tree scrolls, so
  // the selected frame's bytes are always in view.
  els.detail.innerHTML = `
    <div class="detail-head">
      <h2>${formatTime(d.time_ms)} — ${escape(d.header_name)}${d.crc_ok ? "" : ' <span class="bad">CRC error</span>'}</h2>
      ${d.application_summary ? `<p class="summary">${escape(d.application_summary)}</p>` : ""}
      ${renderRetransmission(d)}
      <div class="hexview" id="hex">${renderHex(d.hex, null)}</div>
    </div>
    <div class="detail-body">
      ${d.header.length ? '<p class="section">MPDU header</p>' + renderTree(d.header) : ""}
      ${renderApplication(d)}
    </div>
  `;

  // Jump to the frame this one repeats.
  const original = document.getElementById("go-original");
  original?.addEventListener("click", (e) => {
    e.preventDefault();
    const offset = Number((original as HTMLElement).dataset.offset);
    const inRecord = Number((original as HTMLElement).dataset.inRecord);
    void goToFrame(offset, inRecord);
  });

  // Hovering a field highlights the bytes it came from.
  const hex = $<HTMLDivElement>("hex");
  for (const li of els.detail.querySelectorAll<HTMLLIElement>("li[data-start]")) {
    li.addEventListener("mouseenter", () => {
      hex.innerHTML = renderHex(d.hex, [Number(li.dataset.start), Number(li.dataset.end)]);
    });
    li.addEventListener("mouseleave", () => {
      hex.innerHTML = renderHex(d.hex, null);
    });
  }
}

function clearDetail(): void {
  els.detail.innerHTML =
    '<div class="detail-body"><p class="hint">Select a frame to see its fields.</p></div>';
}

/**
 * Filter by scanning the trace from the start.
 *
 * There is no index, so matching means reading; the scan is incremental and
 * reports as it goes rather than freezing until it finishes.
 */
let filterRun = 0;

async function applyFilter(): Promise<void> {
  if (!trace || !overview) return;
  const query = els.filter.value.trim();
  const run = ++filterRun;

  if (!query) {
    await showWindow(overview.first_offset);
    return;
  }

  rows = [];
  selected = -1;
  // Matches are scattered through the trace, so consecutive result rows are
  // not consecutive frames; numbering them 1, 2, 3 would be a lie.
  firstLine = null;
  clearDetail();

  let offset = overview.first_offset;
  let scanned = 0;
  for (;;) {
    // A newer query supersedes this scan.
    if (run !== filterRun) return;

    const w = (await trace.filter_step(query, offset, 500)) as Window_;
    rows.push(...w.rows);
    scanned += 500;
    renderRows();
    els.status.textContent =
      `${rows.length} matches · scanned ${Math.min(100, Math.round((100 * offset) / overview.size))}%`;

    if (w.next_offset === null || rows.length > 5000) break;
    offset = w.next_offset;
    // Yield so the page keeps painting.
    await new Promise((r) => setTimeout(r, 0));
  }
  els.status.textContent = `${rows.length} matches`;
}

async function load(open: () => Promise<Trace> | Trace, label: string): Promise<void> {
  els.status.textContent = `Opening ${label}…`;
  // Dropping a file while following a capture must not leave the old poll
  // running against the new trace.
  stopLiveTail();
  tailOffset = null;
  try {
    trace = await open();
    overview = (await trace.overview()) as Overview;
  } catch (e) {
    els.status.textContent = `Could not read ${label}: ${e}`;
    return;
  }

  els.drop.hidden = true;
  els.viewer.hidden = false;
  els.filter.disabled = false;
  els.goto.disabled = false;
  els.infoToggle.disabled = false;
  els.info.hidden = true;
  els.info.dataset.filled = "";
  els.infoToggle.setAttribute("aria-expanded", "false");

  measureRowHeight();
  await showWindow(overview.first_offset);
}

/** Read the real row height so scroll maths matches what is drawn. */
function measureRowHeight(): void {
  const probe = els.rows.querySelector("tr");
  if (probe) {
    const h = probe.getBoundingClientRect().height;
    if (h > 0) rowHeight = h;
  }
}

/** The connected dongle, while the region is being chosen. */
let dongle: Dongle | null = null;

/** Connect to a dongle and offer its regions. */
async function connectDongle(): Promise<void> {
  if (capture || dongle) return;
  els.status.textContent = "Waiting for a zniffer\u2026";
  try {
    dongle = await connect();
  } catch (e) {
    // A cancelled picker is a choice, not a failure.
    const message = String(e);
    els.status.textContent = /NotFoundError|cancel|No port selected/i.test(message)
      ? ""
      : `Could not connect: ${message}`;
    return;
  }

  els.pickerDevice.textContent = dongle.version;
  // The list comes from the device, so it shows what this dongle can do
  // rather than what the software knows about.
  els.region.innerHTML = dongle.regions
    .map(
      (r) =>
        `<option value="${r.code}"${r.current ? " selected" : ""}>${escape(r.name)}</option>`,
    )
    .join("");

  // A region named in the URL wins, so a CI page can pin one.
  const wanted = new URLSearchParams(location.search).get("region");
  if (wanted) {
    const match = dongle.regions.find(
      (r) => r.name.toLowerCase() === wanted.toLowerCase() || String(r.code) === wanted,
    );
    if (match) els.region.value = String(match.code);
  }

  els.picker.hidden = false;
  els.connect.disabled = true;
  els.status.textContent = "";
}

/** Tune to the chosen region and start capturing. */
async function startCapture(): Promise<void> {
  if (!dongle) return;
  const code = Number(els.region.value);
  const name = els.region.selectedOptions[0]?.textContent ?? String(code);
  els.status.textContent = `Tuning to ${name}\u2026`;
  els.pickerStart.disabled = true;

  try {
    capture = await dongle.start(code);
  } catch (e) {
    els.status.textContent = `Could not start: ${e}`;
    els.pickerStart.disabled = false;
    return;
  }

  dongle = null;
  els.picker.hidden = true;
  els.pickerStart.disabled = false;
  els.captureStop.hidden = false;
  els.captureSave.hidden = false;
  await load(() => capture!.trace, `the zniffer on ${name}`);
  startLiveTail();
}

/** Release the port without capturing. */
async function cancelConnect(): Promise<void> {
  if (!dongle) return;
  await dongle.cancel();
  dongle = null;
  els.picker.hidden = true;
  els.connect.disabled = false;
  els.status.textContent = "";
}

/** Stop the capture, leaving the frames on screen. */
async function stopCapture(): Promise<void> {
  if (!capture) return;
  await capture.stop();
  stopLiveTail();
  els.captureStop.hidden = true;
  els.connect.disabled = false;
  updateStatus();
}

/** Save the capture as a .zlf. */
function saveCapture(): void {
  if (!capture) return;
  // Copy into a plain ArrayBuffer: the WASM view is over memory that may be
  // shared, which Blob will not take.
  const bytes = capture.bytes();
  const buffer = new ArrayBuffer(bytes.length);
  new Uint8Array(buffer).set(bytes);
  const blob = new Blob([buffer], { type: "application/octet-stream" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  const stamp = new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
  a.download = `capture-${stamp}.zlf`;
  a.click();
  URL.revokeObjectURL(url);
}

/**
 * Show or hide the trace information panel.
 *
 * The contents are read from the trace the first time it is opened, since
 * that samples frames and there is no reason to pay for it unasked.
 */
async function toggleInfo(): Promise<void> {
  if (!trace) return;
  const showing = els.info.hidden;
  if (showing && !els.info.dataset.filled) {
    els.info.innerHTML = "<p>Reading\u2026</p>";
    els.info.hidden = false;
    els.infoToggle.setAttribute("aria-expanded", "true");
    try {
      await showInfo();
      els.info.dataset.filled = "1";
    } catch (e) {
      els.info.innerHTML = `<p>Could not read the trace information: ${escape(String(e))}</p>`;
    }
    return;
  }
  els.info.hidden = !showing;
  els.infoToggle.setAttribute("aria-expanded", String(showing));
}

function wireUp(): void {
  setUpColumns();

  els.infoToggle.addEventListener("click", () => void toggleInfo());

  // Only offered where the browser can actually talk to a serial port.
  if (serialSupported()) {
    els.serial.hidden = false;
    els.connect.addEventListener("click", () => void connectDongle());
    els.pickerStart.addEventListener("click", () => void startCapture());
    els.pickerCancel.addEventListener("click", () => void cancelConnect());
    els.captureStop.addEventListener("click", () => void stopCapture());
    els.captureSave.addEventListener("click", saveCapture);
  }

  els.rows.addEventListener("click", (e) => {
    const tr = (e.target as HTMLElement).closest("tr");
    if (tr?.dataset.pos) void showDetail(Number(tr.dataset.pos));
  });

  document.addEventListener("keydown", (e) => {
    if (!trace || document.activeElement === els.filter) return;
    if (e.key === "ArrowDown" && selected < rows.length - 1) {
      void showDetail(selected + 1);
      e.preventDefault();
    } else if (e.key === "ArrowUp" && selected > 0) {
      void showDetail(selected - 1);
      e.preventDefault();
    } else if (e.key === "End") {
      // Load more rather than only moving within what is loaded.
      void appendNext();
      e.preventDefault();
    } else if (e.key === "Home") {
      // Likewise backwards, for rows that were trimmed off the front.
      void prependPrevious();
      e.preventDefault();
    }
  });

  // The list grows as it is scrolled. Loading starts before the bottom is
  // reached so the rows are usually already there.
  els.list.addEventListener("scroll", () => {
    if (paging) return;
    const f = scrollFraction();
    // Halfway: start reading, so the latency is spent while still scrolling.
    if (f > 0.5) prefetchNext();
    // Near the bottom: add the rows to the list.
    if (f > 0.85) void appendNext();
    // Near the top: put back the rows that were trimmed off the front.
    if (f < 0.15) void prependPrevious();
  });

  let debounce: number;
  els.filter.addEventListener("input", () => {
    clearTimeout(debounce);
    debounce = setTimeout(() => void applyFilter(), 200);
  });

  // "Go to time" is how a large trace is navigated, so make it direct.
  els.goto.addEventListener("change", () => {
    if (!overview || !els.goto.value) return;
    const ms = Date.parse(els.goto.value);
    if (!Number.isNaN(ms)) void showTime(ms);
  });

  els.file.addEventListener("change", async () => {
    const file = els.file.files?.[0];
    if (!file) return;
    const bytes = new Uint8Array(await file.arrayBuffer());
    void load(() => Trace.open_bytes(bytes), file.name);
  });

  for (const type of ["dragenter", "dragover"]) {
    els.drop.addEventListener(type, (e) => {
      e.preventDefault();
      els.drop.classList.add("over");
    });
  }
  for (const type of ["dragleave", "drop"]) {
    els.drop.addEventListener(type, () => els.drop.classList.remove("over"));
  }
  document.addEventListener("dragover", (e) => e.preventDefault());
  document.addEventListener("drop", async (e) => {
    e.preventDefault();
    const file = e.dataTransfer?.files?.[0];
    if (!file) return;
    const bytes = new Uint8Array(await file.arrayBuffer());
    void load(() => Trace.open_bytes(bytes), file.name);
  });
}


async function main(): Promise<void> {
  await init();
  wireUp();
  els.status.textContent = "";

  // A ?trace= parameter loads a trace straight from CI. The trace is read
  // over ranges, so a large one does not have to be downloaded first.
  //
  // Named "trace" rather than "url" because Vite's dev server treats a
  // ?url= parameter as a filesystem path and refuses the request.
  const params = new URLSearchParams(location.search);
  const url = params.get("trace") ?? params.get("url");
  if (url) {
    await load(() => Trace.open_url(url), url);
    // `?live=1` follows a trace that something else is still appending to.
    if (params.get("live") === "1" && trace) startLiveTail();
  }
}

main();
