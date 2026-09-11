// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Random access to the bytes of a trace.
//!
//! A trace may be far larger than the memory available to decode it, so the
//! reader never takes the whole file: it asks for the ranges it needs. The
//! same interface serves an in-memory buffer, a local file and ranged HTTP.
use std::io;

/// A source of trace bytes that can be read at any offset.
pub trait TraceSource {
    /// Total length of the trace in bytes.
    fn len(&self) -> u64;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Read into `buf` starting at `offset`, returning how many bytes were
    /// read. A short read means end of file, not an error.
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize>;

    /// Read exactly `buf.len()` bytes, or fail.
    fn read_exact_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<()> {
        let mut done = 0;
        while done < buf.len() {
            let n = self.read_at(offset + done as u64, &mut buf[done..])?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "read past the end of the trace",
                ));
            }
            done += n;
        }
        Ok(())
    }
}

/// A trace already held in memory.
pub struct SliceSource<'a> {
    bytes: &'a [u8],
}

impl<'a> SliceSource<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// The underlying bytes, for callers that can borrow rather than copy.
    pub fn as_slice(&self) -> &'a [u8] {
        self.bytes
    }
}

impl TraceSource for SliceSource<'_> {
    fn len(&self) -> u64 {
        self.bytes.len() as u64
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        let Ok(start) = usize::try_from(offset) else {
            return Ok(0);
        };
        if start >= self.bytes.len() {
            return Ok(0);
        }
        let n = buf.len().min(self.bytes.len() - start);
        buf[..n].copy_from_slice(&self.bytes[start..start + n]);
        Ok(n)
    }
}

#[cfg(feature = "std-file")]
mod file {
    use std::fs::File;
    use std::io::{
        self,
        Read,
        Seek,
        SeekFrom,
    };

    use super::TraceSource;

    /// A trace read from a local file, seeking rather than loading it all.
    pub struct FileSource {
        file: File,
        len: u64,
    }

    impl FileSource {
        pub fn open(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
            let file = File::open(path)?;
            let len = file.metadata()?.len();
            Ok(Self { file, len })
        }
    }

    impl TraceSource for FileSource {
        fn len(&self) -> u64 {
            self.len
        }

        fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
            self.file.seek(SeekFrom::Start(offset))?;
            self.file.read(buf)
        }
    }
}

#[cfg(feature = "std-file")]
pub use file::FileSource;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_windows_of_a_slice() {
        let data: Vec<u8> = (0..=255u8).collect();
        let mut src = SliceSource::new(&data);
        assert_eq!(src.len(), 256);

        let mut buf = [0u8; 4];
        assert_eq!(src.read_at(0, &mut buf).unwrap(), 4);
        assert_eq!(buf, [0, 1, 2, 3]);

        assert_eq!(src.read_at(252, &mut buf).unwrap(), 4);
        assert_eq!(buf, [252, 253, 254, 255]);
    }

    #[test]
    fn a_short_read_at_the_end_is_not_an_error() {
        let data = [1u8, 2, 3];
        let mut src = SliceSource::new(&data);
        let mut buf = [0u8; 8];
        assert_eq!(src.read_at(1, &mut buf).unwrap(), 2);
        assert_eq!(src.read_at(3, &mut buf).unwrap(), 0);
        assert_eq!(src.read_at(99, &mut buf).unwrap(), 0);
    }

    #[test]
    fn read_exact_at_fails_past_the_end() {
        let data = [1u8, 2, 3];
        let mut src = SliceSource::new(&data);
        let mut buf = [0u8; 4];
        assert!(src.read_exact_at(0, &mut buf).is_err());
        assert!(src.read_exact_at(0, &mut buf[..3]).is_ok());
    }
}
