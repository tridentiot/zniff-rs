// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Walks a [`FrameDefinition`] over MPDU bytes to produce a field tree.
use serde::{
    Deserialize,
    Serialize,
};

use super::schema::{
    FrameDefinition,
    Param,
    ParamType,
};

/// Where a field lives within the MPDU, to the bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct BitSpan {
    /// Offset of the first bit from the start of the MPDU.
    pub start_bit: usize,
    /// Width of the field in bits.
    pub bits: usize,
}

impl BitSpan {
    /// Byte range covering this field, for highlighting in a hex view.
    pub fn byte_range(&self) -> std::ops::Range<usize> {
        let first = self.start_bit / 8;
        let last = (self.start_bit + self.bits).div_ceil(8);
        first..last.max(first + 1)
    }

    /// True when the field occupies whole bytes.
    pub fn is_byte_aligned(&self) -> bool {
        self.start_bit % 8 == 0 && self.bits % 8 == 0
    }
}

/// A decoded field, with its value already resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodedParam {
    /// Field name, e.g. `HeaderType`.
    pub name: String,
    /// Human-readable label, e.g. `Header Type`.
    pub text: String,
    pub span: BitSpan,
    /// Numeric value, for fields narrow enough to hold one.
    pub value: Option<u64>,
    /// Raw bytes, for fields wider than 64 bits.
    pub bytes: Vec<u8>,
    /// Symbolic name from the field's `DefineSet`, when one applies.
    pub symbol: Option<String>,
    pub param_type: ParamType,
    /// Bit fields packed inside this one.
    pub children: Vec<DecodedParam>,
}

impl DecodedParam {
    /// Formats the value the way the Zniffer UI shows it.
    pub fn display_value(&self) -> String {
        if let Some(symbol) = &self.symbol {
            return symbol.clone();
        }
        match self.param_type {
            ParamType::Boolean => match self.value {
                Some(0) => "false".to_string(),
                Some(_) => "true".to_string(),
                None => String::new(),
            },
            ParamType::Number | ParamType::NodeNumber => {
                self.value.map(|v| v.to_string()).unwrap_or_default()
            }
            ParamType::NumberSigned => self
                .value
                .map(|v| (v as i8).to_string())
                .unwrap_or_default(),
            ParamType::Hex | ParamType::Bitmask => {
                if self.bytes.is_empty() {
                    self.value.map(|v| format!("0x{v:02X}")).unwrap_or_default()
                } else {
                    hex::encode_upper(&self.bytes)
                }
            }
        }
    }
}

/// A fully decoded MPDU header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodedHeader {
    /// Key of the matched `Header`, e.g. 13 for singlecast.
    pub header_key: u8,
    /// Name of the matched header, e.g. `SINGLECAST`.
    pub header_name: String,
    /// Human-readable header name, e.g. `Singlecast`.
    pub header_text: String,
    pub is_ack: bool,
    pub is_routed: bool,
    pub is_multicast: bool,
    pub is_error: bool,
    /// Base header fields followed by the header-specific fields.
    pub params: Vec<DecodedParam>,
    /// Where the application payload starts, in bytes from the MPDU start.
    pub payload_offset: usize,
    /// True when the trailing checksum/CRC verified.
    pub crc_ok: bool,
}

impl DecodedHeader {
    /// Look up a decoded field by its dotted path, e.g. `Properties1.HeaderType`.
    pub fn find(&self, path: &str) -> Option<&DecodedParam> {
        find_param(&self.params, path)
    }

    /// Value of a field by dotted path.
    pub fn value(&self, path: &str) -> Option<u64> {
        self.find(path).and_then(|p| p.value)
    }
}

fn find_param<'a>(params: &'a [DecodedParam], path: &str) -> Option<&'a DecodedParam> {
    let (head, rest) = match path.split_once('.') {
        Some((h, r)) => (h, Some(r)),
        None => (path, None),
    };
    let found = params.iter().find(|p| p.name == head)?;
    match rest {
        None => Some(found),
        Some(rest) => find_param(&found.children, rest),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("frame is too short to decode")]
    TooShort,
    #[error("no base header with key {0}")]
    UnknownBaseHeader(u8),
    #[error("no header matched the frame")]
    NoMatchingHeader,
}

