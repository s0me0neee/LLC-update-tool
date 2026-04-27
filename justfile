test_cache_dir := "./test/llc/cache/"
test_lang_dir := "./test/llc/lang/"
lbc_lang_dir := "./test/LimbusCompany_Data/Lang/"

rust_log := "info"
cargo := "cargo"

default:
    @just --list

help:
    @just --list

run: clean
    RUST_LOG={{rust_log}} {{cargo}} run

build:
    {{cargo}} build

check:
    {{cargo}} check

test:
    {{cargo}} test

test_verbose:
    {{cargo}} test --verbose

test_integration:
    LLC_RUN_INTEGRATION_TESTS=1 {{cargo}} test

fmt:
    {{cargo}} fmt

clippy:
    {{cargo}} clippy --all-targets --all-features

prepare_test_dirs:
    mkdir -p {{test_cache_dir}} {{test_lang_dir}} {{lbc_lang_dir}}

clean:
    rm -rf {{test_cache_dir}} {{test_lang_dir}} {{lbc_lang_dir}}

