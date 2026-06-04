# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — unreleased

### Added

- `Logger<W>` — structured JSON logger generic over any `std::io::Write + Send` sink
- `AnyLogger` — type alias for `Logger<Box<dyn Write + Send>>`; removes the `<W>` parameter from consumer types
- `Logger::boxed()` — convenience constructor for `AnyLogger`
- `Logger::with()` — returns a `Context` builder for attaching permanent fields to a sublogger
- `Context<W>` — builder for subloggers with pre-encoded context fields
- `Event<W>` — per-event builder returned by level methods; consumed by `.msg()` or `.send()`
- Field methods on `Event` and `Context`: `.str()`, `.int()`, `.uint()`, `.float()`, `.bool()`, `.err()`, `.dur()`, `.time()`
- `Level` enum: `Trace`, `Debug`, `Info`, `Warn`, `Error`, `Fatal`, `Panic`
- Runtime level filtering via `Logger::level()`
- Compile-time level filtering via Cargo features: `level-debug`, `level-info`, `level-warn`, `level-error`, `level-off`
- Thread-local buffer reuse — allocation-free hot path in steady state
- Automatic `"level"` and `"time"` fields on every flush; `"time"` is RFC 3339 with millisecond precision
- `float()` encodes `NaN` and `±Inf` as JSON `null`
- `dur()` encodes `Duration` as whole milliseconds with saturating cast for extreme values
- Criterion benchmarks: disabled event, single field, ten fields (static and dynamic dispatch)
- Examples: `basic`, `layered_static_dispatch`, `layered_dyn_dispatch`, `file_sink`

[0.1.0]: https://github.com/dbrucknr/wirelog/releases/tag/v0.1.0
