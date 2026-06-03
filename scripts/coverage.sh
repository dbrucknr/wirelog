#!/usr/bin/env bash
# Full coverage report. Equivalent to `cargo cov` but runnable as a script.

set -euo pipefail
cargo llvm-cov --show-missing-lines --ignore-filename-regex "rustc-.*-src"
