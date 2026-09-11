// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Prints the records of a ZLF file. Used to sanity-check captures.
use std::fs::File;
use std::io::BufReader;

fn main() {
    let path = std::env::args().nth(1).expect("usage: dump_zlf <file>");
    let file = BufReader::new(File::open(&path).expect("open"));
    let mut reader = zniff_rs_core::zlf::ZlfReader::new(file).expect("valid ZLF header");
    let mut n = 0;
    while let Some(r) = reader.next_record().expect("record") {
        println!(
            "{:>3} {:>14} {} {:?} {}",
            n,
            r.timestamp.unix_millis(),
            if r.is_outcome { "TX" } else { "RX" },
            r.api_type,
            hex::encode(&r.payload)
        );
        n += 1;
    }
    println!("{n} records");
}
