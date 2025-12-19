.PHONY: run build check test test-update clean resume continue lint update release codex claude

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
