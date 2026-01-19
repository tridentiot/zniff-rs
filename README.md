# zniff-rs

zniff-rs (pronounced "sniffers" or "sniff-r-s") is a Z-Wave sniffer written in Rust.

zniff-rs was initially created as an excuse to learn the Rust programming language,
and the project is still in its early days, but already with some basics working
(although a little unassembled and buggy for now 🙈):

- Reading from a [Trident IoT](https://github.com/tridentiot/) Z-Wave [zniffer device](https://github.com/tridentiot/z-wave-zniffer-specs/pull/1)
- Reading from a ZLF file
- [Parsing of Z-Wave frames](https://github.com/tridentiot/zniff-rs/pull/18)
- Run as a PTI server (`zniff-rs run`) with the Z-Wave (PC) Zniffer as a client

[Future functionality](https://github.com/tridentiot/zniff-rs/issues):
- Decryption of S0 and S2 encrypted frames
- GUI

# Usage

```bash
zniff-rs --help
```

# Development

```bash
cargo build
```

```bash
cargo run -- --help
```
