<!--
SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
SPDX-License-Identifier: MIT
-->
# Test traces

The tests that read a real capture are behind the `fixtures` feature, and
the traces themselves are not in the repository: a capture carries the
home ids, node ids and security key exchange of whatever network it was
taken from, which is not something to publish with the source.

To run them, put a trace here and enable the feature:

```bash
cp your-capture.zlf crates/core/tests/fixtures/pti-800.zlf
cargo test -p zniff-rs-core --features fixtures
```

## The reference baseline

`pti_fixture.rs` compares the decoder against the C# Zniffer, so it also
needs that tool's output for the same trace. `ZlfDump` in
[z-wave-tools-core](https://github.com/tridentiot/z-wave-tools-core) is a
headless build of the reference decoder:

```bash
dotnet run --project ZlfDump -- your-capture.zlf --brief \
  | grep '^#' > crates/core/tests/fixtures/pti-800.oracle.txt
```

The comparison covers header type, source node, sequence number, ack flag,
CRC, channel, speed, region and RSSI. It is the check that the Rust
decoders agree with the implementation they were ported from, so it is
worth running against any capture that exercises a frame type the unit
tests do not: Long Range, multicast, beams, or the legacy Zniffer dialect.
