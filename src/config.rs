use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
    pub auth: AuthConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct AuthConfig {
    pub token: String,
}

/// Where the token was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSource {
    EnvVar,
    ConfigFile,
}

/// Resolve the API token. Precedence: env var > config file.
/// Returns the token and its source.
pub fn resolve_token() -> Result<(String, TokenSource)> {
    // 1. Environment variable
    if let Ok(t) = std::env::var("FASTMAIL_API_TOKEN")
        && !t.is_empty()
    {
        return Ok((t, TokenSource::EnvVar));
    }

    // 2. Config file
    let path = config_path();
    if path.exists() {
        check_permissions(&path)?;
        let contents = fs::read_to_string(&path).map_err(Error::Io)?;
        let config: Config = toml::from_str(&contents).map_err(|e| {
            Error::InvalidParams(format!("invalid config file {}: {e}", path.display()))
        })?;
        if !config.auth.token.is_empty() {
            return Ok((config.auth.token, TokenSource::ConfigFile));
        }
    }

    Err(Error::MissingToken)
}

/// Write a config file with the given token. Creates parent dirs and sets 0600.
pub fn write_config(token: &str) -> Result<PathBuf> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(Error::Io)?;
    }

    let content = format!(
        "[auth]\ntoken = {}\n",
        toml::Value::String(token.to_string())
    );

    fs::write(&path, &content).map_err(Error::Io)?;
    set_permissions_0600(&path)?;
    Ok(path)
}

/// Standard config file path: ~/.config/fastermail/config.toml
pub fn config_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        });
    base.join("fastermail").join("config.toml")
}

/// Check that the config file is not world-readable (Unix only).
#[cfg(unix)]
fn check_permissions(path: &PathBuf) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path).map_err(Error::Io)?;
    let mode = metadata.permissions().mode();
    // Check if group or other has any read/write/exec bits
    if mode & 0o077 != 0 {
        return Err(Error::InvalidParams(format!(
            "config file {} has insecure permissions {:04o} (expected 0600). \
             Fix with: chmod 600 {}",
            path.display(),
            mode & 0o777,
            path.display()
        )));
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_permissions(_path: &PathBuf) -> Result<()> {
    Ok(())
}

/// Set file permissions to 0600 (Unix only).
#[cfg(unix)]
fn set_permissions_0600(path: &PathBuf) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::Permissions::from_mode(0o600);
    fs::set_permissions(path, perms).map_err(Error::Io)?;
    Ok(())
}

#[cfg(not(unix))]
fn set_permissions_0600(_path: &PathBuf) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_config_path_uses_xdg() {
        // Just verify the function doesn't panic
        let path = config_path();
        assert!(path.ends_with("fastermail/config.toml"));
    }

    #[test]
    fn test_parse_valid_config() {
        let toml_str = r#"
[auth]
token = "fmu1-test-token"
"#;
        let config: Config = toml::from_str(toml_str).expect("valid toml");
        assert_eq!(config.auth.token, "fmu1-test-token");
    }

    #[test]
    fn test_parse_empty_config() {
        let config: Config = toml::from_str("").expect("empty toml");
        assert_eq!(config.auth.token, "");
    }

    #[test]
    fn test_parse_missing_auth_section() {
        let toml_str = r#"
[other]
key = "value"
"#;
        let config: Config = toml::from_str(toml_str).expect("valid toml");
        assert_eq!(config.auth.token, "");
    }

    #[cfg(unix)]
    #[test]
    fn test_check_permissions_rejects_world_readable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = std::env::temp_dir().join("fastermail-test-perms");
        fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("config.toml");

        let mut f = fs::File::create(&path).expect("create file");
        f.write_all(b"[auth]\ntoken = \"test\"\n")
            .expect("write file");

        // Set world-readable
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("set permissions");
        let result = check_permissions(&path);
        assert!(result.is_err());

        // Set correct permissions
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).expect("set permissions");
        let result = check_permissions(&path);
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(&dir);
    }
}
