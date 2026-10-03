#!/usr/bin/env bash
cargo fmt
cargo clippy
cargo test
cargo llvm-cov --lib --show-missing-lines
