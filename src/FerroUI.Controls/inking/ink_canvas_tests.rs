//! Canvas integration tests exercise real routed input and the framework's
//! drawing contract without requiring a window, compositor, or native ink host.

use super::*;
use crate::test_support::{test_scope, TestRoot};
use crate::test_support_shapes::MockRenderInterfaceScope;
use crate::Control;
use ferroui_base::input::raw::RawPointerPoint;
use ferroui_base::input::{
    IPointer, InputElement, IntermediatePoints, KeyModifiers, MouseButton, Pointer,
    PointerEventArgs, PointerPointProperties, PointerPressedEventArgs, PointerReleasedEventArgs,
    PointerType, PointerUpdateKind, RawInputModifiers,
};
use ferroui_base::media::{
    BoxShadows, Color, DrawingContext, Geometry, GlyphRun, IBrush, IDrawingContextCore, IEffect,
    IPen, RenderOptions, TextOptions,
};
use ferroui_base::platform::{IGeometryImpl, SharedBitmapImpl};
use ferroui_base::rendering::scene_graph::ICustomDrawOperation;
use ferroui_base::utilities::RefCounted;
use ferroui_base::{Matrix, Point, Rect, Ref, RoundedRect, Size};
use std::cell::{Cell, LazyCell};
use std::rc::Rc;
use std::sync::Arc;

struct GeometryDraw {
    brush: Rc<dyn IBrush>,
    geometry: Arc<dyn IGeometryImpl>,
}

#[derive(Default)]
struct RecordingDrawingContext {
    geometries: Vec<GeometryDraw>,
    rectangles: Vec<(Rect, Option<Rc<dyn IBrush>>)>,
    clips: Vec<Rect>,
    clip_pops: usize,
}

impl IDrawingContextCore for RecordingDrawingContext {
    fn draw_geometry_impl_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        geometry: &Arc<dyn IGeometryImpl>,
    ) {
        assert!(pen.is_none(), "ink outlines are filled rather than stroked");
        self.geometries.push(GeometryDraw {
            brush: brush.expect("ink has a fill brush").clone(),
            geometry: geometry.clone(),
        });
    }

    fn draw_rectangle_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        _pen: Option<&Rc<dyn IPen>>,
        rect: RoundedRect,
        _shadows: &BoxShadows,
    ) {
        self.rectangles.push((rect.rect, brush.cloned()));
    }

    fn push_clip_core(&mut self, rect: Rect) {
        self.clips.push(rect);
    }

    fn pop_clip_core(&mut self) {
        self.clip_pops += 1;
    }

    fn draw_line_core(&mut self, _pen: &Rc<dyn IPen>, _p1: Point, _p2: Point) {}
    fn draw_ellipse_core(
        &mut self,
        _brush: Option<&Rc<dyn IBrush>>,
        _pen: Option<&Rc<dyn IPen>>,
        _rect: Rect,
    ) {
    }
    fn draw_bitmap(
        &mut self,
        _source: &RefCounted<SharedBitmapImpl>,
        _opacity: f64,
        _source_rect: Rect,
        _dest_rect: Rect,
    ) {
    }
    fn custom(&mut self, _custom: &Arc<dyn ICustomDrawOperation>) {}
    fn draw_glyph_run(&mut self, _brush: Option<&Rc<dyn IBrush>>, _run: &Rc<GlyphRun>) {}
    fn push_rounded_clip_core(&mut self, _rect: RoundedRect) {}
    fn push_geometry_clip_core(&mut self, _clip: &Ref<Geometry>) {}
    fn push_opacity_core(&mut self, _opacity: f64) {}
    fn push_opacity_mask_core(&mut self, _mask: &Rc<dyn IBrush>, _bounds: Rect) {}
    fn push_transform_core(&mut self, _matrix: Matrix) {}
    fn push_render_options_core(&mut self, _options: RenderOptions) {}
    fn push_text_options_core(&mut self, _options: TextOptions) {}
    fn push_effect_core(&mut self, _effect: &Rc<dyn IEffect>, _bounds: Rect) {}
    fn pop_geometry_clip_core(&mut self) {}
    fn pop_opacity_core(&mut self) {}
    fn pop_opacity_mask_core(&mut self) {}
    fn pop_transform_core(&mut self) {}
    fn pop_render_options_core(&mut self) {}
    fn pop_text_options_core(&mut self) {}
    fn pop_effect_core(&mut self) {}
    fn dispose_core(&mut self) {}
}

