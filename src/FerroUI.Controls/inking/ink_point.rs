use ferroui_base::Point;
use std::fmt;

/// Validation and persistence failures in the portable ink engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InkError {
    Platform(String),
    InvalidOperation(&'static str),
    InvalidInput(&'static str),
    InvalidPoint,
    InvalidDrawingAttributes,
    InvalidTransform,
    EmptyStroke,
    DuplicateStrokeId(u32),
    StrokeIdExhausted,
    InvalidData(&'static str),
    UnsupportedFormat,
    LimitExceeded,
}

impl fmt::Display for InkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Platform(reason) => f.write_str(reason),
            Self::InvalidOperation(reason) | Self::InvalidInput(reason) => f.write_str(reason),
            Self::InvalidPoint => f.write_str(
                "Ink coordinates and device measurements must be finite and within range",
            ),
            Self::InvalidDrawingAttributes => f.write_str("Invalid ink drawing attributes"),
            Self::InvalidTransform => f.write_str("Ink transforms must be finite affine matrices"),
            Self::EmptyStroke => f.write_str("An ink stroke must contain at least one point"),
            Self::DuplicateStrokeId(id) => write!(f, "Duplicate ink stroke ID {id}"),
            Self::StrokeIdExhausted => f.write_str("Ink stroke IDs are exhausted"),
            Self::InvalidData(reason) => write!(f, "Invalid ink document: {reason}"),
            Self::UnsupportedFormat => f.write_str("Unsupported ink persistence format"),
            Self::LimitExceeded => {
                f.write_str("Ink document exceeds the configured resource limits")
            }
        }
    }
}

impl std::error::Error for InkError {}

/// An immutable device sample. Positions are in device-independent pixels,
/// pressure is normalized to 0..=1, angles are degrees and timestamps are
/// monotonically increasing microseconds supplied by the input adapter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InkPoint {
    pub position: Point,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub twist: f32,
    pub timestamp: u64,
}

impl InkPoint {
    pub fn new(position: Point, pressure: f32) -> Result<Self, InkError> {
        let point = Self {
            position,
            pressure,
            tilt_x: 0.0,
            tilt_y: 0.0,
            twist: 0.0,
            timestamp: 0,
        };
        point.validate()?;
        Ok(point)
    }

    pub fn validate(&self) -> Result<(), InkError> {
        if !valid_position(self.position)
            || !self.pressure.is_finite()
            || !(0.0..=1.0).contains(&self.pressure)
            || !self.tilt_x.is_finite()
            || !(-90.0..=90.0).contains(&self.tilt_x)
            || !self.tilt_y.is_finite()
            || !(-90.0..=90.0).contains(&self.tilt_y)
            || !self.twist.is_finite()
            || !(0.0..360.0).contains(&self.twist)
        {
            return Err(InkError::InvalidPoint);
        }
        Ok(())
    }

    pub(crate) fn interpolate(a: Self, b: Self, t: f64) -> Self {
        let mix = |a: f32, b: f32| a + (b - a) * t as f32;
        // Twist follows the shortest arc, including the 359 -> 0 boundary.
        let twist_delta = (b.twist - a.twist + 540.0).rem_euclid(360.0) - 180.0;
        Self {
            position: Point::new(
                a.position.x + (b.position.x - a.position.x) * t,
                a.position.y + (b.position.y - a.position.y) * t,
            ),
            pressure: mix(a.pressure, b.pressure),
            tilt_x: mix(a.tilt_x, b.tilt_x),
            tilt_y: mix(a.tilt_y, b.tilt_y),
            twist: (a.twist + twist_delta * t as f32).rem_euclid(360.0),
            timestamp: a
                .timestamp
                .saturating_add(((b.timestamp.saturating_sub(a.timestamp)) as f64 * t) as u64),
        }
    }
}

pub(crate) fn valid_position(value: Point) -> bool {
    // A finite working range also prevents intermediate geometry overflow.
    value.x.is_finite() && value.y.is_finite() && value.x.abs() <= 1.0e12 && value.y.abs() <= 1.0e12
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_device_measurements() {
        assert!(InkPoint::new(Point::new(f64::NAN, 0.0), 0.5).is_err());
        assert!(InkPoint::new(Point::default(), -0.1).is_err());
        assert!(InkPoint::new(Point::default(), 0.0).is_ok());
        let mut p = InkPoint::new(Point::default(), 0.5).unwrap();
        p.tilt_y = 91.0;
        assert!(p.validate().is_err());
    }
    #[test]
    fn twist_interpolates_across_zero() {
        let mut a = InkPoint::new(Point::default(), 0.5).unwrap();
        let mut b = a;
        a.twist = 359.0;
        b.twist = 1.0;
        assert_eq!(InkPoint::interpolate(a, b, 0.5).twist, 0.0);
    }
}
