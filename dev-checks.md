# Dev checks

Before committing: `cargo test && cargo clippy && cargo fmt --check`

---

## Coverage

```sh
cargo cov          # quick alias
./scripts/coverage.sh  # equivalent, runnable as a script
```

Both run `cargo llvm-cov` with `--features non-blocking`.

**Expected uncovered lines** — `non_blocking.rs: 76-79, 88-91`. These are two
defensive `BrokenPipe` error paths in `NonBlocking::write`:

- `tx == None` — only set during `Drop`; impossible to call `write` on a dropped value
- `Disconnected` — only if the background thread panics

Both are unreachable via the public API and are not worth testing.

**Note on compile-time level features:** `logger.rs` uses `#[cfg(...)]` attribute
blocks (not `if cfg!(...)` macros) for the early-return guards, so each
compilation sees only the relevant branch. Coverage is 100% for `logger.rs`
in a default (no level features) run. Running `cargo test --features level-off`
will fail many tests because all logging is disabled — that is expected and not
the intended way to test that feature. Use `cargo test --features level-debug`
(and the other specific flags) to run the `compile_time_*` tests.
