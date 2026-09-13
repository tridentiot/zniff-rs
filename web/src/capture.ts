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
  /** Bytes recorded so far. */
  size(): number;
}

/** One region the device supports. */
export interface RegionChoice {
  code: number;
  name: string;
  current: boolean;
}

/** A connected dongle, before capturing has started. */
export interface Dongle {
  /** Firmware and chip identification. */
  version: string;
  /** What this device can tune to, as it reported them. */
  regions: RegionChoice[];
  /** Tune to a region and begin capturing. */
  start(code: number): Promise<Capture>;
  /** Release the port without capturing. */
  cancel(): Promise<void>;
}

/**
 * Ask the user for a dongle and identify it.
 *
 * Must be called from a click handler: `requestPort` needs a user gesture,
 * and the gesture does not survive an await before it.
 *
 * Regions come from the device rather than a built-in list, because the
 * hardware reports variants — the LR end-device ones — that the `Region`
 * enum does not name.
 */
export async function connect(): Promise<Dongle> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const port = await (navigator as any).serial.requestPort();

  const device = await SerialZniffer.open(port);
  const version = await device.version();
  const active = await device.current_region();
  const regions: RegionChoice[] = (await device.regions()).map((line) => {
    const [code, name] = line.split("\t");
    return {
      code: Number(code),
      name: name || `region ${code}`,
      current: Number(code) === active,
    };
  });

  return {
    version,
    regions,
    async cancel() {
      try {
        await device.close();
      } catch {
        // Already gone; nothing to release.
      }
    },
    async start(code: number) {
      await device.set_region_code(code);

      const recorder = new LiveRecorder(`web capture \u00b7 ${version}`);
      // Record the handshake, the way the bridge does, so the trace shows
      // how the capture was configured.
      for (const sent of device.take_sent()) {
        recorder.record(sent, true, Date.now());
      }

      await device.start();
      for (const sent of device.take_sent()) {
        recorder.record(sent, true, Date.now());
      }

      // The viewer starts from what has been recorded so far, and is then
      // fed each new record as it is written.
      const trace = Trace.open_live(recorder.bytes());
      let running = true;

      // Read until stopped. A quiet radio yields empty reads, which is
      // normal, not an error.
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

          // Record, then hand the viewer exactly the bytes that record
          // added, so neither side has to copy the whole trace.
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
    },
  };
}