fn attached_canvas() -> (Ref<InkCanvas>, Ref<TestRoot>) {
    let canvas = InkCanvas::new();
    canvas.set_width(160.0);
    canvas.set_height(100.0);
    let root = TestRoot::with_child(canvas.clone());
    root.set_client_size(Size::new(160.0, 100.0));
    root.execute_initial_layout_pass();
    (canvas, root)
}

fn render(canvas: &InkCanvas) -> RecordingDrawingContext {
    let mut core = RecordingDrawingContext::default();
    {
        let mut context = DrawingContext::new(&mut core);
        canvas.render(&mut context);
    }
    core
}

fn pen_state(kind: PointerUpdateKind, pressure: f32) -> PointerPointProperties {
    PointerPointProperties::with_pen_state(
        if kind == PointerUpdateKind::LeftButtonReleased {
            RawInputModifiers::NONE
        } else {
            RawInputModifiers::LEFT_MOUSE_BUTTON
        },
        kind,
        12.0,
        pressure,
        3.0,
        -4.0,
    )
}

fn press(
    canvas: &InkCanvas,
    root: &TestRoot,
    pointer: &Rc<Pointer>,
    position: Point,
) -> PointerPressedEventArgs {
    let event = PointerPressedEventArgs::new(
        canvas.to_ref(),
        pointer.clone(),
        root,
        position,
        1,
        pen_state(PointerUpdateKind::LeftButtonPressed, 0.25),
        KeyModifiers::NONE,
        1,
    );
    canvas.raise_event(&event);
    event
}

#[test]
fn overlapping_highlighter_is_one_fill_and_geometry_is_cached_until_changed() {
    let _scope = test_scope();
    let _renderer = MockRenderInterfaceScope::install();
    let (canvas, _root) = attached_canvas();
    let attributes = InkDrawingAttributes {
        color: Color::from_uint32(0xffffee00),
        size: Size::new(12.0, 8.0),
        draw_as_highlighter: true,
        fit_to_curve: false,
        ..Default::default()
    };
    let points = [(10.0, 10.0), (90.0, 70.0), (10.0, 70.0), (90.0, 10.0)]
        .into_iter()
        .map(|(x, y)| InkPoint::new(Point::new(x, y), 1.0).unwrap())
        .collect();
    let stroke = InkStroke::new(points, attributes).unwrap();
    assert!(stroke.outline_polygons().len() > 1);
    canvas
        .ink_presenter()
        .stroke_container()
        .add_stroke(stroke.clone())
        .unwrap();

    let first = render(&canvas);
    assert_eq!(first.geometries.len(), 1);
    assert_eq!(first.geometries[0].brush.opacity(), 0.5);
    assert_eq!(
        first.geometries[0]
            .brush
            .as_solid_color_brush()
            .unwrap()
            .color(),
        attributes.color
    );
    assert_eq!(first.clips, vec![Rect::new(0.0, 0.0, 160.0, 100.0)]);
    assert_eq!(first.clip_pops, 1);
    assert_eq!(first.rectangles.len(), 1);
    assert_eq!(
        first.rectangles[0]
            .1
            .as_ref()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Color::from_uint32(0)
    );

    let second = render(&canvas);
    assert!(Arc::ptr_eq(
        &first.geometries[0].geometry,
        &second.geometries[0].geometry
    ));
    stroke
        .set_point_transform(Matrix::create_translation(7.0, 4.0))
        .unwrap();
    let moved = render(&canvas);
    assert_eq!(moved.geometries.len(), 1);
    assert!(!Arc::ptr_eq(
        &first.geometries[0].geometry,
        &moved.geometries[0].geometry
    ));
    let old_bounds = first.geometries[0].geometry.bounds();
    let new_bounds = moved.geometries[0].geometry.bounds();
    assert!((new_bounds.x - old_bounds.x - 7.0).abs() < 1e-9);
    assert!((new_bounds.y - old_bounds.y - 4.0).abs() < 1e-9);
    canvas.ink_presenter().stroke_container().clear();
    assert!(render(&canvas).geometries.is_empty());
}

