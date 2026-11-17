//! Compiling scripts, and remembering the result.
//!
//! Two backends, chosen by what the script needs:
//!
//! - **rustc** (no dependencies): one direct `rustc` call. Output is cached
//!   under a key made of the script's name, content, edition and profile, so
//!   running an unchanged script costs one hash and one `exec`.
//! - **cargo** (the script declares `// dep:` lines): a generated manifest
//!   points *at the script file itself* (no copy) and all scripts share one
//!   target directory, so each dependency compiles only once.

use crate::exec;
use crate::hash::Fnv64;
use crate::report::{self, Problem};
use crate::script::Script;
use crate::term::{self, Style};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Profile {
    Debug,
    Release,
}

impl Profile {
    pub fn from_flag(release: bool) -> Self {
        if release { Profile::Release } else { Profile::Debug }
    }

    fn dir(self) -> &'static str {
        match self {
            Profile::Debug => "debug",
            Profile::Release => "release",
        }
    }
}

/// What a rustc build produces; part of the cache key.
#[derive(Clone, Copy)]
enum Kind {
    Binary,
    Tests,
}

pub struct Builder {
    cache: PathBuf,
}

impl Builder {
    pub fn new(cache: PathBuf) -> Self {
        Self { cache }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache
    }

    /// Type-checks without producing a binary. Fast feedback while editing.
    pub fn check(&self, script: &Script) -> Result<(), Problem> {
        if uses_cargo(script) {
            return self.cargo(script, &["check", "--quiet"], Profile::Debug);
        }
        let out_dir = self.cache.join("check");
        fs::create_dir_all(&out_dir).map_err(|e| report::io("create the cache directory", &out_dir, &e))?;
        let mut command = rustc(script, Profile::Debug);
        command.arg("--emit=metadata").arg("--out-dir").arg(&out_dir);
        run_rustc(command, script)
    }

    /// Returns the path of an up-to-date executable, compiling only if needed.
    pub fn compile(&self, script: &Script, profile: Profile) -> Result<PathBuf, Problem> {
        if uses_cargo(script) {
            return self.cargo_build(script, profile);
        }
        let output = self.artifact(script, profile, Kind::Binary);
        if output.is_file() {
            return Ok(output);
        }
        announce(script);
        self.produce(rustc(script, profile), script, &output)?;
        Ok(output)
    }

    /// Builds and runs the script's `#[test]` functions. Returns the exit code.
    pub fn test(&self, script: &Script, args: &[String]) -> Result<u8, Problem> {
        if uses_cargo(script) {
            let mut command = self.cargo_command(script, &["test"], Profile::Debug)?;
            command.arg("--").args(args);
            return cargo_status(command, script, true);
        }
        let output = self.artifact(script, Profile::Debug, Kind::Tests);
        if !output.is_file() {
            announce(script);
            let mut command = rustc(script, Profile::Debug);
            // The script's own `main` is unused in a test build; that is not worth a warning.
            command.args(["--test", "-A", "dead_code"]);
            self.produce(command, script, &output)?;
        }
        exec::wait(&output, args)
    }

    // -- rustc backend ------------------------------------------------------

    fn artifact(&self, script: &Script, profile: Profile, kind: Kind) -> PathBuf {
        let tag: &[u8] = match kind {
            Kind::Binary => b"bin",
            Kind::Tests => b"test",
        };
        let key = Fnv64::new()
            .field(script.name.as_bytes())
            .field(script.manifest.edition.as_bytes())
            .field(profile.dir().as_bytes())
            .field(tag)
            .field(script.source.as_bytes())
            .hex();
        self.cache.join("builds").join(key).join(&script.name)
    }

    /// Compiles to a temporary name, then renames, so a build that is
    /// interrupted (or runs in parallel) can never leave a half-written binary.
    fn produce(&self, mut command: Command, script: &Script, output: &Path) -> Result<(), Problem> {
        let dir = output.parent().expect("artifact paths always have a parent");
        fs::create_dir_all(dir).map_err(|e| report::io("create the build directory", dir, &e))?;
        let partial = dir.join(format!(".{}.partial", std::process::id()));
        command.arg("-o").arg(&partial);
        run_rustc(command, script)?;
        fs::rename(&partial, output).map_err(|e| report::io("move the finished build into place", output, &e))
    }

