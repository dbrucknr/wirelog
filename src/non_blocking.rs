use std::io::{self, Write};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;

/// A non-blocking writer adapter that buffers log lines in a bounded `mpsc` channel
/// and drains them on a dedicated background thread.
///
/// Log calls on the hot path become a channel send and never block on I/O.
/// When the channel is full, lines are silently dropped and counted by [`dropped`].
///
/// Implements [`std::io::Write`], making it a drop-in sink for [`Logger`](crate::Logger).
///
/// # Shutdown
///
/// When `NonBlocking<W>` is dropped the sender is closed, the background thread
/// drains any remaining messages, flushes the underlying writer, and exits.
/// `Drop` blocks until the thread finishes so no log lines are silently lost
/// at program exit.
///
/// # Example
///
/// ```rust
/// use wirelog::{Logger, NonBlocking};
///
/// let nb = NonBlocking::new(std::io::stdout(), 1024);
/// let logger = Logger::new(nb);
/// logger.info().msg("written without blocking the caller");
/// ```
pub struct NonBlocking<W> {
    tx: Option<mpsc::SyncSender<Vec<u8>>>,
    dropped: Arc<AtomicU64>,
    handle: Option<thread::JoinHandle<()>>,
    _phantom: PhantomData<W>,
}

impl<W: Write + Send + 'static> NonBlocking<W> {
    /// Creates a new `NonBlocking<W>` adapter wrapping `writer`.
    ///
    /// `capacity` is the maximum number of pending log lines buffered in the
    /// channel. Lines written when the channel is full are dropped and counted
    /// by [`dropped`].
    pub fn new(writer: W, capacity: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(capacity);
        let dropped = Arc::new(AtomicU64::new(0));

        let handle = thread::spawn(move || {
            let mut w = writer;
            for msg in rx {
                let _ = w.write_all(&msg);
                let _ = w.flush();
            }
            let _ = w.flush();
        });

        Self {
            tx: Some(tx),
            dropped,
            handle: Some(handle),
            _phantom: PhantomData,
        }
    }

    /// Returns the number of log lines dropped due to a full channel.
    pub fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

impl<W> Write for NonBlocking<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let tx = match &self.tx {
            Some(tx) => tx,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "NonBlocking has been shut down",
                ));
            }
        };
        match tx.try_send(buf.to_vec()) {
            Ok(()) => Ok(buf.len()),
            Err(mpsc::TrySendError::Full(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                Ok(buf.len())
            }
            Err(mpsc::TrySendError::Disconnected(_)) => Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "NonBlocking writer thread has exited",
            )),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<W> Drop for NonBlocking<W> {
    fn drop(&mut self) {
        drop(self.tx.take());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::NonBlocking;
    use crate::Logger;

    struct SharedBuf(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuf {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct SlowWriter {
        inner: SharedBuf,
        delay: Duration,
    }

    impl Write for SlowWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            std::thread::sleep(self.delay);
            self.inner.write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn dropped_starts_at_zero() {
        let nb: NonBlocking<Vec<u8>> = NonBlocking::new(Vec::new(), 16);
        assert_eq!(nb.dropped(), 0);
    }

    #[test]
    fn non_blocking_delivers_all_lines_under_load() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let nb = NonBlocking::new(SharedBuf(Arc::clone(&buf)), 512);
        let logger = Logger::new(nb);
        let n = 200usize;

        let handles: Vec<_> = (0..n as i64)
            .map(|i| {
                let log = logger.clone();
                std::thread::spawn(move || log.info().int("i", i).msg("load"))
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        drop(logger); // blocks until background thread drains and exits

        let raw = buf.lock().unwrap();
        let output = std::str::from_utf8(&raw).unwrap();
        assert_eq!(output.lines().count(), n, "all lines must be delivered");
        for line in output.lines() {
            serde_json::from_str::<serde_json::Value>(line).expect("each line must be valid JSON");
        }
    }

    #[test]
    fn non_blocking_counts_dropped_lines() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        // Tiny channel + slow writer guarantees overflow when we burst many writes.
        let nb = NonBlocking::new(
            SlowWriter {
                inner: SharedBuf(Arc::clone(&buf)),
                delay: Duration::from_millis(5),
            },
            2,
        );
        let dropped = nb.dropped.clone(); // peek at the shared counter
        let logger = Logger::new(nb);

        for i in 0..100i64 {
            logger.info().int("i", i).msg("burst");
        }
        drop(logger);

        assert!(
            dropped.load(std::sync::atomic::Ordering::Relaxed) > 0,
            "at least one line must have been dropped"
        );
    }

    #[test]
    fn non_blocking_clean_shutdown_loses_no_queued_lines() {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let n = 50usize;
        {
            let nb = NonBlocking::new(SharedBuf(Arc::clone(&buf)), 512);
            let logger = Logger::new(nb);
            for i in 0..n as i64 {
                logger.info().int("i", i).msg("shutdown");
            }
            // logger drops here; Drop on NonBlocking closes the sender and joins
            // the background thread, ensuring all queued messages are written.
        }

        let raw = buf.lock().unwrap();
        let output = std::str::from_utf8(&raw).unwrap();
        assert_eq!(
            output.lines().count(),
            n,
            "no lines must be lost during shutdown"
        );
    }
}
