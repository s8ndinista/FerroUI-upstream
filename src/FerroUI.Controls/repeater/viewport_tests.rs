// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md for license information.
// Behavioral ports of ViewportTests, EffectiveViewportScrollViewerTests and
// EffectiveViewportScrollPresenterTests. Native ScrollContentPresenter and
// LayoutTransformControl provide scrolling and zoom geometry in FerroUI.

use super::*;
use crate::presenters::ScrollContentPresenter;
use crate::primitives::{Popup, ScrollBarVisibility};
use crate::templates::{FuncControlTemplate, FuncDataTemplate};
use crate::test_support::{test_scope, TestRoot};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{
    Border, Control, Grid, ItemsSource, LayoutTransformControl, ScrollViewer, TextBlock, Window,
};
use ferroui_base::data::BindingPriority;
use ferroui_base::layout::{HorizontalAlignment, ILayoutManager, VerticalAlignment};
use ferroui_base::media::ScaleTransform;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObject, FerroObjectExtensions,
    FerroObjectImpl, Point, Rect, Ref, Size, StyledElement, Thickness, Vector,
};
use std::cell::{Cell, RefCell};

#[repr(C)]
struct ViewportProbe {
    base: VirtualizingLayout,
    extent: Cell<Size>,
    measures: RefCell<Vec<Rect>>,
    arranges: RefCell<Vec<Rect>>,
    visible: RefCell<Vec<Rect>>,
}
ferro_class!(ViewportProbe: VirtualizingLayout);
ferro_impl_classes!(ViewportProbe: FerroObjectImpl, LayoutImpl);
impl VirtualizingLayoutImpl for ViewportProbe {
    fn measure_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        _available: Size,
    ) -> Size {
        this.measures.borrow_mut().push(context.realization_rect());
        this.visible.borrow_mut().push(context.visible_rect());
        this.extent.get()
    }
    fn arrange_override(
        this: &Self,
        context: &Ref<VirtualizingLayoutContext>,
        final_size: Size,
    ) -> Size {
        this.arranges.borrow_mut().push(context.realization_rect());
        final_size
    }
}
impl ViewportProbe {
    fn new(extent: Size) -> Ref<Self> {
        instantiate(Self {
            base: VirtualizingLayout::construct(),
            extent: Cell::new(extent),
            measures: RefCell::new(Vec::new()),
            arranges: RefCell::new(Vec::new()),
            visible: RefCell::new(Vec::new()),
        })
    }
    fn last(&self) -> Rect {
        *self
            .measures
            .borrow()
            .last()
            .expect("The layout was measured.")
    }
}

fn probe(extent: Size) -> (Ref<ItemsRepeater>, Ref<ViewportProbe>) {
    let layout = ViewportProbe::new(extent);
    let repeater = ItemsRepeater::new();
    repeater.set_layout(Some(layout.clone().upcast()));
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    (repeater, layout)
}

fn viewer(content: &Ref<Control>, size: Option<Size>) -> Ref<ScrollViewer> {
    let viewer = ScrollViewer::new();
    viewer.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
        crate::scroll_viewer_tests::create_template,
    )));
    viewer.set_content(Some(Control::boxed(content)));
    viewer.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    viewer.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    if let Some(size) = size {
        viewer.set_width(size.width);
        viewer.set_height(size.height);
    }
    viewer
}

fn settle(root: &TestRoot) {
    // Native cache growth queues background measure work; no wall-clock waits.
    for _ in 0..16 {
        root.layout_manager().execute_layout_pass();
        Dispatcher::ui_thread().run_jobs(None);
    }
}

fn assert_close(actual: Rect, expected: Rect) {
    for (actual, expected) in [
        (actual.x, expected.x),
        (actual.y, expected.y),
        (actual.width, expected.width),
        (actual.height, expected.height),
    ] {
        assert!(
            (actual - expected).abs() < 0.01,
            "expected {expected}, observed {actual}"
        );
    }
}

