default:
    @just --list

fmt:
    cargo fmt

lint:
    cargo clippy -- -D warnings

test:
    cargo test

check: fmt lint test

publish:
    cargo publish
