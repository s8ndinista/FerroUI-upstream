use super::{InkError, InkPresenter};
use ferroui_base::{Matrix, Point};
use std::cell::Cell;
use std::rc::Weak;

fn validate_transform(value: Matrix) -> Result<(), InkError> {
    let a = value.transform(Point::new(0.0, 0.0));
    let b = value.transform(Point::new(1.0, 1.0));
    if ![a.x, a.y, b.x, b.y].iter().all(|v| v.is_finite()) || value.try_invert().is_none() {
        return Err(InkError::InvalidInput(
            "stencil transform must be finite and invertible",
        ));
    }
    Ok(())
}

/// Portable ruler service. The local horizontal edges constrain nearby ink.
pub struct InkPresenterRuler {
    presenter: Weak<InkPresenter>,
    pub(crate) visible: Cell<bool>,
    transform: Cell<Matrix>,
    length: Cell<f64>,
    width: Cell<f64>,
    tick_marks: Cell<bool>,
}

impl PartialEq for InkPresenterRuler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl InkPresenterRuler {
    pub(crate) fn new(presenter: Weak<InkPresenter>) -> Self {
        Self {
            presenter,
            visible: Cell::new(false),
            transform: Cell::new(Matrix::IDENTITY),
            length: Cell::new(600.0),
            width: Cell::new(60.0),
            tick_marks: Cell::new(true),
        }
    }
    pub fn is_visible(&self) -> bool {
        self.visible.get()
    }
    pub fn set_is_visible(&self, value: bool) {
        if let Some(p) = self.presenter.upgrade() {
            p.set_ruler_visible(value);
        } else {
            self.visible.set(value);
        }
    }
    pub fn transform(&self) -> Matrix {
        self.transform.get()
    }
    pub fn set_transform(&self, value: Matrix) -> Result<(), InkError> {
        validate_transform(value)?;
        self.transform.set(value);
        self.changed();
        Ok(())
    }
    pub fn length(&self) -> f64 {
        self.length.get()
    }
    pub fn set_length(&self, value: f64) -> Result<(), InkError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(InkError::InvalidInput("ruler length must be positive"));
        }
        self.length.set(value);
        self.changed();
        Ok(())
    }
    pub fn width(&self) -> f64 {
        self.width.get()
    }
    pub fn set_width(&self, value: f64) -> Result<(), InkError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(InkError::InvalidInput("ruler width must be positive"));
        }
        self.width.set(value);
        self.changed();
        Ok(())
    }
    pub fn are_tick_marks_visible(&self) -> bool {
        self.tick_marks.get()
    }
    pub fn set_are_tick_marks_visible(&self, value: bool) {
        self.tick_marks.set(value);
        self.changed();
    }
    fn changed(&self) {
        if let Some(p) = self.presenter.upgrade() {
            p.invalidate();
        }
    }

    pub(crate) fn constrain(&self, point: Point, tolerance: f64) -> Point {
        if !self.is_visible() {
            return point;
        }
        let Some(inverse) = self.transform().try_invert() else {
            return point;
        };
        let p = inverse.transform(point);
        if p.x < 0.0 || p.x > self.length() {
            return point;
        }
        let edge = if p.y.abs() < (p.y - self.width()).abs() {
            0.0
        } else {
            self.width()
        };
        let candidate = self.transform().transform(Point::new(p.x, edge));
        if (candidate.x - point.x).hypot(candidate.y - point.y) <= tolerance {
            candidate
        } else {
            point
        }
    }
}

/// Portable circular guide, sharing the presenter's exclusive stencil state.
pub struct InkPresenterProtractor {
    presenter: Weak<InkPresenter>,
    pub(crate) visible: Cell<bool>,
    transform: Cell<Matrix>,
    radius: Cell<f64>,
    center_marker: Cell<bool>,
    angle_readout: Cell<bool>,
}

impl PartialEq for InkPresenterProtractor {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl InkPresenterProtractor {
    pub(crate) fn new(presenter: Weak<InkPresenter>) -> Self {
        Self {
            presenter,
            visible: Cell::new(false),
            transform: Cell::new(Matrix::IDENTITY),
            radius: Cell::new(150.0),
            center_marker: Cell::new(true),
            angle_readout: Cell::new(true),
        }
    }
    pub fn is_visible(&self) -> bool {
        self.visible.get()
    }
    pub fn set_is_visible(&self, value: bool) {
        if let Some(p) = self.presenter.upgrade() {
            p.set_protractor_visible(value);
        } else {
            self.visible.set(value);
        }
    }
    pub fn transform(&self) -> Matrix {
        self.transform.get()
    }
    pub fn set_transform(&self, value: Matrix) -> Result<(), InkError> {
        validate_transform(value)?;
        self.transform.set(value);
        self.changed();
        Ok(())
    }
    pub fn radius(&self) -> f64 {
        self.radius.get()
    }
    pub fn set_radius(&self, value: f64) -> Result<(), InkError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(InkError::InvalidInput("protractor radius must be positive"));
        }
        self.radius.set(value);
        self.changed();
        Ok(())
    }
    pub fn is_center_marker_visible(&self) -> bool {
        self.center_marker.get()
    }
    pub fn set_is_center_marker_visible(&self, value: bool) {
        self.center_marker.set(value);
        self.changed();
    }
    pub fn are_angle_readouts_visible(&self) -> bool {
        self.angle_readout.get()
    }
    pub fn set_are_angle_readouts_visible(&self, value: bool) {
        self.angle_readout.set(value);
        self.changed();
    }
    fn changed(&self) {
        if let Some(p) = self.presenter.upgrade() {
            p.invalidate();
        }
    }
    pub(crate) fn constrain(&self, point: Point, tolerance: f64) -> Point {
        if !self.is_visible() {
            return point;
        }
        let Some(inverse) = self.transform().try_invert() else {
            return point;
        };
        let p = inverse.transform(point);
        let distance = p.x.hypot(p.y);
        if distance <= f64::EPSILON {
            return point;
        }
        let candidate = self.transform().transform(Point::new(
            p.x * self.radius() / distance,
            p.y * self.radius() / distance,
        ));
        if (candidate.x - point.x).hypot(candidate.y - point.y) <= tolerance {
            candidate
        } else {
            point
        }
    }
}
