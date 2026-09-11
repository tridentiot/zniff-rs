// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Frame colouring.
//!
//! Keeps the desktop Zniffer's structure — text colour by frame type, row
//! background by bit rate, a failed checksum overriding the frame type
//! (`ZWaveZniffer/Xml/ColorSettings.xml`, `CellConverters.cs`) — but replaces
//! the .NET named colours with a palette drawn from the Trident IoT brand.
//!
//! The hues sit in the brand's cool blue family wherever the frame types
//! allow, anchored on the navy #15244F and blue #2EA3F2, with the CRC-error
//! red derived from the brand's #CF2E2E.
//!
//! Each colour is tuned to clear 4.5:1 against the *darkest* row background,
//! which is the selected row on the Long Range tint, so it stays legible on
//! every row: base, alternating stripe and selection. The .NET palette this
//! replaced dropped as low as 2.15:1 for explorer frames.
use zniff_rs_core::trace::TraceFrame;

/// A rule from `FrameForeColors`: header types sharing one colour.
struct ForeRule {
    /// Header keys this colour applies to; empty means any.
    headers: &'static [u8],
    color: &'static str,
}

/// Text colours by frame type, keeping the desktop Zniffer's groupings.
const FORE_RULES: &[ForeRule] = &[
    // Ack: TRANSFER_ACKNOWLEDGE, 24, LR. The brand blue.
    ForeRule { headers: &[19, 20, 21, 71], color: "#0A629D" },
    // Routed ack.
    ForeRule { headers: &[25, 26, 27], color: "#1E5EA8" },
    // Broadcast, including LR. Closest to the brand navy.
    ForeRule { headers: &[10, 11, 12, 72], color: "#3257BD" },
    // Singlecast, including LR.
    ForeRule { headers: &[13, 14, 15, 70], color: "#136A58" },
    // Routed singlecast.
    ForeRule { headers: &[22, 23, 24], color: "#0D6779" },
    // Routed error.
    ForeRule { headers: &[28, 29, 30], color: "#A04028" },
    // Multicast.
    ForeRule { headers: &[16, 17, 18], color: "#7C3EBB" },
    // Flooded.
    ForeRule { headers: &[31, 32], color: "#555F6D" },
    // Explorer, all variants.
    ForeRule { headers: &[33, 34, 35, 36, 37, 38], color: "#86530B" },
];

/// Text colour for a failed checksum. Checked first, so it wins outright.
/// This is the brand's own red.
const CRC_ERROR: &str = "#B32727";

/// Text colour for a wake-up beam, shared with multicast.
const WAKE_UP_BEAM: &str = "#7C3EBB";

/// Fallback when no rule matches.
const DEFAULT_FORE: &str = "#15244F"; // Brand navy

/// How far a row background is mixed towards [`ROW_SHADE`] for the
/// alternating stripe and for the selected row. The UI applies these; they
/// live here so the contrast test covers the colours actually rendered.
pub const STRIPE_MIX: f64 = 0.06;
pub const SELECTED_MIX: f64 = 0.12;

/// The tint mixed into a row background to darken it.
pub const ROW_SHADE: &str = "#15244F";

/// Row backgrounds by bit rate: 9.6k, 40k, 100k, Long Range. Tinted towards
/// the brand blues, and kept close enough in luminance that the frame
/// colours stay legible on all of them.
const BACK_BY_SPEED: [&str; 4] = [
    "#FFFFFF", // 9.6 kbps
    "#F4F6FA", // 40 kbps
    "#EDF4FB", // 100 kbps
    "#EAF6F3", // Long Range
];

