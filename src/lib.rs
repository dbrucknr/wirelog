//! A structured JSON logger with [zerolog](https://github.com/rs/zerolog)-inspired ergonomics.
//!
//! Each log call produces a single newline-terminated JSON object written to any
//! [`std::io::Write`] sink. Fields are typed and appended via a fluent builder; the
//! event is flushed only when `.msg()` or `.send()` is called.
//!
//! # Dispatch strategy
//!
//! wirelog offers two ways to hold a logger. Choose based on how much the generic
//! parameter matters to your codebase.
//!
//! ## Static dispatch — `Logger<W>`
//!
//! The writer type is a generic parameter. The compiler monomorphizes the logger
//! for your exact writer and can fully inline and optimize the write path. The
//! tradeoff: the `<W>` parameter propagates into every struct and `impl` that
//! stores a logger.
//!
//! ```rust
//! use wirelog::Logger;
//! use std::io;
//!
//! // W is known at compile time — no allocation, no indirection.
//! let logger = Logger::new(io::stdout());
//! logger.info().str("key", "value").msg("hello");
//! ```
//!
//! Best for: library crates, shallow call chains, situations where the writer
//! type is fixed and you want zero overhead.
//!
//! ## Dynamic dispatch — `AnyLogger`
//!
//! A type alias for `Logger<Box<dyn Write + Send>>`. The writer is erased behind a
//! trait object, removing the `<W>` parameter from every consumer type.
//!
//! ```rust
//! use wirelog::AnyLogger;
//!
//! // No generic parameter anywhere — easier to thread through application layers.
//! struct Server {
//!     logger: AnyLogger,
//! }
//! ```
//!
//! Construct one with [`Logger::boxed`]:
//!
//! ```rust
//! use wirelog::Logger;
//! use std::io;
//!
//! let logger = Logger::boxed(io::stdout());
//! logger.info().msg("hello");
//! ```
//!
//! Best for: application crates, layered architectures (controller → service →
//! repository), anywhere ergonomics matter more than squeezing the last nanosecond.
//!
//! ## Performance
//!
//! Measured with criterion on an Apple M-series chip, writing to [`io::sink()`]:
//!
//! | | `Logger<W>` | `AnyLogger` |
//! |---|---|---|
//! | single field | 174 ns | 173 ns |
//! | ten fields | 290 ns | 296 ns |
//! | disabled event (filtered) | 2 ns | 2 ns |
//!
//! The vtable lookup is indistinguishable from static dispatch in practice.
//! Every write is already serialized through a `Mutex`; the lock acquisition
//! dominates and makes the dispatch cost unmeasurable.
//!
//! ## Memory
//!
//! Both variants have the same stack size — `Logger<W>` is always a pointer
//! (`Arc`), a level byte, and a prefix `Vec`, regardless of `W`.
//!
//! The heap differs:
//!
//! - **Static**: `Arc` → `Mutex<W>` (writer embedded directly in the Arc allocation)
//! - **Dynamic**: `Arc` → `Mutex<Box<dyn Write + Send>>` → writer (one extra
//!   indirection and one extra heap allocation)
//!
//! This is a fixed, one-time cost paid at logger construction — invisible at
//! runtime on any path that actually writes a log line.

mod context;
mod encode;
mod event;
mod level;
mod logger;

pub use context::Context;
pub use event::Event;
pub use level::Level;
pub use logger::Logger;

