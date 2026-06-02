use std::io::Write;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::Level;

pub struct Event {
    buf: Vec<u8>,
    writer: Option<Arc<Mutex<dyn Write + Send>>>,
    level: Level,
}

impl Event {
    pub(crate) fn new(writer: Arc<Mutex<dyn Write + Send>>, level: Level) -> Self {
        let mut buf = Vec::with_capacity(256);
        buf.extend_from_slice(b"{\"level\":\"");
        buf.extend_from_slice(level.as_str().as_bytes());
        buf.push(b'"');
        Self {
            buf,
            writer: Some(writer),
            level,
        }
    }

    pub(crate) fn disabled() -> Self {
        Self {
            buf: Vec::new(),
            writer: None,
            level: Level::Trace,
        }
    }

    pub fn str(mut self, key: &str, val: &str) -> Self {
        if self.writer.is_some() {
            self.append_key(key);
            self.append_str_val(val);
        }
        self
    }

    pub fn int(mut self, key: &str, val: i64) -> Self {
        if self.writer.is_some() {
            self.append_key(key);
            let mut b = itoa::Buffer::new();
            self.buf.extend_from_slice(b.format(val).as_bytes());
        }
        self
    }

    pub fn uint(mut self, key: &str, val: u64) -> Self {
        if self.writer.is_some() {
            self.append_key(key);
            let mut b = itoa::Buffer::new();
            self.buf.extend_from_slice(b.format(val).as_bytes());
        }
        self
    }

    pub fn float(mut self, key: &str, val: f64) -> Self {
        if self.writer.is_some() {
            self.append_key(key);
            let mut b = ryu::Buffer::new();
            self.buf.extend_from_slice(b.format(val).as_bytes());
        }
        self
    }

    pub fn bool(mut self, key: &str, val: bool) -> Self {
        if self.writer.is_some() {
            self.append_key(key);
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
            self.append_key(key);
            let mut b = itoa::Buffer::new();
            self.buf
                .extend_from_slice(b.format(val.as_millis() as u64).as_bytes());
        }
        self
    }

    pub fn time(mut self, key: &str, val: std::time::SystemTime) -> Self {
        if self.writer.is_some() {
            self.append_key(key);
            self.buf.push(b'"');
            let _ = OffsetDateTime::from(val).format_into(&mut self.buf, &Rfc3339);
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
                self.append_key("message");
                self.append_str_val(m);
            }
            self.buf.extend_from_slice(b",\"time\":\"");
            let _ = OffsetDateTime::now_utc().format_into(&mut self.buf, &Rfc3339);
            self.buf.extend_from_slice(b"\"}\n");

            let mut w = writer.lock().unwrap();
            let _ = w.write_all(&self.buf);
            let _ = w.flush();
            drop(w);

            match self.level {
                Level::Fatal => {
                    #[cfg(not(test))]
                    std::process::exit(1);
                    #[cfg(test)]
                    panic!("wirelog: fatal exit");
                }
                Level::Panic => panic!("wirelog: panic-level event"),
                _ => {}
            }
        }
    }

    fn append_key(&mut self, key: &str) {
        self.buf.push(b',');
        self.buf.push(b'"');
        write_escaped(&mut self.buf, key);
        self.buf.extend_from_slice(b"\":");
    }

    fn append_str_val(&mut self, val: &str) {
        self.buf.push(b'"');
        write_escaped(&mut self.buf, val);
        self.buf.push(b'"');
    }
}

fn write_escaped(buf: &mut Vec<u8>, s: &str) {
    for byte in s.bytes() {
        match byte {
            b'"' => buf.extend_from_slice(b"\\\""),
            b'\\' => buf.extend_from_slice(b"\\\\"),
            b'\n' => buf.extend_from_slice(b"\\n"),
            b'\r' => buf.extend_from_slice(b"\\r"),
            b'\t' => buf.extend_from_slice(b"\\t"),
            0x08 => buf.extend_from_slice(b"\\b"),
            0x0c => buf.extend_from_slice(b"\\f"),
            b if b < 0x20 => {
                buf.extend_from_slice(b"\\u00");
                buf.push(b"0123456789abcdef"[(b >> 4) as usize]);
                buf.push(b"0123456789abcdef"[(b & 0xf) as usize]);
            }
            _ => buf.push(byte),
        }
    }
}
