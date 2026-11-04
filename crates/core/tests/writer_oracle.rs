// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Checks the writer against a capture produced by the C# zniffer.

use std::io::Cursor;

use zniff_rs_core::zlf::{
    ZLF_HEADER_SIZE,
    ZlfReader,
    ZlfWriter,
};

const FIXTURE: &[u8] = include_bytes!("fixtures/pti-800.zlf");

/// The version, encoding and CRC a real C# header carries must match ours.
#[test]
fn header_matches_a_csharp_capture() {
    let mut ours = Vec::new();
    ZlfWriter::new(&mut ours, "").unwrap();
    let theirs = &FIXTURE[..ZLF_HEADER_SIZE];

    assert_eq!(&ours[..8], &theirs[..8], "version and text encoding");
    assert_eq!(
        &ours[ZLF_HEADER_SIZE - 2..],
        &theirs[ZLF_HEADER_SIZE - 2..],
        "CRC over an otherwise-identical header",
    );
}

/// Re-writing every record of a real capture must reproduce the file exactly.
#[test]
fn rewriting_a_real_capture_is_byte_identical() {
    let comment = {
        // Recover the original comment so the header we write matches.
        let raw = &FIXTURE[8..8 + 512];
        let units: Vec<u16> =
            raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
        String::from_utf16(&units[..end]).unwrap()
    };

    let mut out = Vec::new();
    let mut writer = ZlfWriter::new(&mut out, &comment).unwrap();
    let mut reader = ZlfReader::new(Cursor::new(FIXTURE)).unwrap();
    while let Some(record) = reader.next_record().unwrap() {
        writer
            .write_record(
                record.timestamp,
                record.is_outcome,
                record.session_id,
                record.api_type,
                &record.payload,
            )
            .unwrap();
    }
    writer.flush().unwrap();

    assert_eq!(out.len(), FIXTURE.len(), "file length");
    assert!(out == FIXTURE, "re-written capture differs from the original");
}
