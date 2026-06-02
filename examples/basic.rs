use wirelog::Logger;

// cargo run --example basic

fn main() {
    let logger = Logger::new(std::io::stdout());

    // Basic Hello World
    logger.info().msg("Hello, world!");

    // Log an error with a key/value pair
    logger
        .error()
        .str("some-key", "some-value")
        .msg("Something went wrong");

    // Log a warning with a key/value pair
    logger
        .warn()
        .str("some-key", "some-value")
        .msg("Something is not quite right");

    // Log with duration
    logger
        .info()
        .dur("round-trip (ms)", std::time::Duration::from_millis(230))
        .msg("Response Received");
}
