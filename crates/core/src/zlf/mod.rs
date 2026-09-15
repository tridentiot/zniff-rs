// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
pub mod types;
pub use types::ApiType;

mod reader;
mod writer;
pub use writer::ZlfWriter;
pub use reader::{
    Timestamp,
    ZLF_HEADER_SIZE,
    ZlfError,
    ZlfReader,
    ZlfRecord,
};
