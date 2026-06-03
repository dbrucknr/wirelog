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
    /// Creates a new logger writing to `writer`.
    ///
    /// The default minimum level is [`Level::Trace`] — all levels are active. Call
    /// [`Logger::level`] to raise the minimum at runtime.
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

    /// Sets the runtime minimum level. Events below this level are no-ops.
    ///
    /// Composes with compile-time level features: a level must pass both to produce output.
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

    /// Returns a new [`Event`] at trace level.
    ///
    /// Compile-time no-op when any level feature is enabled. Runtime no-op when the
    /// minimum level is above `Trace`.
    pub fn trace(&self) -> Event<W> {
        #[cfg(any(
            feature = "level-debug",
            feature = "level-info",
            feature = "level-warn",
            feature = "level-error",
            feature = "level-off",
        ))]
        return Event::disabled();

        #[cfg(not(any(
            feature = "level-debug",
            feature = "level-info",
            feature = "level-warn",
            feature = "level-error",
            feature = "level-off",
        )))]
        self.event(Level::Trace)
    }

    /// Returns a new [`Event`] at debug level.
    ///
    /// Compile-time no-op with `level-info`, `level-warn`, `level-error`, or `level-off`.
    pub fn debug(&self) -> Event<W> {
        #[cfg(any(
            feature = "level-info",
            feature = "level-warn",
            feature = "level-error",
            feature = "level-off",
        ))]
        return Event::disabled();

        #[cfg(not(any(
            feature = "level-info",
            feature = "level-warn",
            feature = "level-error",
            feature = "level-off",
        )))]
        self.event(Level::Debug)
    }

    /// Returns a new [`Event`] at info level.
    ///
    /// Compile-time no-op with `level-warn`, `level-error`, or `level-off`.
    pub fn info(&self) -> Event<W> {
        #[cfg(any(feature = "level-warn", feature = "level-error", feature = "level-off",))]
        return Event::disabled();

        #[cfg(not(any(feature = "level-warn", feature = "level-error", feature = "level-off",)))]
        self.event(Level::Info)
    }

    /// Returns a new [`Event`] at warn level.
    ///
    /// Compile-time no-op with `level-error` or `level-off`.
    pub fn warn(&self) -> Event<W> {
        #[cfg(any(feature = "level-error", feature = "level-off"))]
        return Event::disabled();

        #[cfg(not(any(feature = "level-error", feature = "level-off")))]
        self.event(Level::Warn)
    }

    /// Returns a new [`Event`] at error level.
    ///
    /// Compile-time no-op with `level-off`.
    pub fn error(&self) -> Event<W> {
        #[cfg(feature = "level-off")]
        return Event::disabled();

        #[cfg(not(feature = "level-off"))]
        self.event(Level::Error)
    }

    /// Returns a new [`Event`] at fatal level.
    ///
    /// Does not exit the process — that responsibility belongs to the caller.
    /// Compile-time no-op with `level-off`.
    pub fn fatal(&self) -> Event<W> {
        #[cfg(feature = "level-off")]
        return Event::disabled();

        #[cfg(not(feature = "level-off"))]
        self.event(Level::Fatal)
    }

    /// Returns a new [`Event`] at panic level.
    ///
    /// Does not call `panic!()` — that responsibility belongs to the caller.
    /// Compile-time no-op with `level-off`.
    pub fn panic(&self) -> Event<W> {
        #[cfg(feature = "level-off")]
        return Event::disabled();

        #[cfg(not(feature = "level-off"))]
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
