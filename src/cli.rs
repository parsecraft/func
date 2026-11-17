//! Command-line parsing. `argv` in, [`Command`] out; nothing here touches the
//! file system.
//!
//! [`COMMANDS`] is the single list of commands. Help text, "did you mean"
//! suggestions, completions and the completion-drift test are all derived from it, so adding a command means adding one row here (plus its
//! parser arm and its handler).

use crate::report::Problem;
use crate::suggest;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Edit,
    Run,
    Test,
    Check,
    Eval,
    Install,
    List,
    Show,
    Path,
    Rename,
    Remove,
    Clean,
    Completions,
    Help,
}

pub struct Spec {
    pub kind: Kind,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    /// Shown after the command name in usage lines.
    pub args: &'static str,
    /// One line, shown in `func help`.
    pub summary: &'static str,
    /// A few lines, shown in `func help <command>`.
    pub long: &'static str,
    /// Every flag spelling the command accepts.
    pub flags: &'static [&'static str],
    /// What the command's first positional argument completes to.
    pub target: Target,
}

/// What a shell should offer when completing a command's argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Nothing: free text, or no argument at all.
    None,
    /// One saved script name.
    Script,
    /// Saved script names, repeated (`func rm a b c`).
    Scripts,
    /// A saved script name, or a path to a `.rs` file.
    ScriptOrFile,
    Shell,
    Command,
}

