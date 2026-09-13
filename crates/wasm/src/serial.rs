// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Talking to a zniffer from the browser over Web Serial.
//!
//! The protocol lives in `zniff_rs_core::zniffer` and is shared with the
//! capture bridge; only the reading and writing differ. Web Serial needs a
//! secure context and a user gesture for the port picker, so `request_port`
//! must be called from a click handler.

use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    ReadableStreamDefaultReader,
    SerialOptions,
    SerialPort,
    WritableStreamDefaultWriter,
};
use zniff_rs_core::zniffer::{
    BAUD_RATES,
    CMD_GET_FREQUENCIES,
    CMD_GET_FREQUENCY_STR,
    CMD_GET_VERSION,
    CMD_SET_FREQUENCY,
    CMD_START,
    CMD_STOP,
    Version,
    command,
    current_region,
    found_response,
    region_name,
    response_payload,
    supported_regions,
};

/// How long to wait for a command response before giving up on it.
///
/// Responses come back in a few milliseconds; this only has to be longer
/// than that, and short enough that a command which never answers does not
/// stall the caller noticeably.
const READ_TIMEOUT_MS: i32 = 120;

/// How long a capture read waits before returning empty.
///
/// Longer, because silence is the normal state of a quiet radio and each
/// expiry costs a round trip through the event loop.
const CAPTURE_TIMEOUT_MS: i32 = 500;

/// Resolve `promise`, or `None` if `timeout_ms` passes first.
///
/// Races the promise against a `setTimeout`, and marks the timer's result
/// with a sentinel so the two outcomes can be told apart.
async fn race(promise: js_sys::Promise, timeout_ms: i32) -> Option<Result<JsValue, JsValue>> {
    const EXPIRED: &str = "__zniff_timeout";

    let timer = js_sys::Promise::new(&mut |resolve, _reject| {
        let global = js_sys::global();
        // setTimeout lives on the window or the worker scope; both expose it
        // on the global object.
        if let Ok(set_timeout) = js_sys::Reflect::get(&global, &JsValue::from_str("setTimeout"))
            && let Some(set_timeout) = set_timeout.dyn_ref::<js_sys::Function>()
        {
            // Resolve with the sentinel, so a timeout is distinguishable
            // from a read that genuinely returned.
            let _ = set_timeout.call3(
                &global,
                &resolve,
                &JsValue::from(timeout_ms),
                &JsValue::from_str(EXPIRED),
            );
        }
    });

    let raced = js_sys::Promise::race(&js_sys::Array::of2(&promise, &timer));
    match JsFuture::from(raced).await {
        Ok(v) if v.as_string().as_deref() == Some(EXPIRED) => None,
        other => Some(other),
    }
}

/// A zniffer reached over Web Serial.
#[wasm_bindgen]
pub struct SerialZniffer {
    port: SerialPort,
    /// Bytes read but not yet consumed by a response match.
    pending: Vec<u8>,
    /// Every command written, so a capture can record them the way the
    /// bridge does.
    sent: Vec<Vec<u8>>,
}

#[wasm_bindgen]
impl SerialZniffer {
    /// Open a port the user has already picked and identify the device.
    ///
    /// The port comes from `navigator.serial.requestPort()`, which must be
    /// called from a user gesture; doing that in JavaScript keeps the gesture
    /// intact across the WASM boundary.
    pub async fn open(port: SerialPort) -> Result<SerialZniffer, JsError> {
        for baud in BAUD_RATES {
            let options = SerialOptions::new(baud);
            // An already-open port fails here; treat it as this rate having
            // been tried rather than as fatal.
            if JsFuture::from(port.open(&options)).await.is_err() {
                continue;
            }

            let mut device =
                SerialZniffer { port: port.clone(), pending: Vec::new(), sent: Vec::new() };

            // Stop first: a device left capturing would bury the version
            // response under frames.
            let _ = device.request(CMD_STOP, &[0x00], 12).await;
            if let Ok(Some(payload)) = device.request(CMD_GET_VERSION, &[0x00], 20).await
                && Version::parse(&payload).is_some()
            {
                return Ok(device);
            }

            // Not this rate; close before trying the next.
            let _ = JsFuture::from(port.close()).await;
        }
        Err(JsError::new(
            "no zniffer answered on that port at 230400 or 115200 baud",
        ))
    }

