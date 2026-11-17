//! End-to-end tests: run the real binary against a throwaway $FUNC_HOME.
//!
//! These need `rustc` on PATH (they compile real scripts), which is always
//! true wherever `cargo test` runs.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// A private $FUNC_HOME, removed when dropped.
struct Sandbox {
    home: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let home = std::env::temp_dir().join(format!("func-test-{}-{id}", std::process::id()));
        fs::create_dir_all(home.join("scripts")).unwrap();
        Self { home }
    }

    fn script(&self, name: &str, source: &str) -> &Self {
        fs::write(self.home.join("scripts").join(format!("{name}.rs")), source).unwrap();
        self
    }

    fn func(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_func"))
            .args(args)
            .env("FUNC_HOME", &self.home)
            .env("NO_COLOR", "1")
            .env_remove("EDITOR")
            .env_remove("VISUAL")
            .stdin(Stdio::null())
            .output()
            .expect("failed to start func")
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.home);
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn code(output: &Output) -> i32 {
    output.status.code().expect("terminated by signal")
}

const HELLO: &str = "//! Says hello.\nfn main() { println!(\"hello {}\", std::env::args().skip(1).collect::<Vec<_>>().join(\" \")); }\n";

#[test]
fn runs_a_saved_script_by_name_with_arguments() {
    let sandbox = Sandbox::new();
    sandbox.script("hello", HELLO);

    for args in [&["run", "hello", "a", "b"][..], &["r", "hello", "a", "b"]] {
        let out = sandbox.func(args);
        assert_eq!(code(&out), 0, "{}", stderr(&out));
        assert_eq!(stdout(&out), "hello a b\n");
    }
}

#[test]
fn flags_after_the_script_name_belong_to_the_script() {
    let sandbox = Sandbox::new();
    sandbox.script("hello", HELLO);
    assert_eq!(stdout(&sandbox.func(&["run", "hello", "-r", "--help"])), "hello -r --help\n");
}

#[test]
fn script_exit_codes_pass_through() {
    let sandbox = Sandbox::new();
    sandbox.script("fail", "fn main() { std::process::exit(7); }\n");
    assert_eq!(code(&sandbox.func(&["run", "fail"])), 7);
}

#[test]
fn runs_a_file_in_place_without_importing_it() {
    let sandbox = Sandbox::new();
    let file = sandbox.home.join("loose.rs");
    fs::write(&file, HELLO).unwrap();

    let out = sandbox.func(&["run", file.to_str().unwrap(), "x"]);
    assert_eq!(stdout(&out), "hello x\n");
    assert!(sandbox.func(&["list", "--names"]).stdout.is_empty(), "running a file must not save it");
}

#[test]
fn compile_errors_are_framed_and_fail() {
    let sandbox = Sandbox::new();
    sandbox.script("bad", "fn main() { let x: i32 = \"no\"; }\n");

    let out = sandbox.func(&["run", "bad"]);
    let text = stderr(&out);
    assert_eq!(code(&out), 1);
    assert!(text.starts_with("-- COMPILE ERROR"), "{text}");
    assert!(text.contains("mismatched types"));
    assert!(text.contains("Hint: Fix the errors above, then continue with `func edit bad`."));
}

#[test]
fn check_reports_success_and_failure() {
    let sandbox = Sandbox::new();
    sandbox.script("good", HELLO).script("bad", "fn main() { nope(); }\n");
    assert_eq!(code(&sandbox.func(&["check", "good"])), 0);
    assert_eq!(code(&sandbox.func(&["check", "bad"])), 1);
}

#[test]
fn test_runs_the_scripts_own_tests_and_passes_filters() {
    let sandbox = Sandbox::new();
    sandbox.script(
        "t",
        "fn main() {}\n#[test] fn adds() { assert_eq!(1 + 1, 2); }\n#[test] fn fails() { assert!(false); }\n",
    );

    let passing = sandbox.func(&["test", "t", "adds"]);
    assert_eq!(code(&passing), 0, "{}", stdout(&passing));
    assert!(stdout(&passing).contains("1 passed"));
    assert_ne!(code(&sandbox.func(&["test", "t"])), 0);
}