pub const COMMANDS: &[Spec] = &[
    Spec {
        kind: Kind::Edit,
        name: "edit",
        aliases: &["new", "e"],
        args: "<name>",
        summary: "Create or edit a script, then type-check it",
        long: "Opens the script in $VISUAL or $EDITOR (creating it from a template if it is new).\n\
               When you close the editor the script is type-checked. If there are errors you can\n\
               reopen the editor right away, with your changes intact.",
        flags: &[],
        target: Target::Script,
    },
    Spec {
        kind: Kind::Run,
        name: "run",
        aliases: &["r"],
        args: "[-r] <name|file> [args...]",
        summary: "Compile (once, then cached) and run a script",
        long: "Everything after the script name goes to the script, flags included.\n\
               `func r hello` is the short form. Scripts only ever run through `run`, so a script\n\
               named `edit` or `list` can never be confused with the command of that name.\n\
               To get a plain `hello` command in your shell, use `func install hello`.\n\
               A path (./tool.rs) runs that file in place, without importing it.\n\
               -r, --release   optimize the build (slower to compile, faster to run)",
        flags: &["-r", "--release"],
        target: Target::ScriptOrFile,
    },
    Spec {
        kind: Kind::Test,
        name: "test",
        aliases: &["t"],
        args: "<name|file> [filter...]",
        summary: "Run the script's #[test] functions",
        long: "Builds the script with the test harness and runs it. Extra words filter tests by\n\
               name, exactly like `cargo test <filter>`.",
        flags: &[],
        target: Target::ScriptOrFile,
    },
    Spec {
        kind: Kind::Check,
        name: "check",
        aliases: &[],
        args: "<name|file>",
        summary: "Type-check a script without building it",
        long: "The fastest way to find compile errors and warnings. Produces no binary.",
        flags: &[],
        target: Target::ScriptOrFile,
    },
    Spec {
        kind: Kind::Eval,
        name: "eval",
        aliases: &[],
        args: "[-r] <code>",
        summary: "Run a snippet without saving it",
        long: "The code becomes the body of `fn main`, so end statements with `;` and print\n\
               explicitly: func eval 'println!(\"{}\", 6 * 7);'\n\
               Code that already contains `fn main` is compiled as a whole program.",
        flags: &["-r", "--release"],
        target: Target::None,
    },
    Spec {
        kind: Kind::Install,
        name: "install",
        aliases: &[],
        args: "[--force] <name|file>",
        summary: "Build a release binary and put it on your PATH",
        long: "Copies an optimized build to the bin directory (~/.local/bin, or $FUNC_BIN).\n\
               func will not overwrite a program it did not install itself unless you pass --force.",
        flags: &["-f", "--force"],
        target: Target::ScriptOrFile,
    },
    Spec {
        kind: Kind::List,
        name: "list",
        aliases: &["ls"],
        args: "[--names]",
        summary: "List your scripts",
        long: "Shows each script with the first `//!` line of its header as the description.\n\
               --names prints bare names, one per line, for use in other programs.",
        flags: &["--names"],
        target: Target::None,
    },
    Spec {
        kind: Kind::Show,
        name: "show",
        aliases: &["cat"],
        args: "<name>",
        summary: "Print a script's source",
        long: "Writes the source to stdout, so `func show hello > hello.rs` makes a copy.",
        flags: &[],
        target: Target::Script,
    },
    Spec {
        kind: Kind::Path,
        name: "path",
        aliases: &[],
        args: "[name]",
        summary: "Print where a script (or the script directory) lives",
        long: "Handy in other commands: `cd $(func path)`, or `wc -l $(func path hello)`.",
        flags: &[],
        target: Target::Script,
    },
    Spec {
        kind: Kind::Rename,
        name: "rename",
        aliases: &["mv"],
        args: "<old> <new>",
        summary: "Rename a script",
        long: "Fails rather than overwrite an existing script.",
        flags: &[],
        target: Target::Script,
    },
    Spec {
        kind: Kind::Remove,
        name: "remove",
        aliases: &["rm"],
        args: "[-y] <name>...",
        summary: "Delete scripts",
        long: "Asks for confirmation in a terminal; -y, --yes skips the question.",
        flags: &["-y", "--yes"],
        target: Target::Scripts,
    },
    Spec {
        kind: Kind::Clean,
        name: "clean",
        aliases: &[],
        args: "",
        summary: "Delete cached builds",
        long: "Your scripts are never touched. The next run of each script recompiles it.",
        flags: &[],
        target: Target::None,
    },
    Spec {
        kind: Kind::Completions,
        name: "completions",
        aliases: &[],
        args: "<bash|zsh|fish>",
        summary: "Print a shell completion script",
        long: "Typical setup:\n\
               bash   eval \"$(func completions bash)\"          (in ~/.bashrc)\n\
               zsh    func completions zsh > ~/.zfunc/_func    (with ~/.zfunc in $fpath)\n\
               fish   func completions fish > ~/.config/fish/completions/func.fish",
        flags: &[],
        target: Target::Shell,
    },
    Spec {
        kind: Kind::Help,
        name: "help",
        aliases: &[],
        args: "[command]",
        summary: "Show help",
        long: "`func help` lists commands; `func help run` explains one. `func <command> --help` works too.",
        flags: &[],
        target: Target::Command,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    pub const NAMES: [&'static str; 3] = ["bash", "zsh", "fish"];

    fn parse(name: &str) -> Option<Self> {
        match name {
            "bash" => Some(Shell::Bash),
            "zsh" => Some(Shell::Zsh),
            "fish" => Some(Shell::Fish),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Edit { name: String },
    Run { target: String, args: Vec<String>, release: bool },
    Test { target: String, filter: Vec<String> },
    Check { target: String },
    Eval { code: String, release: bool },
    Install { target: String, force: bool },
    List { names_only: bool },
    Show { name: String },
    Path { name: Option<String> },
    Rename { from: String, to: String },
    Remove { names: Vec<String>, yes: bool },
    Clean,
    Completions { shell: Shell },
    Help { topic: Option<String> },
    Version,
}

pub fn find(name: &str) -> Option<&'static Spec> {
    COMMANDS.iter().find(|spec| spec.name == name || spec.aliases.contains(&name))
}

pub fn parse(args: Vec<String>) -> Result<Command, Problem> {
    let Some((first, rest)) = args.split_first() else {
        return Ok(Command::Help { topic: None });
    };

    match first.as_str() {
        "-h" | "--help" => return Ok(Command::Help { topic: None }),
        "-V" | "--version" => return Ok(Command::Version),
        _ => {}
    }
    if let Some(spec) = find(first) {
        return parse_command(spec, rest.to_vec());
    }
    if first.starts_with('-') {
        return Err(unknown_top_level_flag(first));
    }
    // Commands and scripts never share a namespace: scripts run only through `func run`.
    Err(unknown_command(first))
}

fn parse_command(spec: &'static Spec, rest: Vec<String>) -> Result<Command, Problem> {
    if wants_help(spec, &rest) {
        return Ok(Command::Help { topic: Some(spec.name.to_string()) });
    }

    match spec.kind {
        Kind::Edit => Ok(Command::Edit { name: only_positional(spec, rest, "a script name")? }),
        Kind::Run => {
            let (mut flags, mut tail) = leading_flags(rest);
            let release = flags.take(&["-r", "--release"]);
            flags.finish(spec)?;
            let target = take_first(spec, &mut tail, "a script name or file")?;
            if tail.first().is_some_and(|arg| arg == "--") {
                tail.remove(0); // `func run x -- args` works like cargo; the separator is not an argument
            }
            Ok(Command::Run { target, args: tail, release })
        }
        Kind::Test => {
            let (flags, mut tail) = leading_flags(rest);
            flags.finish(spec)?;
            let target = take_first(spec, &mut tail, "a script name or file")?;
            Ok(Command::Test { target, filter: tail })
        }
        Kind::Check => Ok(Command::Check { target: only_positional(spec, rest, "a script name or file")? }),
        Kind::Eval => {
            let (mut flags, tail) = leading_flags(rest);
            let release = flags.take(&["-r", "--release"]);
            flags.finish(spec)?;
            if tail.is_empty() {
                return Err(missing(spec, "some code to run"));
            }
            Ok(Command::Eval { code: tail.join(" "), release })
        }
        Kind::Install => {
            let (mut flags, positional) = partition(rest);
            let force = flags.take(&["-f", "--force"]);
            flags.finish(spec)?;
            Ok(Command::Install {
                target: only_positional(spec, positional, "a script name or file")?,
                force,
            })
        }
        Kind::List => {
            let (mut flags, positional) = partition(rest);
            let names_only = flags.take(&["--names"]);
            flags.finish(spec)?;
            no_positional(spec, positional)?;
            Ok(Command::List { names_only })
        }
        Kind::Show => Ok(Command::Show { name: only_positional(spec, rest, "a script name")? }),
        Kind::Path => {
            let mut positional = no_flags(spec, rest)?;
            if positional.len() > 1 {
                return Err(too_many(spec, &positional[1..]));
            }
            Ok(Command::Path { name: positional.pop() })
        }
        Kind::Rename => {
            let mut positional = no_flags(spec, rest)?;
            if positional.len() < 2 {
                let what =
                    if positional.is_empty() { "the current name and the new name" } else { "the new name" };
                return Err(missing(spec, what));
            }
            if positional.len() > 2 {
                return Err(too_many(spec, &positional[2..]));
            }
            let to = positional.remove(1);
            Ok(Command::Rename { from: positional.remove(0), to })
        }
        Kind::Remove => {
            let (mut flags, names) = partition(rest);
            let yes = flags.take(&["-y", "--yes"]);
            flags.finish(spec)?;
            if names.is_empty() {
                return Err(missing(spec, "at least one script name"));
            }
            Ok(Command::Remove { names, yes })
        }
        Kind::Clean => {
            no_positional(spec, no_flags(spec, rest)?)?;
            Ok(Command::Clean)
        }
        Kind::Completions => {
            let name = only_positional(spec, rest, "a shell name")?;
            Shell::parse(&name).map(|shell| Command::Completions { shell }).ok_or_else(|| {
                Problem::new("UNKNOWN SHELL")
                    .context(format!("func completions {name}"))
                    .text(format!("I do not have completions for `{name}`."))
                    .text(format!("I can write them for: {}.", Shell::NAMES.join(", ")))
                    .usage()
            })
        }
        Kind::Help => {
            let mut positional = no_flags(spec, rest)?;
            if positional.len() > 1 {
                return Err(too_many(spec, &positional[1..]));
            }
            Ok(Command::Help { topic: positional.pop() })
        }
    }
}

// -- argument helpers ---------------------------------------------------------

/// Flags collected from the command line, removed as the parser claims them.
/// Whatever is left over is a mistake.
struct Flags(Vec<String>);

impl Flags {
    fn take(&mut self, spellings: &[&str]) -> bool {
        let before = self.0.len();
        self.0.retain(|flag| !spellings.contains(&flag.as_str()));
        self.0.len() != before
    }

    fn finish(self, spec: &Spec) -> Result<(), Problem> {
        match self.0.first() {
            Some(flag) => Err(unknown_flag(spec, flag)),
            None => Ok(()),
        }
    }
}

fn is_flag(arg: &str) -> bool {
    arg.starts_with('-') && arg.len() > 1
}

/// Flags may appear anywhere: `func rm a -y b`.
fn partition(args: Vec<String>) -> (Flags, Vec<String>) {
    let (flags, positional): (Vec<_>, Vec<_>) = args.into_iter().partition(|arg| is_flag(arg));
    (Flags(flags), positional)
}

/// Flags end at the first positional (or at `--`): everything after belongs
/// to the script or to the code being evaluated.
fn leading_flags(mut args: Vec<String>) -> (Flags, Vec<String>) {
    let split = args.iter().position(|arg| !is_flag(arg)).unwrap_or(args.len());
    let mut tail = args.split_off(split);
    if tail.first().is_some_and(|arg| arg == "--") {
        tail.remove(0);
    }
    (Flags(args), tail)
}

fn wants_help(spec: &Spec, args: &[String]) -> bool {
    // After the script name, `-h` belongs to the script.
    let leading_only = matches!(spec.kind, Kind::Run | Kind::Test | Kind::Eval);
    for arg in args {
        if arg == "-h" || arg == "--help" {
            return true;
        }
        if leading_only && !is_flag(arg) {
            return false;
        }
    }
    false
}

fn no_flags(spec: &Spec, args: Vec<String>) -> Result<Vec<String>, Problem> {
    let (flags, positional) = partition(args);
    flags.finish(spec)?;
    Ok(positional)
}

fn no_positional(spec: &Spec, positional: Vec<String>) -> Result<(), Problem> {
    match positional.first() {
        Some(_) => Err(too_many(spec, &positional)),
        None => Ok(()),
    }
}

fn only_positional(spec: &Spec, args: Vec<String>, what: &str) -> Result<String, Problem> {
    let mut positional = no_flags(spec, args)?;
    let first = take_first(spec, &mut positional, what)?;
    no_positional(spec, positional)?;
    Ok(first)
}

fn take_first(spec: &Spec, args: &mut Vec<String>, what: &str) -> Result<String, Problem> {
    if args.is_empty() { Err(missing(spec, what)) } else { Ok(args.remove(0)) }
}

// -- errors -------------------------------------------------------------------

pub fn usage_line(spec: &Spec) -> String {
    format!("func {} {}", spec.name, spec.args).trim_end().to_string()
}

fn missing(spec: &Spec, what: &str) -> Problem {
    Problem::new("MISSING ARGUMENT")
        .context(format!("func {}", spec.name))
        .text(format!("`func {}` needs {what}. Here is how it is used:", spec.name))
        .code(usage_line(spec))
        .hint(format!("Run `func help {}` for details.", spec.name))
        .usage()
}

fn too_many(spec: &Spec, extra: &[String]) -> Problem {
    Problem::new("TOO MANY ARGUMENTS")
        .context(format!("func {}", spec.name))
        .text("I did not expect these:")
        .code(extra.join(" "))
        .text("Here is how the command is used:")
        .code(usage_line(spec))
        .hint(format!("Run `func help {}` for details. Quote arguments that contain spaces.", spec.name))
        .usage()
}

fn unknown_flag(spec: &Spec, flag: &str) -> Problem {
    let mut problem = Problem::new("UNKNOWN FLAG")
        .context(format!("func {} {flag}", spec.name))
        .text(format!("`func {}` does not have a flag called `{flag}`.", spec.name));

    problem = if spec.flags.is_empty() {
        problem.text("This command takes no flags.")
    } else {
        problem.text(format!("It understands: {}", spec.flags.join(", ")))
    };

    let hint = match suggest::closest(flag, spec.flags.iter().copied()).first() {
        Some(close) => format!("Did you mean `{close}`?"),
        None => format!("Run `func help {}` for details.", spec.name),
    };
    problem.hint(hint).usage()
}

fn unknown_command(word: &str) -> Problem {
    let mut problem = Problem::new("UNKNOWN COMMAND")
        .context(format!("func {word}"))
        .text(format!("I do not have a command called `{word}`."));

    let close = suggest::closest(word, COMMANDS.iter().map(|spec| spec.name));
    if !close.is_empty() {
        problem = problem.text("Did you mean one of these?").code(close.join("\n"));
    }
    problem
        .hint(format!("To run a script called `{word}`, use `func run {word}`. All commands: `func help`."))
        .usage()
}

fn unknown_top_level_flag(flag: &str) -> Problem {
    let hint = match flag {
        "-r" | "--release" => "Flags come after the command: `func run -r <script>`.".to_string(),
        _ => "Run `func help` to see the commands.".to_string(),
    };
    Problem::new("UNKNOWN FLAG")
        .context(format!("func {flag}"))
        .text(format!(
            "`{flag}` is not a flag I know. On their own I only understand `--help` and `--version`."
        ))
        .hint(hint)
        .usage()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(line: &str) -> Result<Command, Problem> {
        parse(line.split_whitespace().map(str::to_string).collect())
    }

    fn ok(line: &str) -> Command {
        parse_str(line).unwrap_or_else(|p| panic!("`{line}` failed:\n{}", p.render(false)))
    }

    fn err(line: &str) -> String {
        parse_str(line).expect_err(line).render(false)
    }

    fn run(target: &str, args: &[&str], release: bool) -> Command {
        Command::Run { target: target.into(), args: args.iter().map(|a| a.to_string()).collect(), release }
    }

    #[test]
    fn no_arguments_shows_help() {
        assert_eq!(ok(""), Command::Help { topic: None });
        assert_eq!(ok("--help"), Command::Help { topic: None });
        assert_eq!(ok("-V"), Command::Version);
    }

    #[test]
    fn aliases_reach_the_same_command() {
        assert_eq!(ok("new hello"), Command::Edit { name: "hello".into() });
        assert_eq!(ok("e hello"), ok("edit hello"));
        assert_eq!(ok("ls"), Command::List { names_only: false });
        assert_eq!(ok("mv a b"), Command::Rename { from: "a".into(), to: "b".into() });
    }

    #[test]
    fn run_passes_everything_after_the_target_to_the_script() {
        assert_eq!(ok("run hello -x --y z"), run("hello", &["-x", "--y", "z"], false));
        assert_eq!(ok("run -r hello a"), run("hello", &["a"], true));
        assert_eq!(ok("run hello -h"), run("hello", &["-h"], false));
    }

    #[test]
    fn a_double_dash_after_the_target_is_dropped() {
        assert_eq!(ok("run hello -- --flag"), run("hello", &["--flag"], false));
    }

    #[test]
    fn a_bare_name_is_not_a_command_and_points_to_run() {
        let text = err("hello a b");
        assert!(text.contains("UNKNOWN COMMAND"));
        assert!(text.contains("use `func run hello`"));
    }

    #[test]
    fn mistyped_commands_get_suggestions_instead_of_running_a_script() {
        assert!(err("edti x").contains("    edit"));
        assert!(err("lst").contains("    list"));
    }

    #[test]
    fn test_collects_filters() {
        assert_eq!(
            ok("test hello parse"),
            Command::Test { target: "hello".into(), filter: vec!["parse".into()] }
        );
    }

    #[test]
    fn eval_joins_words_so_quotes_are_optional() {
        assert_eq!(ok("eval -r let x = 1;"), Command::Eval { code: "let x = 1;".into(), release: true });
    }

    #[test]
    fn flags_may_appear_anywhere_for_simple_commands() {
        assert_eq!(ok("rm a -y b"), Command::Remove { names: vec!["a".into(), "b".into()], yes: true });
        assert_eq!(ok("list --names"), Command::List { names_only: true });
        assert_eq!(ok("install --force x"), Command::Install { target: "x".into(), force: true });
    }

    #[test]
    fn help_is_available_per_command() {
        assert_eq!(ok("run --help"), Command::Help { topic: Some("run".into()) });
        assert_eq!(ok("rm -h"), Command::Help { topic: Some("remove".into()) });
        assert_eq!(ok("help run"), Command::Help { topic: Some("run".into()) });
    }

    #[test]
    fn missing_arguments_say_what_is_needed() {
        let text = err("edit");
        assert!(text.contains("MISSING ARGUMENT"));
        assert!(text.contains("func edit <name>"));
        assert!(err("rename a").contains("the new name"));
    }

    #[test]
    fn extra_arguments_are_named() {
        let text = err("show a b c");
        assert!(text.contains("TOO MANY ARGUMENTS"));
        assert!(text.contains("    b c"));
    }

    #[test]
    fn unknown_flags_suggest_the_closest_spelling() {
        assert!(err("run --releas x").contains("Did you mean `--release`?"));
        assert!(err("list --bogus").contains("It understands: --names"));
        assert!(err("clean -x").contains("takes no flags"));
    }

    #[test]
    fn top_level_flags_are_explained() {
        assert!(err("-r hello").contains("func run -r <script>"));
    }

    #[test]
    fn unknown_shells_list_the_known_ones() {
        assert!(err("completions powershell").contains("bash, zsh, fish"));
        assert_eq!(ok("completions zsh"), Command::Completions { shell: Shell::Zsh });
    }

    #[test]
    fn every_command_name_and_alias_is_unique() {
        let mut seen = std::collections::HashSet::new();
        for spec in COMMANDS {
            for name in std::iter::once(&spec.name).chain(spec.aliases) {
                assert!(seen.insert(*name), "`{name}` is used twice");
            }
        }
    }

    #[test]
    fn script_names_need_no_reserving_because_scripts_only_run_through_run() {
        assert_eq!(ok("run edit"), run("edit", &[], false));
        assert_eq!(ok("r list"), run("list", &[], false));
    }
}
