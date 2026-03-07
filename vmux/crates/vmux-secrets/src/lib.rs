use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Errors that can occur during secret resolution.
#[derive(Debug, Error)]
pub enum SecretError {
    #[error("1Password CLI (`op`) not found in PATH")]
    OpCliNotFound,

    #[error("1Password CLI failed: {0}")]
    OpCliFailed(String),

    #[error("sops decryption failed for {path}: {reason}")]
    SopsDecryptFailed { path: PathBuf, reason: String },

    #[error("environment variable `{var}` is not set")]
    EnvVarNotSet { var: String },

    #[error("secret file not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("I/O error reading secret: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse secret source `{input}`: expected format op://..., sops:..., env:..., or file:...")]
    InvalidSource { input: String },
}

/// Where a secret comes from.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SecretSource {
    /// 1Password reference: `op://vault/item/field`
    OnePassword { reference: String },
    /// sops-encrypted file: `sops:path/to/file`
    Sops { path: PathBuf },
    /// Host environment variable: `env:VAR_NAME`
    Env { var: String },
    /// Raw file content: `file:path`
    File { path: PathBuf },
}

impl SecretSource {
    /// Parse a secret source from a CLI string.
    ///
    /// Accepted formats:
    /// - `op://vault/item/field`
    /// - `sops:path/to/encrypted.yaml`
    /// - `env:VARIABLE_NAME`
    /// - `file:/path/to/secret`
    pub fn parse(input: &str) -> Result<Self, SecretError> {
        if input.starts_with("op://") {
            Ok(SecretSource::OnePassword {
                reference: input.to_string(),
            })
        } else if let Some(path) = input.strip_prefix("sops:") {
            Ok(SecretSource::Sops {
                path: PathBuf::from(path),
            })
        } else if let Some(var) = input.strip_prefix("env:") {
            Ok(SecretSource::Env {
                var: var.to_string(),
            })
        } else if let Some(path) = input.strip_prefix("file:") {
            Ok(SecretSource::File {
                path: PathBuf::from(path),
            })
        } else {
            Err(SecretError::InvalidSource {
                input: input.to_string(),
            })
        }
    }
}

/// A resolved secret value that is zeroed from memory on drop.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecretValue(String);

impl SecretValue {
    /// Create a new secret value.
    pub fn new(value: String) -> Self {
        Self(value)
    }

    /// Access the secret value. Avoid logging this.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretValue(***)")
    }
}

/// A named secret (name + source specification).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretSpec {
    pub name: String,
    pub source: SecretSource,
}

impl SecretSpec {
    /// Parse from CLI format: `NAME=source_spec`
    pub fn parse_cli(input: &str) -> Result<Self, SecretError> {
        let (name, source_str) = input.split_once('=').ok_or_else(|| SecretError::InvalidSource {
            input: input.to_string(),
        })?;
        let source = SecretSource::parse(source_str)?;
        Ok(SecretSpec {
            name: name.to_string(),
            source,
        })
    }
}

/// Resolves secrets from their various sources.
pub struct SecretResolver;

impl SecretResolver {
    pub fn new() -> Self {
        Self
    }

    /// Resolve a single secret from its source.
    pub async fn resolve(&self, source: &SecretSource) -> Result<SecretValue, SecretError> {
        match source {
            SecretSource::OnePassword { reference } => self.resolve_op(reference).await,
            SecretSource::Sops { path } => self.resolve_sops(path).await,
            SecretSource::Env { var } => self.resolve_env(var),
            SecretSource::File { path } => self.resolve_file(path).await,
        }
    }

    /// Resolve all secrets from a list of specs.
    pub async fn resolve_all(
        &self,
        specs: &[SecretSpec],
    ) -> Result<Vec<(String, SecretValue)>, SecretError> {
        let mut results = Vec::with_capacity(specs.len());
        for spec in specs {
            tracing::debug!(name = %spec.name, "resolving secret");
            let value = self.resolve(&spec.source).await?;
            results.push((spec.name.clone(), value));
        }
        Ok(results)
    }

