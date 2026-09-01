.PHONY: run build check test test-update clean resume continue lint update release codex codex-yolo claude claude-yolo opencode grok grok-yolo pr prs reinstall-hooks deploy typecheck cf-usage dev reset-usage sync-usage e2e-codex-tmux portability-check

PROVIDER_FILE := .crabigator-provider
DEFAULT_PROVIDER := claude
WRANGLER_CONFIG ?= wrangler.jsonc
WRANGLER_PROFILE ?=
D1_DATABASE ?= DB
WRANGLER_FLAGS := --config $(WRANGLER_CONFIG) $(if $(WRANGLER_PROFILE),--profile $(WRANGLER_PROFILE))

run:
	@provider=$$(cat $(PROVIDER_FILE) 2>/dev/null | head -n 1 | tr -d ' \t\r\n'); \
	if [ -z "$$provider" ]; then provider=$(DEFAULT_PROVIDER); fi; \
	echo "Using provider: $$provider"; \
	extra=""; \
	if [ "$$provider" = "claude" ]; then extra="--dangerously-skip-permissions"; fi; \
	RUST_BACKTRACE=1 cargo run -- $$provider $$extra

resume:
	@provider=$$(cat $(PROVIDER_FILE) 2>/dev/null | head -n 1 | tr -d ' \t\r\n'); \
	if [ -z "$$provider" ]; then provider=$(DEFAULT_PROVIDER); fi; \
	echo "Using provider: $$provider"; \
	extra=""; \
	if [ "$$provider" = "claude" ]; then extra="--dangerously-skip-permissions"; fi; \
	RUST_BACKTRACE=1 cargo run -- $$provider --resume $$extra

continue:
	@provider=$$(cat $(PROVIDER_FILE) 2>/dev/null | head -n 1 | tr -d ' \t\r\n'); \
	if [ -z "$$provider" ]; then provider=$(DEFAULT_PROVIDER); fi; \
	echo "Using provider: $$provider"; \
	extra=""; \
	if [ "$$provider" = "claude" ]; then extra="--dangerously-skip-permissions"; fi; \
	RUST_BACKTRACE=1 cargo run -- $$provider --continue $$extra

codex:
	@echo "codex" > $(PROVIDER_FILE)
	@$(MAKE) run

codex-yolo:
	@echo "codex" > $(PROVIDER_FILE)
	env -u CODEX_THREAD_ID -u CODEX_ROLLOUT_PATH RUST_BACKTRACE=1 cargo run -- codex --dangerously-bypass-approvals-and-sandbox

claude:
	@echo "claude" > $(PROVIDER_FILE)
	@$(MAKE) run

opencode:
	@echo "opencode" > $(PROVIDER_FILE)
	@$(MAKE) run

grok:
	@echo "grok" > $(PROVIDER_FILE)
	@$(MAKE) run

grok-yolo:
	@echo "grok" > $(PROVIDER_FILE)
	RUST_BACKTRACE=1 cargo run -- grok --yolo

claude-yolo:
	@echo "claude" > $(PROVIDER_FILE)
	RUST_BACKTRACE=1 cargo run -- claude --dangerously-skip-permissions

# Live cross-session PR board
prs:
	RUST_BACKTRACE=1 cargo run --release -- prs

pr: prs

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

portability-check:
	@./scripts/check-portability.sh

cf-usage:
	@./scripts/cf-usage.sh

e2e-codex-tmux:
	@./scripts/e2e-codex-tmux.sh

reset-usage:
	@echo "Resetting all usage for today..."
	@today=$$(date -u +%Y-%m-%d); \
	cd workers/crabigator-api && \
	wrangler d1 execute $(D1_DATABASE) --remote $(WRANGLER_FLAGS) --command "DELETE FROM daily_usage WHERE date = '$$today'" && \
	echo "Usage reset for $$today. Note: Durable Objects may still have cached state until they're accessed again."

sync-usage:
	@if [ -z "$(GROUP)" ]; then \
		echo "Usage: make sync-usage GROUP=<group_id>"; \
		echo ""; \
		echo "Find group_id with:"; \
		cd workers/crabigator-api && wrangler d1 execute $(D1_DATABASE) --remote $(WRANGLER_FLAGS) --command "SELECT d.group_id, d.name FROM devices d WHERE d.group_id IS NOT NULL GROUP BY d.group_id ORDER BY MAX(d.last_seen_at) DESC LIMIT 10"; \
	else \
		if [ -z "$(CLOUD_URL)" ] || [ -z "$(STAFF_COOKIE)" ]; then echo "Set CLOUD_URL and STAFF_COOKIE."; exit 1; fi; \
		curl -s -X POST '$(CLOUD_URL)/api/staff/sync-usage' \
			-H 'Content-Type: application/json' \
			-H 'Origin: $(CLOUD_URL)' \
			-H 'Cookie: crabigator_staff=$(STAFF_COOKIE)' \
			-d "{\"group_id\":\"$(GROUP)\"}" && echo ""; \
	fi
