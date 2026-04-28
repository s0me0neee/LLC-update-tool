TEST_CACHE_DIR := ./test/llc/cache
TEST_LANG_DIR := ./test/llc/lang
LBC_LANG_DIR := ./test/LimbusCompany_Data/Lang

CARGO ?= cargo
RUST_LOG ?= info

.PHONY: default help build check run test test-integration test-verbose fmt clippy prepare-test-dirs clean

default: help

help:
	@echo "Available targets:"
	@echo "  make build              - Build"
	@echo "  make check              - Cargo check"
	@echo "  make run                - Clean and run (RUST_LOG=$(RUST_LOG))"
	@echo "  make test               - Run tests"
	@echo "  make test-verbose        - Run tests (verbose)"
	@echo "  make test-integration    - Run integration tests (TEST=1)"
	@echo "  make fmt                - Format (cargo fmt)"
	@echo "  make clippy             - Lint (cargo clippy)"
	@echo "  make prepare-test-dirs  - Create local test directories"
	@echo "  make clean              - Remove local test directories"

build:
	$(CARGO) build

check:
	$(CARGO) check

run: clean
	RUST_LOG=$(RUST_LOG) $(CARGO) run

test:
	$(CARGO) test

test-verbose:
	$(CARGO) test --verbose

test-integration:
	TEST=1 $(CARGO) test

fmt:
	$(CARGO) fmt

clippy:
	$(CARGO) clippy --all-targets --all-features

prepare-test-dirs:
	mkdir -p $(TEST_CACHE_DIR) $(TEST_LANG_DIR) $(LBC_LANG_DIR)

clean:
	rm -rf $(TEST_CACHE_DIR) $(TEST_LANG_DIR) $(LBC_LANG_DIR)
