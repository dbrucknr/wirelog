/// Severity level for a log event.
///
/// Levels are ordered from least to most severe: `Trace < Debug < Info < Warn < Error < Fatal < Panic`.
/// Use [`Logger::level`](crate::Logger::level) to set a runtime minimum; events below it become no-ops.
/// Use Cargo feature flags (`level-debug`, `level-info`, etc.) to eliminate levels at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// Fine-grained diagnostic information. Typically enabled only during development.
    Trace,
    /// Diagnostic information useful for debugging.
    Debug,
    /// General messages about normal program behavior.
    Info,
    /// Potentially harmful situations that do not prevent normal operation.
    Warn,
    /// Error conditions. The operation failed but the program can continue.
    Error,
    /// Severe errors. The caller is expected to exit the process.
    Fatal,
    /// Programming errors. The caller is expected to call `panic!()`.
    Panic,
}

impl Level {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Level::Trace => "trace",
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
            Level::Fatal => "fatal",
            Level::Panic => "panic",
        }
    }
}
