# TODO

## Phase 1 — Core (MVP)

- [x] Define `Level` enum (`Trace`, `Debug`, `Info`, `Warn`, `Error`, `Fatal`, `Panic`)
- [x] Define `Event` struct with owned `Vec<u8>` buffer and enabled/disabled flag
- [x] Implement scalar field methods on `Event`: `.str()`, `.int()`, `.uint()`, `.float()`, `.bool()`
- [x] Implement terminal methods: `.msg()`, `.send()`
- [x] Define `Logger` struct with sink (`Arc<Mutex<dyn Write + Send>>`) and minimum level
- [x] Implement level shorthand methods on `Logger`: `.info()`, `.warn()`, `.error()`, etc.
- [x] Auto-append `"time"` (RFC 3339) and `"level"` fields on every event flush
- [x] Disabled event fast-path: level below minimum returns a no-op `Event`
- [x] Implement `.err(e)` field method
- [x] Implement `.dur(key, val)` and `.time(key, val)` field methods
- [x] Implement `fatal()` and `panic()` as pure level designators (no exit/panic — caller's responsibility)
- [x] Write basic unit tests: field encoding, JSON validity, level filtering
- [ ] Enforce RFC 3339 millisecond precision on all timestamps (`"time"` field and `.time()`) — `Rfc3339` omits sub-seconds when zero, violating the spec
- [x] Fix `dur` truncation: `as_millis()` returns `u128`; cast to `u64` silently overflows for extreme durations — use saturating conversion
- [ ] Add `proptest` as a `dev-dependency` and write property tests for the JSON encoder:
  - Arbitrary `(key, value)` string pairs → emitted line is valid JSON and value round-trips exactly
  - All 256 possible byte values in a string value → each produces valid JSON

## Phase 2 — Subloggers & context

- [x] Define `Context` builder type
- [x] Implement `Logger::with()` returning `Context`
- [x] Implement field methods on `Context` (mirrors `Event`)
- [x] Implement `Context::logger()` returning a new `Logger` with pre-encoded context fields
- [x] Ensure context fields prepend correctly without per-event allocation
- [x] Tests: sublogger field inheritance, context fields appear before event fields

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