/// Type-erased logger. A convenience alias for `Logger<Box<dyn std::io::Write + Send>>`.
///
/// Removes the `<W>` generic parameter from every type that stores a logger, at the
/// cost of one extra heap allocation and one pointer indirection per logger instance.
/// In practice this overhead is unmeasurable — see the crate-level docs for benchmarks.
///
/// Construct one with [`Logger::boxed`].
pub type AnyLogger = Logger<Box<dyn std::io::Write + Send>>;

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    use proptest::prelude::*;
    use serde_json::Value;

    use super::*;

    // Shared buffer so tests can inspect output after the Logger takes ownership of the writer.
    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn make_logger() -> (Logger<SharedBuf>, Arc<Mutex<Vec<u8>>>) {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let log = Logger::new(SharedBuf(Arc::clone(&buf)));
        (log, buf)
    }

    fn parse(buf: &Arc<Mutex<Vec<u8>>>) -> Value {
        let raw = buf.lock().unwrap();
        let line = std::str::from_utf8(&raw).unwrap().trim_end();
        serde_json::from_str(line).unwrap()
    }

    #[test]
    fn level_field() {
        let (log, buf) = make_logger();
        log.info().msg("hi");
        assert_eq!(parse(&buf)["level"], "info");
    }

    #[test]
    fn message_field() {
        let (log, buf) = make_logger();
        log.info().msg("hello world");
        assert_eq!(parse(&buf)["message"], "hello world");
    }

    #[test]
    fn str_field() {
        let (log, buf) = make_logger();
        log.info().str("request_id", "abc-123").msg("ok");
        assert_eq!(parse(&buf)["request_id"], "abc-123");
    }

    #[test]
    fn int_field() {
        let (log, buf) = make_logger();
        log.info().int("status", 200).msg("ok");
        assert_eq!(parse(&buf)["status"], 200);
    }

    #[test]
    fn uint_field() {
        let (log, buf) = make_logger();
        log.info().uint("count", 42).msg("ok");
        assert_eq!(parse(&buf)["count"], 42);
    }

    #[test]
    fn float_field() {
        let (log, buf) = make_logger();
        log.info().float("ratio", 0.5).msg("ok");
        assert_eq!(parse(&buf)["ratio"], 0.5);
    }

    #[test]
    fn bool_field() {
        let (log, buf) = make_logger();
        log.info().bool("ok", true).msg("done");
        assert_eq!(parse(&buf)["ok"], true);
    }

    #[test]
    fn dur_field() {
        let (log, buf) = make_logger();
        log.info()
            .dur("latency", std::time::Duration::from_millis(42))
            .msg("ok");
        assert_eq!(parse(&buf)["latency"], 42);
    }

    #[test]
    fn time_field() {
        let (log, buf) = make_logger();
        log.info()
            .time("at", std::time::SystemTime::UNIX_EPOCH)
            .msg("ok");
        let v = parse(&buf);
        assert_eq!(v["at"], "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn err_field() {
        let (log, buf) = make_logger();
        let e = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        log.error().err(&e).msg("failed");
        assert_eq!(parse(&buf)["error"], "file missing");
    }

    #[test]
    fn string_escaping() {
        let (log, buf) = make_logger();
        let original =
            "newline\nnull\ttab\"quote\\backslash\rreturn\x08backspace\x0cformfeed\x01ctrl";
        log.info().str("msg", original).msg("ok");
        assert_eq!(parse(&buf)["msg"], original);
    }

    #[test]
    fn time_field_present() {
        let (log, buf) = make_logger();
        log.info().msg("check");
        let v = parse(&buf);
        assert!(v["time"].is_string(), "time field must be present");
    }

    #[test]
    fn send_no_message_field() {
        let (log, buf) = make_logger();
        log.info().str("k", "v").send();
        let v = parse(&buf);
        assert!(
            v["message"].is_null(),
            "send() must not emit a message field"
        );
    }

    #[test]
    fn level_filtering_disabled_event() {
        let (log, buf) = make_logger();
        let log = log.level(Level::Warn);
        log.debug()
            .str("dropped", "yes")
            .bool("flag", true)
            .msg("should not appear");
        assert!(
            buf.lock().unwrap().is_empty(),
            "debug event must be suppressed"
        );
    }

    #[test]
    fn level_filtering_passes_at_threshold() {
        let (log, buf) = make_logger();
        let log = log.level(Level::Warn);
        log.warn().msg("visible");
        assert!(!buf.lock().unwrap().is_empty());
    }

    #[test]
    fn logger_is_clone() {
        let (log, buf) = make_logger();
        let log2 = log.clone();
        log2.info().msg("from clone");
        assert_eq!(parse(&buf)["level"], "info");
    }

    #[test]
    fn bool_false_field() {
        let (log, buf) = make_logger();
        log.info().bool("active", false).msg("ok");
        assert_eq!(parse(&buf)["active"], false);
    }

    #[test]
    fn trace_level() {
        let (log, buf) = make_logger();
        log.trace().msg("trace event");
        assert_eq!(parse(&buf)["level"], "trace");
    }

    #[test]
    fn debug_level() {
        let (log, buf) = make_logger();
        log.debug().msg("debug event");
        assert_eq!(parse(&buf)["level"], "debug");
    }

    #[test]
    fn fatal_level() {
        let (log, buf) = make_logger();
        log.fatal().msg("fatal event");
        assert_eq!(parse(&buf)["level"], "fatal");
    }

    #[test]
    fn panic_level() {
        let (log, buf) = make_logger();
        log.panic().msg("panic event");
        assert_eq!(parse(&buf)["level"], "panic");
    }

    #[test]
    fn output_is_valid_json_line() {
        let (log, buf) = make_logger();
        log.info().str("a", "b").int("n", 1).msg("test");
        let raw = buf.lock().unwrap();
        let line = std::str::from_utf8(&raw).unwrap();
        assert!(line.ends_with('\n'), "output must be newline-terminated");
        serde_json::from_str::<Value>(line.trim_end()).expect("must be valid JSON");
    }

    // --- Phase 2: context & subloggers ---

    #[test]
    fn context_fields_appear_in_sublogger_events() {
        let (log, buf) = make_logger();
        let sub = log.with().str("service", "auth").logger();
        sub.info().msg("ok");
        assert_eq!(parse(&buf)["service"], "auth");
    }

    #[test]
    fn context_fields_precede_event_fields() {
        let (log, buf) = make_logger();
        let sub = log.with().str("ctx", "c").logger();
        sub.info().str("evt", "e").msg("ok");
        let raw = buf.lock().unwrap();
        let line = std::str::from_utf8(&raw).unwrap();
        let ctx_pos = line.find("\"ctx\"").unwrap();
        let evt_pos = line.find("\"evt\"").unwrap();
        assert!(
            ctx_pos < evt_pos,
            "context fields must precede event fields"
        );
    }

    #[test]
    fn level_field_precedes_context_fields() {
        let (log, buf) = make_logger();
        let sub = log.with().str("ctx", "c").logger();
        sub.info().msg("ok");
        let raw = buf.lock().unwrap();
        let line = std::str::from_utf8(&raw).unwrap();
        let level_pos = line.find("\"level\"").unwrap();
        let ctx_pos = line.find("\"ctx\"").unwrap();
        assert!(
            level_pos < ctx_pos,
            "level field must precede context fields"
        );
    }

    #[test]
    fn parent_logger_unaffected_by_sublogger() {
        let (log, buf) = make_logger();
        let sub = log.with().str("service", "auth").logger();
        sub.info().msg("from sub");
        buf.lock().unwrap().clear();
        log.info().msg("from parent");
        let v = parse(&buf);
        assert!(
            v["service"].is_null(),
            "parent must not inherit sublogger context"
        );
    }

    #[test]
    fn nested_contexts_accumulate_fields() {
        let (log, buf) = make_logger();
        let sub1 = log.with().str("a", "1").logger();
        let sub2 = sub1.with().str("b", "2").logger();
        sub2.info().msg("ok");
        let v = parse(&buf);
        assert_eq!(v["a"], "1");
        assert_eq!(v["b"], "2");
    }

    #[test]
    fn context_level_filtering_inherited() {
        let (log, buf) = make_logger();
        let log = log.level(Level::Warn);
        let sub = log.with().str("service", "auth").logger();
        sub.debug().msg("filtered");
        assert!(
            buf.lock().unwrap().is_empty(),
            "sublogger must inherit level filter"
        );
    }

    #[test]
    fn context_all_field_types() {
        let (log, buf) = make_logger();
        let sub = log
            .with()
            .str("s", "v")
            .int("i", -1)
            .uint("u", 2)
            .float("f", 1.5)
            .bool("b", true)
            .dur("d", std::time::Duration::from_millis(10))
            .time("t", std::time::SystemTime::UNIX_EPOCH)
            .logger();
        sub.info().msg("ok");
        let v = parse(&buf);
        assert_eq!(v["s"], "v");
        assert_eq!(v["i"], -1);
        assert_eq!(v["u"], 2);
        assert_eq!(v["f"], 1.5);
        assert_eq!(v["b"], true);
        assert_eq!(v["d"], 10);
        assert_eq!(v["t"], "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn context_err_field() {
        let (log, buf) = make_logger();
        let e = std::io::Error::new(std::io::ErrorKind::Other, "context error");
        let sub = log.with().err(&e).logger();
        sub.info().msg("ok");
        assert_eq!(parse(&buf)["error"], "context error");
    }

    #[test]
    fn sublogger_is_clone() {
        let (log, buf) = make_logger();
        let sub = log.with().str("svc", "api").logger();
        let sub2 = sub.clone();
        sub2.info().msg("from clone");
        assert_eq!(parse(&buf)["svc"], "api");
    }

    #[test]
    fn float_nan_emits_null() {
        let (log, buf) = make_logger();
        log.info().float("x", f64::NAN).send();
        assert!(parse(&buf)["x"].is_null());
    }

    #[test]
    fn float_infinity_emits_null() {
        let (log, buf) = make_logger();
        log.info().float("x", f64::INFINITY).send();
        assert!(parse(&buf)["x"].is_null());
    }

    #[test]
    fn float_neg_infinity_emits_null() {
        let (log, buf) = make_logger();
        log.info().float("x", f64::NEG_INFINITY).send();
        assert!(parse(&buf)["x"].is_null());
    }

    #[test]
    fn concurrent_writes_are_not_interleaved() {
        use std::thread;
        let buf = Arc::new(Mutex::new(Vec::new()));
        let log = Logger::new(SharedBuf(Arc::clone(&buf)));
        let n = 100usize;
        let handles: Vec<_> = (0..n as i64)
            .map(|i| {
                let log = log.clone();
                thread::spawn(move || log.info().int("i", i).msg("concurrent"))
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let raw = buf.lock().unwrap();
        let output = std::str::from_utf8(&raw).unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(
            lines.len(),
            n,
            "expected {n} complete lines, got {}",
            lines.len()
        );
        for line in lines {
            serde_json::from_str::<Value>(line).expect("each line must be valid JSON");
        }
    }

    // --- Property tests ---

    proptest! {
        /// Arbitrary (key, value) string pairs must produce a valid JSON line where the value
        /// round-trips exactly, covering every Unicode code point and escape sequence.
        #[test]
        fn prop_str_field_roundtrip(
            // "time" is excluded: the auto-appended timestamp field appears last, so serde_json
            // would resolve a duplicate "time" key to the timestamp rather than the user value.
            key in any::<String>().prop_filter("not time", |k| k != "time"),
            val in any::<String>()
        ) {
            let (log, buf) = make_logger();
            log.info().str(&key, &val).send();
            let raw = buf.lock().unwrap();
            let line = std::str::from_utf8(&raw).unwrap().trim_end();
            let v: Value = serde_json::from_str(line).unwrap();
            prop_assert_eq!(v[&key].as_str(), Some(val.as_str()));
        }

        #[test]
        fn prop_int_field_roundtrip(
            key in any::<String>().prop_filter("not time", |k| k != "time"),
            val in any::<i64>()
        ) {
            let (log, buf) = make_logger();
            log.info().int(&key, val).send();
            let raw = buf.lock().unwrap();
            let line = std::str::from_utf8(&raw).unwrap().trim_end();
            let v: Value = serde_json::from_str(line).unwrap();
            prop_assert_eq!(v[&key].as_i64(), Some(val));
        }

        #[test]
        fn prop_uint_field_roundtrip(
            key in any::<String>().prop_filter("not time", |k| k != "time"),
            val in any::<u64>()
        ) {
            let (log, buf) = make_logger();
            log.info().uint(&key, val).send();
            let raw = buf.lock().unwrap();
            let line = std::str::from_utf8(&raw).unwrap().trim_end();
            let v: Value = serde_json::from_str(line).unwrap();
            prop_assert_eq!(v[&key].as_u64(), Some(val));
        }

        /// Finite values must produce a JSON number; NaN and infinities must produce null.
        /// Exact decimal round-trip is not asserted: ryu and serde_json's parsers can disagree
        /// by 1 ULP on values near a midpoint between two representable floats.
        #[test]
        fn prop_float_field(
            key in any::<String>().prop_filter("not time", |k| k != "time"),
            val in any::<f64>()
        ) {
            let (log, buf) = make_logger();
            log.info().float(&key, val).send();
            let raw = buf.lock().unwrap();
            let line = std::str::from_utf8(&raw).unwrap().trim_end();
            let v: Value = serde_json::from_str(line).unwrap();
            if val.is_finite() {
                prop_assert!(v[&key].is_number(), "finite float must produce a JSON number, got {:?}", v[&key]);
            } else {
                prop_assert!(v[&key].is_null(), "non-finite float must produce null, got {:?}", v[&key]);
            }
        }

        /// A random sequence of str fields (0–8) must always produce a valid JSON line,
        /// stressing the comma-separator logic for all field counts including zero.
        #[test]
        fn prop_multi_str_fields_valid_json(
            fields in proptest::collection::vec((any::<String>(), any::<String>()), 0..=8)
        ) {
            let (log, buf) = make_logger();
            let mut ev = log.info();
            for (k, v) in &fields {
                ev = ev.str(k, v);
            }
            ev.send();
            let raw = buf.lock().unwrap();
            let line = std::str::from_utf8(&raw).unwrap().trim_end();
            serde_json::from_str::<Value>(line).unwrap();
        }
    }

    /// Exhaustively checks all 128 ASCII code points as a string value, ensuring every
    /// control-character escape branch in `encode::write_escaped` produces valid JSON.
    /// (Bytes 0x80–0xFF only appear as parts of multi-byte UTF-8 sequences and pass through
    /// unchanged, so ASCII coverage is sufficient to exercise all escape logic.)
    #[test]
    fn all_ascii_bytes_produce_valid_json() {
        for b in 0u8..=127 {
            let s = char::from(b).to_string();
            let (log, buf) = make_logger();
            log.info().str("v", &s).send();
            let raw = buf.lock().unwrap();
            let line = std::str::from_utf8(&raw).unwrap().trim_end();
            serde_json::from_str::<Value>(line)
                .unwrap_or_else(|e| panic!("byte 0x{b:02x} produced invalid JSON: {e}"));
        }
    }
}
