use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::{Event, Level};

#[derive(Clone)]
pub struct Logger {
    writer: Arc<Mutex<dyn Write + Send>>,
    level: Level,
}

impl Logger {
    pub fn new<W: Write + Send + 'static>(writer: W) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
            level: Level::Trace,
        }
    }

    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    fn event(&self, level: Level) -> Event {
        if level < self.level {
            Event::disabled()
        } else {
            Event::new(Arc::clone(&self.writer), level)
        }
    }

    pub fn trace(&self) -> Event { self.event(Level::Trace) }
    pub fn debug(&self) -> Event { self.event(Level::Debug) }
    pub fn info(&self)  -> Event { self.event(Level::Info)  }
    pub fn warn(&self)  -> Event { self.event(Level::Warn)  }
    pub fn error(&self) -> Event { self.event(Level::Error) }
    pub fn fatal(&self) -> Event { self.event(Level::Fatal) }
    pub fn panic(&self) -> Event { self.event(Level::Panic) }
}
