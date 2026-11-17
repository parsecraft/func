//! The script store: a flat directory of `<name>.rs` files. Scripts are
//! never copied anywhere else; the store is the single source of truth.

use crate::report::{self, Problem};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_NAME_LEN: usize = 64;

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{name}.rs"))
    }

    /// Malformed names (`../x`, `a/b`) never exist, so lookups cannot escape the store.
    pub fn exists(&self, name: &str) -> bool {
        is_well_formed(name) && self.path(name).is_file()
    }

    /// Every saved script, sorted. A store that does not exist yet is empty.
    pub fn names(&self) -> Result<Vec<String>, Problem> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(report::io("list your scripts in", &self.dir, &error)),
        };

        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                let stem = path.file_stem()?.to_str()?;
                (path.extension()? == "rs" && is_well_formed(stem)).then(|| stem.to_string())
            })
            .collect();
        names.sort();
        Ok(names)
    }

    pub fn write(&self, name: &str, contents: &str) -> Result<PathBuf, Problem> {
        fs::create_dir_all(&self.dir)
            .map_err(|e| report::io("create the scripts directory", &self.dir, &e))?;
        let path = self.path(name);
        fs::write(&path, contents).map_err(|e| report::io("write the script", &path, &e))?;
        Ok(path)
    }

    pub fn remove(&self, name: &str) -> Result<(), Problem> {
        let path = self.path(name);
        fs::remove_file(&path).map_err(|e| report::io("delete the script", &path, &e))
    }

    pub fn rename(&self, from: &str, to: &str) -> Result<(), Problem> {
        let source = self.path(from);
        fs::rename(&source, self.path(to)).map_err(|e| report::io("rename the script", &source, &e))
    }
}

fn is_well_formed(name: &str) -> bool {
    let mut chars = name.chars();
    let starts_with_letter = chars.next().is_some_and(|c| c.is_ascii_alphabetic());
    starts_with_letter
        && name.len() <= MAX_NAME_LEN
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Turns anything into something that would pass [`validate_name`].
fn sanitize(name: &str) -> String {
    let dashed: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let trimmed = dashed.trim_start_matches(|c: char| !c.is_ascii_alphabetic());
    trimmed.chars().take(MAX_NAME_LEN).collect::<String>().trim_matches('-').to_string()
}

/// Names must be safe in a file name, a shell, and a Cargo target. They may
/// equal a `func` command (`edit`, `list`): scripts run only via `func run`,
/// so the two never share a namespace.
pub fn validate_name(name: &str) -> Result<(), Problem> {
    if !is_well_formed(name) {
        let mut problem = Problem::new("INVALID NAME")
            .text(format!("`{name}` cannot be used as a script name."))
            .text(format!(
                "Names start with a letter, then use letters, digits, `-` and `_` (at most {MAX_NAME_LEN} characters)."
            ));
        let suggestion = sanitize(name);
        if !suggestion.is_empty() && suggestion != name {
            problem = problem.hint(format!("How about `{suggestion}`?"));
        }
        return Err(problem.usage());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_ordinary_names() {
        for name in ["hello", "hello-world", "hello_world", "a1", "Zed"] {
            assert!(validate_name(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn rejects_unsafe_names_with_a_suggestion() {
        for name in ["", "1abc", "-x", "has space", "a/b", "dot.rs", &"x".repeat(65)] {
            assert!(validate_name(name).is_err(), "{name}");
        }
        let text = validate_name("my script!").unwrap_err().render(false);
        assert!(text.contains("How about `my-script`?"));
    }

    #[test]
    fn command_names_are_fine_because_scripts_run_via_run() {
        for name in ["list", "rm", "edit", "run"] {
            assert!(validate_name(name).is_ok(), "{name}");
        }
    }

    #[test]
    fn sanitize_always_yields_valid_or_empty() {
        for raw in ["my script!", "123", "--", "é", "ok"] {
            let clean = sanitize(raw);
            assert!(clean.is_empty() || is_well_formed(&clean), "{raw} -> {clean}");
        }
    }
}