#[test]
fn eval_runs_snippets() {
    let sandbox = Sandbox::new();
    assert_eq!(stdout(&sandbox.func(&["eval", "println!(\"{}\", 6 * 7);"])), "42\n");
}

#[test]
fn unknown_scripts_get_suggestions_and_a_usage_exit_code_only_for_misuse() {
    let sandbox = Sandbox::new();
    sandbox.script("hello", HELLO);

    let out = sandbox.func(&["run", "helo"]);
    assert_eq!(code(&out), 1);
    let text = stderr(&out);
    assert!(text.contains("UNKNOWN SCRIPT") && text.contains("    hello"), "{text}");

    assert_eq!(code(&sandbox.func(&["run"])), 2, "missing argument is a usage error");
}

#[test]
fn bad_header_directives_point_at_the_line() {
    let sandbox = Sandbox::new();
    sandbox.script("hdr", "//! ok\n// edition: 1999\nfn main() {}\n");
    let text = stderr(&sandbox.func(&["run", "hdr"]));
    assert!(text.contains("BAD HEADER LINE") && text.contains("hdr:2"), "{text}");
}

#[test]
fn list_shows_descriptions_and_bare_names() {
    let sandbox = Sandbox::new();
    sandbox.script("hello", HELLO).script("plain", "fn main() {}\n");

    assert_eq!(stdout(&sandbox.func(&["list", "--names"])), "hello\nplain\n");
    let table = stdout(&sandbox.func(&["ls"]));
    assert!(table.contains("hello  Says hello."), "{table}");
    assert!(table.lines().any(|line| line == "plain"), "{table}");
}

#[test]
fn remove_requires_confirmation_without_a_terminal() {
    let sandbox = Sandbox::new();
    sandbox.script("hello", HELLO);

    assert_eq!(code(&sandbox.func(&["rm", "hello"])), 1);
    assert!(sandbox.home.join("scripts/hello.rs").exists());
    assert_eq!(code(&sandbox.func(&["rm", "-y", "hello"])), 0);
    assert!(!sandbox.home.join("scripts/hello.rs").exists());
}

#[test]
fn rename_never_overwrites() {
    let sandbox = Sandbox::new();
    sandbox.script("a", HELLO).script("b", HELLO);
    assert_eq!(code(&sandbox.func(&["mv", "a", "b"])), 1);
    assert_eq!(code(&sandbox.func(&["mv", "a", "c"])), 0);
    assert_eq!(stdout(&sandbox.func(&["list", "--names"])), "b\nc\n");
}

#[test]
fn install_copies_a_release_build_and_refuses_to_clobber() {
    let sandbox = Sandbox::new();
    sandbox.script("hello", HELLO);

    assert_eq!(code(&sandbox.func(&["install", "hello"])), 0);
    let installed = sandbox.home.join("bin/hello");
    let out = Command::new(&installed).arg("again").output().unwrap();
    assert_eq!(stdout(&out), "hello again\n");

    // Reinstalling our own program is fine; a stranger's program is protected.
    assert_eq!(code(&sandbox.func(&["install", "hello"])), 0);
    sandbox.script("other", HELLO);
    fs::write(sandbox.home.join("bin/other"), "not ours").unwrap();
    assert_eq!(code(&sandbox.func(&["install", "other"])), 1);
    assert_eq!(code(&sandbox.func(&["install", "--force", "other"])), 0);
}

#[test]
fn names_cannot_escape_the_store() {
    let sandbox = Sandbox::new();
    assert_eq!(code(&sandbox.func(&["show", "../scripts/x"])), 1);
    assert_eq!(code(&sandbox.func(&["edit", "../evil"])), 2);
}