#[test]
fn validate_no_scrolling_surface_scenario() {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(500.0, 500.0));
    let root = TestRoot::with_child(&repeater);
    root.set_client_size(Size::new(800.0, 600.0));
    // TestRoot measures its child against ClientSize but otherwise sizes to
    // content. Give it the fixed bounds of the upstream test window too.
    root.set_width(800.0);
    root.set_height(600.0);
    root.execute_initial_layout_pass();
    assert_eq!(layout.measures.borrow()[0], Rect::default());
    assert_close(layout.last(), Rect::new(0.0, 0.0, 800.0, 600.0));
    assert_eq!(*layout.measures.borrow(), *layout.visible.borrow());
    repeater.set_vertical_cache_length(10.0);
    settle(&root);
    assert_close(layout.last(), Rect::new(0.0, 0.0, 800.0, 600.0));
}

#[test]
fn validate_no_scrolling_surface_in_popup_scenario() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let (repeater, layout) = probe(Size::new(200.0, 150.0));
    let target = Border::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&target)));
    window.show();
    window.layout_manager().execute_initial_layout_pass();
    let popup = Popup::new();
    popup.set_child(&repeater);
    popup.set_placement_target(Some(target.upcast()));
    popup.set_is_open(true);
    Dispatcher::ui_thread().run_jobs(None);
    let host = popup.host().expect("The native popup opened.").as_control();
    let layout_root = host
        .get_layout_root()
        .expect("The popup has a layout root.");
    layout_root.layout_manager().execute_initial_layout_pass();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(!layout.measures.borrow().is_empty());
    let viewport = layout.last();
    assert!(viewport.width.is_finite() && viewport.height.is_finite());
    assert!(viewport.width > 0.0 && viewport.height > 0.0);
    assert_eq!(*layout.measures.borrow(), *layout.visible.borrow());
    popup.set_is_open(false);
    assert!(!repeater.is_attached_to_visual_tree());
    window.close();
}

fn single_viewer(with_host: bool) {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(500.0, 600.0));
    let scroll = viewer(&repeater.clone().upcast(), Some(Size::new(200.0, 300.0)));
    let host = with_host.then(|| {
        let host = ItemsRepeaterScrollHost::new();
        host.set_scroll_viewer(Some(scroll.clone()));
        host
    });
    let root = TestRoot::with_child(
        host.as_ref()
            .map(|host| host.clone().upcast::<Control>())
            .unwrap_or_else(|| scroll.clone().upcast()),
    );
    root.execute_initial_layout_pass();
    assert_eq!(layout.measures.borrow()[0], Rect::default());
    assert_close(layout.last(), Rect::new(0.0, 0.0, 200.0, 300.0));
    scroll.set_offset(Vector::new(0.0, 100.0));
    settle(&root);
    assert_close(layout.last(), Rect::new(0.0, 100.0, 200.0, 300.0));
    scroll.set_offset(Vector::new(400.0, 100.0));
    settle(&root);
    assert_eq!(scroll.offset(), Vector::new(300.0, 100.0));
    assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 300.0));
    assert_eq!(*layout.measures.borrow(), *layout.visible.borrow());
}

#[test]
fn validate_basic_scroll_viewer_scenario() {
    single_viewer(false);
}

#[test]
fn validate_items_repeater_scroll_host_scenario() {
    single_viewer(true);
}

