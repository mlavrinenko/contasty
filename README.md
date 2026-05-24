# contasty

[![CI](https://github.com/speconaut/contasty/actions/workflows/ci.yml/badge.svg)](https://github.com/speconaut/contasty/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/contasty.svg)](https://crates.io/crates/contasty)
[![License: MIT](https://img.shields.io/crates/l/contasty.svg)](LICENSE-MIT)

Strips all executable likes from your code to prepare tasty context for your agent.

## Install

### From crates.io

```bash
cargo install contasty
```

### From binary releases

Download a pre-built binary from the
[latest release](https://github.com/speconaut/contasty/releases/latest).

## Usage

```bash
contasty
```

## Development

Prerequisites: [Nix](https://nixos.org/) with flakes enabled.

```bash
direnv allow   # or: nix develop

just check     # fmt + clippy + tests + file-size check
just build
just test
just cover     # code coverage (70% minimum)
just fmt       # format code
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for coding conventions.

## License

MIT
