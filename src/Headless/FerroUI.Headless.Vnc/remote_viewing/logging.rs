//! The part of `Microsoft.Extensions.Logging.Abstractions` (.NET runtime,
//! MIT) that RemoteViewing logs through: the levels, the event id, the
//! logger contract and the extension methods it calls (`LogDebug`,
//! `LogInformation`, `LogWarning`, `LogError`).
//!
//! The state of a log entry is the message itself: the extension methods
//! of the original wrap a message without arguments in a value whose text
//! is the message, and the formatter returns that text.

use ferroui_base::reactive::IDisposable;
use std::any::Any;
use std::error::Error;
use std::fmt;
use std::rc::Rc;

/// Defines logging severity levels.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum LogLevel {
    /// Logs that contain the most detailed messages. These messages may
    /// contain sensitive application data. These messages are disabled by
    /// default and should never be enabled in a production environment.
    Trace = 0,
    /// Logs that are used for interactive investigation during development.
    /// These logs should primarily contain information useful for debugging
    /// and have no long-term value.
    Debug = 1,
    /// Logs that track the general flow of the application. These logs
    /// should have long-term value.
    Information = 2,
    /// Logs that highlight an abnormal or unexpected event in the
    /// application flow, but do not otherwise cause the application
    /// execution to stop.
    Warning = 3,
    /// Logs that highlight when the current flow of execution is stopped
    /// due to a failure. These should indicate a failure in the current
    /// activity, not an application-wide failure.
    Error = 4,
    /// Logs that describe an unrecoverable application or system crash, or
    /// a catastrophic failure that requires immediate attention.
    Critical = 5,
    /// Not used for writing log messages. Specifies that a logging category
    /// should not write any messages.
    None = 6,
}

/// Identifies a logging event. The primary identifier is the `id` property,
/// with the `name` property providing a short description of this type of
/// event.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct EventId {
    id: i32,
    name: Option<String>,
}

impl EventId {
    /// Initializes an instance of the [`EventId`] struct.
    pub fn new(id: i32) -> Self {
        Self { id, name: None }
    }

    /// Initializes an instance of the [`EventId`] struct with a name.
    pub fn with_name(id: i32, name: Option<&str>) -> Self {
        Self {
            id,
            name: name.map(str::to_owned),
        }
    }

    /// Gets the numeric identifier for this event.
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Gets the name of this event.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => f.write_str(name),
            None => write!(f, "{}", self.id),
        }
    }
}

/// The function that creates the message of a log entry from its state and
/// its error.
pub type LogFormatter<'a> = &'a dyn Fn(&dyn Any, Option<&(dyn Error + 'static)>) -> String;

/// Represents a type used to perform logging.
///
/// Loggers are shared with the threads of a server session, hence `Send`
/// and `Sync`.
pub trait ILogger: Send + Sync {
    /// Writes a log entry.
    ///
    /// `state` is the entry to be written (the message, for the extension
    /// methods of [`LoggerExtensions`]); `formatter` creates the message
    /// of the entry from `state` and `exception`.
    fn log(
        &self,
        log_level: LogLevel,
        event_id: EventId,
        state: &dyn Any,
        exception: Option<&(dyn Error + 'static)>,
        formatter: LogFormatter<'_>,
    );

    /// Checks if the given `log_level` is enabled.
    fn is_enabled(&self, log_level: LogLevel) -> bool;

    /// Begins a logical operation scope. Returns a disposable object that
    /// ends the logical operation scope on dispose.
    fn begin_scope(&self, state: &dyn Any) -> Rc<dyn IDisposable>;
}

/// The formatter of the extension methods: the message the state holds.
fn message_formatter(state: &dyn Any, _exception: Option<&(dyn Error + 'static)>) -> String {
    state.downcast_ref::<String>().cloned().unwrap_or_default()
}

/// ILogger extension methods for common scenarios.
pub trait LoggerExtensions {
    /// Formats and writes a debug log message.
    fn log_debug(&self, message: &str);

    /// Formats and writes an informational log message.
    fn log_information(&self, message: &str);

    /// Formats and writes a warning log message.
    fn log_warning(&self, message: &str);

    /// Formats and writes an error log message.
    fn log_error(
        &self,
        event_id: EventId,
        exception: Option<&(dyn Error + 'static)>,
        message: &str,
    );
}

impl<T: ILogger + ?Sized> LoggerExtensions for T {
    fn log_debug(&self, message: &str) {
        self.log(
            LogLevel::Debug,
            EventId::default(),
            &message.to_owned(),
            None,
            &message_formatter,
        );
    }

    fn log_information(&self, message: &str) {
        self.log(
            LogLevel::Information,
            EventId::default(),
            &message.to_owned(),
            None,
            &message_formatter,
        );
    }

    fn log_warning(&self, message: &str) {
        self.log(
            LogLevel::Warning,
            EventId::default(),
            &message.to_owned(),
            None,
            &message_formatter,
        );
    }

    fn log_error(
        &self,
        event_id: EventId,
        exception: Option<&(dyn Error + 'static)>,
        message: &str,
    ) {
        self.log(
            LogLevel::Error,
            event_id,
            &message.to_owned(),
            exception,
            &message_formatter,
        );
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use ferroui_base::reactive::Disposable;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingLogger {
        entries: Mutex<Vec<(LogLevel, i32, String)>>,
    }

    impl ILogger for RecordingLogger {
        fn log(
            &self,
            log_level: LogLevel,
            event_id: EventId,
            state: &dyn Any,
            exception: Option<&(dyn Error + 'static)>,
            formatter: LogFormatter<'_>,
        ) {
            self.entries.lock().unwrap().push((
                log_level,
                event_id.id(),
                formatter(state, exception),
            ));
        }

        fn is_enabled(&self, _log_level: LogLevel) -> bool {
            true
        }

        fn begin_scope(&self, _state: &dyn Any) -> Rc<dyn IDisposable> {
            Disposable::empty()
        }
    }

    #[test]
    fn the_extension_methods_write_the_message_at_their_level() {
        let logger = RecordingLogger::default();
        logger.log_debug("one");
        logger.log_information("two");
        logger.log_warning("three");
        logger.log_error(EventId::new(2), None, "four");

        assert_eq!(
            vec![
                (LogLevel::Debug, 0, "one".to_owned()),
                (LogLevel::Information, 0, "two".to_owned()),
                (LogLevel::Warning, 0, "three".to_owned()),
                (LogLevel::Error, 2, "four".to_owned()),
            ],
            *logger.entries.lock().unwrap()
        );
    }

    #[test]
    fn an_event_id_shows_its_name_or_its_number() {
        assert_eq!("7", EventId::new(7).to_string());
        assert_eq!(
            "Capture",
            EventId::with_name(7, Some("Capture")).to_string()
        );
    }
}
