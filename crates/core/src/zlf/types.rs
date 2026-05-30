// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
pub const ZLF_VERSION: u32 = 104;

#[repr(u8)]
pub enum ApiType {
    Pti = 0xF5,
    Attachment = 0xF8,
    Zniffer = 0xFE,
}

impl TryFrom<u8> for ApiType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0xF5 => Ok(ApiType::Pti),
            0xF8 => Ok(ApiType::Attachment),
            0xFE => Ok(ApiType::Zniffer),
            _ => Err(()),
        }
    }
}
