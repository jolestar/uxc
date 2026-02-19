.PHONY: all build test clean install run

all: build

build:
	cargo build --release

test:
	cargo test

clean:
	cargo clean

install: build
	cargo install --path .

run:
	cargo run

dev:
	cargo run -- --

check:
	cargo check
	cargo clippy

fmt:
	cargo fmt

docs:
	cargo doc --open
