//! Port of upstream's `PathGeometryTests.cs` of the Skia unit tests.

use super::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::{PathGeometry, TranslateTransform};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::{Point, Rect};
use ferroui_controls::testing::UnitTestApplication;
use std::rc::Rc;

#[test]
fn open_keeps_content_when_transform_is_set() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_render_interface(Rc::new(PlatformRenderInterface::default())),
    );

    let geometry = PathGeometry::new();
    geometry.set_transform(TranslateTransform::with_offset(10.0, 20.0));

    {
        let mut context = geometry.open();
        context.begin_figure(Point::new(20.0, 20.0), true);
        context.line_to(Point::new(100.0, 20.0), true);
        context.line_to(Point::new(100.0, 100.0), true);
        context.end_figure(true);
        context.dispose();
    }

    assert_eq!(Rect::new(30.0, 40.0, 80.0, 80.0), geometry.bounds());
}
