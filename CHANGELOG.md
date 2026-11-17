# Changelog

All notable changes are listed here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed
- Bash completion was a syntax error on macOS's default bash 3.2 (it used an extended glob). It now sticks to bash 3.2 syntax, and a test keeps it that way.
- `install.sh` passes ShellCheck.
- Tab after `func run` (and `test`, `check`, `install`) listed every file and folder in the current directory. It now offers saved scripts and `.rs` files, plus folders once you type a path. After the script name, normal file completion applies, since those arguments belong to the script.

## [0.1.0]

First release.

### Added
- `func edit`: create or edit a script in `$VISUAL`/`$EDITOR`, type-check on close, reopen on errors without losing work.
- `func run` (`r`), `test`, `check`, `eval`: build once, cache by content, run with `exec`.
- `func install`: put an optimized build on your PATH, never overwriting programs func did not install.
- `func list`, `show`, `path`, `rename`, `rm`, `clean`.
- Crates via header comments (`// dep: regex = "1"`), built through Cargo with one shared target directory.
- Elm-style error messages with hints and "did you mean" suggestions.
- Completions for bash, zsh and fish, generated from the command table.
- `install.sh` and `uninstall.sh`.

### Design
- Scripts run only through `func run`, so commands and scripts never share a namespace.

[Unreleased]: https://github.com/parsecraft/func/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/parsecraft/func/releases/tag/v0.1.0
