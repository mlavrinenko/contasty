//! Regenerate the committed rule-file JSON Schema.
//!
//! Run via `just gen-schema`. The companion `schema_in_sync` test fails if the
//! committed `schemas/contasty-rules.schema.json` diverges from the types.

use std::path::Path;

fn main() -> std::io::Result<()> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/contasty-rules.schema.json");
    std::fs::write(&path, contasty::rules_schema_json())?;
    println!("wrote {}", path.display());
    Ok(())
}
