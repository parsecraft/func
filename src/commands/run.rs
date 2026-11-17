//! Commands that build and run: `run`, `test`, `check`, `eval`.

use super::{Ctx, load_target};
use crate::build::Profile;
use crate::exec;
use crate::hash::Fnv64;
use crate::report::{self, Problem};
use crate::script::{self, Origin, Script};
use std::fs;

pub fn run(ctx: &Ctx, target: &str, args: &[String], release: bool) -> Result<u8, Problem> {
    let script = load_target(ctx, target)?;
    let binary = ctx.builder.compile(&script, Profile::from_flag(release))?;
    exec::run(&binary, args)
}

pub fn test(ctx: &Ctx, target: &str, filter: &[String]) -> Result<u8, Problem> {
    let script = load_target(ctx, target)?;
    ctx.builder.test(&script, filter)
}

pub fn check(ctx: &Ctx, target: &str) -> Result<u8, Problem> {
    let script = load_target(ctx, target)?;
    ctx.builder.check(&script)?;
    eprintln!("`{}` type-checks.", script.name);
    Ok(0)
}

/// Snippets are stored by content hash, so repeating one reuses its build
/// and nothing accumulates beyond one small file per distinct snippet.
pub fn eval(ctx: &Ctx, code: &str, release: bool) -> Result<u8, Problem> {
    let source = script::eval_source(code);
    let dir = ctx.builder.cache_dir().join("eval");
    let path = dir.join(format!("{}.rs", Fnv64::new().field(source.as_bytes()).hex()));

    if !path.is_file() {
        fs::create_dir_all(&dir).map_err(|e| report::io("create the cache directory", &dir, &e))?;
        fs::write(&path, &source).map_err(|e| report::io("write the snippet", &path, &e))?;
    }

    let script = Script::from_source("eval", &path, source, Origin::File)?;
    let binary = ctx.builder.compile(&script, Profile::from_flag(release))?;
    exec::run(&binary, &[])
}
