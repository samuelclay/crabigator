.PHONY: run build check test test-update clean resume continue lint update

run:
	RUST_BACKTRACE=1 cargo run

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
