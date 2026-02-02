// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT

//! zniff-rs - Z-Wave sniffer library and tools
//! 
//! This crate provides Z-Wave packet sniffing, parsing, and analysis capabilities.
//! 
//! ## Features
//! 
//! - **PTI Parser**: Parse Silabs PTI (Packet Trace Interface) frames
//! - **Zniffer Protocol**: Parse frames from Trident IoT Z-Wave sniffer devices
//! - **ZLF Files**: Read and write Z-Wave Log Format files
//! - **Z-Wave Parsing**: Decode Z-Wave protocol frames
//! 
//! ## Example: Parsing PTI frames
//! 
//! ```no_run
//! use zniff_rs::pti_parser::PtiParser;
//! 
//! let mut parser = PtiParser::new();
//! 
//! // Simulate receiving data from a TCP stream
//! let data = vec![/* PTI/DCH frame bytes */];
//! let frames = parser.parse(&data);
//! 
//! for frame in frames {
//!     println!("Received frame on channel {} with RSSI {}", 
//!              frame.channel, frame.rssi);
//! }
//! ```

pub mod types;
pub mod pti_parser;
pub mod zniffer_parser;

// Re-export commonly used types
pub use types::{Frame, Region};
