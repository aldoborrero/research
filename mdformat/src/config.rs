use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Formatter configuration options.
#[derive(Debug, Clone)]
pub struct Config {
    /// Line ending style.
    pub line_ending: LineEnding,
    /// Plugin-specific configuration, keyed by plugin name.
    pub plugin: HashMap<String, toml::Value>,
}

/// Line ending normalization mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    /// Unix-style line endings (`\n`).
    Lf,
    /// Windows-style line endings (`\r\n`).
    CrLf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::Lf,
            plugin: HashMap::new(),
        }
    }
}

/// Raw TOML representation of `.mdformat.toml`.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct TomlConfig {
    end_of_line: Option<String>,
    plugin: HashMap<String, toml::Value>,
}

impl Config {
    /// Load configuration by searching for `.mdformat.toml` starting from
    /// `start_dir` and walking up to the filesystem root.
    ///
    /// Returns the default config if no file is found.
    pub fn from_directory(start_dir: &Path) -> Result<Self, ConfigError> {
        match find_config_file(start_dir) {
            Some(path) => Self::from_file(&path),
            None => Ok(Self::default()),
        }
    }

    /// Load configuration from a specific TOML file.
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(|e| ConfigError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Self::from_str(&content)
    }

    /// Parse configuration from a TOML string.
    pub fn from_str(toml_str: &str) -> Result<Self, ConfigError> {
        let raw: TomlConfig = toml::from_str(toml_str).map_err(ConfigError::Parse)?;

        let line_ending = match raw.end_of_line.as_deref() {
            Some("lf") | None => LineEnding::Lf,
            Some("crlf") => LineEnding::CrLf,
            Some(other) => {
                return Err(ConfigError::InvalidValue {
                    key: "end_of_line".into(),
                    value: other.into(),
                    expected: "\"lf\" or \"crlf\"".into(),
                })
            }
        };

        Ok(Self {
            line_ending,
            plugin: raw.plugin,
        })
    }

    /// Get plugin-specific config as a typed value.
    ///
    /// Deserializes the `[plugin.<name>]` table into the requested type.
    pub fn plugin_config<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
    ) -> Result<T, ConfigError> {
        match self.plugin.get(name) {
            Some(value) => {
                value.clone().try_into().map_err(|e: toml::de::Error| ConfigError::Parse(e))
            }
            None => {
                // No plugin section — deserialize empty table to get defaults
                toml::Value::Table(toml::map::Map::new())
                    .try_into()
                    .map_err(|e: toml::de::Error| ConfigError::Parse(e))
            }
        }
    }

    /// Merge CLI overrides into this config. CLI values take precedence.
    pub fn merge_plugin_value(&mut self, plugin_name: &str, key: &str, value: toml::Value) {
        let table = self
            .plugin
            .entry(plugin_name.to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));

        if let toml::Value::Table(t) = table {
            t.insert(key.to_string(), value);
        }
    }
}

/// Errors that can occur when loading configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid value for {key}: got \"{value}\", expected {expected}")]
    InvalidValue {
        key: String,
        value: String,
        expected: String,
    },
}

/// Search for `.mdformat.toml` starting from `start_dir`, walking up.
fn find_config_file(start_dir: &Path) -> Option<PathBuf> {
    let mut dir = start_dir.to_path_buf();
    loop {
        let candidate = dir.join(".mdformat.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.line_ending, LineEnding::Lf);
        assert!(config.plugin.is_empty());
    }

    #[test]
    fn test_parse_empty_toml() {
        let config = Config::from_str("").unwrap();
        assert_eq!(config.line_ending, LineEnding::Lf);
    }

    #[test]
    fn test_parse_end_of_line() {
        let config = Config::from_str("end_of_line = \"crlf\"").unwrap();
        assert_eq!(config.line_ending, LineEnding::CrLf);
    }

    #[test]
    fn test_parse_invalid_end_of_line() {
        let result = Config::from_str("end_of_line = \"mac\"");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid value"));
    }

    #[test]
    fn test_parse_plugin_section() {
        let toml_str = r#"
[plugin.mkdocs]
align_semantic_breaks_in_lists = true
no_mkdocs_math = true
"#;
        let config = Config::from_str(toml_str).unwrap();
        assert!(config.plugin.contains_key("mkdocs"));
    }

    #[test]
    fn test_plugin_config_deserialization() {
        #[derive(Debug, Deserialize, Default)]
        #[serde(default)]
        struct TestPluginConfig {
            enabled: bool,
            indent: usize,
        }

        let toml_str = r#"
[plugin.test]
enabled = true
indent = 8
"#;
        let config = Config::from_str(toml_str).unwrap();
        let plugin_cfg: TestPluginConfig = config.plugin_config("test").unwrap();
        assert!(plugin_cfg.enabled);
        assert_eq!(plugin_cfg.indent, 8);
    }

    #[test]
    fn test_plugin_config_defaults_when_missing() {
        #[derive(Debug, Deserialize)]
        #[serde(default)]
        struct TestPluginConfig {
            enabled: bool,
        }

        impl Default for TestPluginConfig {
            fn default() -> Self {
                Self { enabled: false }
            }
        }

        let config = Config::from_str("").unwrap();
        let plugin_cfg: TestPluginConfig = config.plugin_config("nonexistent").unwrap();
        assert!(!plugin_cfg.enabled);
    }

    #[test]
    fn test_merge_plugin_value() {
        let mut config = Config::default();
        config.merge_plugin_value("mkdocs", "no_mkdocs_math", toml::Value::Boolean(true));

        let table = config.plugin.get("mkdocs").unwrap();
        if let toml::Value::Table(t) = table {
            assert_eq!(t.get("no_mkdocs_math"), Some(&toml::Value::Boolean(true)));
        } else {
            panic!("expected table");
        }
    }

    #[test]
    fn test_find_config_file_in_tempdir() {
        let dir = std::env::temp_dir().join("mdformat_test_config");
        let _ = std::fs::create_dir_all(&dir);
        let config_path = dir.join(".mdformat.toml");
        std::fs::write(&config_path, "end_of_line = \"lf\"").unwrap();

        let found = find_config_file(&dir);
        assert_eq!(found, Some(config_path.clone()));

        // Cleanup
        let _ = std::fs::remove_file(&config_path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn test_find_config_file_walks_up() {
        let parent = std::env::temp_dir().join("mdformat_test_walk");
        let child = parent.join("subdir");
        let _ = std::fs::create_dir_all(&child);
        let config_path = parent.join(".mdformat.toml");
        std::fs::write(&config_path, "").unwrap();

        let found = find_config_file(&child);
        assert_eq!(found, Some(config_path.clone()));

        // Cleanup
        let _ = std::fs::remove_file(&config_path);
        let _ = std::fs::remove_dir_all(&parent);
    }
}
