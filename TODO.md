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
- [x] Enforce RFC 3339 millisecond precision on all timestamps (`"time"` field and `.time()`) — `Rfc3339` omits sub-seconds when zero, violating the spec
- [x] Fix `dur` truncation: `as_millis()` returns `u128`; cast to `u64` silently overflows for extreme durations — use saturating conversion
- [x] Add `proptest` as a `dev-dependency` and write property tests for the JSON encoder:
  - Arbitrary `(key, value)` string pairs → emitted line is valid JSON and value round-trips exactly
  - All 256 possible byte values in a string value → each produces valid JSON
  - Arbitrary `i64`/`u64` field values round-trip exactly
  - Arbitrary `f64` field values: finite → JSON number, non-finite → `null`
  - Random multi-field events (0–8 fields) always produce valid JSON
- [x] Fix `float()` NaN/Infinity: `ryu` formats these as `NaN`/`inf` which are not valid JSON — emit `null` instead
- [x] Add concurrent write test: 100 threads writing simultaneously produce 100 complete, non-interleaved JSON lines

## Phase 2 — Subloggers & context

- [x] Define `Context` builder type
- [x] Implement `Logger::with()` returning `Context`
- [x] Implement field methods on `Context` (mirrors `Event`)
- [x] Implement `Context::logger()` returning a new `Logger` with pre-encoded context fields
- [x] Ensure context fields prepend correctly without per-event allocation
- [x] Tests: sublogger field inheritance, context fields appear before event fields

## Phase 3 — Performance

- [x] Benchmark baseline vs `tracing` + JSON subscriber
- [x] Add `benches/` with criterion benchmarks: disabled event, single field, ten fields (static + dynamic dispatch variants)
- [x] Evaluate buffer reuse strategy: `thread_local!` pool vs per-logger pool — chose `thread_local!`; per-logger pool would put the buffer behind the same `Mutex` as the writer, gaining nothing
- [x] Implement `thread_local!` buffer pool: `Event::new()` takes buffer from TLS via `mem::take`; `Drop` returns it after flush or discard
- [x] Profile and eliminate remaining allocations on hot path — `Vec::with_capacity(256)` per event was the only allocation; eliminated (~7 ns saved). Remaining bottleneck is `Mutex` acquisition (~160 ns); addressed in Phase 5 via `NonBlocking<W>`
- [x] Add `BENCHMARKS.md` with milestone snapshots, criterion baseline workflow, and principles for tracking progress and regressions

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

- [x] `AnyLogger` type alias (`Logger<Box<dyn Write + Send>>`) and `Logger::boxed()` constructor — removes `<W>` generic from consumer types; benchmarked as zero overhead vs static dispatch
- [x] Add runnable examples under `examples/`: `basic.rs`, `layered_static_dispatch.rs`
- [ ] `any(key, val)` field method via `serde::Serialize`
- [ ] Pretty-print console writer (optional feature `pretty`)
- [ ] `log` crate facade compatibility (optional feature `log-compat`)
- [ ] Expand README with full API docs — defer benchmark numbers until Phase 5 (`NonBlocking`) is complete so the "zero allocation" claim and the data are aligned
- [ ] `CHANGELOG.md`
- [ ] CI (GitHub Actions): test + clippy + fmt check on stable + beta
- [ ] Publish to crates.io

## Future / stretch

- [ ] `wirelog-derive`: `#[derive(LogFields)]` proc macro for structured types (requires workspace)
- [ ] `TokioNonBlocking<W>`: channel-based adapter backed by `tokio::io::AsyncWrite` + spawned task (optional feature `tokio`)
- [ ] `tracing` subscriber backend
