# TODO

## Phase 1 — Core (MVP)

- [ ] Define `Level` enum (`Trace`, `Debug`, `Info`, `Warn`, `Error`, `Fatal`, `Panic`)
- [ ] Define `Event` struct with owned `Vec<u8>` buffer and enabled/disabled flag
- [ ] Implement scalar field methods on `Event`: `.str()`, `.int()`, `.uint()`, `.float()`, `.bool()`
- [ ] Implement terminal methods: `.msg()`, `.send()`
- [ ] Define `Logger` struct with sink (`Arc<Mutex<dyn Write + Send>>`) and minimum level
- [ ] Implement level shorthand methods on `Logger`: `.info()`, `.warn()`, `.error()`, etc.
- [ ] Auto-append `"time"` (RFC 3339) and `"level"` fields on every event flush
- [ ] Disabled event fast-path: level below minimum returns a no-op `Event`
- [ ] Implement `.err(e)` field method
- [ ] Implement `.dur(key, val)` and `.time(key, val)` field methods
- [ ] Implement `fatal()` (flush + `process::exit(1)`) and `panic()` (flush + `panic!()`)
- [ ] Write basic unit tests: field encoding, JSON validity, level filtering

## Phase 2 — Subloggers & context

- [ ] Define `Context` builder type
- [ ] Implement `Logger::with()` returning `Context`
- [ ] Implement field methods on `Context` (mirrors `Event`)
- [ ] Implement `Context::logger()` returning a new `Logger` with pre-encoded context fields
- [ ] Ensure context fields prepend correctly without per-event allocation
- [ ] Tests: sublogger field inheritance, context fields appear before event fields

## Phase 3 — Performance

- [ ] Benchmark baseline vs `tracing` + JSON subscriber
- [ ] Evaluate buffer reuse strategy: `thread_local!` pool vs per-logger pool
- [ ] Profile and eliminate remaining allocations on hot path
- [ ] Add `benches/` with criterion benchmarks: disabled event, single field, ten fields

## Phase 4 — Compile-time level filtering

- [ ] Define Cargo features: `level-trace`, `level-debug`, `level-info`, `level-warn`, `level-error`, `level-off`
- [ ] Gate level methods behind `#[cfg(feature = ...)]` so disabled levels are zero-cost
- [ ] Document feature flag behavior in README

## Phase 5 — Non-blocking writer

- [ ] Implement `NonBlocking<W>` writer adapter (optional feature `non-blocking`)
  - [ ] `mpsc` channel (configurable capacity) + background writer thread
  - [ ] Drop counter for overflow — exposed via a `dropped()` method
  - [ ] `NonBlocking<W>` implements `std::io::Write` (drop-in sink for `Logger`)
- [ ] Tests: non-blocking path under load, drop counting, clean shutdown on `Logger` drop

## Phase 6 — Polish & publish

- [ ] `any(key, val)` field method via `serde::Serialize`
- [ ] Pretty-print console writer (optional feature `pretty`)
- [ ] `log` crate facade compatibility (optional feature `log-compat`)
- [ ] Expand README with full API docs and runnable examples under `examples/`
- [ ] `CHANGELOG.md`
- [ ] CI (GitHub Actions): test + clippy + fmt check on stable + beta
- [ ] Publish to crates.io

## Future / stretch

- [ ] `wirelog-derive`: `#[derive(LogFields)]` proc macro for structured types (requires workspace)
- [ ] `TokioNonBlocking<W>`: channel-based adapter backed by `tokio::io::AsyncWrite` + spawned task (optional feature `tokio`)
- [ ] `tracing` subscriber backend