    // -- cargo backend ------------------------------------------------------

    fn cargo_build(&self, script: &Script, profile: Profile) -> Result<PathBuf, Problem> {
        let binary = self.target_dir().join(profile.dir()).join(&script.name);
        if !binary.is_file() {
            announce_dependencies(script);
        }
        let flags: &[&str] = &["build", "--quiet"];
        self.cargo(script, flags, profile)?;
        Ok(binary)
    }

    fn cargo(&self, script: &Script, args: &[&str], profile: Profile) -> Result<(), Problem> {
        let command = self.cargo_command(script, args, profile)?;
        cargo_status(command, script, false).map(|_| ())
    }

    fn cargo_command(&self, script: &Script, args: &[&str], profile: Profile) -> Result<Command, Problem> {
        let manifest = self.write_cargo_project(script)?;
        let mut command = Command::new("cargo");
        command.args(args).arg("--manifest-path").arg(manifest).arg("--target-dir").arg(self.target_dir());
        if profile == Profile::Release {
            command.arg("--release");
        }
        Ok(command)
    }

    fn target_dir(&self) -> PathBuf {
        self.cache.join("cargo-target")
    }

    /// One tiny project per script path, holding only a `Cargo.toml`.
    fn write_cargo_project(&self, script: &Script) -> Result<PathBuf, Problem> {
        let id = Fnv64::new().field(script.path.to_string_lossy().as_bytes()).hex();
        let dir = self.cache.join("projects").join(format!("{}-{}", script.name, &id[..8]));
        fs::create_dir_all(&dir).map_err(|e| report::io("create the project directory", &dir, &e))?;

        let manifest = dir.join("Cargo.toml");
        let wanted = cargo_manifest(script);
        if fs::read_to_string(&manifest).ok().as_deref() != Some(wanted.as_str()) {
            fs::write(&manifest, wanted)
                .map_err(|e| report::io("write the generated Cargo.toml", &manifest, &e))?;
        }
        Ok(manifest)
    }
}

fn uses_cargo(script: &Script) -> bool {
    !script.manifest.deps.is_empty()
}

fn rustc(script: &Script, profile: Profile) -> Command {
    let mut command = Command::new("rustc");
    command
        .arg("--edition")
        .arg(&script.manifest.edition)
        .arg("--crate-name")
        .arg(crate_name(&script.name))
        .arg("--color")
        .arg(if term::stderr_color() { "always" } else { "never" });
    if profile == Profile::Release {
        command.args(["-C", "opt-level=3"]);
    }
    command.arg(&script.path);
    command
}

/// Crate names are identifiers: `my-script` becomes `my_script`.
fn crate_name(script_name: &str) -> String {
    let name: String = script_name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    if name.starts_with(|c: char| c.is_ascii_digit()) { format!("_{name}") } else { name }
}

/// rustc output is captured so it can be framed, and so warnings appear only
/// when something was actually compiled (never when the cache is hit).
fn run_rustc(mut command: Command, script: &Script) -> Result<(), Problem> {
    let output = command.stdin(Stdio::null()).output().map_err(|e| toolchain_missing("rustc", &e))?;
    let messages = String::from_utf8_lossy(&output.stderr);

    if output.status.success() {
        if !messages.trim().is_empty() {
            eprint!("{messages}");
        }
        return Ok(());
    }
    Err(compile_error(script).raw(messages.into_owned()).hint(retry_hint(script)))
}

fn cargo_status(mut command: Command, script: &Script, tests: bool) -> Result<u8, Problem> {
    // Cargo talks to the terminal itself: downloads and compiler output appear live.
    let status = command.status().map_err(|e| toolchain_missing("cargo", &e))?;
    let code = exec::code(status);
    if status.success() || tests {
        return Ok(code);
    }
    Err(compile_error(script).text("Cargo reported the problems above.").hint(retry_hint(script)))
}

fn compile_error(script: &Script) -> Problem {
    Problem::new("COMPILE ERROR").context(script.label())
}

