//! Port of upstream's `Media/TextFormatting/TextLayoutRangeContractTests.cs`
//! of the Skia unit tests.

use crate::unit_tests::media::CustomFontManagerImpl;
use crate::unit_tests::mock_platform_render_interface;
use crate::PlatformRenderInterface;
use ferroui_base::media::text_formatting::{TextLayout, TextLayoutOptions, TextLine};
use ferroui_base::media::{Brushes, Typeface};
use ferroui_base::Rect;
use ferroui_controls::testing::{UnitTestApplication, UnitTestApplicationScope};
use std::rc::Rc;

/// `new TextLayout(text, Typeface.Default, 12, Brushes.Black)`.
fn layout(text: &str) -> TextLayout {
    TextLayout::new(
        text,
        Typeface::default_typeface(),
        TextLayoutOptions { font_size: 12.0, foreground: Some(Brushes::black()), ..Default::default() },
    )
}

#[test]
fn empty_range_returns_no_geometry() {
    let rows: [(&str, i32); 4] = [("abc", 0), ("abc", 1), ("abc", 3), ("a\r\nb\r\n", 3)];

    for (text, start) in rows {
        let _app = start_app();
        let layout = layout(text);

        assert!(layout.hit_test_text_range(start, 0).is_empty(), "{text:?} {start}");
    }
}

#[test]
fn completed_range_does_not_add_geometry_from_following_lines() {
    let _app = start_app();
    let layout = layout("a\r\nb\r\n");
    let first_line = layout.text_lines()[0].clone();
    let expected_bounds = first_line.get_text_bounds(0, first_line.length());
    assert_eq!(1, expected_bounds.len());
    let expected = expected_bounds[0].rectangle();

    let actual = layout.hit_test_text_range(0, first_line.length());
    assert_eq!(1, actual.len());

    assert_eq!(expected, actual[0]);
}

#[test]
fn negative_range_length_preserves_line_geometry() {
    let _app = start_app();
    let layout = layout("abc");
    let expected: Vec<Rect> =
        layout.text_lines()[0].get_text_bounds(2, -1).iter().map(|bounds| bounds.rectangle()).collect();
    assert!(expected.is_empty());

    assert_eq!(expected, layout.hit_test_text_range(2, -1));
}

fn start_app() -> UnitTestApplicationScope {
    UnitTestApplication::start(
        mock_platform_render_interface()
            .with_render_interface(Rc::new(PlatformRenderInterface::new(None, None)))
            .with_font_manager_impl(Rc::new(CustomFontManagerImpl::new())),
    )
}
