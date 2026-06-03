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

- [x] Define Cargo features: `level-debug`, `level-info`, `level-warn`, `level-error`, `level-off` (no `level-trace` — all levels enabled is the default, no feature needed)
- [x] Gate level methods behind `cfg!()` checks so disabled levels return `Event::disabled()` at compile time — branch is a constant bool, dead code is eliminated by the optimizer
- [x] Add dedicated `compile_time_*` tests gated behind each feature flag; existing test suite runs cleanly with no features, `level-debug`, and `level-info`
- [ ] Document feature flag behavior in README

## Phase 5 — Non-blocking writer

Removed before v0.1.0. A `NonBlocking<W>` implementation was prototyped and
benchmarked but removed due to design issues: the `dropped()` counter was
inaccessible after construction, `Drop` blocked on the background thread drain,
and the single-threaded benchmark showed it was slower than the blocking path
(`buf.to_vec()` + channel atomics outweigh an uncontested mutex). The correct
API requires a `WorkerGuard` pattern. The strongest use case (async executors)
calls for `TokioNonBlocking<W>` rather than this synchronous adapter — see
stretch goals.

## Phase 6 — Polish & publish

- [x] `AnyLogger` type alias (`Logger<Box<dyn Write + Send>>`) and `Logger::boxed()` constructor — removes `<W>` generic from consumer types; benchmarked as zero overhead vs static dispatch
- [x] Add runnable examples under `examples/`: `basic.rs`, `layered_static_dispatch.rs`
- [x] Add `examples/layered_dynamic_dispatch.rs` demonstrating `AnyLogger` and `Logger::boxed()` as the ergonomic complement to `layered_static_dispatch.rs`
- [x] Complete `Cargo.toml` metadata required for crates.io: `description`, `license`, `repository`, `keywords`, `categories`, `rust-version` (MSRV)
- [x] Doc completeness pass: add `#[warn(missing_docs)]` and ensure all public items (`Level`, `Context`, `Event`, field methods) have doc comments
- [x] API surface review before v0.1.0: `Event<W>` and `Context<W>` remain public — constructors are `pub(crate)`, but users can name the types in helper function signatures
- [x] Qualify the "zero allocation" claim in README — the per-event allocation is eliminated; the Performance section documents the `Mutex` cost (~160 ns) as the dominant overhead
- [ ] `any(key, val)` field method via `serde::Serialize`
- [ ] Pretty-print console writer (optional feature `pretty`)
- [ ] `log` crate facade compatibility (optional feature `log-compat`)
- [ ] Expand README with full API docs and benchmark numbers — defer until Phase 5 is complete so data and claims are aligned
- [x] `CHANGELOG.md`
- [x] CI (GitHub Actions): test + clippy + fmt check on stable + beta
- [ ] Publish to crates.io

## Future / stretch

- [ ] `wirelog-derive`: `#[derive(LogFields)]` proc macro for structured types (requires workspace)
- [ ] `TokioNonBlocking<W>`: channel-based adapter backed by `tokio::io::AsyncWrite` + spawned task (optional feature `tokio`)
- [ ] `tracing` subscriber backend

## Async context — open design question

wirelog is usable in async code today. The logging chain is synchronous but brief;
the `thread_local!` buffer is safe across task migrations because it is moved into
the `Event` at construction and returned to TLS on `Drop`, travelling with the task
regardless of which thread it resumes on.

Phase 5 `NonBlocking<W>` and the `TokioNonBlocking<W>` stretch goal address the
executor-blocking concern by making the write path a channel send. However,
`NonBlocking<W>` uses `std::thread` and is orthogonal to Tokio's runtime — it
does not eliminate executor thread blocking, it just moves the blocking to a
different OS thread. `TokioNonBlocking<W>` (using `tokio::sync::mpsc` +
`tokio::spawn` + `tokio::io::AsyncWrite`) is the correct solution when the goal
is genuinely non-blocking I/O within an async executor.

The open question is **context propagation across `.await` points** — the ability
to attach fields to a task and have them appear automatically on every log event
emitted within that task, even after the task has yielded and resumed. This is what
`tracing` spans solve, and it requires a fundamentally different design:

- `task_local!` (tokio) or equivalent runtime-specific storage to carry context
  across yield points, rather than the current `prefix: Vec<u8>` embedded in the
  logger struct
- A subscriber or hook model so context can be injected without explicit sublogger
  threading
- Per-task context that survives thread migration — incompatible with the current
  `thread_local!` buffer pool if context were stored there (the buffer is fine;
  context would not be)

Implementing this would make wirelog a structural competitor to `tracing` rather
than a complement to it. That is a deliberate scope decision, not a technical
blocker. If pursued, it warrants its own design document before any code is
written.
