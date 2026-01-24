pub const ZLF_VERSION: u8 = 104;

#[repr(u8)]
pub enum ApiType {
    Pti = 0xF5,
    Zniffer = 0xFE,
    Unknown(u8),
}

impl From<u8> for ApiType {
    fn from(value: u8) -> Self {
        match value {
            0xF5 => ApiType::Pti,
            0xFE => ApiType::Zniffer,
            _    => ApiType::Unknown(value),
        }
    }
}
