use std::time::{Duration, SystemTime};
use wirelog::{Level, Logger};

// cargo run --example basic

fn main() {
    let logger = Logger::new(std::io::stdout());

    // Plain message — no fields
    logger.info().msg("server started");

    // String fields
    logger
        .info()
        .str("request_id", "abc-123")
        .str("method", "GET")
        .str("path", "/api/users")
        .msg("request received");

    // Numeric fields
    logger
        .info()
        .int("status", 200)
        .uint("bytes", 1_024_u64)
        .float("ratio", 0.95)
        .msg("response sent");

    // Boolean field
    logger
        .debug()
        .bool("cache_hit", false)
        .str("key", "user:42")
        .msg("cache lookup");

    // Duration field — encoded as whole milliseconds
    logger
        .info()
        .str("request_id", "abc-123")
        .dur("latency", Duration::from_millis(47))
        .msg("request complete");

    // SystemTime field — encoded as RFC 3339 with millisecond precision
    logger
        .info()
        .str("event", "token_issued")
        .time("issued_at", SystemTime::now())
        .msg("auth event");

    // Error field — key is always "error"
    let e = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
    logger
        .error()
        .err(&e)
        .str("host", "db.internal")
        .msg("database unreachable");

    // .send() — flush without a "message" field
    logger
        .info()
        .str("request_id", "abc-123")
        .int("status", 204)
        .send();

    // Runtime level filtering — events below the minimum are no-ops
    let filtered = logger.clone().level(Level::Warn);
    filtered
        .debug()
        .str("k", "v")
        .msg("suppressed — below warn");
    filtered.info().str("k", "v").msg("suppressed — below warn");
    filtered.warn().msg("emitted — at threshold");
    filtered.error().msg("emitted — above threshold");

    // Sublogger with context fields encoded once, prepended to every event
    let req_log = logger
        .with()
        .str("request_id", "def-456")
        .str("service", "auth")
        .logger();

    req_log.info().msg("token validated");
    req_log.debug().int("user_id", 42).msg("user loaded");
    req_log
        .error()
        .err(&std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "forbidden",
        ))
        .msg("authorization failed");
}
