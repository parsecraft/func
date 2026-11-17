# Design

func does one thing: turn "I have an idea in Rust" into "it runs" with as little ceremony as a shell function. This page explains how the code is organized and why, so changes stay small.

## Map

```text
src/
  main.rs        argv -> parse -> dispatch -> exit code. Nothing else.
  cli.rs         The command table (COMMANDS) and the parser. Pure: no I/O.
  commands/      One handler per command, grouped by theme.
    run.rs         run, test, check, eval
    edit.rs        edit (the funced loop)
    install.rs     install (the funcsave step)
    manage.rs      list, show, path, rename, remove, clean
    info.rs        help, version, completions
  store.rs       The script directory: names, files, validation.
  script.rs      Reading a script: header directives, description, templates.
  build.rs       Compiling and caching (rustc backend, cargo backend).
  exec.rs        Running programs and mapping exit codes.
  paths.rs       Where things live (XDG, FUNC_HOME).
  report.rs      Problem: the one way errors are shown.
  completions.rs Shell completion generators, driven by cli::COMMANDS.
  editor.rs, term.rs, suggest.rs, hash.rs   Small, single-purpose helpers.
tests/cli.rs     End-to-end tests against the real binary.
completions/     Generated files, kept in sync by a test.
```

Dependencies point downward: `commands` use `store`, `script`, `build`; those use `report` and the helpers. Nothing below `commands` knows what a command is, except `store` (to reserve names) and `completions`, both reading the table.

## Decisions

**No dependencies.** The binary builds in seconds, starts in a millisecond, and there is nothing to audit or keep updated. The few things that normally justify a crate (argument parsing, colors, hashing, edit distance) are each under 60 lines here and fully tested.

**Commands and scripts never share a namespace.** Scripts run only through `func run`; there is no bare `func <name>` shorthand. That removes a whole class of confusion (`edit` vs a script called `edit2`, or a typo silently becoming "run a script") and means script names need no reserved words. The parser is pure: an unknown first word is simply an unknown command, with suggestions. `func install` is the way to get a bare command, as a real program on PATH.

**One source of truth per fact.**
- Commands, aliases, flags, help text and completion targets live in `cli::COMMANDS`. Help, "did you mean" and all three completion generators read it. Committed completion files are compared against the generator in a test, so they cannot go stale.
- A script's source lives in one file. Saved scripts live in the store; loose files run where they are.

**Two build backends, chosen by need.** Scripts without dependencies are compiled by `rustc` directly into a content-addressed cache (`cache/builds/<hash>/<name>`). The key covers name, edition, profile, kind and source, so there is no staleness check, only existence. Scripts with `// dep:` lines get a generated `Cargo.toml` whose `[[bin]] path` points at the script itself, with one shared `--target-dir` so each crate compiles once. Cargo is only paid for when it is needed.

**`exec`, not spawn.** On Unix `func run name` replaces itself with the script. There is no wrapper to forward signals, stdin or exit codes, and nothing to show up in `ps`.

**Atomic writes.** Builds go to a temporary name and are renamed into place; installs do the same. An interrupted or parallel build never leaves a broken binary.

**Errors are a type.** Every user-visible failure is a `report::Problem` (title, context, explanation, hint, exit code). Handlers return `Result<u8, Problem>`; only `main` prints. Messages are written for the person, in full sentences, and suggest the next command.

**Safe by construction.**
- Names are validated against a strict pattern. `Store::exists` refuses malformed names, so `../x` can never reach the file system.
- Destructive actions ask first (or need `-y`); a closed stdin means "no".
- `install` never overwrites a program it did not create, tracked in a small ledger, unless `--force`.
- A compile error in `edit` never loses work: the file is saved before it is checked.

## Adding a command

1. Add a `Kind` variant and a row in `cli::COMMANDS` (name, aliases, usage, summary, details, flags, completion target).
2. Add its parser arm in `cli::parse_command`, reusing the helpers (`only_positional`, `partition`, `leading_flags`).
3. Add the handler in the matching `commands/*.rs` file and a line in `commands::dispatch`.
4. Add parser unit tests in `cli.rs` and an end-to-end test in `tests/cli.rs`.
5. Regenerate `completions/*` (a test tells you if you forget).

Help and completions need no further work.

## Not included, on purpose

Things the other 1% asks for, left out to keep func small: a REPL, watch mode, scripts that import each other, remote scripts, Windows `exec` semantics (it falls back to spawn-and-wait), sandboxing. Most are one `cargo` command away when you outgrow a single file.
