use std::io::Write;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;

use crate::Level;
use crate::encode;
use crate::logger::Logger;

/// A builder for attaching permanent fields to a [`Logger`]. Created by [`Logger::with`].
/// Call [`Context::logger`] to produce a sublogger that prepends these fields to every event.
pub struct Context<W> {
    writer: Arc<Mutex<W>>,
    level: Level,
    buf: Vec<u8>,
}

impl<W: Write + Send + 'static> Context<W> {
    pub(crate) fn new(writer: Arc<Mutex<W>>, level: Level, prefix: &[u8]) -> Self {
        let mut buf = Vec::with_capacity(prefix.len() + 64);
        buf.extend_from_slice(prefix);
        Self { writer, level, buf }
    }

    pub fn str(mut self, key: &str, val: &str) -> Self {
        encode::append_key(&mut self.buf, key);
        encode::append_str_val(&mut self.buf, val);
        self
    }

    pub fn int(mut self, key: &str, val: i64) -> Self {
        encode::append_key(&mut self.buf, key);
        let mut b = itoa::Buffer::new();
        self.buf.extend_from_slice(b.format(val).as_bytes());
        self
    }

    pub fn uint(mut self, key: &str, val: u64) -> Self {
        encode::append_key(&mut self.buf, key);
        let mut b = itoa::Buffer::new();
        self.buf.extend_from_slice(b.format(val).as_bytes());
        self
    }

    pub fn float(mut self, key: &str, val: f64) -> Self {
        encode::append_key(&mut self.buf, key);
        if val.is_finite() {
            let mut b = ryu::Buffer::new();
            self.buf.extend_from_slice(b.format(val).as_bytes());
        } else {
            self.buf.extend_from_slice(b"null");
        }
        self
    }

    pub fn bool(mut self, key: &str, val: bool) -> Self {
        encode::append_key(&mut self.buf, key);
        self.buf
            .extend_from_slice(if val { b"true" } else { b"false" });
        self
    }

    pub fn err(self, e: &dyn std::error::Error) -> Self {
        self.str("error", &e.to_string())
    }

    pub fn dur(mut self, key: &str, val: std::time::Duration) -> Self {
        encode::append_key(&mut self.buf, key);
        let mut b = itoa::Buffer::new();
        self.buf.extend_from_slice(
            b.format(u64::try_from(val.as_millis()).unwrap_or(u64::MAX))
                .as_bytes(),
        );
        self
    }

    pub fn time(mut self, key: &str, val: std::time::SystemTime) -> Self {
        encode::append_key(&mut self.buf, key);
        self.buf.push(b'"');
        let _ = OffsetDateTime::from(val).format_into(&mut self.buf, encode::RFC3339_MILLIS);
        self.buf.push(b'"');
        self
    }

    /// Consumes this context and returns a [`Logger`] that prepends all accumulated fields
    /// to every event it produces.
    pub fn logger(self) -> Logger<W> {
        Logger::from_context(self.writer, self.level, self.buf)
    }
}
