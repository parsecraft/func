//! Command handlers. Each takes a [`Ctx`] (where things live) and returns the
//! process exit code, or a [`Problem`] to show.

mod edit;
mod info;
mod install;
mod manage;
mod run;

use crate::build::Builder;
use crate::cli::{self, Command};
use crate::paths::Dirs;
use crate::report::Problem;
use crate::script::{Origin, Script};
use crate::store::Store;
use std::path::Path;

pub struct Ctx {
    pub dirs: Dirs,
    pub store: Store,
    pub builder: Builder,
}

impl Ctx {
    fn new() -> Result<Self, Problem> {
        let dirs = Dirs::discover()?;
        Ok(Self { store: Store::new(dirs.scripts.clone()), builder: Builder::new(dirs.cache.clone()), dirs })
    }
}

pub fn dispatch(command: Command) -> Result<u8, Problem> {
    match command {
        // Commands that need no directories work even when $HOME is unset.
        Command::Help { topic } => info::help(topic.as_deref()),
        Command::Version => info::version(),
        Command::Completions { shell } => info::completions(shell),

        Command::Edit { name } => edit::edit(&Ctx::new()?, &name),
        Command::Run { target, args, release } => run::run(&Ctx::new()?, &target, &args, release),
        Command::Test { target, filter } => run::test(&Ctx::new()?, &target, &filter),
        Command::Check { target } => run::check(&Ctx::new()?, &target),
        Command::Eval { code, release } => run::eval(&Ctx::new()?, &code, release),
        Command::Install { target, force } => install::install(&Ctx::new()?, &target, force),
        Command::List { names_only } => manage::list(&Ctx::new()?, names_only),
        Command::Show { name } => manage::show(&Ctx::new()?, &name),
        Command::Path { name } => manage::path(&Ctx::new()?, name.as_deref()),
        Command::Rename { from, to } => manage::rename(&Ctx::new()?, &from, &to),
        Command::Remove { names, yes } => manage::remove(&Ctx::new()?, &names, yes),
        Command::Clean => manage::clean(&Ctx::new()?),
    }
}

/// Turns what the person typed into a script: a saved name (`hello`) or a
/// file path (`./hello.rs`). Paths run in place; nothing is imported.
pub fn load_target(ctx: &Ctx, arg: &str) -> Result<Script, Problem> {
    let looks_like_path = arg.contains('/') || arg.ends_with(".rs");
    if !looks_like_path {
        return load_stored(ctx, arg);
    }

    let path = Path::new(arg);
    if path.is_file() {
        let name = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("script");
        return Script::load(name, path, Origin::File);
    }
    // `func run hello.rs` where only the saved script `hello` exists.
    if let Some(stem) = arg.strip_suffix(".rs").filter(|stem| !stem.contains('/') && ctx.store.exists(stem)) {
        return load_stored(ctx, stem);
    }
    Err(Problem::new("FILE NOT FOUND")
        .context(arg)
        .text("I could not find this file:")
        .code(arg)
        .hint("Check the path, or run `func list` to see your saved scripts."))
}

fn load_stored(ctx: &Ctx, name: &str) -> Result<Script, Problem> {
    if ctx.store.exists(name) {
        return Script::load(name, &ctx.store.path(name), Origin::Store);
    }
    Err(unknown_script(ctx, name))
}

pub fn unknown_script(ctx: &Ctx, name: &str) -> Problem {
    let saved = ctx.store.names().unwrap_or_default();
    let close = crate::suggest::closest(name, saved.iter().map(String::as_str));

    let mut problem = Problem::new("UNKNOWN SCRIPT")
        .context(name)
        .text(format!("I could not find a script named `{name}`."));

    // `func run list` usually means the command `func list`.
    if cli::find(name).is_some() {
        problem =
            problem.text(format!("`{name}` is a func command, not a script. Did you mean `func {name}`?"));
    }
    if !close.is_empty() {
        problem = problem.text("Did you mean one of these scripts?").code(close.join("\n"));
    } else if saved.is_empty() {
        problem = problem.text("You do not have any scripts yet.");
    }
    problem.hint(format!("Create it with `func edit {name}`, or see what you have with `func list`."))
}

/// Used by commands that only accept saved scripts (`show`, `rename`, `remove`).
pub fn require_stored(ctx: &Ctx, name: &str) -> Result<(), Problem> {
    if ctx.store.exists(name) { Ok(()) } else { Err(unknown_script(ctx, name)) }
}
