<!--
SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
SPDX-License-Identifier: MIT
-->
# Z-Wave Trace Viewer

Reads `.zlf` traces in the browser, so a CI test report can link straight to
the trace of a failing run. The decoding is `zniff-rs-core` compiled to
WebAssembly; there is no server and nothing to install.

## Use

Open the page and drop a `.zlf` file on it, or point it at one directly:

```
https://<host>/?trace=https://ci.example/artifacts/run-123/trace.zlf
```

A cross-origin artifact host must send permissive CORS headers for `?trace=`
to work; otherwise serve the trace from the same origin or via a proxy.

### Columns

The columns follow the desktop Zniffer: Date, Time, Speed, RSSI, Ch, Delta,
Src, Dst, Home, Data, Application, Hex Data.

The desktop tool's **Line** column is absent. Numbering a frame absolutely
means counting every frame before it, and for a trace read over the network
that means fetching the whole file — the one thing this viewer avoids.
Frames are identified by capture time instead, which is also what **Go to**
navigates by.

The list grows as it is scrolled: reaching the bottom appends the next
frames rather than replacing them. Reading past halfway starts fetching
them, so they are usually there already — a window whose bytes are not
cached takes over a second on a high-latency connection.

Rows are dropped from the top once about 500 are loaded, with the scroll
position moved to match so the view stays still. Use **Go to** to jump
back to an earlier point.

Drag the divider at the right edge of a column heading to resize it, and
double-click a divider to restore that column's default width. Widths are
remembered in `localStorage`.

### Large traces

The trace is never decoded in full. The viewer seeks to the part of the
file it needs, so opening is quick whatever the size — a 300 MB trace opens
in about 40 ms and holds a couple of megabytes, where decoding it all would
have needed 28 GB.

Over `?trace=` the trace is read with HTTP range requests, so only the parts
that are read are fetched: opening a 50 MB trace and seeking into it
transfers about 8 MB. The server must support ranges; one that ignores them
is reported rather than silently misread.

Because nothing is counted up front, the frame total in the status bar is an
estimate. The scrollbar addresses a position in the file rather than a row — millions of
rows are far more than an element can be tall — so navigate by **Go to**
time, the arrow keys, and Page Up/Down.

Filtering scans the trace and reports matches as it finds them, since
matching without an index means reading.

### Trace info

**Trace info** in the header folds out what is known about the capture:
when it was taken and for how long, the region and bit rates, the
channels, the networks seen, the file size and ZLF version.

ZLF stores very little about a capture — a format version, a text
encoding, and a comment that the capture tool leaves empty unless someone
types one. There is no region or source filename in the file. So most of
this is read back off the frames, from a sample rather than the whole
trace, and the panel says how many frames it was drawn from.

### The Application column

Beams show `WakeUp Beam(n)` with their repetition count, routed frames
show `Routed Ack` or `Routed Error`, and plain acknowledgements are left
blank — as in the desktop Zniffer. Otherwise it shows the command name
alone — `S2 Nonce Get`, `Switch Binary Set` — as
the desktop Zniffer does: its converter takes the command's text when one
matches and falls back to the class text when none does. The detail pane
has room for both, so it shows `Command Class Security 2 · S2 Message
Encapsulation`.

### Retransmissions

A frame the sender repeated because no acknowledgement arrived is marked
**RETX** in the Data column, with the attempt number on hover. Its detail
pane says which attempt it is and links back to the first transmission,
loading that part of the trace if it is no longer on screen.

Only singlecasts are considered, routed ones included: a singlecast is
the only MPDU that is acknowledged, and so the only one that is
retransmitted (G.9959 8.1.5.1.4.2). Explorer frames and broadcasts reuse
sequence numbers freely.

G.9959 8.1.3.3.7 requires a node to keep the same sequence number across
an initial transmission and all its retransmissions, and to advance it
for each new one. So the last sequence number each sender used is
tracked, per home id and source node, and a sender repeating that value
is retrying. Timing plays no part, and neither does the destination: a
sender advances its sequence number per MPDU whoever it is addressed
to.

### Colours

Rows keep the desktop Zniffer's scheme — text colour gives the frame type,
row background gives the bit rate, and a failed checksum overrides the frame
type — but the palette is drawn from the Trident IoT brand rather than the
original .NET named colours.

Hues stay in the brand's cool blue family wherever the frame types allow,
anchored on the navy `#15244F` and blue `#2EA3F2`; the CRC-error red is derived
from the brand's `#CF2E2E`.

| Frame type | Colour | | Bit rate | Background |
| --- | --- | --- | --- | --- |
| Singlecast | `#136A58` | | 9.6 kbps | `#FFFFFF` |
| Routed singlecast | `#0D6779` | | 40 kbps | `#F4F6FA` |
| Ack | `#0A629D` | | 100 kbps | `#EDF4FB` |
| Routed ack | `#1E5EA8` | | Long Range | `#EAF6F3` |
| Broadcast | `#3257BD` | | | |
| Multicast, wake-up beam | `#7C3EBB` | | | |
| Explorer | `#86530B` | | | |
| Flooded | `#555F6D` | | | |
| Routed error | `#A04028` | | | |
| CRC error | `#B32727` | | | |

Rows come in three shades of their bit-rate background: the plain colour,
a darker alternating stripe, and a darker one still for the selected row.

Every frame colour clears 4.5:1 against all three, and a unit test enforces
it against the same mix amounts the stylesheet uses. The .NET palette this
replaced fell to 2.15:1 for explorer frames. Because the colours need a
light ground, the frame list stays light even when the rest of the page
follows a dark system theme.

### Detail pane

Selecting a frame shows its fields on the right. The Application section
names the command class and the command, each with its key and the class
version, above the decoded parameters — as the desktop Zniffer heads that
pane. The heading and the hex
dump stay pinned while the field tree scrolls, so the selected frame's
bytes are always in view; hovering a field highlights the bytes it came
from.

### Filtering

Terms combine with AND. Anything that is not a recognised `key:value` pair is
matched as free text against the frame type, the application summary and the
home id.

| Term | Matches |
| --- | --- |
| `src:1` | source node 1 |
| `dst:2` | destination node 2 |
| `node:3` | node 3 as either endpoint |
| `home:e7a4ac25` | home id substring |
| `speed:2` | 0 = 9.6k, 1 = 40k, 2 = 100k, 3 = LR |
| `ch:1` | channel |
| `crc:bad` | frames that failed the checksum |
| `retx:yes` | frames the sender repeated |

## Develop

```bash
npm install
npm run dev      # builds the wasm bundle, then serves with hot reload
npm run build    # static bundle in dist/
```

`npm run build:wasm` alone rebuilds only the WebAssembly, which is what most
Rust changes need.
