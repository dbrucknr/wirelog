use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::context::Context;
use crate::{Event, Level};

/// A structured JSON logger, generic over its writer `W`.
///
/// See the [crate-level documentation](crate) for a full explanation of when to use
/// `Logger<W>` (static dispatch) versus [`AnyLogger`](crate::AnyLogger) (dynamic dispatch).
pub struct Logger<W> {
    writer: Arc<Mutex<W>>,
    level: Level,
    prefix: Vec<u8>,
}

impl<W: Write + Send + 'static> Logger<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer: Arc::new(Mutex::new(writer)),
            level: Level::Trace,
            prefix: Vec::new(),
        }
    }

    pub(crate) fn from_context(writer: Arc<Mutex<W>>, level: Level, prefix: Vec<u8>) -> Self {
        Self {
            writer,
            level,
            prefix,
        }
    }

    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Returns a [`Context`] builder for attaching permanent fields to a sublogger.
    pub fn with(&self) -> Context<W> {
        Context::new(Arc::clone(&self.writer), self.level, &self.prefix)
    }

    fn event(&self, level: Level) -> Event<W> {
        if level < self.level {
            Event::disabled()
        } else {
            Event::new(Arc::clone(&self.writer), level, &self.prefix)
        }
    }

    pub fn trace(&self) -> Event<W> {
        self.event(Level::Trace)
    }
    pub fn debug(&self) -> Event<W> {
        self.event(Level::Debug)
    }
    pub fn info(&self) -> Event<W> {
        self.event(Level::Info)
    }
    pub fn warn(&self) -> Event<W> {
        self.event(Level::Warn)
    }
    pub fn error(&self) -> Event<W> {
        self.event(Level::Error)
    }
    pub fn fatal(&self) -> Event<W> {
        self.event(Level::Fatal)
    }
    pub fn panic(&self) -> Event<W> {
        self.event(Level::Panic)
    }
}

impl Logger<Box<dyn Write + Send>> {
    /// Constructs a type-erased [`AnyLogger`](crate::AnyLogger) by boxing the writer.
    ///
    /// Equivalent to `Logger::new(Box::new(writer))`. Use this when you want to avoid
    /// propagating the `<W>` generic parameter through your types.
    ///
    /// ```rust
    /// use wirelog::Logger;
    ///
    /// let logger = Logger::boxed(std::io::stdout());
    /// logger.info().str("env", "production").msg("server started");
    /// ```
    pub fn boxed(writer: impl Write + Send + 'static) -> Self {
        Self::new(Box::new(writer))
    }
}

impl<W: Write + Send + 'static> Clone for Logger<W> {
    fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
            level: self.level,
            prefix: self.prefix.clone(),
        }
    }
}