fn retry_hint(script: &Script) -> String {
    match script.edit_hint() {
        Some(edit) => format!("Fix the errors above, then continue with `{edit}`."),
        None => "Fix the errors above and run the command again.".to_string(),
    }
}

fn toolchain_missing(tool: &str, error: &std::io::Error) -> Problem {
    if error.kind() == std::io::ErrorKind::NotFound {
        Problem::new("RUST NOT FOUND")
            .text(format!("I could not find `{tool}` on your PATH."))
            .hint("Install Rust from https://rustup.rs, open a new terminal, and try again.")
    } else {
        Problem::new("CANNOT START COMPILER").text(format!("I could not start `{tool}`: {error}"))
    }
}

/// Progress note, only when something will actually be compiled and a person
/// is watching. Silent when piped, so scripts' output stays clean.
fn announce(script: &Script) {
    if term::stderr_color() {
        eprintln!("{} {}", term::paint("Compiling", Style::Dim, true), script.name);
    }
}

fn announce_dependencies(script: &Script) {
    if term::stderr_color() {
        let names: Vec<&str> = script.manifest.deps.iter().map(|d| d.name.as_str()).collect();
        eprintln!(
            "{} {} (first build fetches {}; later runs are instant)",
            term::paint("Compiling", Style::Dim, true),
            script.name,
            names.join(", ")
        );
    }
}

/// The generated `Cargo.toml`. The binary target points at the script where
/// it already lives, so there is never a second copy of the source.
fn cargo_manifest(script: &Script) -> String {
    let dependencies: String =
        script.manifest.deps.iter().map(|dep| format!("{} = {}\n", dep.name, dep.spec)).collect();
    let path = script.path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");

    format!(
        "# Generated by func. Edits will be overwritten.\n\
         [package]\n\
         name = \"func-{name}\"\n\
         version = \"0.0.0\"\n\
         edition = \"{edition}\"\n\
         publish = false\n\
         \n\
         [[bin]]\n\
         name = \"{name}\"\n\
         path = \"{path}\"\n\
         \n\
         [dependencies]\n\
         {dependencies}\n\
         [profile.dev]\n\
         debug = 0\n\
         \n\
         [workspace]\n",
        name = script.name,
        edition = script.manifest.edition,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::Origin;

    fn script(name: &str, source: &str) -> Script {
        Script::from_source(name, Path::new("/scripts/x.rs"), source.to_string(), Origin::Store).unwrap()
    }

    #[test]
    fn crate_names_are_identifiers() {
        assert_eq!(crate_name("my-script"), "my_script");
        assert_eq!(crate_name("ok_name"), "ok_name");
    }

    #[test]
    fn cache_key_tracks_content_profile_and_kind() {
        let builder = Builder::new(PathBuf::from("/cache"));
        let a = script("a", "fn main() {}");
        let changed = script("a", "fn main() { }");

        let base = builder.artifact(&a, Profile::Debug, Kind::Binary);
        assert_eq!(base, builder.artifact(&a, Profile::Debug, Kind::Binary));
        assert_ne!(base, builder.artifact(&changed, Profile::Debug, Kind::Binary));
        assert_ne!(base, builder.artifact(&a, Profile::Release, Kind::Binary));
        assert_ne!(base, builder.artifact(&a, Profile::Debug, Kind::Tests));
        assert!(base.ends_with("a"));
    }

    #[test]
    fn manifest_points_at_the_script_and_lists_deps() {
        let s = script("tool", "// dep: regex = \"1\"\n// edition: 2021\nfn main() {}");
        let toml = cargo_manifest(&s);
        assert!(toml.contains("path = \"/scripts/x.rs\""));
        assert!(toml.contains("regex = \"1\""));
        assert!(toml.contains("edition = \"2021\""));
        assert!(toml.contains("[workspace]"));
    }

    #[test]
    fn only_scripts_with_deps_use_cargo() {
        assert!(!uses_cargo(&script("a", "fn main() {}")));
        assert!(uses_cargo(&script("a", "// dep: rand\nfn main() {}")));
    }
}
