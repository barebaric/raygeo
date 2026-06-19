.PHONY: build dev stubs format format-rust format-python lint lint-rust lint-python test check visual doc docs install-test-deps install-visual-deps install-docs-deps

build:
	maturin build --release --out dist

dev: install-test-deps
	maturin develop

install-test-deps:
	pip install -e ".[test]" --quiet

install-visual-deps:
	pip install -e ".[visual]" --quiet

install-docs-deps:
	pip install -e ".[docs]" --quiet

stubs:
	cargo clean -p raygeo && cargo run --bin stub_gen

format: format-rust format-python

format-rust:
	cargo fmt

format-python:
	ruff format tests/ python/ tools/
	ruff check --fix tests/ python/ tools/

lint: lint-rust lint-python

lint-rust:
	cargo fmt --check
	cargo clippy -- -D warnings

lint-python:
	ruff check tests/ python/ tools/
	ruff format --check tests/ python/ tools/
	npx pyright python/raygeo tests tools

test:
	pytest -v

check: lint test

visual: install-visual-deps
	streamlit run tools/visual_test.py

doc docs: install-docs-deps
	python -m tools.cli all
	npx prettier --write --prose-wrap always --print-width 100 "docs/**/*.md"