    /// Firmware and chip identification.
    pub async fn version(&mut self) -> Result<String, JsError> {
        let payload = self
            .request(CMD_GET_VERSION, &[0x00], 20)
            .await?
            .ok_or_else(|| JsError::new("the device did not report its version"))?;
        Version::parse(&payload)
            .map(|v| v.to_string())
            .ok_or_else(|| JsError::new("malformed version response"))
    }

    /// Select the capture region by the code the device reported.
    ///
    /// Takes a raw code rather than a name: the hardware lists regions the
    /// `Region` enum does not name, such as the LR end-device variants, and
    /// those must still be selectable.
    pub async fn set_region_code(&mut self, code: u8) -> Result<(), JsError> {
        let _ = self.request(CMD_SET_FREQUENCY, &[0x01, code], 8).await;
        match self.current_region().await? {
            Some(actual) if actual == code => Ok(()),
            Some(actual) => Err(JsError::new(&format!(
                "the device stayed on region {actual:#04x} instead of {code:#04x}"
            ))),
            None => Err(JsError::new("the device did not report its region")),
        }
    }

    /// The region code the device is tuned to.
    pub async fn current_region(&mut self) -> Result<Option<u8>, JsError> {
        let payload = self.request(CMD_GET_FREQUENCIES, &[0x00], 16).await?;
        Ok(payload.as_deref().and_then(current_region))
    }

    /// The regions the device supports, as `code\tname` lines.
    pub async fn regions(&mut self) -> Result<Vec<String>, JsError> {
        let Some(payload) = self.request(CMD_GET_FREQUENCIES, &[0x00], 16).await? else {
            return Ok(Vec::new());
        };

        let mut out = Vec::new();
        for code in supported_regions(&payload).to_vec() {
            let name = self
                .request(CMD_GET_FREQUENCY_STR, &[0x01, code], 12)
                .await?
                .map(|p| region_name(&p))
                .unwrap_or_default();
            out.push(format!("{code}\t{name}"));
        }
        Ok(out)
    }

    /// Begin capturing.
    pub async fn start(&mut self) -> Result<(), JsError> {
        self.request(CMD_START, &[0x00], 12).await?;
        Ok(())
    }

    /// Stop capturing and release the port.
    pub async fn close(&mut self) -> Result<(), JsError> {
        let _ = self.request(CMD_STOP, &[0x00], 12).await;
        JsFuture::from(self.port.close())
            .await
            .map_err(|e| JsError::new(&format!("closing the port failed: {e:?}")))?;
        Ok(())
    }

    /// The handshake commands written so far, so a capture can log them.
    ///
    /// Taken rather than copied: each is recorded once.
    pub fn take_sent(&mut self) -> Vec<Uint8Array> {
        std::mem::take(&mut self.sent)
            .iter()
            .map(|b| Uint8Array::from(&b[..]))
            .collect()
    }

    /// Read whatever capture data has arrived, or an empty array when the
    /// radio is quiet. A silent trace is normal, not an error.
    pub async fn read(&mut self) -> Result<Uint8Array, JsError> {
        // Anything buffered while matching a response is capture data.
        let mut out = std::mem::take(&mut self.pending);
        if let Some(chunk) = self.read_chunk(CAPTURE_TIMEOUT_MS).await? {
            out.extend_from_slice(&chunk);
        }
        Ok(Uint8Array::from(&out[..]))
    }
}

impl SerialZniffer {
    /// Write a command and wait for its response, reading up to `attempts`
    /// chunks before giving up.
    async fn request(
        &mut self,
        cmd: u8,
        parameters: &[u8],
        attempts: usize,
    ) -> Result<Option<Vec<u8>>, JsError> {
        let framed = command(cmd, parameters);
        self.write(&framed).await?;
        self.sent.push(framed);

        for _ in 0..attempts {
            if let Some((found, end)) = found_response(&self.pending, cmd) {
                // Keep whatever followed the reply: on a busy network that
                // is captured frames, and clearing it would drop them.
                self.pending.drain(..end);
                return Ok(Some(found));
            }
            let Some(chunk) = self.read_chunk(READ_TIMEOUT_MS).await? else {
                break;
            };
            self.pending.extend_from_slice(&chunk);
        }
        Ok(response_payload(&self.pending, cmd))
    }