    async fn resolve_op(&self, reference: &str) -> Result<SecretValue, SecretError> {
        let output = tokio::process::Command::new("op")
            .args(["read", reference])
            .output()
            .await
            .map_err(|_| SecretError::OpCliNotFound)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SecretError::OpCliFailed(stderr.into_owned()));
        }

        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(SecretValue::new(value))
    }

    async fn resolve_sops(&self, path: &PathBuf) -> Result<SecretValue, SecretError> {
        let output = tokio::process::Command::new("sops")
            .args(["--decrypt", "--output-type", "raw"])
            .arg(path)
            .output()
            .await
            .map_err(|e| SecretError::SopsDecryptFailed {
                path: path.clone(),
                reason: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SecretError::SopsDecryptFailed {
                path: path.clone(),
                reason: stderr.into_owned(),
            });
        }

        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(SecretValue::new(value))
    }

    fn resolve_env(&self, var: &str) -> Result<SecretValue, SecretError> {
        let value = std::env::var(var).map_err(|_| SecretError::EnvVarNotSet {
            var: var.to_string(),
        })?;
        Ok(SecretValue::new(value))
    }

    async fn resolve_file(&self, path: &PathBuf) -> Result<SecretValue, SecretError> {
        if !path.exists() {
            return Err(SecretError::FileNotFound { path: path.clone() });
        }
        let content = tokio::fs::read_to_string(path).await?;
        Ok(SecretValue::new(content.trim().to_string()))
    }
}

impl Default for SecretResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_op_source() {
        let source = SecretSource::parse("op://Personal/Anthropic/api_key").unwrap();
        assert!(matches!(source, SecretSource::OnePassword { reference } if reference == "op://Personal/Anthropic/api_key"));
    }

    #[test]
    fn parse_env_source() {
        let source = SecretSource::parse("env:MY_SECRET").unwrap();
        assert!(matches!(source, SecretSource::Env { var } if var == "MY_SECRET"));
    }

    #[test]
    fn parse_file_source() {
        let source = SecretSource::parse("file:/tmp/secret.txt").unwrap();
        assert!(matches!(source, SecretSource::File { path } if path == PathBuf::from("/tmp/secret.txt")));
    }

    #[test]
    fn parse_sops_source() {
        let source = SecretSource::parse("sops:secrets/keys.yaml").unwrap();
        assert!(matches!(source, SecretSource::Sops { path } if path == PathBuf::from("secrets/keys.yaml")));
    }

    #[test]
    fn parse_invalid_source() {
        assert!(SecretSource::parse("invalid").is_err());
    }

    #[test]
    fn parse_secret_spec_cli() {
        let spec = SecretSpec::parse_cli("API_KEY=op://vault/item/field").unwrap();
        assert_eq!(spec.name, "API_KEY");
        assert!(matches!(spec.source, SecretSource::OnePassword { .. }));
    }

    #[test]
    fn secret_value_debug_does_not_leak() {
        let secret = SecretValue::new("super_secret".to_string());
        let debug = format!("{:?}", secret);
        assert!(!debug.contains("super_secret"));
        assert!(debug.contains("***"));
    }

    #[tokio::test]
    async fn resolve_env_var() {
        std::env::set_var("VMUX_TEST_SECRET", "test_value");
        let resolver = SecretResolver::new();
        let value = resolver
            .resolve(&SecretSource::Env {
                var: "VMUX_TEST_SECRET".to_string(),
            })
            .await
            .unwrap();
        assert_eq!(value.expose(), "test_value");
        std::env::remove_var("VMUX_TEST_SECRET");
    }

    #[tokio::test]
    async fn resolve_missing_env_var() {
        let resolver = SecretResolver::new();
        let result = resolver
            .resolve(&SecretSource::Env {
                var: "VMUX_NONEXISTENT_VAR".to_string(),
            })
            .await;
        assert!(result.is_err());
    }
}