/// Reads `bits` bits starting at `start_bit`, most significant bit first.
fn read_bits(data: &[u8], start_bit: usize, bits: usize) -> Option<u64> {
    if bits == 0 || bits > 64 || start_bit + bits > data.len() * 8 {
        return None;
    }
    let mut value = 0u64;
    for i in 0..bits {
        let bit = start_bit + i;
        let byte = data[bit / 8];
        // Bits are numbered from the most significant end of each byte.
        let set = (byte >> (7 - (bit % 8))) & 1;
        value = (value << 1) | set as u64;
    }
    Some(value)
}

/// Decodes one `Param` and any bit fields nested inside it.
fn decode_param(
    fd: &FrameDefinition,
    data: &[u8],
    param: &Param,
    start_bit: usize,
    parent: &[DecodedParam],
) -> Option<DecodedParam> {
    // An optional field is present only when the flag it references is set.
    if let Some(opt) = &param.opt_ref
        && find_param(parent, opt).and_then(|p| p.value).unwrap_or(0) == 0
    {
        return None;
    }

    // A sized field takes its length in bytes from another field.
    let bits = match &param.size_ref {
        Some(size_ref) => {
            let n = find_param(parent, size_ref).and_then(|p| p.value).unwrap_or(0);
            (n as usize) * 8
        }
        None => param.bits as usize,
    };
    if bits == 0 || start_bit + bits > data.len() * 8 {
        return None;
    }

    let span = BitSpan { start_bit, bits };
    let value = read_bits(data, start_bit, bits);
    let bytes = if bits > 64 && span.is_byte_aligned() {
        data[span.byte_range()].to_vec()
    } else {
        Vec::new()
    };

    // Resolve the symbolic name, when the field declares a DefineSet.
    let symbol = param
        .defines
        .as_deref()
        .and_then(|set| fd.define_set(set))
        .and_then(|set| {
            let v = value?;
            set.defines.iter().find(|d| d.key as u64 == v).map(|d| d.text.clone())
        });

    let mut decoded = DecodedParam {
        name: param.name.clone(),
        text: param.text.clone(),
        span,
        value,
        bytes,
        symbol,
        param_type: param.param_type,
        children: Vec::new(),
    };

    // Bit fields are laid out from the least significant end upwards, so the
    // first child sits at the bottom of the parent field.
    let mut children = Vec::new();
    let mut offset = bits;
    let mut sorted: Vec<&Param> = param.params.iter().collect();
    sorted.sort_by_key(|p| p.order);
    for child in sorted {
        let child_bits = child.bits as usize;
        if child_bits > offset {
            break;
        }
        offset -= child_bits;
        if let Some(c) = decode_param(fd, data, child, start_bit + offset, &children) {
            children.push(c);
        }
    }
    decoded.children = children;
    Some(decoded)
}

/// Decode the MPDU header of a Z-Wave frame.
///
/// `region` selects the base header layout, and `speed` selects Long Range
/// (speed 3) and the checksum width. Mirrors `ParseHeaderWithCrc` in
/// z-wave-tools-core.
pub fn decode_header(
    fd: &FrameDefinition,
    mpdu: &[u8],
    region: u8,
    speed: u8,
) -> Result<DecodedHeader, DecodeError> {
    if mpdu.len() < 2 {
        return Err(DecodeError::TooShort);
    }

    let base_key = if speed == 3 { 2 } else { fd.base_header_key_for_region(region) };
    let base = fd.base_header(base_key).ok_or(DecodeError::UnknownBaseHeader(base_key))?;

    // Long Range and 100k use a 2-byte CRC; the slower rates use a 1-byte checksum.
    let crc_bytes = if speed > 1 { 2 } else { 1 };
    let crc_ok = check_crc(mpdu, crc_bytes);

    let mut params: Vec<DecodedParam> = Vec::new();
    let mut bit = 0usize;
    let mut sorted: Vec<&Param> = base.params.iter().collect();
    sorted.sort_by_key(|p| p.order);
    for param in sorted {
        let Some(decoded) = decode_param(fd, mpdu, param, bit, &params) else {
            break;
        };
        bit += decoded.span.bits;
        params.push(decoded);
    }

    // Validations may reference fields defined by the candidate header itself
    // (explorer frames key off Properties3.ExploreCommandType), so each
    // candidate is decoded speculatively and then tested.
    let base_params = params;
    let mut matched = None;
    for header in fd
        .headers
        .iter()
        .filter(|h| h.base_header_key == Some(base_key) && !h.validations.is_empty())
    {
        let mut params = base_params.clone();
        let mut bit = bit;
        let mut sorted: Vec<&Param> = header.params.iter().collect();
        sorted.sort_by_key(|p| p.order);
        for param in sorted {
            let Some(decoded) = decode_param(fd, mpdu, param, bit, &params) else {
                break;
            };
            bit += decoded.span.bits;
            params.push(decoded);
        }

        if header.validations.iter().all(|v| {
            find_param(&params, &v.param_name)
                .and_then(|p| p.value)
                .is_some_and(|value| value == v.param_hex_value as u64)
        }) {
            matched = Some((header, params, bit));
            break;
        }
    }

    let (header, params, bit) = matched.ok_or(DecodeError::NoMatchingHeader)?;

    Ok(DecodedHeader {
        header_key: header.key,
        header_name: header.name.clone(),
        header_text: header.text.clone(),
        is_ack: header.is_ack,
        is_routed: header.is_routed,
        is_multicast: header.is_multicast,
        is_error: header.is_error,
        params,
        payload_offset: bit.div_ceil(8),
        crc_ok,
    })
}

