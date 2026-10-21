//! Captures the actual WinUI-themed gallery with Skia on the headless platform.
//! These are rendering smoke artifacts, not upstream pixel-parity baselines.

use ferroui_base::media::imaging::PngBitmapEncoderOptions;
use ferroui_base::styling::ThemeVariant;
use ferroui_controls::{
    winui::AutoSuggestBox, AppBuilder, Application, ComboBox, Control, Image, Window,
};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessWindowExtensions,
};
use ferroui_skia::SkiaApplicationExtensions;
use std::error::Error;
use std::path::PathBuf;
use winui_gallery::{App, GalleryView};

fn main() -> Result<(), Box<dyn Error>> {
    let mut pages = vec!["Button".to_string()];
    let mut output_dir = PathBuf::from("gallery-captures");
    let mut dark = false;
    let mut query = None;
    for argument in std::env::args().skip(1) {
        if let Some(value) = argument.strip_prefix("--pages=") {
            pages = value.split(',').map(str::to_string).collect();
        } else if let Some(value) = argument.strip_prefix("--output-dir=") {
            output_dir = value.into();
        } else if argument == "--dark" {
            dark = true;
        } else if let Some(value) = argument.strip_prefix("--query=") {
            query = Some(value.to_string());
        } else if argument == "--all" {
            pages = winui_gallery::CATALOG
                .iter()
                .map(|page| page.id.to_string())
                .collect();
        } else {
            return Err(format!("Unknown argument: {argument}. Use --pages=Button,NumberBox, --all, --dark, --query=Button or --output-dir=PATH.").into());
        }
    }
    for page in &pages {
        if winui_gallery::catalog::find(page).is_none() {
            return Err(format!("Unknown catalog page: {page}").into());
        }
    }
    std::fs::create_dir_all(&output_dir)?;
    let _builder = AppBuilder::configure::<App>()
        .use_harfbuzz()
        .use_skia()
        .use_headless(FerroHeadlessPlatformOptions {
            use_headless_drawing: false,
            ..FerroHeadlessPlatformOptions::default()
        })
        .setup_without_starting();
    Application::current()
        .expect("application initialized")
        .set_requested_theme_variant(Some(if dark {
            ThemeVariant::dark()
        } else {
            ThemeVariant::light()
        }));
    let gallery = GalleryView::with_preview_options(
        ferroui_markup_xaml_loader::winui::WinUIXamlOptions::for_winui_theme(),
    )?;
    gallery
        .root()
        .get_control::<ComboBox>("Theme")
        .set_selected_index(if dark { 2 } else { 1 });
    let window = Window::new();
    window.set_width(1280.0);
    window.set_height(900.0);
    window.set_content(Some(Control::boxed(gallery.root())));
    window.show();
    for page in pages {
        eprintln!("Rendering {page}");
        gallery.navigate(&page);
        if let Some(error) = gallery.native_example_error() {
            return Err(format!("{page}: native example failed to load: {error}").into());
        }
        if page == "AutoSuggestBox" {
            if let Some(query) = &query {
                let native = gallery
                    .native_example()
                    .ok_or("AutoSuggestBox example missing")?;
                let search = native.get_control::<AutoSuggestBox>("ControlSuggestions");
                search.set_text(query);
                search.submit_query();
                if native.get_control::<Control>("ControlDetails").is_visible()
                    && native
                        .get_control::<Image>("ControlImage")
                        .source()
                        .is_none()
                {
                    return Err(
                        "The control detail image could not be decoded by the real renderer."
                            .into(),
                    );
                }
            }
        }
        let frame = window
            .capture_rendered_frame()
            .ok_or("The renderer produced no frame.")?;
        let path = output_dir.join(format!("{page}.png"));
        frame.save_to_file(
            path.to_str().ok_or("The output path is not valid UTF-8.")?,
            &PngBitmapEncoderOptions::default().into(),
        )?;
        println!("{}: {}", page, path.display());
    }
    window.close();
    Ok(())
}
