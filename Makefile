.PHONY: build
build:
	cargo build

.PHONY: build-release
build-release:
	cargo build --release

.PHONY: test
test:
	cargo test

.PHONY: lint
lint:
	cargo fmt -- --check
	cargo clippy
	cargo machete
	cargo audit

.PHONY: fix-lints
fix-lints:
	cargo fmt
	cargo clippy --fix

.PHONY: docs
docs:
	cargo doc --open

.PHONY: clean
clean:
	cargo clean