#[test]
fn pressure_controls_drawn_nib_and_pencil_opacity_reaches_the_brush() {
    let _scope = test_scope();
    let _renderer = MockRenderInterfaceScope::install();
    let (canvas, _root) = attached_canvas();
    let attributes = InkDrawingAttributes {
        size: Size::new(8.0, 8.0),
        fit_to_curve: false,
        ..Default::default()
    };
    let full = InkStroke::new(
        vec![InkPoint::new(Point::new(20.0, 30.0), 1.0).unwrap()],
        attributes,
    )
    .unwrap();
    let light = InkStroke::new(
        vec![InkPoint::new(Point::new(50.0, 30.0), 0.25).unwrap()],
        attributes,
    )
    .unwrap();
    let pencil = InkStroke::new(
        vec![InkPoint::new(Point::new(80.0, 30.0), 1.0).unwrap()],
        InkDrawingAttributes {
            kind: InkDrawingAttributesKind::Pencil,
            pencil_opacity: 0.3,
            ..attributes
        },
    )
    .unwrap();
    canvas
        .ink_presenter()
        .stroke_container()
        .add_strokes([full, light, pencil])
        .unwrap();
    let drawn = render(&canvas);
    assert_eq!(drawn.geometries.len(), 3);
    assert!((drawn.geometries[0].geometry.bounds().height - 8.0).abs() < 1e-9);
    assert!((drawn.geometries[1].geometry.bounds().height - 2.0).abs() < 1e-9);
    assert_eq!(drawn.geometries[0].brush.opacity(), 1.0);
    assert!((drawn.geometries[2].brush.opacity() - 0.3).abs() < 1e-6);
}

#[test]
fn routed_pen_input_captures_releases_and_preserves_intermediate_telemetry() {
    let _scope = test_scope();
    let (canvas, root) = attached_canvas();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Pen, true);
    let down = press(&canvas, &root, &pointer, Point::new(5.0, 10.0));
    assert!(down.handled());
    assert!(pointer
        .captured()
        .is_some_and(|v| v == canvas.clone().upcast::<InputElement>()));
    let mut first = RawPointerPoint::new();
    first.position = Point::new(15.0, 12.0);
    first.pressure = 0.4;
    first.twist = 30.0;
    first.x_tilt = -15.0;
    first.y_tilt = 20.0;
    let mut second = RawPointerPoint::new();
    second.position = Point::new(25.0, 14.0);
    second.pressure = 0.6;
    second.twist = 45.0;
    second.x_tilt = -20.0;
    second.y_tilt = 25.0;
    let previous: IntermediatePoints =
        Rc::new(LazyCell::new(Box::new(move || Some(vec![first, second]))));
    let moved = PointerEventArgs::new(
        Some(InputElement::pointer_moved_event()),
        canvas.clone(),
        pointer.clone(),
        Some(&root),
        Point::new(35.0, 16.0),
        2,
        pen_state(PointerUpdateKind::Other, 0.8),
        KeyModifiers::NONE,
    )
    .with_previous_points(Some(previous));
    canvas.raise_event(&moved);
    assert!(moved.handled());
    let released = PointerReleasedEventArgs::new(
        canvas.clone(),
        pointer.clone(),
        &root,
        Point::new(45.0, 18.0),
        3,
        pen_state(PointerUpdateKind::LeftButtonReleased, 0.1),
        KeyModifiers::NONE,
        MouseButton::Left,
    );
    canvas.raise_event(&released);
    assert!(released.handled());
    assert!(pointer.captured().is_none());
    assert!(!canvas.ink_presenter().has_active_pointer(pointer.id()));
    let strokes = canvas.ink_presenter().stroke_container().get_strokes();
    assert_eq!(strokes.len(), 1);
    let points = strokes[0].ink_points();
    assert_eq!(points.len(), 5);
    assert_eq!(
        points.iter().map(|p| p.position.x).collect::<Vec<_>>(),
        [5.0, 15.0, 25.0, 35.0, 45.0]
    );
    assert_eq!(
        points.iter().map(|p| p.timestamp).collect::<Vec<_>>(),
        [1_000, 2_000, 2_000, 2_000, 3_000]
    );
    assert_eq!(points[1].pressure, 0.4);
    assert_eq!(
        (points[1].tilt_x, points[1].tilt_y, points[1].twist),
        (-15.0, 20.0, 30.0)
    );
    assert_eq!(points[2].pressure, 0.6);
    assert_eq!(
        (points[2].tilt_x, points[2].tilt_y, points[2].twist),
        (-20.0, 25.0, 45.0)
    );
}

