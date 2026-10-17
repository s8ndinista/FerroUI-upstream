use crate::geometry_impl::{impl_geometry_impl, register, FillPath, GeometryImpl, GeometryImplBase, VelloPath};
use ferroui_base::Rect;
use std::sync::Arc;

/// A kurbo geometry defined by stroke and fill paths.
pub struct SimpleGeometryImpl {
    base: GeometryImplBase,
    bounds: Rect,
    stroke_path: VelloPath,
    fill: FillPath,
}

impl SimpleGeometryImpl {
    /// Creates a geometry from a stroke path and a fill. `bounds` defaults to
    /// the tight bounds of the stroke path.
    pub fn new(stroke_path: VelloPath, fill: FillPath, bounds: Option<Rect>) -> Arc<Self> {
        let bounds = bounds.unwrap_or_else(|| stroke_path.tight_bounds());

        register(Self { base: GeometryImplBase::new(), bounds, stroke_path, fill })
    }

    fn geometry_bounds(&self) -> Rect {
        self.bounds
    }
}

impl GeometryImpl for SimpleGeometryImpl {
    fn base(&self) -> &GeometryImplBase {
        &self.base
    }

    fn stroke_path(&self) -> Option<VelloPath> {
        Some(self.stroke_path.clone())
    }

    fn fill(&self) -> FillPath {
        self.fill.clone()
    }
}

impl_geometry_impl!(SimpleGeometryImpl);
