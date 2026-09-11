// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! XML-driven decoding of Z-Wave MPDU headers.
mod decode;
mod schema;

pub use decode::{
    BitSpan,
    DecodeError,
    DecodedHeader,
    DecodedParam,
    decode_header,
};
pub use schema::{
    BaseHeader,
    Define,
    DefineSet,
    FrameDefinition,
    Header,
    Param,
    ParamType,
    Validation,
};