#[test]
fn disabling_or_detaching_the_canvas_cancels_and_releases_captured_input() {
    let _scope = test_scope();
    let (canvas, root) = attached_canvas();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Pen, true);
    let canceled = Rc::new(Cell::new(0));
    let count = canceled.clone();
    canvas
        .ink_presenter()
        .stroke_input()
        .stroke_canceled(move |_| count.set(count.get() + 1));
    press(&canvas, &root, &pointer, Point::new(10.0, 10.0));
    assert!(pointer.captured().is_some());
    canvas.set_is_enabled(false);
    assert!(pointer.captured().is_none());
    assert!(!canvas.ink_presenter().has_active_pointer(pointer.id()));
    assert_eq!(canceled.get(), 1);
    canvas.set_is_enabled(true);
    press(&canvas, &root, &pointer, Point::new(20.0, 20.0));
    assert!(pointer.captured().is_some());
    root.set_child(None::<Ref<Control>>);
    assert!(pointer.captured().is_none());
    assert!(!canvas.ink_presenter().has_active_pointer(pointer.id()));
    assert_eq!(canceled.get(), 2);
    assert!(canvas
        .ink_presenter()
        .stroke_container()
        .get_strokes()
        .is_empty());
}

#[test]
fn cloned_unprocessed_args_share_handled_and_update_the_routed_event() {
    let _scope = test_scope();
    let (canvas, root) = attached_canvas();
    let presenter = canvas.ink_presenter();
    presenter
        .input_processing_configuration()
        .set_mode(InkInputProcessingMode::None);
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    presenter.unprocessed_input().pointer_pressed(move |event| {
        assert!(!event.handled());
        let clone = event.clone();
        clone.set_handled(true);
        assert!(event.handled());
        count.set(count.get() + 1);
    });
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Pen, true);
    let down = press(&canvas, &root, &pointer, Point::new(10.0, 10.0));
    assert_eq!(calls.get(), 1);
    assert!(down.handled());
    assert!(pointer.captured().is_none());
    assert!(!presenter.has_active_pointer(pointer.id()));
    assert!(presenter.stroke_container().get_strokes().is_empty());
}

#[test]
fn active_preview_reuses_geometry_and_invalidation_updates_only_its_stroke() {
    let _scope = test_scope();
    let _renderer = MockRenderInterfaceScope::install();
    let (canvas, root) = attached_canvas();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Pen, true);
    press(&canvas, &root, &pointer, Point::new(10.0, 10.0));
    let first = render(&canvas);
    let second = render(&canvas);
    assert_eq!(first.geometries.len(), 1);
    assert!(Arc::ptr_eq(
        &first.geometries[0].geometry,
        &second.geometries[0].geometry
    ));
    let moved = PointerEventArgs::new(
        Some(InputElement::pointer_moved_event()),
        canvas.clone(),
        pointer.clone(),
        Some(&root),
        Point::new(40.0, 10.0),
        2,
        pen_state(PointerUpdateKind::Other, 0.9),
        KeyModifiers::NONE,
    );
    canvas.raise_event(&moved);
    let changed = render(&canvas);
    assert_eq!(changed.geometries.len(), 1);
    assert!(!Arc::ptr_eq(
        &first.geometries[0].geometry,
        &changed.geometries[0].geometry
    ));
    assert!(
        changed.geometries[0].geometry.bounds().width > first.geometries[0].geometry.bounds().width
    );
    canvas.ink_presenter().cancel_all();
    assert!(pointer.captured().is_none());
    assert!(render(&canvas).geometries.is_empty());
}
