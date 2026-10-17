//! The table of the pages and the native pages beside the documents.

use super::support::start_application;
use crate::capture::{Options, Setting};
use crate::theme::{Density, Variant};
use crate::{PageInfo, PAGES, SAMPLE};
use ferroui_base::Point;
use ferroui_controls::testing::MockWindowingPlatform;
use ferroui_controls::{Control, TextBlock, TextLayoutRoundingPlacement, Window};
use std::collections::BTreeSet;
use std::path::Path;

/// The XML namespace of WinUI XAML.
const WINUI_PRESENTATION: &str = "http://schemas.microsoft.com/winfx/2006/xaml/presentation";

#[test]
fn the_ids_and_titles_of_the_pages_are_unique() {
    let ids: BTreeSet<&str> = PAGES.iter().map(|page| page.id).collect();
    let titles: BTreeSet<&str> = PAGES.iter().map(|page| page.title).collect();
    assert_eq!(ids.len(), PAGES.len());
    assert_eq!(titles.len(), PAGES.len());
    for page in PAGES {
        assert!(
            page.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "{}: an id is lower case letters and digits, the start of the name of a picture",
            page.id
        );
    }
}

/// Every document under `Pages/` is a page of the table, and every page of the table is a
/// document of the gallery.
#[test]
fn every_document_under_pages_is_in_the_table() {
    let documents: BTreeSet<&str> =
        SAMPLE.documents.iter().map(|(path, _)| *path).filter(|path| path.starts_with("/Pages/")).collect();
    let pages: BTreeSet<&str> = PAGES.iter().map(|page| page.document).collect();
    assert_eq!(documents, pages);
}

/// Every page has its native page beside its document: a document of WinUI XAML whose root is
/// an element of the presentation namespace.
#[test]
fn every_page_has_a_native_page_of_winui_xaml() {
    let crate_directory = Path::new(env!("CARGO_MANIFEST_DIR"));
    for page in PAGES {
        let file = crate_directory.join(page.native_file);
        assert!(file.is_file(), "{}: {} does not exist", page.id, file.display());
        assert_eq!(
            page.native_file.trim_end_matches(".winui.xml"),
            page.document.trim_start_matches('/').trim_end_matches(".xaml"),
            "{}: the native page is beside the document, with its name",
            page.id
        );
        let document = roxmltree::Document::parse(page.native)
            .unwrap_or_else(|error| panic!("{}: {} is not XML: {error}", page.id, page.native_file));
        let root = document.root_element();
        assert_eq!(root.tag_name().namespace(), Some(WINUI_PRESENTATION), "{}: the root of the native page", page.id);
        assert!(
            !page.native.contains("<?xml"),
            "{}: the native page is put inside the wrapper of the comparison host, so it has no XML declaration",
            page.id
        );
    }
}

#[test]
fn the_names_of_the_settings_are_those_of_the_pictures() {
    let name = |variant, density, right_to_left| Setting { variant, density, right_to_left }.name();
    assert_eq!(name(Variant::Light, Density::Normal, false), "light");
    assert_eq!(name(Variant::Dark, Density::Compact, false), "dark-compact");
    assert_eq!(name(Variant::HighContrast, Density::Normal, true), "highcontrast-rtl");
    assert_eq!(name(Variant::Dark, Density::Compact, true), "dark-compact-rtl");
}

#[test]
fn the_default_options_show_every_page_in_light_and_dark_in_both_densities() {
    let options = Options::from_env("FERROUI_THEME_GALLERY_TEST_UNSET").expect("the defaults");
    assert_eq!(options.pages.len(), PAGES.len());
    assert_eq!(options.settings.len(), 4);
    assert_eq!((options.width, options.height, options.scale), (720.0, 960.0, 1.0));
    assert_eq!((options.pixel_size().width, options.pixel_size().height), (720, 960));
}

/// The text calibration page puts every line at the top of its grid row: the rows have the
/// heights the document gives them, as WinUI's grid has (the comparison measures the baseline of
/// each row against the same row of the native page), and its text blocks have the placement of
/// the baseline of the WinUI theme. At the scalings of the comparisons, 1 (CI) and 2 (the virtual
/// machine), every row is a whole number of device pixels; at others both grids round them.
#[test]
fn the_lines_of_the_text_calibration_page_are_at_the_tops_of_their_rows() {
    let _app = start_application();
    let page = PageInfo::find("textcal").expect("the text calibration page");
    let rows = [30.0, 32.0, 33.0, 33.0, 34.0, 36.0, 38.0, 41.0, 41.0, 46.0, 52.0, 52.0, 57.0];
    for scaling in [1.0, 2.0] {
        let window_impl = MockWindowingPlatform::create_window_mock();
        window_impl.render_scaling.set(scaling);
        let window = Window::with_impl(window_impl);
        let control = (page.create)();
        window.set_content(Some(Control::boxed(&control)));
        window.show();
        window.update_layout();

        let lines: Vec<_> = control.get_visual_descendants().filter_map(|visual| visual.cast::<TextBlock>()).collect();
        assert_eq!(rows.len(), lines.len(), "the lines of the page");
        let mut top = 24.0;
        for (line, height) in lines.iter().zip(rows) {
            let y = line.translate_point(Point::default(), &control).expect("the line is in the page").y;
            assert!((y - top).abs() < 1e-6, "{:?} at {scaling}: at {y}, its row at {top}", line.text());
            assert_eq!(TextLayoutRoundingPlacement::Above, line.layout_rounding_placement());
            top += height;
        }
        window.close();
    }
}
