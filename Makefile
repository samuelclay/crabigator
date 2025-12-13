.PHONY: run build check test clean resume continue

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

clean:
	cargo clean
