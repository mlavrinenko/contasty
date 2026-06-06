//! Throwaway strip runner: `cargo run --example strip -- <file>`.
//!
//! Strips a file with every category dropped (tests, comments, imports) at the
//! default thresholds and prints the result. Used while authoring rule files to
//! eyeball output and pipe it back through `dump-ast` for an ERROR-node check.

use std::path::Path;

use contasty::Registry;
use contasty::config::CompactConfig;

fn main() {
    let mut args = std::env::args().skip(1);
    let file = args.next().expect("usage: strip <file> [max_string_bytes]");
    let max_string = args.next().and_then(|n| n.parse().ok());
    let path = Path::new(&file);
    let src = std::fs::read_to_string(path).expect("read file");
    let reg = Registry::new().expect("registry init");
    let lang = reg
        .detect(path)
        .expect("language not registered for extension");
    let compact = CompactConfig {
        max_string_bytes: max_string.unwrap_or(CompactConfig::default().max_string_bytes),
        ..CompactConfig::default()
    };
    let out = lang
        .strip(&src, path, true, true, true, true, &compact)
        .expect("strip");
    print!("{out}");
}
