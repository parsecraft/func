//! Where `func` keeps things. Follows the XDG layout; `FUNC_HOME` moves
//! everything under one directory (handy for tests and throwaway sandboxes).
//!
//! | what              | default                              |
//! |-------------------|--------------------------------------|
//! | scripts           | `~/.local/share/func/scripts`        |
//! | install ledger    | `~/.local/share/func/installed`      |
//! | build cache       | `~/.cache/func`                      |
//! | installed binaries| `~/.local/bin` (override: `FUNC_BIN`) |

use crate::report::Problem;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Dirs {
    /// Parent of `scripts`; also holds the install ledger.
    pub data: PathBuf,
    pub scripts: PathBuf,
    pub cache: PathBuf,
    pub bin: PathBuf,
}

impl Dirs {
    pub fn discover() -> Result<Self, Problem> {
        Self::from_env(|key| std::env::var_os(key).filter(|v| !v.is_empty()).map(PathBuf::from))
    }

    /// Takes the environment as a function so tests need no global state.
    pub fn from_env(get: impl Fn(&str) -> Option<PathBuf>) -> Result<Self, Problem> {
        if let Some(home) = get("FUNC_HOME") {
            return Ok(Self {
                scripts: home.join("scripts"),
                cache: home.join("cache"),
                bin: get("FUNC_BIN").unwrap_or_else(|| home.join("bin")),
                data: home,
            });
        }

        let user_home = get("HOME").or_else(|| get("USERPROFILE")).ok_or_else(|| {
            Problem::new("NO HOME DIRECTORY")
                .text("I need somewhere to keep your scripts, but $HOME is not set.")
                .hint("Set $HOME, or set $FUNC_HOME to the directory func should use.")
        })?;

        let data = get("XDG_DATA_HOME").unwrap_or_else(|| user_home.join(".local/share")).join("func");
        let cache = get("XDG_CACHE_HOME").unwrap_or_else(|| user_home.join(".cache")).join("func");
        let bin = get("FUNC_BIN").unwrap_or_else(|| user_home.join(".local/bin"));
        Ok(Self { scripts: data.join("scripts"), data, cache, bin })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<PathBuf> {
        let map: HashMap<String, PathBuf> =
            pairs.iter().map(|(k, v)| (k.to_string(), PathBuf::from(v))).collect();
        move |key| map.get(key).cloned()
    }

    #[test]
    fn defaults_follow_xdg_conventions() {
        let dirs = Dirs::from_env(env(&[("HOME", "/home/me")])).unwrap();
        assert_eq!(dirs.scripts, PathBuf::from("/home/me/.local/share/func/scripts"));
        assert_eq!(dirs.cache, PathBuf::from("/home/me/.cache/func"));
        assert_eq!(dirs.bin, PathBuf::from("/home/me/.local/bin"));
    }

    #[test]
    fn xdg_overrides_are_respected() {
        let dirs = Dirs::from_env(env(&[
            ("HOME", "/home/me"),
            ("XDG_DATA_HOME", "/data"),
            ("XDG_CACHE_HOME", "/cache"),
        ]))
        .unwrap();
        assert_eq!(dirs.scripts, PathBuf::from("/data/func/scripts"));
        assert_eq!(dirs.cache, PathBuf::from("/cache/func"));
    }

    #[test]
    fn func_home_moves_everything() {
        let dirs = Dirs::from_env(env(&[("FUNC_HOME", "/sandbox"), ("HOME", "/home/me")])).unwrap();
        assert_eq!(dirs.scripts, PathBuf::from("/sandbox/scripts"));
        assert_eq!(dirs.cache, PathBuf::from("/sandbox/cache"));
        assert_eq!(dirs.bin, PathBuf::from("/sandbox/bin"));
    }

    #[test]
    fn missing_home_is_a_friendly_error() {
        let problem = Dirs::from_env(env(&[])).unwrap_err();
        assert!(problem.render(false).contains("NO HOME DIRECTORY"));
    }
}
