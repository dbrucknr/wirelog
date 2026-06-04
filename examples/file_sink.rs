use std::{
    fs::File,
    io::{BufWriter, LineWriter},
};
use wirelog::Logger;

// cargo run --example file_sink
//
// Demonstrates the two recommended file sink patterns from the docs.
// Creates wirelog-buf.log and wirelog-line.log in the current directory,
// then removes them when done.

fn main() -> std::io::Result<()> {
    // BufWriter — best for throughput.
    // Batches events into 8 KB chunks, reducing syscall count and Mutex hold time
    // under load. Trade-off: the last partial chunk is not flushed if the process
    // exits abnormally. BufWriter flushes automatically on Drop for clean shutdowns.
    {
        let log = Logger::new(BufWriter::new(File::create("wirelog-buf.log")?));

        log.info().msg("server started");
        log.info()
            .str("request_id", "abc-123")
            .int("status", 200)
            .msg("request complete");
        log.warn()
            .str("queue", "jobs")
            .uint("depth", 9_500)
            .msg("queue depth high");
        log.error()
            .str("host", "db.internal")
            .msg("database unreachable");
        log.info().msg("server stopped");
        // BufWriter::flush() is called here by Drop.
    }
    println!("wrote wirelog-buf.log  (BufWriter — batched, high throughput)");

    // LineWriter — best for crash safety.
    // Flushes after every '\n', which is exactly how wirelog terminates each event.
    // Every event reaches disk immediately — at the cost of one syscall per event
    // instead of one per 8 KB chunk.
    {
        let log = Logger::new(LineWriter::new(File::create("wirelog-line.log")?));

        log.info().msg("server started");
        log.info()
            .str("request_id", "abc-123")
            .int("status", 200)
            .msg("request complete");
        log.warn()
            .str("queue", "jobs")
            .uint("depth", 9_500)
            .msg("queue depth high");
        log.error()
            .str("host", "db.internal")
            .msg("database unreachable");
        log.info().msg("server stopped");
        // Each event was already on disk the moment it was written.
    }
    println!("wrote wirelog-line.log (LineWriter — per-event flush, crash-safe)");

    std::fs::remove_file("wirelog-buf.log")?;
    std::fs::remove_file("wirelog-line.log")?;

    Ok(())
}
