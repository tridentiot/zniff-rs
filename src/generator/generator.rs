use std::fs::File;
use crate::zlf::{
    ZlfReader,
    ZlfRecord,
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
        while let Some(rec) = reader.next()? {
            n += 1;
            match rec {
                ZlfRecord::Data(df) => {
                    println!(
                        "#{:06} DATA ts={} ch/s={} region={} rssi={} mpdu_len={}",
                        n, df.timestamp, df.ch_and_speed, df.region, df.rssi, df.mpdu.len()
                    );
                    // … further MPDU parsing here …
                }
                ZlfRecord::Other(raw) => {
                    println!(
                        "#{:06} {:?} sof='{}' len={}",
                        n, raw.typ, raw.sof as char, raw.payload.len()
                    );
                    stream.write_all(&raw.payload)?;
                    std::thread::sleep(std::time::Duration::from_millis(self.delay as u64));
                }
            }
        }
        Ok(())
    }
}
