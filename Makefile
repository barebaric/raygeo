.PHONY: build dev stubs format format-rust format-python lint lint-rust lint-python test test-fast test-all test-geometry test-engine test-encode test-driver test-slow check check-wasm visual doc docs install-test-deps install-visual-deps install-docs-deps venv

# Prefer the rustup toolchain when one exists: distro cargos are
# often too old for this project's Cargo.lock format (v4), and cargo
# resolves rustc via PATH, so the rustup bin dir must come first.
ifneq ($(wildcard $(HOME)/.cargo/bin/cargo),)
export PATH := $(HOME)/.cargo/bin:$(PATH)
endif

build:
	maturin build --release --out dist

dev: install-test-deps
	maturin develop --profile dev

install-test-deps:
	pip install -e ".[test,cli]" --quiet

install-visual-deps:
	pip install -e ".[visual]" --quiet

install-docs-deps:
	pip install -e ".[docs]" --quiet

venv:
	python3 -m venv .venv
	@echo "Run 'source .venv/bin/activate' to activate the virtual environment."

stubs:
	cargo clean -p raygeo && cargo run --bin stub_gen --features python

format: format-rust format-python

format-rust:
	cargo fmt

format-python:
	ruff format tests/ python/ tools/
	ruff check --fix tests/ python/ tools/

lint: lint-rust lint-python

# Mirror the CI clippy passes: the python feature gates the PyO3
# bindings, the pure-Rust core is what embedding consumers build.
lint-rust:
	cargo fmt --check
	cargo clippy --lib --features python -- -D warnings
	cargo clippy --lib --no-default-features -- -D warnings

lint-python:
	ruff check tests/ python/ tools/
	ruff format --check tests/ python/ tools/
	npx pyright python/raygeo tests tools

# Embedding applications build the core for wasm32. The second check
# compiles the browser bundle (currently the Web Serial transport).
check-wasm:
	cargo check --lib --target wasm32-unknown-unknown --no-default-features
	cargo check --lib --target wasm32-unknown-unknown --no-default-features --features browser
	cargo clippy --lib --target wasm32-unknown-unknown --no-default-features --features browser -- -D warnings

test: test-fast

# Fast lane: no slow/stress/hardware tests, parallel by module. This is
# the default local and PR target.
test-fast:
	cargo test --lib --no-default-features
	pytest -v -m "not slow and not stress and not hardware" \
		-n auto --dist loadscope

# Everything, including slow/stress tests. Use before releases/nightly.
test-all:
	cargo test
	pytest -v -n auto --dist loadscope

test-geometry:
	pytest -v tests/geo tests/formats tests/image tests/sketch \
		tests/mesh tests/barcode tests/template

test-engine:
	pytest -v tests/ops tests/cnc tests/pipeline tests/transform

test-encode:
	pytest -v tests/encode

test-driver:
	pytest -v tests/driver

test-slow:
	pytest -v -m "slow or stress" -n 0

check: lint test

visual: install-visual-deps
	streamlit run tools/visual_test.py

doc docs: install-docs-deps
	python -m tools.cli all