#[test]
fn path_prints_script_locations() {
    let sandbox = Sandbox::new();
    sandbox.script("hello", HELLO);
    let dir = stdout(&sandbox.func(&["path"]));
    assert!(Path::new(dir.trim()).ends_with("scripts"));
    assert!(stdout(&sandbox.func(&["path", "hello"])).trim().ends_with("hello.rs"));
}

#[test]
fn clean_removes_only_the_cache() {
    let sandbox = Sandbox::new();
    sandbox.script("hello", HELLO);
    sandbox.func(&["run", "hello"]);
    assert!(sandbox.home.join("cache").exists());
    assert_eq!(code(&sandbox.func(&["clean"])), 0);
    assert!(!sandbox.home.join("cache").exists());
    assert!(sandbox.home.join("scripts/hello.rs").exists());
}

#[test]
fn committed_completions_match_the_generator() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("completions");
    let sandbox = Sandbox::new();
    for (shell, file) in [("bash", "func.bash"), ("zsh", "_func"), ("fish", "func.fish")] {
        let generated = stdout(&sandbox.func(&["completions", shell]));
        let committed = fs::read_to_string(root.join(file)).unwrap_or_default();
        assert_eq!(
            generated, committed,
            "completions/{file} is stale. Regenerate: func completions {shell} > completions/{file}"
        );
    }
}

#[test]
fn bash_completion_is_valid_bash() {
    let Ok(_) = Command::new("bash").arg("--version").output() else { return };
    let sandbox = Sandbox::new();
    let script = stdout(&sandbox.func(&["completions", "bash"]));
    let status = Command::new("bash")
        .arg("-n")
        .stdin(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(script.as_bytes())?;
            child.wait()
        })
        .unwrap();
    assert!(status.success());
}

/// What bash would offer after the given words, using the real completion script.
fn bash_completes(sandbox: &Sandbox, words: &[&str]) -> Vec<String> {
    bash_completes_in(sandbox, &sandbox.home, words)
}

/// Like `bash_completes`, with the shell's working directory set (completion depends on it).
fn bash_completes_in(sandbox: &Sandbox, cwd: &Path, words: &[&str]) -> Vec<String> {
    let exe = Path::new(env!("CARGO_BIN_EXE_func"));
    let path = format!("{}:{}", exe.parent().unwrap().display(), std::env::var("PATH").unwrap_or_default());
    let quoted: Vec<String> = words.iter().map(|w| format!("'{w}'")).collect();
    let program = format!(
        "eval \"$(func completions bash)\"; COMP_WORDS=({}); COMP_CWORD=$(( ${{#COMP_WORDS[@]}} - 1 )); _func; printf '%s\\n' \"${{COMPREPLY[@]}}\"",
        quoted.join(" ")
    );
    let out = Command::new("bash")
        .args(["-c", &program])
        .env("PATH", path)
        .env("FUNC_HOME", &sandbox.home)
        .current_dir(cwd)
        .output()
        .expect("bash should start");
    // An empty COMPREPLY still prints one blank line; that is not a completion.
    stdout(&out).lines().filter(|line| !line.is_empty()).map(str::to_string).collect()
}

#[test]
fn first_word_completes_commands_only_never_scripts() {
    if Command::new("bash").arg("--version").output().is_err() {
        return;
    }
    let sandbox = Sandbox::new();
    for i in 0..50 {
        sandbox.script(&format!("tool{i}"), "fn main() {}\n");
    }
    sandbox.script("hello", HELLO).script("edit2", HELLO);

    let empty = bash_completes(&sandbox, &["func", ""]);
    assert!(empty.contains(&"edit".to_string()), "{empty:?}");
    assert!(!empty.iter().any(|w| w == "hello" || w.starts_with("tool")), "{empty:?}");

    // `edit2` is a script, so it never competes with the `edit` command.
    assert_eq!(bash_completes(&sandbox, &["func", "ed"]), ["edit"]);
    assert_eq!(bash_completes(&sandbox, &["func", "he"]), ["help"]);

    // Where a script is expected, scripts are offered.
    let after_run = bash_completes(&sandbox, &["func", "run", ""]);
    assert!(after_run.contains(&"hello".to_string()) && after_run.contains(&"edit2".to_string()));
}

