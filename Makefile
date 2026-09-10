PREFIX ?= $(HOME)/.local

.PHONY: build debug check test fmt clippy install uninstall clean

build:
	cargo build --locked --release

debug:
	cargo build --locked

check: fmt clippy test
	bash -n bin/tide install.sh scripts/install.sh scripts/uninstall.sh
	bash tests/launcher-test.sh

test:
	cargo test --locked

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --all-targets --all-features -- -D warnings

install:
	./scripts/install.sh --prefix "$(PREFIX)"

uninstall:
	./scripts/uninstall.sh --prefix "$(PREFIX)"

clean:
	cargo clean
