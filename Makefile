.PHONY: run build check test test-update clean resume continue lint update claude codex

# Default target uses last-used platform (or claude if none saved)
run:
	RUST_BACKTRACE=1 cargo run

# Platform-specific targets (saves preference for future runs)
claude:
	RUST_BACKTRACE=1 cargo run -- claude

codex:
	RUST_BACKTRACE=1 cargo run -- codex

resume:
	RUST_BACKTRACE=1 cargo run -- --resume

continue:
	RUST_BACKTRACE=1 cargo run -- --continue

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
