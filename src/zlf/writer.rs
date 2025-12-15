use std::io::{Seek, Write};
use crate::zlf::types::ZLF_VERSION;
use crate::zlf::reader::ZlfError;

pub struct ZlfWriter<W: Write + Seek> {
    w: W,
}

impl<W: Write + Seek> ZlfWriter<W> {
    /// Construct writer and write the static 2048-byte header.
    pub fn new(mut w: W) -> Result<Self, ZlfError> {
        w.write_all(&[ZLF_VERSION])?;
        Ok(ZlfWriter { w })
    }
}
