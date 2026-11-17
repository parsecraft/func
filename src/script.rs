//! A script is one `.rs` file plus an optional header of directives.
//!
//! ```text
//! //! Fetches a page and counts its links.      <- description (shown by `func list`)
//! // edition: 2021                              <- optional, defaults to 2024
//! // dep: regex = "1"                           <- optional, repeatable
//!
//! fn main() { /* ... */ }
//! ```
//!
//! The header is the run of comment lines at the top of the file. Directives
//! are ordinary comments, so the file stays valid Rust for editors and tools.

use crate::report::{self, Problem};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_EDITION: &str = "2024";
const EDITIONS: [&str; 4] = ["2015", "2018", "2021", "2024"];

/// Common misspellings of `dep`. Caught loudly, because a silently ignored
/// dependency line produces a confusing "unresolved import" much later.
const DEP_TYPOS: [&str; 4] = ["deps", "dependency", "dependencies", "crate"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    /// The right-hand side of a Cargo.toml entry: `"1"` or `{ version = "1", features = ["x"] }`.
    pub spec: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub edition: String,
    pub deps: Vec<Dependency>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Lives in the script store; addressed by name.
    Store,
    /// Addressed by path (`func run ./x.rs`) or generated (`func eval`).
    File,
}

#[derive(Debug)]
pub struct Script {
    pub name: String,
    pub path: PathBuf,
    pub source: String,
    pub manifest: Manifest,
    pub origin: Origin,
}

impl Script {
    pub fn load(name: &str, path: &Path, origin: Origin) -> Result<Self, Problem> {
        let source = fs::read_to_string(path).map_err(|e| report::io("read the script", path, &e))?;
        Self::from_source(name, path, source, origin)
    }

    pub fn from_source(name: &str, path: &Path, source: String, origin: Origin) -> Result<Self, Problem> {
        let label = match origin {
            Origin::Store => name.to_string(),
            Origin::File => path.display().to_string(),
        };
        let manifest = parse_manifest(&source, &label)?;
        let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        Ok(Self { name: name.to_string(), path, source, manifest, origin })
    }

    /// How to refer to this script in messages.
    pub fn label(&self) -> String {
        match self.origin {
            Origin::Store => self.name.clone(),
            Origin::File => self.path.display().to_string(),
        }
    }

    /// The command that reopens this script, when there is one.
    pub fn edit_hint(&self) -> Option<String> {
        (self.origin == Origin::Store).then(|| format!("func edit {}", self.name))
    }
}

pub fn template(name: &str) -> String {
    format!(
        "//! TODO: say what `{name}` does in one line.\n\nfn main() {{\n    println!(\"hello from {name}\");\n}}\n"
    )
}

/// Wraps statements from `func eval` into a program. The code starts on the
/// same line as `fn main`, so compiler line numbers match what was typed.
pub fn eval_source(code: &str) -> String {
    if code.contains("fn main") { format!("{code}\n") } else { format!("fn main() {{ {code}\n}}\n") }
}

/// The first `//!` line, without parsing or validating the rest.
pub fn describe(source: &str) -> Option<String> {
    header_lines(source).find_map(|line| doc_text(line).filter(|text| !text.is_empty())).map(str::to_string)
}

pub fn parse_manifest(source: &str, label: &str) -> Result<Manifest, Problem> {
    let mut edition = DEFAULT_EDITION.to_string();
    let mut deps: Vec<Dependency> = Vec::new();
    let mut description = None;

    for (index, line) in header_lines(source).enumerate() {
        let location = format!("{label}:{}", index + 1);
        if let Some(text) = doc_text(line) {
            if description.is_none() && !text.is_empty() {
                description = Some(text.to_string());
            }
            continue;
        }

        match directive(line) {
            Some(("dep", value)) => match parse_dependency(value) {
                Ok(dep) => deps.push(dep),
                Err(why) => return Err(bad_directive(&location, line, &why, "// dep: regex = \"1\"")),
            },
            Some(("edition", value)) if EDITIONS.contains(&value) => edition = value.to_string(),
            Some(("edition", _)) => {
                let why = format!("The editions I know are {}.", EDITIONS.join(", "));
                return Err(bad_directive(&location, line, &why, "// edition: 2024"));
            }
            Some((key, _)) if DEP_TYPOS.contains(&key.to_ascii_lowercase().as_str()) => {
                let why = format!("`{key}` is not a directive. Dependencies are declared with `dep`.");
                return Err(bad_directive(&location, line, &why, "// dep: regex = \"1\""));
            }
            _ => {}
        }
    }
    Ok(Manifest { edition, deps, description })
}

