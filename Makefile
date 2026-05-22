.PHONY: build dev stubs format format-rust format-python lint lint-rust lint-python test check

build:
	maturin build --release --out dist

dev:
	maturin develop

stubs:
	cargo run --bin stub_gen

format: format-rust format-python

format-rust:
	cargo fmt

format-python:
	ruff format tests/ python/
	ruff check --fix tests/ python/

lint: lint-rust lint-python

lint-rust:
	cargo fmt --check
	cargo clippy -- -D warnings

lint-python:
	ruff check tests/ python/
	ruff format --check tests/ python/
	npx pyright python/raygeo tests

test:
	pytest -v

check: lint test
