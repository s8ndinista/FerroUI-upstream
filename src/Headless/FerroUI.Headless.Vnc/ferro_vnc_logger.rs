//! Port of the logger file of the upstream VNC project: the logger the VNC
//! server sessions write to, which writes to the logger of the framework
//! (area `VncPlatform`).

use crate::remote_viewing::logging::{EventId, ILogger, LogFormatter, LogLevel};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::reactive::{Disposable, IDisposable};
use std::any::Any;
use std::error::Error;
use std::rc::Rc;

/// The logger of the VNC server sessions.
pub(crate) struct FerroVncLogger;

impl ILogger for FerroVncLogger {
    fn log(
        &self,
        log_level: LogLevel,
        _event_id: EventId,
        state: &dyn Any,
        exception: Option<&(dyn Error + 'static)>,
        formatter: LogFormatter<'_>,
    ) {
        if let Some(logger) =
            Logger::try_get(Self::to_log_event_level(log_level), LogArea::VNC_PLATFORM)
        {
            logger.log(Some(state), &formatter(state, exception));
        }
    }

    fn is_enabled(&self, log_level: LogLevel) -> bool {
        Logger::is_enabled(Self::to_log_event_level(log_level), LogArea::VNC_PLATFORM)
    }

    fn begin_scope(&self, _state: &dyn Any) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

impl FerroVncLogger {
    /// # Panics
    /// For [`LogLevel::None`], which is not a level of an entry
    /// (`ArgumentOutOfRangeException`).
    fn to_log_event_level(log_level: LogLevel) -> LogEventLevel {
        match log_level {
            LogLevel::Trace => LogEventLevel::Verbose,
            LogLevel::Debug => LogEventLevel::Debug,
            LogLevel::Information => LogEventLevel::Information,
            LogLevel::Warning => LogEventLevel::Warning,
            LogLevel::Error => LogEventLevel::Error,
            LogLevel::Critical => LogEventLevel::Fatal,
            LogLevel::None => panic!(
                "Specified argument was out of the range of valid values. (Parameter 'logLevel')"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn the_levels_map_to_the_levels_of_the_framework() {
        assert_eq!(
            LogEventLevel::Verbose,
            FerroVncLogger::to_log_event_level(LogLevel::Trace)
        );
        assert_eq!(
            LogEventLevel::Information,
            FerroVncLogger::to_log_event_level(LogLevel::Information)
        );
        assert_eq!(
            LogEventLevel::Fatal,
            FerroVncLogger::to_log_event_level(LogLevel::Critical)
        );
    }

    #[test]
    #[should_panic(expected = "Parameter 'logLevel'")]
    fn none_is_not_a_level_of_an_entry() {
        FerroVncLogger::to_log_event_level(LogLevel::None);
    }
}