    /// Write bytes to the port.
    async fn write(&self, bytes: &[u8]) -> Result<(), JsError> {
        let writable = self.port.writable();
        let writer: WritableStreamDefaultWriter = writable
            .get_writer()
            .map_err(|e| JsError::new(&format!("the port is not writable: {e:?}")))?;
        let data = Uint8Array::from(bytes);
        let result = JsFuture::from(writer.write_with_chunk(&data)).await;
        // Release before reporting, or the port stays locked on failure.
        writer.release_lock();
        result.map_err(|e| JsError::new(&format!("writing to the port failed: {e:?}")))?;
        Ok(())
    }

    /// Read one chunk, giving up after `timeout_ms`.
    ///
    /// A serial read never completes on its own when the device has nothing
    /// to say, and several commands — `SetFrequency` among them — send no
    /// reply at all. Without a timeout those calls wait forever.
    ///
    /// `Ok(Some(empty))` means the wait expired, which is not an error: a
    /// quiet radio looks exactly the same as one that has finished talking.
    async fn read_chunk(&self, timeout_ms: i32) -> Result<Option<Vec<u8>>, JsError> {
        let readable = self.port.readable();
        let reader: ReadableStreamDefaultReader = readable
            .get_reader()
            .unchecked_into();

        let result = match race(reader.read(), timeout_ms).await {
            Some(result) => result,
            None => {
                // Cancel the pending read, or the next one inherits it and
                // the stream stays locked.
                let _ = reader.cancel();
                reader.release_lock();
                return Ok(Some(Vec::new()));
            },
        };
        reader.release_lock();

        let value = result
            .map_err(|e| JsError::new(&format!("reading from the port failed: {e:?}")))?;
        let done = js_sys::Reflect::get(&value, &JsValue::from_str("done"))
            .map(|d| d.as_bool().unwrap_or(false))
            .unwrap_or(false);
        if done {
            return Ok(None);
        }
        let chunk = js_sys::Reflect::get(&value, &JsValue::from_str("value"))
            .map_err(|e| JsError::new(&format!("malformed read result: {e:?}")))?;
        if chunk.is_undefined() || chunk.is_null() {
            return Ok(Some(Vec::new()));
        }
        Ok(Some(Uint8Array::new(&chunk).to_vec()))
    }
}

/// Builds a ZLF in memory from bytes read off a zniffer.
///
/// The same `ZlfWriter` the capture bridge uses, so a trace recorded in the
/// browser is byte-for-byte the same shape as one recorded on the command
/// line — and is verified against a C# capture by the core's writer tests.
#[wasm_bindgen]
pub struct LiveRecorder {
    writer: zniff_rs_core::zlf::ZlfWriter<Vec<u8>>,
}

#[wasm_bindgen]
impl LiveRecorder {
    /// Start a recording, writing the 2048-byte header.
    #[wasm_bindgen(constructor)]
    pub fn new(comment: &str) -> Result<LiveRecorder, JsError> {
        let writer = zniff_rs_core::zlf::ZlfWriter::new(Vec::new(), comment)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(LiveRecorder { writer })
    }

    /// Append one record holding raw transport bytes.
    ///
    /// `outgoing` marks a command written to the device, matching what the
    /// bridge records, so the trace shows how the capture was configured.
    pub fn record(
        &mut self,
        bytes: &[u8],
        outgoing: bool,
        time_ms: f64,
    ) -> Result<(), JsError> {
        if bytes.is_empty() {
            return Ok(());
        }
        self.writer
            .write_record(
                zniff_rs_core::zlf::Timestamp::from_unix_millis(time_ms as i64),
                outgoing,
                0,
                zniff_rs_core::zlf::ApiType::Zniffer,
                bytes,
            )
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Everything written so far, header included.
    pub fn bytes(&self) -> Vec<u8> {
        self.writer.buffer().to_vec()
    }

    /// Bytes written so far.
    pub fn len(&self) -> f64 {
        self.writer.bytes_written() as f64
    }
}
