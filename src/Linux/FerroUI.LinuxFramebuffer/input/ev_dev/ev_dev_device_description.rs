//! The description of an event device (the port of
//! `EvDevDeviceDescription.cs`): which file it is and what kind of device.
//!
//! The reference has an abstract class with one subclass, the touch
//! screen. The kinds are the cases of an enumeration here; the keyboard,
//! the mouse and the multi-touch screen are additions
//! (docs/porting/linux-framebuffer-platform.md, section 7).

use super::{
    EvDevKeyboardDeviceDescription, EvDevMouseDeviceDescription, EvDevMultiTouchScreenDeviceDescription,
    EvDevTouchScreenDeviceDescription,
};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub enum EvDevDeviceDescription {
    TouchScreen(EvDevTouchScreenDeviceDescription),
    Keyboard(EvDevKeyboardDeviceDescription),
    Mouse(EvDevMouseDeviceDescription),
    MultiTouchScreen(EvDevMultiTouchScreenDeviceDescription),
}

const FORMAT_ERROR: &str =
    "Invalid device format, expected `(path):type=(touchscreen|multitouch|keyboard|mouse):[calibration=m11,m12,m21,m22,m31,m32]";

impl EvDevDeviceDescription {
    pub fn path(&self) -> Option<&str> {
        match self {
            Self::TouchScreen(description) => description.path.as_deref(),
            Self::Keyboard(description) => description.path.as_deref(),
            Self::Mouse(description) => description.path.as_deref(),
            Self::MultiTouchScreen(description) => description.path.as_deref(),
        }
    }

    /// Parses the value of an environment variable that names a device.
    pub(crate) fn parse_from_env(env: &str) -> Result<Self, String> {
        let format_ex = || FORMAT_ERROR.to_owned();

        let items: Vec<&str> = env.split(':').filter(|item| !item.is_empty()).collect();
        if items.len() < 2 {
            return Err(format_ex());
        }

        let path = items[0];
        let mut dic = HashMap::new();
        for item in &items[1..] {
            // An item without a value is an error (the reference fails
            // with an index out of range there).
            let (key, value) = item.split_once('=').ok_or_else(format_ex)?;
            if dic.insert(key.to_owned(), value.to_owned()).is_some() {
                return Err(format_ex());
            }
        }

        let type_ = dic.get("type").ok_or_else(format_ex)?;

        match type_.as_str() {
            "touchscreen" => Ok(Self::TouchScreen(EvDevTouchScreenDeviceDescription::parse_from_env(path, &dic)?)),
            // Additions.
            "multitouch" => {
                let touch = EvDevTouchScreenDeviceDescription::parse_from_env(path, &dic)?;
                Ok(Self::MultiTouchScreen(EvDevMultiTouchScreenDeviceDescription {
                    path: touch.path,
                    calibration_matrix: touch.calibration_matrix,
                }))
            }
            "keyboard" => Ok(Self::Keyboard(EvDevKeyboardDeviceDescription { path: Some(path.to_owned()) })),
            "mouse" => Ok(Self::Mouse(EvDevMouseDeviceDescription { path: Some(path.to_owned()) })),
            _ => Err(format_ex()),
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use ferroui_base::Matrix;

    #[test]
    fn a_touch_screen_is_a_path_a_type_and_an_optional_calibration() {
        let description = EvDevDeviceDescription::parse_from_env("/dev/input/event3:type=touchscreen").unwrap();
        assert_eq!(description.path(), Some("/dev/input/event3"));
        assert_eq!(
            description,
            EvDevDeviceDescription::TouchScreen(EvDevTouchScreenDeviceDescription {
                path: Some("/dev/input/event3".to_owned()),
                calibration_matrix: Matrix::IDENTITY,
            })
        );

        let description =
            EvDevDeviceDescription::parse_from_env("/dev/input/event3:type=touchscreen:calibration=0,1,-1,0,10,20")
                .unwrap();
        let EvDevDeviceDescription::TouchScreen(touch) = description else {
            panic!("not a touch screen");
        };
        assert_eq!(touch.calibration_matrix, Matrix::new(0.0, 1.0, -1.0, 0.0, 10.0, 20.0));
    }

    #[test]
    fn the_added_types_are_named_by_type() {
        let parse = |env: &str| EvDevDeviceDescription::parse_from_env(env).unwrap();
        assert!(matches!(parse("/dev/input/event0:type=keyboard"), EvDevDeviceDescription::Keyboard(_)));
        assert!(matches!(parse("/dev/input/event1:type=mouse"), EvDevDeviceDescription::Mouse(_)));
        assert!(matches!(parse("/dev/input/event2:type=multitouch"), EvDevDeviceDescription::MultiTouchScreen(_)));
    }

    #[test]
    fn what_is_not_in_the_format_is_an_error() {
        for env in [
            "",
            "/dev/input/event3",
            "/dev/input/event3:calibration=1,0,0,1,0,0",
            "/dev/input/event3:type=joystick",
            "/dev/input/event3:type",
            "/dev/input/event3:type=touchscreen:calibration=x",
        ] {
            assert!(EvDevDeviceDescription::parse_from_env(env).is_err(), "{env}");
        }
    }
}