/// Verify the frame's trailing checksum.
///
/// 9.6k and 40k frames use an XOR checksum seeded with 0xFF; 100k and Long
/// Range frames use CRC-CCITT seeded with 0x1D0F.
fn check_crc(mpdu: &[u8], crc_bytes: usize) -> bool {
    if mpdu.len() <= crc_bytes {
        return false;
    }
    let (body, trailer) = mpdu.split_at(mpdu.len() - crc_bytes);
    match crc_bytes {
        1 => {
            let checksum = body.iter().fold(0xFFu8, |acc, b| acc ^ b);
            checksum == trailer[0]
        }
        2 => {
            let mut crc = 0x1D0Fu16;
            for &b in body {
                crc ^= (b as u16) << 8;
                for _ in 0..8 {
                    crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
                }
            }
            crc == u16::from_be_bytes([trailer[0], trailer[1]])
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_bits_msb_first() {
        let data = [0b1010_0000, 0b0000_1111];
        assert_eq!(read_bits(&data, 0, 4), Some(0b1010));
        assert_eq!(read_bits(&data, 0, 8), Some(0xA0));
        assert_eq!(read_bits(&data, 12, 4), Some(0x0F));
        assert_eq!(read_bits(&data, 0, 16), Some(0xA00F));
        // Past the end of the buffer.
        assert_eq!(read_bits(&data, 12, 8), None);
    }

    #[test]
    fn byte_range_covers_partial_bytes() {
        assert_eq!(BitSpan { start_bit: 0, bits: 4 }.byte_range(), 0..1);
        assert_eq!(BitSpan { start_bit: 4, bits: 4 }.byte_range(), 0..1);
        assert_eq!(BitSpan { start_bit: 0, bits: 32 }.byte_range(), 0..4);
        assert_eq!(BitSpan { start_bit: 8, bits: 12 }.byte_range(), 1..3);
    }

    /// A real 100k singlecast from the fixture: home id E7A4AC25, src 1.
    const SINGLECAST: &[u8] = &[
        0xE7, 0xA4, 0xAC, 0x25, 0x01, 0x41, 0x03, 0x0A, 0x02, 0x00, 0x00, 0x25, 0x02,
    ];

    #[test]
    fn decodes_a_singlecast() {
        let fd = FrameDefinition::load().unwrap();
        let h = decode_header(&fd, SINGLECAST, 1, 2).expect("decodes");

        assert_eq!(h.header_name, "SINGLECAST");
        assert!(!h.is_ack);
        assert!(!h.is_routed);
        assert_eq!(h.value("SourceNodeID"), Some(1));
        assert_eq!(h.value("Properties1.HeaderType"), Some(1));
        assert_eq!(h.value("DestinationNodeID"), Some(2));

        // HomeID is 32 bits, so it decodes as a single value.
        let home = h.find("HomeID").unwrap();
        assert_eq!(home.value, Some(0xE7A4_AC25));
        assert_eq!(home.span.byte_range(), 0..4);
    }

    #[test]
    fn rejects_short_frames() {
        let fd = FrameDefinition::load().unwrap();
        assert!(matches!(decode_header(&fd, &[0x01], 1, 2), Err(DecodeError::TooShort)));
    }
}