#[test]
fn validate_one_scroll_presenter_scenario() {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(500.0, 600.0));
    let presenter = ScrollContentPresenter::new();
    presenter.set_can_horizontally_scroll(true);
    presenter.set_can_vertically_scroll(true);
    presenter.set_content(Some(Control::boxed(&repeater)));
    presenter.set_width(200.0);
    presenter.set_height(300.0);
    let root = TestRoot::with_child(&presenter);
    root.execute_initial_layout_pass();
    assert_eq!(layout.measures.borrow()[0], Rect::default());
    assert_close(layout.last(), Rect::new(0.0, 0.0, 200.0, 300.0));
    presenter.set_offset(Vector::new(0.0, 100.0));
    settle(&root);
    assert_close(layout.last(), Rect::new(0.0, 100.0, 200.0, 300.0));
    presenter.set_offset(Vector::new(400.0, 100.0));
    settle(&root);
    assert_eq!(presenter.offset(), Vector::new(300.0, 100.0));
    assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 300.0));
}

fn two_scrollers(presenters: bool) {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(500.0, 500.0));
    if presenters {
        let horizontal = ScrollContentPresenter::new();
        horizontal.set_can_horizontally_scroll(true);
        horizontal.set_can_vertically_scroll(false);
        horizontal.set_content(Some(Control::boxed(&repeater)));
        let grid = Grid::new();
        grid.children().add(&horizontal);
        let vertical = ScrollContentPresenter::new();
        vertical.set_can_horizontally_scroll(false);
        vertical.set_can_vertically_scroll(true);
        vertical.set_content(Some(Control::boxed(&grid)));
        vertical.set_width(200.0);
        vertical.set_height(200.0);
        let root = TestRoot::with_child(&vertical);
        root.execute_initial_layout_pass();
        assert_close(layout.last(), Rect::new(0.0, 0.0, 200.0, 200.0));
        vertical.set_offset(Vector::new(0.0, 100.0));
        settle(&root);
        assert_close(layout.last(), Rect::new(0.0, 100.0, 200.0, 200.0));
        horizontal.set_offset(Vector::new(400.0, 0.0));
        settle(&root);
        assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 200.0));
    } else {
        let horizontal = viewer(&repeater.clone().upcast(), None);
        horizontal.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        let vertical = viewer(&horizontal.clone().upcast(), Some(Size::new(200.0, 200.0)));
        vertical.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        let root = TestRoot::with_child(&vertical);
        root.execute_initial_layout_pass();
        assert_close(layout.last(), Rect::new(0.0, 0.0, 200.0, 200.0));
        vertical.set_offset(Vector::new(0.0, 100.0));
        settle(&root);
        assert_close(layout.last(), Rect::new(0.0, 100.0, 200.0, 200.0));
        horizontal.set_offset(Vector::new(400.0, 0.0));
        settle(&root);
        assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 200.0));
    }
}

#[test]
fn validate_two_scroll_viewer_scenario() {
    two_scrollers(false);
}

#[test]
fn validate_two_scroll_presenters_scenario() {
    two_scrollers(true);
}

#[test]
fn scale_transform_supplies_the_zoomed_effective_viewport() {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(500.0, 600.0));
    let zoom = LayoutTransformControl::new();
    zoom.set_child(&repeater);
    let scale = ScaleTransform::with_scale(1.0, 1.0);
    zoom.set_layout_transform(Some(scale.clone().into()));
    let scroll = viewer(&zoom.clone().upcast(), Some(Size::new(200.0, 300.0)));
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    scroll.set_offset(Vector::new(300.0, 100.0));
    settle(&root);
    assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 300.0));
    scale.set_scale_x(2.0);
    scale.set_scale_y(2.0);
    settle(&root);
    assert_close(layout.last(), Rect::new(150.0, 50.0, 100.0, 150.0));
    assert_eq!(scroll.extent(), Size::new(1000.0, 1200.0));
    scale.set_scale_x(1.0);
    scale.set_scale_y(1.0);
    settle(&root);
    assert_close(layout.last(), Rect::new(300.0, 100.0, 200.0, 300.0));
}

