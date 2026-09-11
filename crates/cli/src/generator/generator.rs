// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
use std::fs::File;
use zniff_rs_core::zlf::{
    ApiType,
    ZlfReader,
};
use std::net::{
    TcpListener,
};
use std::io::{Write};

pub struct FrameGenerator {
    file: String,
    delay: u16,
}

impl FrameGenerator {
    pub fn new(file: String, delay: u16) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {file, delay})
    }

    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        //println!("Generate something! {:?}", from_file);
        let file = File::open(self.file)?;
        //let file_length = file.metadata()?.len();
        let mut reader = ZlfReader::new(file)?;

        let listener = TcpListener::bind("0.0.0.0:9000")?;
        println!("Waiting for client...");
        let (mut stream, addr) = listener.accept()?;
        println!("Client connected from {addr}");

        let mut n = 0usize;
        while let Some(rec) = reader.next_record()? {
            n += 1;
            match rec.api_type {
                ApiType::Attachment => {
                    // Attachments carry keys/comments, not frames.
                }
                _ => {
                    println!(
                        "#{:06} {:?} len={}",
                        n,
                        rec.api_type,
                        rec.payload.len()
                    );
                    stream.write_all(&rec.payload)?;
                    std::thread::sleep(std::time::Duration::from_millis(self.delay as u64));
                }
            }
        }
        println!("End of file reached after {} frames", n);
        println!("Total frames read: {}", reader.record_count());
        Ok(())
    }
}
