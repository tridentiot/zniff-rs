// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Appends records to a ZLF file over time, to exercise live tailing without
//! needing radio traffic.
use std::fs::File;
use std::io::BufWriter;

use zniff_rs_core::zlf::{
    ApiType,
    Timestamp,
    ZlfWriter,
};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: grow_zlf <file> [batches] [per_batch]");
    let batches: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(10);
    let per: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(5);

    // Real PTI frames from the committed fixture, so the viewer decodes and
    // displays them the way it would a genuine capture.
    let fixture = include_bytes!("../tests/fixtures/pti-800.zlf");
    let mut reader =
        zniff_rs_core::zlf::ZlfReader::new(std::io::Cursor::new(&fixture[..])).unwrap();
    let mut payloads = Vec::new();
    while let Some(r) = reader.next_record().unwrap() {
        if r.api_type == ApiType::Pti && !r.payload.is_empty() {
            payloads.push(r.payload);
        }
    }

    let mut writer =
        ZlfWriter::new(BufWriter::new(File::create(&path).unwrap()), "growing.zlf").unwrap();
    let mut n = 0usize;
    for batch in 0..batches {
        for _ in 0..per {
            let payload = &payloads[n % payloads.len()];
            let millis = 1_700_000_000_000 + n as i64 * 10;
            writer
                .write_record(
                    Timestamp::from_unix_millis(millis),
                    false,
                    0,
                    ApiType::Pti,
                    payload,
                )
                .unwrap();
            n += 1;
        }
        writer.flush().unwrap();
        println!("batch {} → {} records, {} bytes", batch + 1, n, writer.bytes_written());
        std::thread::sleep(std::time::Duration::from_millis(1500));
    }
    println!("done: {n} records");
}
