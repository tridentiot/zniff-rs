<!--
SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
SPDX-License-Identifier: MIT
-->
# Test traces

`pti-800.zlf` is a 181-frame SmartStart inclusion captured from an
800-series device on a test network: explorer frames, singlecasts, acks
and an S2 key exchange, at 40 and 100 kbps on channels 0 and 1.

## The reference baseline

`pti-800.oracle.txt` is the same trace decoded by the C# Zniffer, and
`pti_fixture.rs` asserts the Rust decoders agree with it on header type,
source node, sequence number, ack flag, CRC, channel, speed, region and
RSSI. It is the check that the port still matches the implementation it
came from.

`ZlfDump` in
[z-wave-tools-core](https://github.com/tridentiot/z-wave-tools-core) is a
headless build of that decoder. To regenerate the baseline, or to make
one for another trace:

```bash
dotnet run --project ZlfDump -- pti-800.zlf --brief \
  | grep '^#' > pti-800.oracle.txt
```

## What is not covered

This capture is PTI only, one region, and mostly singlecast and ack. The
legacy Zniffer dialect, Long Range, multicast and wake-up beams are
exercised by unit tests alone. A capture containing any of those would
be worth adding here, with its baseline.

Only commit a trace from a test network: a capture carries the home ids,
node ids and key exchange of whatever network it was taken from.
