use wirelog::{Logger, NonBlocking};

// cargo run --example non_blocking --features non-blocking

fn main() {
    // Wrap any `io::Write` sink with NonBlocking. The second argument is the
    // channel capacity — the maximum number of log lines buffered before the
    // background thread catches up. Lines beyond that are silently dropped and
    // counted by `dropped()`.
    let nb = NonBlocking::new(std::io::stdout(), 1024);
    let logger = Logger::new(nb);

    logger.info().msg("server started");

    logger
        .info()
        .str("request_id", "abc-123")
        .int("status", 200)
        .dur("latency", std::time::Duration::from_millis(42))
        .msg("request handled");

    logger
        .error()
        .err(&std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "upstream timeout",
        ))
        .str("service", "payments")
        .msg("dependency failed");

    // Subloggers work identically — context fields are encoded once and prepended
    // to every event, independent of the non-blocking transport.
    let req_log = logger
        .with()
        .str("service", "api")
        .str("version", "v2")
        .logger();

    req_log.warn().str("path", "/health").msg("slow response");
    req_log.info().msg("request complete");

    // Dropping the logger closes the channel. Drop blocks until the background
    // thread has drained all queued lines and flushed the writer — no lines are
    // silently lost at shutdown.
}
