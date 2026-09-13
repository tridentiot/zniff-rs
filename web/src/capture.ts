// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Capturing from a zniffer in the browser over Web Serial.
//
// The protocol and the ZLF writing live in WebAssembly, shared with the
// capture bridge. This file owns only what must be JavaScript: the port
// picker, which needs a user gesture, and the read loop.

import { LiveRecorder, SerialZniffer, Trace } from "../pkg/zniff_rs_wasm.js";

/** Usable only in a secure context, and not in every browser. */
export function serialSupported(): boolean {
  return typeof navigator !== "undefined" && "serial" in navigator;
}

export interface Capture {
  /** The growing trace, for the viewer to read. */
  trace: Trace;
  /** Stop capturing and release the port. */
  stop(): Promise<void>;
  /** The recorded trace, for saving. */
  bytes(): Uint8Array;
  /** Frames captured so far, as a byte count. */
  size(): number;
}

/**
 * Ask the user for a dongle, configure it, and start capturing.
 *
 * Must be called from a click handler: `requestPort` needs a user gesture,
 * and the gesture does not survive an await before it.
 */
export async function connect(region: string | null): Promise<Capture> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const port = await (navigator as any).serial.requestPort();

  const device = await SerialZniffer.open(port);
  const version = await device.version();
  if (region) await device.set_region(region);

  const recorder = new LiveRecorder(`web capture · ${version}`);
  // Record the handshake, the way the bridge does, so the trace shows how
  // the capture was configured.
  for (const sent of device.take_sent()) {
    recorder.record(sent, true, Date.now());
  }

  await device.start();
  for (const sent of device.take_sent()) {
    recorder.record(sent, true, Date.now());
  }

  // The viewer starts from what has been recorded so far, and is then fed
  // each new record as it is written.
  const trace = Trace.open_live(recorder.bytes());
  let running = true;

  // Read until stopped. A quiet radio yields empty reads, which is normal.
  const pump = (async () => {
    while (running) {
      let chunk: Uint8Array;
      try {
        chunk = await device.read();
      } catch {
        break; // The port went away; stop rather than spin.
      }
      if (!running) break;
      if (chunk.length === 0) continue;

      // Record, then hand the viewer exactly the bytes that record added,
      // so neither side has to copy the whole trace.
      const before = recorder.len();
      recorder.record(chunk, false, Date.now());
      trace.append(recorder.bytes().subarray(before, recorder.len()));
    }
  })();

  return {
    trace,
    async stop() {
      running = false;
      try {
        await device.close();
      } catch {
        // Already gone; the capture is still valid.
      }
      await pump;
    },
    bytes: () => recorder.bytes(),
    size: () => recorder.len(),
  };
}
