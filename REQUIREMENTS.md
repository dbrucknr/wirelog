# Requirements

## Core API

### Logger

- `Logger` is the root type, constructed with any `std::io::Write` sink
- `Logger` is cheaply cloneable (shared sink via `Arc<Mutex<W>>` or similar)
- `Logger::with()` returns a `Context` builder for attaching permanent fields to a sublogger
- `Logger::level(Level)` sets the minimum level; events below it are no-ops

### Event builder

- `Logger::trace()`, `debug()`, `info()`, `warn()`, `error()`, `fatal()`, `panic()` each return an `Event`
- If the event's level is below the logger's minimum, return a disabled `Event` that no-ops all calls (zero cost)
- `Event` owns a reusable `Vec<u8>` buffer; all field methods append JSON fragments to it and return `Self`
- `.msg(s)` is the only terminal method — it appends the message field, timestamps, and flushes to the sink
- `.send()` is a terminal method for events with no message
- Consuming `Self` at each step ensures `.msg()` / `.send()` must be called to flush (compile-time enforcement)

### Field methods on `Event`

| Method | Go zerolog equivalent | Notes |
|---|---|---|
| `.str(key, val)` | `.Str()` | `&str` / `String` |
| `.int(key, val)` | `.Int()` | `i64` |
| `.uint(key, val)` | `.Uint()` | `u64` |
| `.float(key, val)` | `.Float64()` | `f64` |
| `.bool(key, val)` | `.Bool()` | |
| `.err(e)` | `.Err()` | `&dyn std::error::Error`; key is `"error"` |
| `.dur(key, val)` | `.Dur()` | `std::time::Duration`; serialized as milliseconds |
| `.time(key, val)` | `.Time()` | `std::time::SystemTime`; RFC 3339 |
| `.any(key, val)` | `.Interface()` | `serde_json::Value` or serde `Serialize` |

### Context / subloggers

- `Logger::with()` returns a `Context` that accepts the same field methods as `Event`
- `Context::logger()` produces a new `Logger` with those fields pre-encoded and prepended to every subsequent event
- Context fields must not allocate per-event after the sublogger is built

## Output format

- JSON, one object per line (`\n` terminated)
- Fixed key ordering: `level`, then context fields, then event fields, then `message`, then `time`
- Timestamp key is `"time"`, format is RFC 3339 with millisecond precision
- Level values are lowercase strings: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"`, `"fatal"`, `"panic"`
- No trailing comma issues — builder must correctly handle first/subsequent field separators

## Performance

- No heap allocation on the hot path for a fully disabled event (level below minimum)
- Buffer reuse strategy TBD: either `thread_local!` pool or per-`Logger` pool
- No `format!` / `to_string` calls in field methods — write directly to the buffer
- Benchmark target: competitive with `tracing` + `tracing-subscriber` JSON layer
- Static dispatch: `Logger<W>` and `Event<W>` are generic over `W: Write + Send + 'static`; no vtable on the write path, allowing the compiler to inline writes for known sink types (e.g. `Stdout`, `File`). Users who need type erasure can use `Logger<Box<dyn Write + Send>>`.

## Level filtering

- Runtime: `Logger::level(Level)` — events below threshold become no-ops
- Compile-time: Cargo features `level-trace`, `level-debug`, `level-info`, `level-warn`, `level-error`, `level-off`
  - Default feature: `level-trace` (all levels enabled)
  - Disabled levels compile away entirely via `#[cfg(feature = ...)]`

## Error handling

- Sink write errors are silently swallowed (same as zerolog) — logging must not panic or propagate errors into application code
- `fatal()` events flush then call `std::process::exit(1)`
- `panic()` events flush then call `panic!()`

## Non-blocking writer

The core API is fully synchronous — `.msg()` is never `async`. Blocking I/O on an async executor thread is avoided via a `NonBlocking<W>` writer adapter (optional feature `non-blocking`):

- Wraps any `std::io::Write` sink
- `.msg()` sends the fully-serialized JSON line over an `mpsc` channel — effectively free, never blocks the caller
- A background thread drains the channel and performs the real I/O
- Channel capacity is configurable; on overflow, lines are dropped (with an optional drop counter)
- `NonBlocking<W>` implements `std::io::Write` so it is a drop-in for any `Logger` sink
- A `tokio` feature may expose a `TokioNonBlocking<W>` backed by `tokio::io::AsyncWrite` + a spawned task, but this is a stretch goal and adds a heavy dependency

`async fn` log call sites (`.await` on `.msg()`) are explicitly a non-goal — the channel approach covers the async executor case without viral `async` in the API.

## Non-goals (v0.1)

- `log` crate facade compatibility (may add later as optional feature)
- `tracing` subscriber integration
- Non-JSON output formats (pretty-print console format may come later as optional feature)
- Proc-macro field derivation (tracked separately as `wirelog-derive`)
- `async fn` logging API (`async/.await` at call sites)
