// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! A trace read over HTTP with range requests.
//!
//! Fetching a 300 MB trace in full would defeat the point of seeking, so
//! only the blocks actually read are fetched. Blocks are cached, because a
//! binary search revisits the same neighbourhoods and the decoder reads a
//! record header immediately before its payload.
use std::collections::HashMap;
use std::io;

use js_sys::{
    Reflect,
    Uint8Array,
};
use wasm_bindgen::{
    JsCast,
    JsValue,
};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Request,
    RequestInit,
    Response,
};
use zniff_rs_core::source::TraceSource;

/// Bytes fetched per request. Large enough that a window of frames usually
/// costs one round trip, small enough not to pull the file down by accident.
const BLOCK: u64 = 256 * 1024;

/// Blocks kept before the least recently used is dropped (~16 MB).
const MAX_BLOCKS: usize = 64;

/// A trace served over HTTP, read in blocks.
pub struct RangeSource {
    url: String,
    len: u64,
    blocks: HashMap<u64, Vec<u8>>,
    /// Block indices in use order, oldest first.
    order: Vec<u64>,
}

impl RangeSource {
    /// Probe the URL for its length and range support.
    ///
    /// Falls back to downloading the whole trace when the server ignores
    /// `Range`, since a viewer that silently reads the wrong bytes would be
    /// worse than a slow one.
    pub async fn open(url: String) -> Result<Self, JsValue> {
        // Probe with a real block rather than a single byte: some servers
        // (Vite's dev server among them) answer a one-byte range with a 206
        // and the whole file, which would pull the entire trace down here.
        let probe = fetch_range(&url, 0, BLOCK).await?;
        let status = probe.status();
        let len = match status {
            206 => content_range_total(&probe)
                .ok_or_else(|| JsValue::from_str("no Content-Range on a 206 response"))?,
            200 => {
                // The server ignored the range; it will always send the whole
                // file, so a block cache would fetch it repeatedly.
                return Err(JsValue::from_str(
                    "the server does not support range requests for this trace",
                ));
            }
            other => {
                return Err(JsValue::from_str(&format!(
                    "fetching the trace failed with HTTP {other}"
                )));
            }
        };

        // The probe already holds the first block; keep it rather than
        // fetching it again on the first read.
        let mut source =
            Self { url, len, blocks: HashMap::new(), order: Vec::new() };
        if let Ok(buffer) = JsFuture::from(probe.array_buffer()?).await {
            let bytes = Uint8Array::new(&buffer).to_vec();
            // Only cache it if the server honoured the range; a server that
            // sent everything would otherwise fill the cache with one entry.
            if bytes.len() as u64 <= BLOCK {
                source.blocks.insert(0, bytes);
                source.order.push(0);
            }
        }
        Ok(source)
    }

    /// Fetch a block, or return it from the cache.
    async fn block(&mut self, index: u64) -> Result<(), JsValue> {
        if self.blocks.contains_key(&index) {
            self.touch(index);
            return Ok(());
        }

        let start = index * BLOCK;
        let end = (start + BLOCK).min(self.len);
        if start >= self.len {
            return Ok(());
        }

        let response = fetch_range(&self.url, start, end - start).await?;
        let buffer = JsFuture::from(response.array_buffer()?).await?;
        let bytes = Uint8Array::new(&buffer).to_vec();

        self.blocks.insert(index, bytes);
        self.touch(index);
        while self.order.len() > MAX_BLOCKS {
            let oldest = self.order.remove(0);
            self.blocks.remove(&oldest);
        }
        Ok(())
    }

    fn touch(&mut self, index: u64) {
        if let Some(at) = self.order.iter().position(|&i| i == index) {
            self.order.remove(at);
        }
        self.order.push(index);
    }

    /// Fetch every block spanning `offset..offset + len`.
    ///
    /// The synchronous [`TraceSource`] cannot await, so callers prefetch the
    /// range they are about to read.
    pub async fn prefetch(&mut self, offset: u64, len: u64) -> Result<(), JsValue> {
        if offset >= self.len {
            return Ok(());
        }
        let end = (offset + len).min(self.len);
        for index in (offset / BLOCK)..=((end.saturating_sub(1)) / BLOCK) {
            self.block(index).await?;
        }
        Ok(())
    }
}

impl TraceSource for RangeSource {
    fn len(&self) -> u64 {
        self.len
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        if offset >= self.len {
            return Ok(0);
        }
        let mut done = 0usize;
        while done < buf.len() {
            let at = offset + done as u64;
            if at >= self.len {
                break;
            }
            let index = at / BLOCK;
            let Some(block) = self.blocks.get(&index) else {
                // Not prefetched. Reporting a short read is honest; the
                // caller retries after awaiting `prefetch`.
                break;
            };
            let within = (at - index * BLOCK) as usize;
            if within >= block.len() {
                break;
            }
            let n = (buf.len() - done).min(block.len() - within);
            buf[done..done + n].copy_from_slice(&block[within..within + n]);
            done += n;
        }
        Ok(done)
    }
}

/// Issue a ranged GET.
async fn fetch_range(url: &str, start: u64, len: u64) -> Result<Response, JsValue> {
    let opts = RequestInit::new();
    opts.set_method("GET");

    let request = Request::new_with_str_and_init(url, &opts)?;
    request
        .headers()
        .set("Range", &format!("bytes={}-{}", start, start + len - 1))?;

    let global = js_sys::global();
    let promise = if let Ok(window) = Reflect::get(&global, &JsValue::from_str("fetch")) {
        if window.is_function() {
            let f: js_sys::Function = window.unchecked_into();
            f.call1(&global, &request)?.unchecked_into::<js_sys::Promise>()
        } else {
            return Err(JsValue::from_str("fetch is not available here"));
        }
    } else {
        return Err(JsValue::from_str("fetch is not available here"));
    };

    let response = JsFuture::from(promise).await?;
    response.dyn_into::<Response>()
}

/// Total length from a `Content-Range: bytes a-b/total` header.
fn content_range_total(response: &Response) -> Option<u64> {
    let value = response.headers().get("Content-Range").ok()??;
    value.rsplit('/').next()?.trim().parse().ok()
}
