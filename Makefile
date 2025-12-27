.PHONY: run build check test test-update clean resume continue lint update release codex claude reinstall-hooks

PROVIDER_FILE := .crabigator-provider
DEFAULT_PROVIDER := claude

run:
	@provider=$$(cat $(PROVIDER_FILE) 2>/dev/null | head -n 1 | tr -d ' \t\r\n'); \
	if [ -z "$$provider" ]; then provider=$(DEFAULT_PROVIDER); fi; \
	echo "Using provider: $$provider"; \
	RUST_BACKTRACE=1 cargo run -- $$provider

resume:
	@provider=$$(cat $(PROVIDER_FILE) 2>/dev/null | head -n 1 | tr -d ' \t\r\n'); \
	if [ -z "$$provider" ]; then provider=$(DEFAULT_PROVIDER); fi; \
	echo "Using provider: $$provider"; \
	RUST_BACKTRACE=1 cargo run -- $$provider --resume

continue:
	@provider=$$(cat $(PROVIDER_FILE) 2>/dev/null | head -n 1 | tr -d ' \t\r\n'); \
	if [ -z "$$provider" ]; then provider=$(DEFAULT_PROVIDER); fi; \
	echo "Using provider: $$provider"; \
	RUST_BACKTRACE=1 cargo run -- $$provider --continue

codex:
	@echo "codex" > $(PROVIDER_FILE)
	@$(MAKE) run

claude:
	@echo "claude" > $(PROVIDER_FILE)
	@$(MAKE) run

build:
	cargo build

release:
	cargo build --release

check:
	cargo check

test:
	cargo test

test-update:
	CRABIGATOR_UPDATE_FIXTURES=1 cargo test

update: test-update
	@true

lint:
	cargo clippy

clean:
	cargo clean

reinstall-hooks:
	@if [ ! -f ~/.claude/crabigator/hooks-meta.json ]; then \
		echo "Hooks not installed yet. Will install on next crabigator session."; \
	else \
		version=$$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/'); \
		current_hash=$$(sed "s/{VERSION}/$$version/" src/platforms/claude_code/stats_hook.py | md5 -q 2>/dev/null || sed "s/{VERSION}/$$version/" src/platforms/claude_code/stats_hook.py | md5sum | cut -d' ' -f1); \
		installed_hash=$$(grep '"script_hash"' ~/.claude/crabigator/hooks-meta.json 2>/dev/null | sed 's/.*: *"\([^"]*\)".*/\1/'); \
		if [ "$$current_hash" = "$$installed_hash" ]; then \
			echo "Hooks are already up to date (hash: $$current_hash)."; \
		else \
			echo "Hook script changed (installed: $$installed_hash, current: $$current_hash)"; \
			rm -f ~/.claude/crabigator/hooks-meta.json; \
			echo "Cleared hooks metadata. Will reinstall on next crabigator session."; \
		fi \
	fi