fn cache_growth(presenter: bool) {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(1000.0, 2000.0));
    repeater.set_horizontal_cache_length(2.0);
    repeater.set_vertical_cache_length(2.0);
    let content = if presenter {
        let presenter = ScrollContentPresenter::new();
        presenter.set_can_horizontally_scroll(true);
        presenter.set_can_vertically_scroll(true);
        presenter.set_width(400.0);
        presenter.set_height(400.0);
        presenter.set_content(Some(Control::boxed(&repeater)));
        presenter.upcast::<Control>()
    } else {
        viewer(&repeater.clone().upcast(), Some(Size::new(400.0, 400.0))).upcast()
    };
    let root = TestRoot::with_child(content);
    root.execute_initial_layout_pass();
    settle(&root);
    let expected = Rect::new(-400.0, -400.0, 1200.0, 1200.0);
    assert_close(layout.last(), expected);
    assert_close(*layout.arranges.borrow().last().unwrap(), expected);
    for windows in [&layout.measures, &layout.arranges] {
        let mut windows = windows.borrow().clone();
        windows.dedup();
        let windows: Vec<_> = windows
            .into_iter()
            .filter(|rect| rect.width > 0.0)
            .collect();
        for pair in windows.windows(2) {
            assert_eq!(pair[1].x - pair[0].x, -40.0);
            assert_eq!(pair[1].y - pair[0].y, -40.0);
            assert_eq!(pair[1].width - pair[0].width, 80.0);
            assert_eq!(pair[1].height - pair[0].height, 80.0);
        }
    }
}

#[test]
fn can_grow_cache_buffer_with_scroll_viewer() {
    cache_growth(false);
}

#[test]
fn can_grow_cache_buffer_with_scroll_presenter() {
    cache_growth(true);
}

fn aligned_destination(vertical_scroll: bool) {
    let _scope = test_scope();
    for alignment in 0..3 {
        let (repeater, layout) = probe(if vertical_scroll {
            Size::new(100.0, 3000.0)
        } else {
            Size::new(3000.0, 100.0)
        });
        if vertical_scroll {
            repeater.set_horizontal_alignment(
                [
                    HorizontalAlignment::Left,
                    HorizontalAlignment::Center,
                    HorizontalAlignment::Right,
                ][alignment],
            );
        } else {
            repeater.set_vertical_alignment(
                [
                    VerticalAlignment::Top,
                    VerticalAlignment::Center,
                    VerticalAlignment::Bottom,
                ][alignment],
            );
        }
        let scroll = viewer(&repeater.clone().upcast(), Some(Size::new(400.0, 300.0)));
        let root = TestRoot::with_child(&scroll);
        root.execute_initial_layout_pass();
        scroll.set_offset(if vertical_scroll {
            Vector::new(0.0, 1000.0)
        } else {
            Vector::new(1000.0, 0.0)
        });
        // FerroUI commits native offsets synchronously; its next layout has
        // the destination window before the frame is rendered.
        root.layout_manager().execute_layout_pass();
        let expected = if vertical_scroll {
            Rect::new(-(alignment as f64) * 150.0, 1000.0, 400.0, 300.0)
        } else {
            Rect::new(1000.0, -(alignment as f64) * 100.0, 400.0, 300.0)
        };
        assert_close(layout.last(), expected);
        repeater.invalidate_measure();
        settle(&root);
        assert_close(layout.last(), expected);
        root.set_child(None);
    }
}

#[test]
fn validate_scroll_starting_anticipation_for_horizontally_aligned_repeater() {
    aligned_destination(true);
}

#[test]
fn validate_scroll_starting_anticipation_for_vertically_aligned_repeater() {
    aligned_destination(false);
}