/// The leading run of blank and comment lines.
fn header_lines(source: &str) -> impl Iterator<Item = &str> {
    source.lines().map(str::trim).take_while(|line| line.is_empty() || line.starts_with("//"))
}

fn doc_text(line: &str) -> Option<&str> {
    line.strip_prefix("//!").map(str::trim)
}

/// `// key: value` -> `("key", "value")`. Anything else is a plain comment.
fn directive(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.strip_prefix("//")?.trim().split_once(':')?;
    let key = key.trim();
    (!key.is_empty() && key.chars().all(|c| c.is_ascii_alphabetic())).then_some((key, value.trim()))
}

/// Accepts `name = "1"`, `name = 1`, `name = { ... }` and a bare `name`.
fn parse_dependency(value: &str) -> Result<Dependency, String> {
    let (name, rhs) = value.split_once('=').map_or((value, ""), |(n, r)| (n.trim(), r.trim()));

    let valid_name =
        !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid_name {
        return Err(format!("`{name}` is not a crate name."));
    }

    let spec = if rhs.is_empty() {
        "\"*\"".to_string()
    } else if rhs.starts_with('"') || rhs.starts_with('{') {
        rhs.to_string()
    } else if rhs.chars().all(|c| c.is_ascii_alphanumeric() || ".^~*<>=,+- ".contains(c)) {
        format!("\"{rhs}\"")
    } else {
        return Err(format!("I do not understand the version `{rhs}`."));
    };
    Ok(Dependency { name: name.to_string(), spec })
}

fn bad_directive(location: &str, line: &str, why: &str, example: &str) -> Problem {
    Problem::new("BAD HEADER LINE")
        .context(location)
        .text("I could not understand this line at the top of your script:")
        .code(line.to_string())
        .text(why.to_string())
        .hint(format!("It should look like `{example}`."))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> Result<Manifest, Problem> {
        parse_manifest(source, "t")
    }

    #[test]
    fn plain_script_gets_defaults() {
        let m = parse("fn main() {}").unwrap();
        assert_eq!(m.edition, DEFAULT_EDITION);
        assert!(m.deps.is_empty());
        assert_eq!(m.description, None);
    }

    #[test]
    fn reads_description_edition_and_deps() {
        let m = parse("//! Counts links.\n// edition: 2021\n// dep: regex = \"1\"\n// dep: serde = { version = \"1\", features = [\"derive\"] }\n\nfn main() {}").unwrap();
        assert_eq!(m.description.as_deref(), Some("Counts links."));
        assert_eq!(m.edition, "2021");
        assert_eq!(m.deps.len(), 2);
        assert_eq!(m.deps[0], Dependency { name: "regex".into(), spec: "\"1\"".into() });
        assert!(m.deps[1].spec.starts_with('{'));
    }

    #[test]
    fn forgives_unquoted_versions_and_bare_names() {
        assert_eq!(parse_dependency("rand = 0.8").unwrap().spec, "\"0.8\"");
        assert_eq!(parse_dependency("rand").unwrap().spec, "\"*\"");
    }

    #[test]
    fn directives_after_code_are_ignored() {
        let m = parse("fn main() {}\n// dep: regex = \"1\"\n").unwrap();
        assert!(m.deps.is_empty());
    }

    #[test]
    fn ordinary_comments_are_left_alone() {
        assert!(parse("// note: remember to refactor\nfn main() {}").is_ok());
    }

    #[test]
    fn unknown_edition_is_explained() {
        let text = parse("// edition: 2077\n").unwrap_err().render(false);
        assert!(text.contains("t:1"));
        assert!(text.contains("2015, 2018, 2021, 2024"));
    }

    #[test]
    fn misspelled_dep_directive_is_caught() {
        let text = parse("// deps: regex = \"1\"\n").unwrap_err().render(false);
        assert!(text.contains("Dependencies are declared with `dep`"));
    }

    #[test]
    fn bad_crate_names_are_rejected() {
        assert!(parse("// dep: not a crate = \"1\"\n").is_err());
    }

    #[test]
    fn describe_finds_first_doc_line() {
        assert_eq!(describe("\n//! Hi there\n//! more\nfn main() {}").as_deref(), Some("Hi there"));
        assert_eq!(describe("fn main() {}"), None);
    }

    #[test]
    fn eval_keeps_line_numbers() {
        assert_eq!(eval_source("let x = 1;"), "fn main() { let x = 1;\n}\n");
        assert!(eval_source("fn main() {}").starts_with("fn main() {}"));
    }

    #[test]
    fn template_is_valid_and_mentions_the_name() {
        let t = template("demo");
        assert!(t.contains("demo"));
        assert!(parse_manifest(&t, "demo").is_ok());
    }
}
