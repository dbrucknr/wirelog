# wirelog

A zero-allocation structured logger for Rust with [zerolog](https://github.com/rs/zerolog)-inspired ergonomics.

---

[![CI](https://github.com/dbrucknr/wirelog/actions/workflows/ci.yml/badge.svg)](https://github.com/dbrucknr/wirelog/actions/workflows/ci.yml)

## Goals

- Fluent, typed builder API — `.msg()` is the only terminal call that flushes
- Zero allocation on the hot path via buffer reuse
- JSON-only output (structured, wire-format friendly)
- Compile-time level filtering via Cargo feature flags
- Works with any `std::io::Write` sink (stdout, file, socket, etc.)

## Usage

```rust
use wirelog::Logger;

let log = Logger::new(std::io::stdout());

// Basic message
log.info().msg("server started");

// With typed fields
log.info()
    .str("request_id", &req_id)
    .int("status", 200)
    .dur("latency", elapsed)
    .msg("request complete");

// Error with cause
log.error()
    .err(&e)
    .str("path", "/api/users")
    .msg("handler failed");

// Sublogger with context fields
let req_log = log.with().str("service", "auth").logger();
req_log.debug().msg("token validated");
```

## Output

Each `.msg()` call produces a single JSON line:

```json
{"level":"info","request_id":"abc-123","status":200,"latency_ms":42,"message":"request complete","time":"2026-06-02T14:00:00Z"}
```

## Log Levels

`trace` · `debug` · `info` · `warn` · `error` · `fatal` · `panic`

Compile-time filtering: disable all levels below a threshold by enabling the corresponding Cargo feature (e.g., `features = ["level-info"]`).

## Comparison to Go's zerolog

| Feature | zerolog (Go) | wirelog (Rust) |
|---|---|---|
| Zero allocation | ✓ | ✓ (goal) |
| Fluent builder API | ✓ | ✓ |
| JSON output | ✓ | ✓ |
| Typed fields | partial | ✓ (compile-time) |
| Compile-time level filtering | ✓ | ✓ (feature flags) |
| Contextual subloggers | ✓ | ✓ |
| `io.Writer` / `io::Write` sink | ✓ | ✓ |
| Proc-macro field derivation | — | planned (`wirelog-derive`) |

## Installation

```toml
[dependencies]
wirelog = "0.1"
```

## License

MIT
