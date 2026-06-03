#!/usr/bin/env bash
# Full coverage report. Equivalent to `cargo cov` but runnable as a script.
#
# Remaining uncovered lines after a clean run:
#   non_blocking.rs: 76-79, 88-91 — two defensive BrokenPipe error paths in
#   NonBlocking::write (tx == None, Disconnected). Both are unreachable via
#   the public API: None is only set during Drop, and Disconnected only occurs
#   if the background thread panics.

set -euo pipefail
cargo llvm-cov --show-missing-lines --ignore-filename-regex "rustc-.*-src" --features non-blocking
