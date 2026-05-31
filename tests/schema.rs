//! Drift guard: the committed JSON Schema must match what the types generate.

use std::path::Path;

#[test]
fn schema_in_sync() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/contasty-rules.schema.json");
    let committed = std::fs::read_to_string(&path).expect("read committed schema");
    assert_eq!(
        committed,
        contasty::rules_schema_json(),
        "{} is stale; run `just gen-schema`",
        path.display()
    );
}
