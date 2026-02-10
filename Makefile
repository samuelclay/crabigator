.PHONY: run build check test test-update clean resume continue lint update release codex codex-yolo claude claude-yolo reinstall-hooks deploy typecheck cf-usage dev reset-usage sync-usage e2e-codex-tmux

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

codex-yolo:
	@echo "codex" > $(PROVIDER_FILE)
	RUST_BACKTRACE=1 cargo run -- codex --full-auto

claude:
	@echo "claude" > $(PROVIDER_FILE)
	@$(MAKE) run

claude-yolo:
	@echo "claude" > $(PROVIDER_FILE)
	RUST_BACKTRACE=1 cargo run -- claude --dangerously-skip-permissions

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

deploy:
	cd workers/crabigator-api && npm run deploy

dev:
	cd workers/crabigator-api && npm run dev

typecheck:
	cd workers/crabigator-api && npm run typecheck

cf-usage:
	@./scripts/cf-usage.sh

e2e-codex-tmux:
	@./scripts/e2e-codex-tmux.sh

reset-usage:
	@echo "Resetting all usage for today..."
	@today=$$(date -u +%Y-%m-%d); \
	cd workers/crabigator-api && \
	npx wrangler d1 execute crabigator --remote --command "DELETE FROM daily_usage WHERE date = '$$today'" && \
	echo "Usage reset for $$today. Note: Durable Objects may still have cached state until they're accessed again."

sync-usage:
	@if [ -z "$(GROUP)" ]; then \
		echo "Usage: make sync-usage GROUP=<group_id>"; \
		echo ""; \
		echo "Find group_id with:"; \
		cd workers/crabigator-api && npx wrangler d1 execute crabigator --remote --command "SELECT d.group_id, d.name FROM devices d WHERE d.group_id IS NOT NULL GROUP BY d.group_id ORDER BY MAX(d.last_seen_at) DESC LIMIT 10"; \
	else \
		curl -s -X POST 'https://drinkcrabigator.com/api/staff/sync-usage' \
			-H 'Content-Type: application/json' \
			-d "{\"group_id\":\"$(GROUP)\"}" && echo ""; \
	fi
