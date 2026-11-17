# Everyday tasks. `make help` lists them.

.DEFAULT_GOAL := help
.PHONY: help build test lint fmt completions install uninstall clean

help: ## Show this help
	@grep -E '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk -F':.*## ' '{printf "  %-12s %s\n", $$1, $$2}'

build: ## Build an optimized binary into target/release
	cargo build --release

test: ## Run unit and end-to-end tests
	cargo test

lint: ## Check formatting and run clippy (what CI runs)
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

fmt: ## Format the code
	cargo fmt

completions: ## Regenerate completions/ after changing a command or flag
	cargo run -q -- completions bash > completions/func.bash
	cargo run -q -- completions zsh  > completions/_func
	cargo run -q -- completions fish > completions/func.fish

install: ## Install func (and completions) to ~/.local/bin
	./install.sh

uninstall: ## Remove func (keeps your scripts)
	./uninstall.sh

clean: ## Delete build output
	cargo clean
