//! Which program runs a step (`spec/protocol.md` 6.1 and 6.3).
//!
//! `command[0]` is never looked up on `PATH`: a bare name such as `python` means the program the
//! user named once in `config.toml`, so one interpreter is chosen deliberately and the same one is
//! used on every system. The probe also takes an explicit path, as `via build` will through a flag.

use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq, Eq)]
pub enum ProgramError {
    /// No `[program.<name>]` in `config.toml`: `protocol.no_program`, hint
    /// `via config program <name> <path>`.
    NotConfigured { name: String, config: PathBuf },
    /// `config.toml` exists but does not parse, or `path` is not an absolute string.
    BadConfig { config: PathBuf, reason: String },
}

impl std::fmt::Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgramError::NotConfigured { name, config } => write!(
                f,
                "no program named {name} in {}; run `via config program {name} <path>`",
                config.display()
            ),
            ProgramError::BadConfig { config, reason } => {
                write!(f, "{}: {reason}", config.display())
            }
        }
    }
}

/// The user config folder: `VIA_CONFIG_DIR` if set, else the system's place for it.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("VIA_CONFIG_DIR") {
        return Some(PathBuf::from(d));
    }
    if cfg!(windows) {
        return std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("via"));
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        return home.map(|h| h.join("Library/Application Support/via"));
    }
    match std::env::var_os("XDG_CONFIG_HOME") {
        Some(x) if !x.is_empty() => Some(PathBuf::from(x).join("via")),
        _ => home.map(|h| h.join(".config/via")),
    }
}

/// Resolves a program name through `<config_dir>/config.toml`.
pub fn resolve(name: &str, config_dir: &Path) -> Result<PathBuf, ProgramError> {
    let config = config_dir.join("config.toml");
    let not_configured = || ProgramError::NotConfigured {
        name: name.to_owned(),
        config: config.clone(),
    };
    let text = match std::fs::read_to_string(&config) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err(not_configured()),
        Err(e) => {
            return Err(ProgramError::BadConfig {
                config,
                reason: e.to_string(),
            })
        }
    };
    let table: toml::Table =
        text.parse()
            .map_err(|e: toml::de::Error| ProgramError::BadConfig {
                config: config.clone(),
                reason: e.to_string(),
            })?;
    let entry = table
        .get("program")
        .and_then(|p| p.get(name))
        .ok_or_else(not_configured)?;
    let path =
        entry
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ProgramError::BadConfig {
                config: config.clone(),
                reason: format!("[program.{name}] has no string `path`"),
            })?;
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err(ProgramError::BadConfig {
            config,
            reason: format!("[program.{name}] path {} is not absolute", path.display()),
        });
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_only_what_config_names() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            resolve("python", dir.path()),
            Err(ProgramError::NotConfigured { .. })
        ));
        // Absolute on this system: a drive on Windows, where `/opt/...` is not absolute. Written
        // as a TOML literal string, so the backslashes need no escaping.
        let abs = if cfg!(windows) {
            r"C:\py\python.exe"
        } else {
            "/opt/py/bin/python3"
        };
        std::fs::write(
            dir.path().join("config.toml"),
            format!("[program.python]\npath = '{abs}'\n[program.rel]\npath = 'py'\n"),
        )
        .unwrap();
        assert_eq!(resolve("python", dir.path()).unwrap(), PathBuf::from(abs));
        assert!(matches!(
            resolve("python3", dir.path()),
            Err(ProgramError::NotConfigured { .. })
        ));
        assert!(matches!(
            resolve("rel", dir.path()),
            Err(ProgramError::BadConfig { .. })
        ));
    }
}
