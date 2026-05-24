use std::path::{Path, PathBuf};

use serde::Deserialize;

const DEFAULT_CONFIG_NAME: &str = "contasty.toml";

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub compact: CompactConfig,
}

#[derive(Debug, Deserialize)]
pub struct CompactConfig {
    #[serde(default = "default_min_bytes")]
    pub elide_min_bytes: usize,
    #[serde(default = "default_max_string_bytes")]
    pub max_string_bytes: usize,
}

impl Default for CompactConfig {
    fn default() -> Self {
        Self {
            elide_min_bytes: default_min_bytes(),
            max_string_bytes: default_max_string_bytes(),
        }
    }
}

const fn default_min_bytes() -> usize {
    0
}

const fn default_max_string_bytes() -> usize {
    256
}

impl Config {
    pub fn load(from_path: Option<&Path>, working_dir: &Path) -> Self {
        let path = from_path.map_or_else(|| working_dir.join(DEFAULT_CONFIG_NAME), PathBuf::from);
        Self::load_file(&path).unwrap_or_default()
    }

    fn load_file(path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn config_defaults_when_file_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = Config::load(None, dir.path());
        assert_eq!(config.compact.elide_min_bytes, 0);
    }

    #[test]
    fn config_loads_from_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("contasty.toml");
        let mut f = std::fs::File::create(&path).expect("create");
        writeln!(f, "[compact]\nelide_min_bytes = 256").expect("write");

        let config = Config::load(Some(&path), dir.path());
        assert_eq!(config.compact.elide_min_bytes, 256);
    }

    #[test]
    fn config_defaults_when_invalid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("contasty.toml");
        std::fs::write(&path, "not valid toml {{{").expect("write");

        let config = Config::load(Some(&path), dir.path());
        assert_eq!(config.compact.elide_min_bytes, default_min_bytes());
    }
}
