#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --lib
cargo test --locked
cargo llvm-cov --locked --lib --show-missing-lines --fail-under-lines 95