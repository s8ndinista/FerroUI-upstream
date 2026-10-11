//! The description of a touch screen (the port of
//! `EvDevTouchScreenDeviceDescription.cs`).

use ferroui_base::Matrix;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct EvDevTouchScreenDeviceDescription {
    pub path: Option<String>,
    pub calibration_matrix: Matrix,
}

impl Default for EvDevTouchScreenDeviceDescription {
    fn default() -> Self {
        Self { path: None, calibration_matrix: Matrix::IDENTITY }
    }
}

impl EvDevTouchScreenDeviceDescription {
    pub(crate) fn parse_from_env(path: &str, options: &HashMap<String, String>) -> Result<Self, String> {
        let mut calibration_matrix = Matrix::IDENTITY;
        if let Some(calibration) = options.get("calibration") {
            calibration_matrix =
                Matrix::parse(calibration).map_err(|error| format!("Invalid calibration matrix: {error}"))?;
        }
        Ok(Self { path: Some(path.to_owned()), calibration_matrix })
    }
}
