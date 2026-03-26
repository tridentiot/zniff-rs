// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
pub const ZLF_VERSION: u32 = 104;

#[repr(u8)]
pub enum ApiType {
    Pti = 0xF5,
    Attachment = 0xF8,
    Zniffer = 0xFE,
}

impl From<u8> for ApiType {
    fn from(value: u8) -> Self {
        match value {
            0xF5 => ApiType::Pti,
            0xF8 => ApiType::Attachment,
            0xFE => ApiType::Zniffer,
            _ => panic!("Unknown API type: 0x{:02X}", value),
        }
    }
}