#[test]
fn can_bring_into_view_elements_with_native_scroll_viewer() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let item = Border::new();
            item.set_height(100.0);
            Some(item.upcast())
        },
        true,
    )));
    repeater.set_items_source(Some(ItemsSource::from_values(0..400)));
    let scroll = viewer(&repeater.clone().upcast(), Some(Size::new(400.0, 600.0)));
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    for index in [100, 103, 399, 0] {
        let element = repeater.get_or_create_element(index);
        element.bring_into_view();
        settle(&root);
        let position = element.translate_point(Point::default(), &scroll).unwrap();
        assert!(
            position.y >= -0.01 && position.y + element.bounds().height <= 600.01,
            "item {index} at {position:?}, scroll {:?}",
            scroll.offset()
        );
        assert_eq!(repeater.get_element_index(&element), index);
        assert!(repeater.viewport_manager.made_anchor().is_none());
        assert!(repeater.realized_elements().len() <= 10);
    }
}

#[test]
fn verify_uniform_grid_layout_doesnt_crash_when_trying_to_scroll_to_end() {
    let _scope = test_scope();
    let repeater = ItemsRepeater::new();
    let grid = UniformGridLayout::new();
    grid.set_min_item_width(100.0);
    grid.set_min_item_height(40.0);
    grid.set_min_row_spacing(10.0);
    grid.set_min_column_spacing(10.0);
    repeater.set_layout(Some(grid.upcast()));
    repeater.set_items_source(Some(ItemsSource::from_items((0..1000).map(|index| {
        let text = TextBlock::new();
        text.set_text(Some(&format!("#{index}")));
        let border = Border::new();
        border.set_child(&text);
        Some(Control::boxed(border))
    }))));
    let scroll = viewer(&repeater.clone().upcast(), Some(Size::new(400.0, 600.0)));
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    settle(&root);
    scroll.set_offset(Vector::new(0.0, repeater.bounds().height));
    settle(&root);
    assert!(repeater.try_get_element(999).is_some());
    scroll.set_offset(Vector::default());
    settle(&root);
    assert!(repeater.try_get_element(0).is_some());
    assert_eq!(scroll.offset(), Vector::default());
}

#[test]
fn verify_uniform_grid_layout_doesnt_hang_when_trying_to_scroll_to_start() {
    let _scope = test_scope();
    let outer = ItemsRepeater::new();
    outer.set_items_source(Some(ItemsSource::from_values(
        (0..10).map(|_| ItemsSource::from_values(1..7)),
    )));
    outer.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let inner = ItemsRepeater::new();
            let grid = UniformGridLayout::new();
            grid.set_orientation(ferroui_base::layout::Orientation::Horizontal);
            grid.set_min_item_width(150.0);
            grid.set_min_item_height(150.0);
            grid.set_maximum_rows_or_columns(3);
            grid.set_min_row_spacing(4.0);
            grid.set_min_column_spacing(4.0);
            grid.set_items_stretch(UniformGridLayoutItemsStretch::Fill);
            inner.set_layout(Some(grid.upcast()));
            let object: &FerroObject = &inner;
            inner.bind(
                ItemsRepeater::items_source_property(),
                FerroObjectExtensions::get_observable_with(
                    object,
                    StyledElement::data_context_property(),
                    |data: Option<BoxedValue>| {
                        data.and_then(|data| data.downcast_ref::<ItemsSource>().cloned())
                    },
                ),
                BindingPriority::LocalValue,
            );
            Some(inner.upcast())
        },
        true,
    )));
    let scroll = viewer(&outer.clone().upcast(), Some(Size::new(500.0, 400.0)));
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    let root = TestRoot::with_child(&scroll);
    root.execute_initial_layout_pass();
    settle(&root);
    for step in (1..=16).chain((0..16).rev()) {
        let end = (scroll.extent().height - scroll.viewport().height).max(0.0);
        scroll.set_offset(Vector::new(0.0, end * step as f64 / 16.0));
        settle(&root);
        assert!(scroll.offset().y.is_finite());
        assert!(!outer.realized_elements().is_empty());
    }
    assert_eq!(scroll.offset().y, 0.0);
    let first = outer
        .try_get_element(0)
        .unwrap()
        .cast::<ItemsRepeater>()
        .unwrap();
    assert!(first.try_get_element(0).is_some());
}

