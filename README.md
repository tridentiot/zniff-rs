# zniff-rs

zniff-rs (pronounced "sniffers" or "sniff-r-s") is a Z-Wave sniffer written in Rust.

zniff-rs was initially created as an excuse to learn the Rust programming language,
and the project is still in its early days, but already with some basics working
(although a little unassembled and buggy for now 🙈):

- Reading from a [Trident IoT](https://github.com/tridentiot/) Z-Wave [zniffer device](https://github.com/tridentiot/z-wave-zniffer-specs/pull/1)
- Reading from a ZLF file
- Parsing of Z-Wave frames
- Run as a PTI server (`zniff-rs-cli server`) with the Z-Wave (PC) Zniffer as a client
- Terminal User Interface (TUI)
- Browser-based trace viewer (see [`web/`](web/))

[Future functionality](https://github.com/tridentiot/zniff-rs/issues):
- Decryption of S0 and S2 encrypted frames

# Usage

## CLI
```bash
zniff-rs-cli --help
```

## TUI
```bash
zniff-rs-tui
```

## Web
Reads a ZLF trace in the browser, for going through CI test runs and
analyzing frame traces. See [`web/README.md`](web/README.md).

```bash
cd web && npm install && npm run dev
```


# Development

## Prerequisites:
On Linux:
```bash
sudo apt install libudev-dev
```

## Build
```bash
cargo build
```

```bash
cargo run -p <zniff-rs-cli | zniff-rs-tui>
```
