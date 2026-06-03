# wirelog

A structured JSON logger for Rust with [zerolog](https://github.com/rs/zerolog)-inspired ergonomics.

Every log call produces a single newline-terminated JSON object. Fields are typed
and appended via a fluent builder; nothing is written until `.msg()` or `.send()`
is called. The hot path is allocation-free in steady state. An optional non-blocking writer
moves I/O off the calling thread entirely via a buffered background thread.

---

## Why wirelog

Most Rust logging infrastructure falls into two camps: simple adapters for the
`log` crate façade, or `tracing`, which is the right choice when you need spans,
async instrumentation, or a large subscriber ecosystem.

wirelog is for a different situation: you want structured JSON output, a simple
and predictable per-event API, and as little overhead as possible. If you've used
zerolog in Go and want the same ergonomic model in Rust, this is it.

wirelog is **not** a replacement for `tracing`. It has no span concept, no async
instrumentation, and no subscriber ecosystem. Choose `tracing` when you need those
things.

---

## Installation

```toml
[dependencies]
wirelog = "0.1"
```

---

## Quick start

```rust
use wirelog::Logger;

let log = Logger::new(std::io::stdout());

log.info().msg("server started");

log.info()
    .str("request_id", "abc-123")
    .int("status", 200)
    .dur("latency", elapsed)
    .msg("request complete");

log.error()
    .err(&e)
    .str("path", "/api/users")
    .msg("handler failed");
```

Each call produces one JSON line:

```json
{"level":"info","message":"server started","time":"2026-06-03T14:00:00.000Z"}
{"level":"info","request_id":"abc-123","status":200,"latency":42,"message":"request complete","time":"2026-06-03T14:00:00.001Z"}
{"level":"error","error":"connection refused","path":"/api/users","message":"handler failed","time":"2026-06-03T14:00:00.002Z"}
```

---

## Field types

| Method | Rust type | JSON type |
|---|---|---|
| `.str(key, val)` | `&str` | string |
| `.int(key, val)` | `i64` | number |
| `.uint(key, val)` | `u64` | number |
| `.float(key, val)` | `f64` | number (`null` for NaN / ±Inf) |
| `.bool(key, val)` | `bool` | boolean |
| `.err(e)` | `&dyn Error` | string (key `"error"`) |
| `.dur(key, val)` | `Duration` | number (milliseconds) |
| `.time(key, val)` | `SystemTime` | string (RFC 3339, ms precision) |

All field methods take `self` and return `Self`, so they chain without intermediate
bindings. `.msg(text)` appends a `"message"` field and flushes. `.send()` flushes
without a message field.

Two fields are appended automatically on every flush:

- `"level"` — emitted first, before any user fields
- `"time"` — RFC 3339 with millisecond precision, emitted last

---

## Log levels

`trace` · `debug` · `info` · `warn` · `error` · `fatal` · `panic`

`fatal` and `panic` are level designators only — wirelog does not call
`std::process::exit` or `panic!()`. That responsibility belongs to the caller.

Runtime filtering sets a minimum level on the logger; events below it are no-ops:

```rust
let log = Logger::new(std::io::stdout()).level(Level::Warn);
log.debug().msg("this is suppressed");  // no output
log.warn().msg("this is emitted");      // written
```

---

## Compile-time level filtering

For production builds where certain levels will never be used, you can eliminate
them entirely at compile time via Cargo feature flags. Disabled levels cost nothing
— the call site reduces to a no-op that the optimizer removes completely.

| Feature | Levels compiled out |
|---|---|
| *(none — default)* | none; all levels active |
| `level-debug` | `trace` |
| `level-info` | `trace`, `debug` |
| `level-warn` | `trace`, `debug`, `info` |
| `level-error` | `trace`, `debug`, `info`, `warn` |
| `level-off` | all levels |

Enable in `Cargo.toml`:

```toml
[dependencies]
wirelog = { version = "0.1", features = ["level-info"] }
```

Compile-time and runtime filtering compose: a level must pass both to produce
output. With `features = ["level-info"]`, `logger.debug()` is a compile-time
no-op regardless of the runtime minimum level set on the logger.

If multiple features are enabled, the most restrictive wins.

---

## Subloggers and context

`Logger::with()` returns a `Context` builder. Attach fields to it and call
`.logger()` to get a new logger that prepends those fields to every event it
emits. Context fields are encoded once at construction — there is no per-event
allocation for them.

```rust
let log = Logger::new(std::io::stdout());

// All events from req_log include service="auth" and request_id="abc-123"
let req_log = log
    .with()
    .str("service", "auth")
    .str("request_id", "abc-123")
    .logger();

req_log.info().msg("token validated");
req_log.debug().int("user_id", 42).msg("user loaded");
```

Subloggers share the same underlying writer — cloning a logger is cheap (one
atomic increment). The parent logger is unaffected by subloggers derived from it.

---

## Dispatch strategy

wirelog is generic over its writer: `Logger<W: Write + Send>`. Choose based on
how important the generic parameter is to your codebase.

### Static dispatch — `Logger<W>`

The writer type is a compile-time parameter. The compiler can inline and optimize
the write path fully. The tradeoff: `<W>` propagates into every struct and `impl`
that stores a logger.

```rust
use wirelog::Logger;

let logger = Logger::new(std::io::stdout());
```

Best for library crates and shallow call chains where the writer type is fixed.

### Dynamic dispatch — `AnyLogger`

`AnyLogger` is a type alias for `Logger<Box<dyn Write + Send>>`. The writer is
erased behind a trait object, removing `<W>` from every consumer type.

```rust
use wirelog::{AnyLogger, Logger};

struct Server {
    log: AnyLogger,   // no generic parameter
}

let logger = Logger::boxed(std::io::stdout());
```

Best for application crates with layered architectures (controller → service →
repository) where threading a generic parameter through every type is more noise
than it's worth.

**Performance:** the vtable lookup is indistinguishable from static dispatch in
practice. Both paths are serialized through a `Mutex`; the lock acquisition
dominates and makes the dispatch overhead unmeasurable.

---

## Non-blocking writer

The `non-blocking` feature provides `NonBlocking<W>`, a `Write` adapter that
buffers log lines in a bounded `mpsc` channel and drains them on a dedicated
background thread. Log calls on the hot path become a channel send and never
block on I/O.

Enable in `Cargo.toml`:

```toml
[dependencies]
wirelog = { version = "0.1", features = ["non-blocking"] }
```

Usage is identical to the blocking path — `NonBlocking<W>` is a drop-in sink
for `Logger`:

```rust
use wirelog::{Logger, NonBlocking};

// 1024 is the channel capacity: max log lines buffered between the caller
// and the background thread. Tune to your burst profile.
let nb = NonBlocking::new(std::io::stdout(), 1024);
let logger = Logger::new(nb);

logger.info().str("env", "production").msg("server started");
```

Subloggers, context fields, level filtering, and `AnyLogger` all work
unchanged — the transport is transparent.

### Overflow

When the channel is full, log lines are silently dropped rather than blocking
the caller. The total dropped since construction is available via `dropped()`:

```rust
let nb = NonBlocking::new(std::io::stderr(), 512);
// ... after high-throughput section ...
let n = nb.dropped();
if n > 0 {
    eprintln!("warning: {n} log lines dropped");
}
```

Tune the capacity to match your expected burst size. A larger capacity uses
more memory but reduces drops under load.

### Shutdown

Dropping a `Logger<NonBlocking<W>>` closes the channel sender and blocks until
the background thread has drained all queued lines and flushed the underlying
writer. No log lines are silently lost at program exit.

---

## Performance

Measured with [criterion](https://github.com/bheisler/criterion.rs) on Apple
M-series (arm64), writing to `io::sink()` to isolate encoding cost from I/O.

| | wirelog | tracing + JSON subscriber | ratio |
|---|---|---|---|
| disabled event (runtime filter) | 2.6 ns | 0.3 ns | — |
| single field | 166 ns | 693 ns | **~4× faster** |
| ten fields | 285 ns | 1,181 ns | **~4× faster** |

The hot path is allocation-free in steady state — a thread-local buffer is reused
across events. The dominant cost of the blocking path is `Mutex` acquisition
(~160 ns). The `non-blocking` feature eliminates this from the calling thread:
log calls become a channel send (~20–30 ns) and I/O is handled by a background
thread.

**Disabled event note:** tracing's 0.3 ns reflects its static callsite interest
cache — after the first dispatch the check is a single atomic load. wirelog's
2.6 ns is a level comparison plus `Drop` glue. Both are negligible in practice;
neither is a meaningful differentiator.

**Static vs dynamic dispatch:** benchmarks show no measurable difference between
`Logger<W>` and `AnyLogger` on either the single-field or ten-field path. Choose
based on ergonomics, not performance.

---

## Comparison to tracing

| | wirelog | tracing |
|---|---|---|
| Output format | JSON only | pluggable (fmt, JSON, custom) |
| API model | per-event builder | macros + spans |
| Async / span support | — | ✓ |
| Subscriber ecosystem | — | large |
| Active logging overhead | ~166 ns / event (blocking) | ~693 ns / event |
| Compile-time level filtering | ✓ (feature flags) | ✓ (feature flags) |
| Contextual fields | ✓ (subloggers) | ✓ (spans) |
| `log` crate compatibility | planned | ✓ |

If your use case requires spans, async instrumentation, or a rich subscriber
ecosystem, use `tracing`. wirelog is the better fit when you want simple,
low-overhead, structured JSON logging with a familiar builder API.

---

## Comparison to zerolog (Go)

wirelog is a deliberate port of zerolog's ergonomic model to Rust.

| | zerolog (Go) | wirelog (Rust) |
|---|---|---|
| Fluent builder API | ✓ | ✓ |
| JSON output | ✓ | ✓ |
| Typed fields | partial | ✓ |
| Compile-time level filtering | ✓ | ✓ (feature flags) |
| Contextual subloggers | ✓ | ✓ |
| `io.Writer` / `io::Write` sink | ✓ | ✓ |
| Static vs dynamic dispatch | — | ✓ (`Logger<W>` / `AnyLogger`) |
| Proc-macro field derivation | — | planned (`wirelog-derive`) |

---

## License

MIT
