// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.
// Source contracts: LinedFlowLayout::{RequestItemsInfo, ArrangeOverride,
// InvalidateMeasureTimerTick, UninitializeForContextCore} and FerroUI Layoutable.

use super::super::layout_tests::{base, run, TestHost};
use super::*;
use ferroui_base::Thickness;

fn settle(layout: &LinedFlowLayout, context: &Ref<VirtualizingLayoutContext>, available: Size) {
    for _ in 0..5 {
        run(layout, context, available);
    }
}

#[test]
fn maximum_width_keeps_the_control_margin_in_its_allocated_slot() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(3, Size::new(60.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    settle(&layout, &context, Size::new(500.0, 100.0));
    let first = host.element(0);
    first.set_max_width(40.0);
    first.set_margin(Thickness::new(7.0, 0.0, 13.0, 0.0));
    settle(&layout, &context, Size::new(500.0, 100.0));
    assert_eq!(first.desired_size().width, 60.0);
    assert_eq!(first.bounds(), Rect::new(7.0, 0.0, 40.0, 20.0));
    assert_eq!(host.element(1).bounds().x, 60.0);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn conflicting_control_constraints_follow_ferroui_minimum_precedence() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(3, Size::new(60.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    settle(&layout, &context, Size::new(500.0, 100.0));
    let first = host.element(0);
    first.set_min_width(100.0);
    first.set_max_width(20.0);
    first.set_margin(Thickness::new(7.0, 0.0, 13.0, 0.0));
    settle(&layout, &context, Size::new(500.0, 100.0));
    assert_eq!(first.desired_size().width, 120.0);
    assert_eq!(first.bounds(), Rect::new(7.0, 0.0, 100.0, 20.0));
    assert_eq!(host.element(1).bounds().x, 120.0);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn partial_first_and_last_lines_arrange_their_retained_controls() {
    let _scope = crate::test_support::test_scope();
    for metadata in [false, true] {
        let host = TestHost::uniform(10, Size::new(20.0, 20.0));
        let context = host.context();
        let layout = LinedFlowLayout::new();
        layout.set_line_height(20.0);
        let _metadata = metadata.then(|| {
            layout.items_info_requested(|args| {
                args.set_items_range_start_index(0).unwrap();
                args.set_desired_aspect_ratios(&[1.0; 10]).unwrap();
            })
        });
        layout.initialize_for_context(&base(&context));
        settle(&layout, &context, Size::new(100.0, 100.0));
        let expected = (2..8)
            .map(|index| (host.element(index), host.element(index).bounds()))
            .collect::<Vec<_>>();
        layout.element_manager.discard_elements_outside_window(false, 1);
        layout.element_manager.discard_elements_outside_window(true, 8);
        let created = host.created.get();
        for (element, _) in &expected {
            element.arrange(Rect::new(999.0, 999.0, 20.0, 20.0));
        }
        layout.arrange(&base(&context), Size::new(100.0, 100.0));
        for (element, bounds) in expected {
            assert_eq!(element.bounds(), bounds);
        }
        assert_eq!(layout.realized_range(), (2, 7));
        assert_eq!(host.created.get(), created);
        layout.uninitialize_for_context(&base(&context));
    }
}

#[test]
fn declining_one_metadata_interval_falls_back_for_the_complete_window() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(1000, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.initialize_for_context(&base(&context));
    *layout.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo {
        start: 100,
        ratios: vec![2.0; 20],
        ..LinedFlowItemsInfo::default()
    });
    let requests = Rc::new(RefCell::new(Vec::new()));
    let observed = requests.clone();
    let _metadata = layout.items_info_requested(move |args| {
        observed.borrow_mut().push(args.items_range_start_index());
        if args.items_range_start_index() < 100 {
            args.set_desired_aspect_ratios(&vec![2.0; args.items_range_requested_length() as usize])
                .unwrap();
        }
    });
    assert!(!layout.request_items_info(90, 130));
    assert_eq!(*requests.borrow(), vec![90, 120]);
    assert_eq!(layout.requested_range_start_index(), -1);
    assert_eq!(layout.requested_range_length(), 0);
    assert!(!layout.uses_fast_path_layout());
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn expanded_metadata_range_is_retained_and_reused_without_another_request() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(1000, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.initialize_for_context(&base(&context));
    let requests = Rc::new(Cell::new(0));
    let observed = requests.clone();
    let _metadata = layout.items_info_requested(move |args| {
        observed.set(observed.get() + 1);
        args.set_items_range_start_index(80).unwrap();
        args.set_desired_aspect_ratios(&[1.0; 100]).unwrap();
    });
    assert!(!layout.request_items_info(100, 120));
    assert_eq!(layout.requested_range_start_index(), 80);
    assert_eq!(layout.requested_range_length(), 100);
    assert!(!layout.request_items_info(90, 160));
    assert_eq!(requests.get(), 1);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn expanded_metadata_is_clipped_to_the_items_source() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(100, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.initialize_for_context(&base(&context));
    let _metadata = layout.items_info_requested(|args| {
        args.set_desired_aspect_ratios(&[1.0; 100]).unwrap();
    });
    assert!(!layout.request_items_info(80, 100));
    assert_eq!(layout.requested_range_start_index(), 80);
    assert_eq!(layout.requested_range_length(), 20);
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn regular_metadata_stops_delayed_control_measurement() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::new(
        vec![Size::new(20.0, 20.0); 10_000],
        Rect::new(0.0, 0.0, 100.0, 100.0),
    );
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    settle(&layout, &context, Size::new(100.0, 100.0));
    assert!(layout.timer.borrow().is_some());
    let _metadata = layout.items_info_requested(|args| {
        args.set_desired_aspect_ratios(&vec![1.0; args.items_range_requested_length() as usize])
            .unwrap();
    });
    layout.invalidate_items_info();
    settle(&layout, &context, Size::new(100.0, 100.0));
    assert!(!layout.uses_fast_path_layout());
    assert!(layout.requested_range_length() > 0);
    assert!(layout.timer.borrow().is_none());
    layout.uninitialize_for_context(&base(&context));
}

#[test]
fn pending_invalidation_cannot_escape_into_a_replacement_context() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(3, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.initialize_for_context(&base(&context));
    layout.invalidate_measure_async();
    layout.uninitialize_for_context(&base(&context));
    let replacement = host.context();
    layout.initialize_for_context(&base(&replacement));
    let invalidations = Rc::new(Cell::new(0));
    let observed = invalidations.clone();
    let _invalidated = layout.measure_invalidated(move || observed.set(observed.get() + 1));
    layout.invalidate_measure_async();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(invalidations.get(), 1);
    assert!(!layout.async_invalidation_pending.get());
    layout.uninitialize_for_context(&base(&replacement));
}

#[test]
fn detached_anchor_and_frozen_ranges_do_not_follow_the_layout_to_another_host() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::uniform(1000, Size::new(20.0, 20.0));
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    host.anchor.set(900);
    run(&layout, &context, Size::new(100.0, 100.0));
    assert_eq!(layout.anchor_index.get(), 900);
    layout.uninitialize_for_context(&base(&context));
    assert_eq!(layout.frozen_range(), (-1, -1));
    let replacement_host = TestHost::uniform(3, Size::new(20.0, 20.0));
    let replacement = replacement_host.context();
    layout.initialize_for_context(&base(&replacement));
    settle(&layout, &replacement, Size::new(100.0, 100.0));
    assert_eq!(layout.anchor_index.get(), -1);
    assert_eq!(layout.realized_range(), (0, 2));
    assert_eq!(replacement_host.element(0).bounds().y, 0.0);
    layout.uninitialize_for_context(&base(&replacement));
}

#[test]
fn cache_shrink_uses_absolute_source_indices_at_a_distant_viewport() {
    let _scope = crate::test_support::test_scope();
    let host = TestHost::new(
        vec![Size::new(20.0, 20.0); 5000],
        Rect::new(0.0, 4000.0, 100.0, 1.0),
    );
    let context = host.context();
    let layout = LinedFlowLayout::new();
    layout.set_line_height(20.0);
    layout.initialize_for_context(&base(&context));
    layout.actual_line_height.set(20.0);
    layout.current_available_width.set(100.0);
    layout.average_items_per_line.set((5.0, 5.0));
    {
        let mut ratios = layout.aspect_ratios.borrow_mut();
        ratios.resize(384, 1060);
        for index in [0, 64, 128, 1024, 1088, 1152] {
            ratios.set_at(index, ItemAspectRatio { aspect_ratio: 1.0, weight: 16 });
        }
    }
    *layout.items_info.borrow_mut() = Rc::new(LinedFlowItemsInfo {
        start: 1060,
        ratios: vec![1.0; 2],
        ..LinedFlowItemsInfo::default()
    });
    layout.collect_metrics(&context, 1060, 1062, 20.0, true);
    let ratios = layout.aspect_ratios.borrow();
    assert_eq!(ratios.block_count(), 4);
    assert_eq!(ratios.get_at(1088).weight, 16);
    assert_eq!(ratios.get_at(1152).weight, 16);
    assert_eq!(ratios.get_at(0).weight, 0);
    drop(ratios);
    layout.uninitialize_for_context(&base(&context));
}
