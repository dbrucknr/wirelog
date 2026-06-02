use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::{Event, Level};

pub struct Logger<W> {
    writer: Arc<Mutex<W>>,
    level: Level,
}

impl<W: Write + Send + 'static> Logger<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
            level: Level::Trace,
        }
    }

    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    fn event(&self, level: Level) -> Event<W> {
        if level < self.level {
            Event::disabled()
        } else {
            Event::new(Arc::clone(&self.writer), level)
        }
    }

    pub fn trace(&self) -> Event<W> { self.event(Level::Trace) }
    pub fn debug(&self) -> Event<W> { self.event(Level::Debug) }
    pub fn info(&self)  -> Event<W> { self.event(Level::Info)  }
    pub fn warn(&self)  -> Event<W> { self.event(Level::Warn)  }
    pub fn error(&self) -> Event<W> { self.event(Level::Error) }
    pub fn fatal(&self) -> Event<W> { self.event(Level::Fatal) }
    pub fn panic(&self) -> Event<W> { self.event(Level::Panic) }
}

impl<W: Write + Send + 'static> Clone for Logger<W> {
    fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
            level: self.level,
        }
    }
}
