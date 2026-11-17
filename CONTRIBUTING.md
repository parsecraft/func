# Contributing

Thanks for helping. func is small on purpose, so the bar for a change is: does it make the everyday path simpler, safer or faster, without growing the surface?

## Set up

You need Rust (stable). Then:

```sh
git clone https://github.com/parsecraft/func
cd func
make test        # cargo test
make lint        # cargo fmt --check, cargo clippy -D warnings
```

`make help` lists the other tasks.

## Before you open a pull request

1. Read [docs/DESIGN.md](docs/DESIGN.md). It is short, and it explains where things go.
2. Add a test with the change: a unit test beside the code, and an end-to-end test in `tests/cli.rs` if behavior a person can see changed.
3. If you changed a command or flag, run `make completions`. A test fails until the committed completion files match.
4. Run `make lint test`. CI runs the same.
5. Update `CHANGELOG.md` under `[Unreleased]`, and the README if usage changed.

## What a good change looks like

- **No new dependencies.** func has none. If you think one is justified, open an issue first.
- **Errors are written for people.** Use `report::Problem`: say what happened, show the relevant thing, suggest the next step. See the existing messages for tone.
- **One source of truth.** Commands live in `cli::COMMANDS`; do not repeat names in help, completions or error text by hand.
- **Names say what things are.** Prefer a clear name to a comment, and a small function to a long one.
- **Small pull requests** that do one thing are reviewed faster.

## Out of scope

func stays a single-file script runner. A REPL, watch mode, scripts importing each other and remote scripts are deliberately not planned (see the end of DESIGN.md). Issues proposing them will usually be closed with an explanation, so please ask first.

## Reporting bugs

Open an issue with the template. The most useful reports include the exact command, what you expected, what happened, and `func --version`.

Security problems: see [SECURITY.md](SECURITY.md) instead of opening a public issue.