/// Text and background colours for a frame, as `(foreground, background)`.
pub fn frame_colors(frame: &TraceFrame) -> (&'static str, &'static str) {
    let background = BACK_BY_SPEED.get(frame.speed as usize).copied().unwrap_or("#FFFFFF");

    if !frame.crc_ok() {
        return (CRC_ERROR, background);
    }

    let Some(header) = frame.header.as_ref() else {
        return (DEFAULT_FORE, background);
    };

    // Beams are identified by their header type rather than a frame counter.
    if matches!(header.header_key, 60..=63) {
        return (WAKE_UP_BEAM, background);
    }

    let foreground = FORE_RULES
        .iter()
        .find(|rule| rule.headers.contains(&header.header_key))
        .map_or(DEFAULT_FORE, |rule| rule.color);

    (foreground, background)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_header_key_maps_to_one_rule() {
        // A header type in two rules would make the colour order-dependent.
        for rule in FORE_RULES {
            for key in rule.headers {
                let hits = FORE_RULES.iter().filter(|r| r.headers.contains(key)).count();
                assert_eq!(hits, 1, "header {key} appears in {hits} rules");
            }
        }
    }

    #[test]
    fn backgrounds_cover_every_speed() {
        assert_eq!(BACK_BY_SPEED.len(), 4);
        // 9.6 kbps is the plain background.
        assert_eq!(BACK_BY_SPEED[0], "#FFFFFF");
    }

    fn channel(hex: &str, i: usize) -> f64 {
        u8::from_str_radix(&hex[1 + i * 2..3 + i * 2], 16).unwrap() as f64 / 255.0
    }

    /// Relative luminance, per WCAG 2.1.
    fn luminance(hex: &str) -> f64 {
        let f = |c: f64| if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) };
        0.2126 * f(channel(hex, 0)) + 0.7152 * f(channel(hex, 1)) + 0.0722 * f(channel(hex, 2))
    }

    fn contrast(a: &str, b: &str) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// Mix `a` towards `b` by `amount`, as the stylesheet's color-mix does.
    fn blend(a: &str, b: &str, amount: f64) -> String {
        let ch = |i| {
            let v = channel(a, i) * (1.0 - amount) + channel(b, i) * amount;
            (v * 255.0).round() as u8
        };
        format!("#{:02X}{:02X}{:02X}", ch(0), ch(1), ch(2))
    }

    #[test]
    fn every_frame_colour_is_readable_on_every_row() {
        // Covers all three tiers a row can have: the plain bit-rate
        // background, the alternating stripe and the selected row. The
        // palette this replaced fell to 2.15:1 for explorer frames.
        let foregrounds: Vec<&str> = FORE_RULES
            .iter()
            .map(|r| r.color)
            .chain([CRC_ERROR, WAKE_UP_BEAM, DEFAULT_FORE])
            .collect();

        for bg in BACK_BY_SPEED {
            let rows = [
                ("base", bg.to_string()),
                ("stripe", blend(bg, ROW_SHADE, STRIPE_MIX)),
                ("selected", blend(bg, ROW_SHADE, SELECTED_MIX)),
            ];
            for (tier, row) in &rows {
                for fg in &foregrounds {
                    let c = contrast(fg, row);
                    assert!(
                        c >= 4.5,
                        "{fg} on the {tier} row over {bg} ({row}) is only {c:.2}:1"
                    );
                }
            }
        }
    }

    #[test]
    fn row_tiers_are_visibly_different() {
        for bg in BACK_BY_SPEED {
            let stripe = blend(bg, ROW_SHADE, STRIPE_MIX);
            let selected = blend(bg, ROW_SHADE, SELECTED_MIX);
            assert!(luminance(bg) > luminance(&stripe), "stripe is not darker than {bg}");
            assert!(
                luminance(&stripe) > luminance(&selected),
                "selection is not darker than the stripe over {bg}"
            );
        }
    }

    #[test]
    fn frame_colours_are_distinguishable() {
        // Two frame types sharing a colour would defeat the point. Multicast
        // and the wake-up beam intentionally match.
        let mut colours: Vec<&str> = FORE_RULES.iter().map(|r| r.color).collect();
        colours.push(CRC_ERROR);
        let count = colours.len();
        colours.sort_unstable();
        colours.dedup();
        assert_eq!(colours.len(), count, "two frame types share a colour");
    }

    #[test]
    fn backgrounds_stay_light() {
        // A dark row background would break the frame colours.
        for bg in BACK_BY_SPEED {
            assert!(luminance(bg) > 0.8, "{bg} is too dark for the frame palette");
        }
    }

    #[test]
    fn colours_are_six_digit_hex() {
        let all = FORE_RULES
            .iter()
            .map(|r| r.color)
            .chain([CRC_ERROR, WAKE_UP_BEAM, DEFAULT_FORE])
            .chain(BACK_BY_SPEED);
        for c in all {
            assert!(
                c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|d| d.is_ascii_hexdigit()),
                "malformed colour {c}"
            );
        }
    }
}
