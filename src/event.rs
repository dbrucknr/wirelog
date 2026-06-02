use std::io::Write;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;

use crate::encode;
use crate::level::Level;

/// A single log event. Returned by [`Logger`](crate::Logger) level methods and consumed by `.msg()` or
/// `.send()`. When the logger's minimum level filters this event out, all field methods are
/// no-ops and `.msg()` / `.send()` write nothing.
pub struct Event<W> {
    buf: Vec<u8>,
    writer: Option<Arc<Mutex<W>>>,
}

impl<W: Write + Send + 'static> Event<W> {
    pub(crate) fn new(writer: Arc<Mutex<W>>, level: Level, prefix: &[u8]) -> Self {
        let mut buf = Vec::with_capacity(256 + prefix.len());
        buf.extend_from_slice(b"{\"level\":\"");
        buf.extend_from_slice(level.as_str().as_bytes());
        buf.push(b'"');
        buf.extend_from_slice(prefix);
        Self {
            buf,
            writer: Some(writer),
        }
    }

    pub(crate) fn disabled() -> Self {
        Self {
            buf: Vec::new(),
            writer: None,
        }
    }

    pub fn str(mut self, key: &str, val: &str) -> Self {
        if self.writer.is_some() {
            encode::append_key(&mut self.buf, key);
            encode::append_str_val(&mut self.buf, val);
        }
        self
    }

    pub fn int(mut self, key: &str, val: i64) -> Self {
        if self.writer.is_some() {
            encode::append_key(&mut self.buf, key);
            let mut b = itoa::Buffer::new();
            self.buf.extend_from_slice(b.format(val).as_bytes());
        }
        self
    }

    pub fn uint(mut self, key: &str, val: u64) -> Self {
        if self.writer.is_some() {
            encode::append_key(&mut self.buf, key);
            let mut b = itoa::Buffer::new();
            self.buf.extend_from_slice(b.format(val).as_bytes());
        }
        self
    }

    pub fn float(mut self, key: &str, val: f64) -> Self {
        if self.writer.is_some() {
            encode::append_key(&mut self.buf, key);
            if val.is_finite() {
                let mut b = ryu::Buffer::new();
                self.buf.extend_from_slice(b.format(val).as_bytes());
            } else {
                self.buf.extend_from_slice(b"null");
            }
        }
        self
    }

    pub fn bool(mut self, key: &str, val: bool) -> Self {
        if self.writer.is_some() {
            encode::append_key(&mut self.buf, key);
            self.buf
                .extend_from_slice(if val { b"true" } else { b"false" });
        }
        self
    }

    pub fn err(self, e: &dyn std::error::Error) -> Self {
        self.str("error", &e.to_string())
    }

    pub fn dur(mut self, key: &str, val: std::time::Duration) -> Self {
        if self.writer.is_some() {
            encode::append_key(&mut self.buf, key);
            let mut b = itoa::Buffer::new();
            self.buf.extend_from_slice(
                b.format(u64::try_from(val.as_millis()).unwrap_or(u64::MAX))
                    .as_bytes(),
            );
        }
        self
    }

    pub fn time(mut self, key: &str, val: std::time::SystemTime) -> Self {
        if self.writer.is_some() {
            encode::append_key(&mut self.buf, key);
            self.buf.push(b'"');
            let _ = OffsetDateTime::from(val).format_into(&mut self.buf, encode::RFC3339_MILLIS);
            self.buf.push(b'"');
        }
        self
    }

    pub fn msg(self, msg: &str) {
        self.flush(Some(msg));
    }

    pub fn send(self) {
        self.flush(None);
    }

    fn flush(mut self, msg: Option<&str>) {
        if let Some(writer) = self.writer.take() {
            if let Some(m) = msg {
                encode::append_key(&mut self.buf, "message");
                encode::append_str_val(&mut self.buf, m);
            }
            self.buf.extend_from_slice(b",\"time\":\"");
            let _ = OffsetDateTime::now_utc().format_into(&mut self.buf, encode::RFC3339_MILLIS);
            self.buf.extend_from_slice(b"\"}\n");

            let mut w = writer.lock().unwrap();
            let _ = w.write_all(&self.buf);
            let _ = w.flush();
        }
    }
}