#[test]
fn can_bring_into_view_elements_with_alignment_and_animation() {
    let _scope = test_scope();
    let clock = super::services_tests::TestClock::install();
    let repeater = ItemsRepeater::new();
    repeater.set_horizontal_cache_length(0.0);
    repeater.set_vertical_cache_length(0.0);
    repeater.set_items_source(Some(ItemsSource::from_values(0..400)));
    repeater.set_data_template(Some(FuncDataTemplate::new(
        |_| true,
        |_, _| {
            let border = Border::new();
            border.set_height(94.0);
            border.set_margin(Thickness::uniform(5.0));
            Some(border.upcast())
        },
        true,
    )));
    let scroll = viewer(&repeater.clone().upcast(), None);
    scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    let host = ItemsRepeaterScrollHost::new();
    host.set_width(400.0);
    host.set_height(600.0);
    host.set_scroll_viewer(Some(scroll.clone()));
    let root = TestRoot::with_child(&host);
    root.execute_initial_layout_pass();
    settle(&root);

    // Preserve the source sequence and outer dimensions. FerroUI's native
    // compatibility host supplies alignment/animation options, and its clock
    // replaces CompositionTarget ticks without wall-clock sleeps.
    for (step, (index, alignment, animate, expected_first, expected_last)) in [
        (100, 0.5, false, 96, 103),
        (103, 0.5, true, 99, 107),
        (0, 0.5, true, 0, 6),
        (20, 0.0, false, 19, 26),
    ]
    .into_iter()
    .enumerate()
    {
        let target = repeater.get_or_create_element(index);
        host.start_bring_into_view(&target, 0.0, alignment, 0.0, 0.0, animate);
        settle(&root);
        if animate {
            let time = step as f64 * 1000.0;
            clock.pulse(time);
            settle(&root);
            clock.pulse(time + 125.0);
            settle(&root);
            clock.pulse(time + 251.0);
            settle(&root);
        }
        assert!(!host.has_pending_bring_into_view());
        let mut realized: Vec<_> = repeater
            .realized_elements()
            .iter()
            .map(|element| repeater.get_element_index(element))
            .collect();
        realized.sort_unstable();
        assert_eq!(
            realized,
            (expected_first..=expected_last).collect::<Vec<_>>()
        );
        if index == 0 {
            assert_eq!(scroll.offset().y, 0.0);
        }
        let before = scroll.offset();
        host.start_bring_into_view(&target, 0.0, alignment, 0.0, 0.0, false);
        settle(&root);
        assert_eq!(
            scroll.offset(),
            before,
            "repeating the same aligned request is a no-op"
        );
    }
}

#[test]
fn detach_and_reparent_reset_viewport_cache_and_anchor_lifetimes() {
    let _scope = test_scope();
    let (repeater, layout) = probe(Size::new(1000.0, 2000.0));
    repeater.set_vertical_cache_length(2.0);
    let first = viewer(&repeater.clone().upcast(), Some(Size::new(200.0, 300.0)));
    let second = viewer(&Border::new().upcast(), Some(Size::new(400.0, 500.0)));
    let panel = Grid::new();
    panel.children().add(&first);
    panel.children().add(&second);
    let root = TestRoot::with_child(&panel);
    root.execute_initial_layout_pass();
    settle(&root);
    first.set_offset(Vector::new(0.0, 700.0));
    settle(&root);
    assert_eq!(repeater.visible_window().y, 700.0);
    first.set_content(None);
    assert!(!repeater.is_attached_to_visual_tree());
    second.set_content(Some(Control::boxed(&repeater)));
    settle(&root);
    assert_close(
        *layout.visible.borrow().last().unwrap(),
        Rect::new(0.0, 0.0, 400.0, 500.0),
    );
    let weak = repeater.downgrade();
    second.set_content(None);
    drop(repeater);
    settle(&root);
    assert!(weak.upgrade().is_none());
}
