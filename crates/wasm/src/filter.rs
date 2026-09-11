// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
//! Filtering of trace frames.
//!
//! Accepts `key:value` terms for structured fields and bare words for a
//! free-text search. All terms must match.
use zniff_rs_core::trace::TraceFrame;

#[derive(Debug, PartialEq)]
enum Term {
    Source(u64),
    Destination(u64),
    /// Either endpoint.
    Node(u64),
    HomeId(String),
    Speed(u8),
    Channel(u8),
    Direction(&'static str),
    CrcOk(bool),
    /// Frames the sender repeated, or only those it did not.
    Retransmission(bool),
    /// Substring of the frame type or application summary.
    Text(String),
}

/// A parsed filter expression.
#[derive(Debug, Default)]
pub struct Filter {
    terms: Vec<Term>,
}

impl Filter {
    /// Parse a query. Unrecognised `key:value` pairs fall back to free text,
    /// so a typo narrows the results rather than silently matching everything.
    pub fn parse(query: &str) -> Self {
        let mut terms = Vec::new();
        for token in query.split_whitespace() {
            let lower = token.to_ascii_lowercase();
            let term = match lower.split_once(':') {
                Some((key, value)) if !value.is_empty() => match key {
                    "src" | "source" => value.parse().ok().map(Term::Source),
                    "dst" | "dest" | "destination" => value.parse().ok().map(Term::Destination),
                    "node" => value.parse().ok().map(Term::Node),
                    "home" | "homeid" => Some(Term::HomeId(value.trim_start_matches("0x").to_string())),
                    "speed" => value.parse().ok().map(Term::Speed),
                    "ch" | "channel" => value.parse().ok().map(Term::Channel),
                    "dir" | "direction" => match value {
                        "rx" => Some(Term::Direction("Rx")),
                        "tx" => Some(Term::Direction("Tx")),
                        _ => None,
                    },
                    "retx" | "retransmission" => match value {
                        "yes" | "true" | "1" => Some(Term::Retransmission(true)),
                        "no" | "false" | "0" => Some(Term::Retransmission(false)),
                        _ => None,
                    },
                    "crc" => match value {
                        "ok" | "true" | "1" => Some(Term::CrcOk(true)),
                        "bad" | "false" | "0" => Some(Term::CrcOk(false)),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            };
            terms.push(term.unwrap_or(Term::Text(lower)));
        }
        Self { terms }
    }

    /// True when every term matches, and for an empty query.
    ///
    /// `retransmission` is how many times the frame had already been sent,
    /// which the caller tracks across the window.
    pub fn matches(&self, frame: &TraceFrame, retransmission: u32) -> bool {
        self.terms.iter().all(|t| matches_term(t, frame, retransmission))
    }
}

fn matches_term(term: &Term, frame: &TraceFrame, retransmission: u32) -> bool {
    match term {
        Term::Retransmission(want) => (retransmission > 0) == *want,
        Term::Source(n) => frame.source() == Some(*n),
        Term::Destination(n) => frame.destination() == Some(*n),
        Term::Node(n) => frame.source() == Some(*n) || frame.destination() == Some(*n),
        Term::HomeId(want) => frame
            .home_id()
            .is_some_and(|h| format!("{h:08x}").contains(want)),
        Term::Speed(s) => frame.speed == *s,
        Term::Channel(c) => frame.channel == *c,
        Term::Direction(d) => {
            let dir = match frame.direction {
                zniff_rs_core::pti::Direction::Rx => "Rx",
                zniff_rs_core::pti::Direction::Tx => "Tx",
            };
            dir == *d
        }
        Term::CrcOk(want) => frame.crc_ok() == *want,
        Term::Text(needle) => {
            frame.header_text().to_ascii_lowercase().contains(needle)
                || frame.application_summary().to_ascii_lowercase().contains(needle)
                || frame
                    .home_id()
                    .is_some_and(|h| format!("{h:08x}").contains(needle))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_structured_terms() {
        let f = Filter::parse("src:1 dst:2 speed:2 dir:tx crc:bad");
        assert_eq!(
            f.terms,
            vec![
                Term::Source(1),
                Term::Destination(2),
                Term::Speed(2),
                Term::Direction("Tx"),
                Term::CrcOk(false),
            ]
        );
    }

    #[test]
    fn parses_the_retransmission_term() {
        assert_eq!(Filter::parse("retx:yes").terms, vec![Term::Retransmission(true)]);
        assert_eq!(Filter::parse("retx:no").terms, vec![Term::Retransmission(false)]);
        // An unrecognised value falls through to free text rather than
        // silently matching everything.
        assert_eq!(
            Filter::parse("retx:maybe").terms,
            vec![Term::Text("retx:maybe".into())]
        );
    }

    #[test]
    fn treats_bare_words_as_text() {
        let f = Filter::parse("singlecast");
        assert_eq!(f.terms, vec![Term::Text("singlecast".into())]);
    }

    #[test]
    fn an_unknown_key_narrows_rather_than_matching_everything() {
        let f = Filter::parse("bogus:1");
        assert_eq!(f.terms, vec![Term::Text("bogus:1".into())]);
    }

    #[test]
    fn an_empty_query_has_no_terms() {
        assert!(Filter::parse("   ").terms.is_empty());
    }
}
