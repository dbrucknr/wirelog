mod event;
mod level;
mod logger;

pub use event::Event;
pub use level::Level;
pub use logger::Logger;

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::{Arc, Mutex};

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
        log.info().dur("latency", std::time::Duration::from_millis(42)).msg("ok");
        assert_eq!(parse(&buf)["latency"], 42);
    }

    #[test]
    fn time_field() {
        let (log, buf) = make_logger();
        log.info().time("at", std::time::SystemTime::UNIX_EPOCH).msg("ok");
        let v = parse(&buf);
        assert_eq!(v["at"], "1970-01-01T00:00:00Z");
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
        let original = "newline\nnull\ttab\"quote\\backslash\rreturn\x08backspace\x0cformfeed\x01ctrl";
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
        assert!(v["message"].is_null(), "send() must not emit a message field");
    }

    #[test]
    fn level_filtering_disabled_event() {
        let (log, buf) = make_logger();
        let log = log.level(Level::Warn);
        log.debug()
            .str("dropped", "yes")
            .bool("flag", true)
            .msg("should not appear");
        assert!(buf.lock().unwrap().is_empty(), "debug event must be suppressed");
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
    #[should_panic(expected = "wirelog: fatal exit")]
    fn fatal_level_exits() {
        let (log, _buf) = make_logger();
        log.fatal().msg("fatal");
    }

    #[test]
    #[should_panic(expected = "wirelog: panic-level event")]
    fn panic_level_panics() {
        let (log, _buf) = make_logger();
        log.panic().msg("panic");
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
}