#[test]
fn a_bare_name_is_an_unknown_command_that_points_to_run() {
    let sandbox = Sandbox::new();
    sandbox.script("hello", HELLO);

    let out = sandbox.func(&["hello"]);
    let text = stderr(&out);
    assert_eq!(code(&out), 2);
    assert!(text.contains("UNKNOWN COMMAND") && text.contains("func run hello"), "{text}");

    let typo = stderr(&sandbox.func(&["edti", "x"]));
    assert!(typo.contains("    edit"), "{typo}");
}

#[test]
fn scripts_may_share_a_name_with_commands_without_confusion() {
    let sandbox = Sandbox::new();
    sandbox.script("list", "fn main() { println!(\"I am the script\"); }\n");

    assert_eq!(stdout(&sandbox.func(&["run", "list"])), "I am the script\n");
    assert_eq!(stdout(&sandbox.func(&["list", "--names"])), "list\n", "`func list` is still the command");

    let text = stderr(&sandbox.func(&["run", "rm"]));
    assert!(text.contains("is a func command, not a script"), "{text}");
}

/// The examples are documentation people copy. They must always work.
/// Examples with `// dep:` lines need the network to build, so for those we only
/// check that they are listed with their description.
#[test]
fn examples_compile_or_are_listed() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let sandbox = Sandbox::new();
    let mut seen = 0;

    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let source = fs::read_to_string(&path).unwrap();
        let name = path.file_stem().unwrap().to_str().unwrap();
        seen += 1;

        if source.contains("// dep:") {
            fs::copy(&path, sandbox.home.join("scripts").join(path.file_name().unwrap())).unwrap();
            let listing = stdout(&sandbox.func(&["list"]));
            assert!(listing.contains(name), "{name} should be listed: {listing}");
        } else {
            let out = sandbox.func(&["check", path.to_str().unwrap()]);
            assert_eq!(code(&out), 0, "{name} does not compile:\n{}", stderr(&out));
        }
    }
    assert!(seen >= 3, "the examples directory should hold the examples");
}

#[test]
fn run_completion_offers_scripts_and_rs_files_never_every_file() {
    if Command::new("bash").arg("--version").output().is_err() {
        return;
    }
    // Scripts live in `home/scripts`; the shell sits in a cluttered project directory.
    let sandbox = Sandbox::new();
    sandbox.script("hello", HELLO);
    let cwd = sandbox.home.join("project");
    fs::create_dir_all(cwd.join("src")).unwrap();
    for file in ["notes.txt", "Cargo.toml", "tool.rs"] {
        fs::write(cwd.join(file), "").unwrap();
    }
    fs::write(cwd.join("src/lib.rs"), "").unwrap();
    let at = |words: &[&str]| bash_completes_in(&sandbox, &cwd, words);

    // `func run <Tab>`: scripts and .rs files only, no notes.txt, Cargo.toml or folders.
    let first = at(&["func", "run", ""]);
    assert_eq!(first, ["hello", "tool.rs"]);

    // Nothing matches: stay empty rather than falling back to listing everything.
    assert!(at(&["func", "run", "zzz"]).is_empty());

    // Path-like words walk through folders and still show only .rs files.
    assert_eq!(at(&["func", "run", "./"]), ["./src/", "./tool.rs"]);
    assert_eq!(at(&["func", "run", "src/"]), ["src/lib.rs"]);

    // After the script name the arguments belong to the script: normal file completion
    // (bash falls back to its own default, so we only assert that we do not interfere).
    assert!(at(&["func", "run", "hello", ""]).len() <= 5);
}
