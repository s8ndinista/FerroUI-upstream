use ash::vk;
use ferroui_base::logging::{LogEventLevel, Logger};
use std::ffi::{c_void, CStr};

/// Writes the messages of the debug messenger of Vulkan to the log of the
/// framework, in the area `Vulkan`.
pub(crate) struct VulkanDebugLogger;

impl VulkanDebugLogger {
    /// The function Vulkan calls with a message.
    pub fn callback_ptr() -> vk::PFN_vkDebugUtilsMessengerCallbackEXT {
        Some(write_log_event)
    }

    /// The level a message of the given severity is logged at.
    pub fn level_of(message_severity: vk::DebugUtilsMessageSeverityFlagsEXT) -> LogEventLevel {
        if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
            LogEventLevel::Error
        } else if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
            LogEventLevel::Warning
        } else if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE {
            LogEventLevel::Verbose
        } else {
            // The information severity and every combination of severities.
            LogEventLevel::Information
        }
    }
}

unsafe extern "system" fn write_log_event(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    _message_types: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _p_user_data: *mut c_void,
) -> vk::Bool32 {
    let level = VulkanDebugLogger::level_of(message_severity);
    if let Some(logger) = Logger::try_get(level, "Vulkan") {
        // SAFETY: Vulkan passes a valid structure for the duration of the
        // call, whose message is a zero-terminated string or null.
        let msg = unsafe {
            p_callback_data
                .as_ref()
                .filter(|data| !data.p_message.is_null())
                .map(|data| CStr::from_ptr(data.p_message).to_string_lossy().into_owned())
        }
        .unwrap_or_default();
        logger.log_with_values(None, "Vulkan: {0}", &[&msg]);
    }

    vk::FALSE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_severity_maps_to_its_level() {
        use vk::DebugUtilsMessageSeverityFlagsEXT as Severity;
        assert_eq!(VulkanDebugLogger::level_of(Severity::ERROR), LogEventLevel::Error);
        assert_eq!(VulkanDebugLogger::level_of(Severity::WARNING), LogEventLevel::Warning);
        assert_eq!(VulkanDebugLogger::level_of(Severity::VERBOSE), LogEventLevel::Verbose);
        assert_eq!(VulkanDebugLogger::level_of(Severity::INFO), LogEventLevel::Information);
        // More than one bit is none of the named severities.
        assert_eq!(VulkanDebugLogger::level_of(Severity::ERROR | Severity::WARNING), LogEventLevel::Information);
        assert_eq!(VulkanDebugLogger::level_of(Severity::empty()), LogEventLevel::Information);
    }

    #[test]
    fn the_callback_lets_the_call_that_triggered_it_continue() {
        let data = vk::DebugUtilsMessengerCallbackDataEXT::default().message(c"a message of a layer");
        let callback = VulkanDebugLogger::callback_ptr().expect("the callback is a function");
        // SAFETY: the structure is valid and its message is a zero-terminated string.
        let result = unsafe {
            callback(
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
                vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION,
                &data,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result, vk::FALSE);
        // A structure without a message is logged as an empty one.
        let data = vk::DebugUtilsMessengerCallbackDataEXT::default();
        // SAFETY: as above; the message is null.
        let result = unsafe {
            callback(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL,
                &data,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(result, vk::FALSE);
    }
}
