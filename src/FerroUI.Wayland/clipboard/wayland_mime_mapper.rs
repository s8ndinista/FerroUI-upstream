//! The data formats of the framework as the MIME types of a data offer (the
//! port of `WaylandMimeMapper.cs`).

use ferroui_base::input::DataFormat;
use std::rc::Rc;

pub const MIME_TEXT_PLAIN_UTF8: &str = "text/plain;charset=utf-8";
pub const MIME_TEXT_PLAIN: &str = "text/plain";
pub const MIME_TEXT_URI_LIST: &str = "text/uri-list";
pub const MIME_IMAGE_PNG: &str = "image/png";

/// The prefix of the MIME type of an application format. The reference has the name of
/// its project here (DEVIATIONS.md).
pub const APP_PREFIX: &str = "application/x-ferroui-";

/// Returns the MIME type strings that should be advertised for a given format.
/// For the text format, both `text/plain;charset=utf-8` and `text/plain` are returned.
///
/// # Panics
/// For an in-process format, which has no name outside the process.
pub fn to_mime_types(format: &DataFormat) -> Vec<String> {
    if DataFormat::text() == *format {
        return vec![MIME_TEXT_PLAIN_UTF8.to_string(), MIME_TEXT_PLAIN.to_string()];
    }
    if DataFormat::file() == *format {
        return vec![MIME_TEXT_URI_LIST.to_string()];
    }
    if DataFormat::bitmap() == *format {
        return vec![MIME_IMAGE_PNG.to_string()];
    }
    vec![format.to_system_name(APP_PREFIX)]
}

/// Returns the single preferred MIME type for reading a given format.
///
/// # Panics
/// For an in-process format, which has no name outside the process.
pub fn to_preferred_mime_type(format: &DataFormat) -> String {
    if DataFormat::text() == *format {
        return MIME_TEXT_PLAIN_UTF8.to_string();
    }
    if DataFormat::file() == *format {
        return MIME_TEXT_URI_LIST.to_string();
    }
    if DataFormat::bitmap() == *format {
        return MIME_IMAGE_PNG.to_string();
    }
    format.to_system_name(APP_PREFIX)
}

/// Converts a MIME type string to a format. The reference declares "or null if not
/// mappable"; every string maps: one that is none of the known types is an application
/// format (with the prefix) or a platform format, of bytes.
pub fn from_mime_type(mime_type: &str) -> Option<DataFormat> {
    if mime_type == MIME_TEXT_PLAIN_UTF8 || mime_type == MIME_TEXT_PLAIN {
        return Some(DataFormat::text().into());
    }
    if mime_type == MIME_TEXT_URI_LIST {
        return Some(DataFormat::file().into());
    }
    if mime_type == MIME_IMAGE_PNG {
        return Some(DataFormat::bitmap().into());
    }
    Some(DataFormat::from_system_name::<Rc<[u8]>>(mime_type, APP_PREFIX).into())
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use ferroui_base::input::DataFormatKind;

    #[test]
    fn the_universal_formats_have_the_usual_mime_types() {
        assert_eq!(to_mime_types(&DataFormat::text().into()), ["text/plain;charset=utf-8", "text/plain"]);
        assert_eq!(to_mime_types(&DataFormat::file().into()), ["text/uri-list"]);
        assert_eq!(to_mime_types(&DataFormat::bitmap().into()), ["image/png"]);
        assert_eq!(to_preferred_mime_type(&DataFormat::text().into()), "text/plain;charset=utf-8");
        assert_eq!(to_preferred_mime_type(&DataFormat::file().into()), "text/uri-list");
        assert_eq!(to_preferred_mime_type(&DataFormat::bitmap().into()), "image/png");
    }

    #[test]
    fn an_application_format_has_the_prefix_and_a_platform_format_its_own_name() {
        let application: DataFormat = DataFormat::create_bytes_application_format("my-format").into();
        assert_eq!(to_mime_types(&application), ["application/x-ferroui-my-format"]);
        assert_eq!(to_preferred_mime_type(&application), "application/x-ferroui-my-format");
        let platform: DataFormat = DataFormat::create_string_platform_format("text/html").into();
        assert_eq!(to_mime_types(&platform), ["text/html"]);
    }

    #[test]
    fn mime_types_map_back_to_the_formats() {
        assert!(DataFormat::text() == from_mime_type("text/plain").unwrap());
        assert!(DataFormat::text() == from_mime_type("text/plain;charset=utf-8").unwrap());
        assert!(DataFormat::file() == from_mime_type("text/uri-list").unwrap());
        assert!(DataFormat::bitmap() == from_mime_type("image/png").unwrap());

        let application = from_mime_type("application/x-ferroui-my-format").unwrap();
        assert_eq!(application.kind(), DataFormatKind::Application);
        assert_eq!(application.identifier(), "my-format");

        let platform = from_mime_type("text/html").unwrap();
        assert_eq!(platform.kind(), DataFormatKind::Platform);
        assert_eq!(platform.identifier(), "text/html");
        // Text with another parameter is not the text format.
        assert_eq!(from_mime_type("text/plain;charset=iso-8859-1").unwrap().kind(), DataFormatKind::Platform);
    }
}
