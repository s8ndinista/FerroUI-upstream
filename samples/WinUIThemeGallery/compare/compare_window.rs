//! The window of the comparison: a window of WinUI whose content is a grid with two cells of the
//! same size, the native cell (a grid that holds the native page, loaded with `XamlReader.Load`)
//! and the swap chain panel of `ferroui-winui`, whose content is the host of the framework's
//! page ([`CaptureHost`]). Side by side, the native cell is the first column and the panel the
//! second; on a screen too small for both, the two are in the first column with the native cell
//! over the panel, and the framework's page is drawn by the framework alone.
//!
//! The markup of the grid is loaded at run time, as the WinUI samples load theirs
//! (docs/porting/winui.md, section 7.6), and the panel is added to it in code.

use crate::app::App;
use ferroui_base::Ref;
use ferroui_controls::application_lifetimes::ISingleViewApplicationLifetime;
use ferroui_microcom::{ComPtr, HResult};
use ferroui_win32::win_rt::{HStringInterop, NativeWinRTMethods};
use ferroui_winui::interop::{IFrameworkElement, IGridStatics, IPanel, IUIElement, IXamlReaderStatics, Rect};
use ferroui_winui::{FerroSwapChainPanel, XamlWindow};
use std::rc::Rc;
use winui_theme_gallery::CaptureHost;

/// The content of the window: the grid of the two cells, on the page background of WinUI. The
/// panel is added in code.
const MARKUP: &str = r#"<Grid
    xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
    Background="{ThemeResource ApplicationPageBackgroundThemeBrush}">
        <Grid.ColumnDefinitions>
            <ColumnDefinition Width="Auto" />
            <ColumnDefinition Width="Auto" />
            <ColumnDefinition Width="*" />
        </Grid.ColumnDefinitions>
        <Grid x:Name="NativeCell" Grid.Column="0" HorizontalAlignment="Left" VerticalAlignment="Top" />
    </Grid>"#;

/// `HorizontalAlignment.Left` and `VerticalAlignment.Top` of WinUI.
const ALIGNMENT_START: i32 = 0;

/// A string handle with the text.
fn hstring(text: &str) -> Result<HStringInterop, HResult> {
    HStringInterop::new(Some(text))
}

pub struct CompareWindow {
    window: XamlWindow,
    title: String,
    native_cell: ComPtr<IFrameworkElement>,
    ferro_panel: Rc<FerroSwapChainPanel>,
    panel_element: ComPtr<IFrameworkElement>,
    reader: ComPtr<IXamlReaderStatics>,
    grid_statics: ComPtr<IGridStatics>,
    host: Ref<CaptureHost>,
}

impl CompareWindow {
    pub fn new() -> Result<Rc<CompareWindow>, HResult> {
        let window = XamlWindow::new()?;
        // The title names the process: the run finds the handle of the window by it.
        let title = format!("WinUI theme comparison {}", std::process::id());
        window.set_title(&title)?;
        let reader = NativeWinRTMethods::create_activation_factory::<IXamlReaderStatics>("Microsoft.UI.Xaml.Markup.XamlReader")?;
        let root = reader.load(hstring(MARKUP)?.handle())?.ok_or(HResult::POINTER)?.cast::<IFrameworkElement>()?;
        let native_cell =
            root.find_name(hstring("NativeCell")?.handle())?.ok_or(HResult::POINTER)?.cast::<IFrameworkElement>()?;

        let ferro_panel = FerroSwapChainPanel::new()?;
        let panel_element = ferro_panel.as_ui_element().cast::<IFrameworkElement>()?;
        panel_element.set_horizontal_alignment(ALIGNMENT_START)?;
        panel_element.set_vertical_alignment(ALIGNMENT_START)?;
        let grid_statics = NativeWinRTMethods::create_activation_factory::<IGridStatics>("Microsoft.UI.Xaml.Controls.Grid")?;
        // The first child: when both cells are in the first column, the native cell is over it.
        root.cast::<IPanel>()?.children()?.ok_or(HResult::POINTER)?.insert_at(0, Some(&ferro_panel.as_ui_element()))?;
        window.set_content(&*root.cast::<IUIElement>()?)?;

        let host = CaptureHost::new();
        App::lifetime().set_main_view(Some(host.clone().upcast()));
        ferro_panel.set_content(App::lifetime().main_view());

        Ok(Rc::new(CompareWindow { window, title, native_cell, ferro_panel, panel_element, reader, grid_statics, host }))
    }

    pub fn activate(&self) -> Result<(), HResult> {
        self.window.activate()
    }

    pub fn window(&self) -> &XamlWindow {
        &self.window
    }

    /// The title of the window, by which its handle is found.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The host of the framework's page.
    pub fn host(&self) -> &Ref<CaptureHost> {
        &self.host
    }

    /// The swap chain panel.
    pub fn ferro_panel(&self) -> &Rc<FerroSwapChainPanel> {
        &self.ferro_panel
    }

    /// Gives both cells the size of a page, in device-independent pixels, and places the panel
    /// beside the native cell (`side_by_side`) or under it.
    pub fn arrange(&self, side_by_side: bool, width: f64, height: f64) -> Result<(), HResult> {
        self.native_cell.set_width(width)?;
        self.native_cell.set_height(height)?;
        self.panel_element.set_width(width)?;
        self.panel_element.set_height(height)?;
        self.host.set_width(width);
        self.host.set_height(height);
        self.grid_statics.set_column(Some(&self.panel_element), if side_by_side { 1 } else { 0 })
    }

    /// Shows the native page `markup` (the page in its wrapper,
    /// `winui_theme_gallery::comparison::native_markup`) in the native cell, in place of the page
    /// it showed. An error of `XamlReader.Load` is a page WinUI does not load.
    pub fn show_native(&self, markup: &str) -> Result<(), HResult> {
        let children = self.native_cell.cast::<IPanel>()?.children()?.ok_or(HResult::POINTER)?;
        children.clear()?;
        let page = self.reader.load(hstring(markup)?.handle())?.ok_or(HResult::POINTER)?.cast::<IUIElement>()?;
        children.append(Some(&page))
    }

    /// Empties the native cell.
    pub fn clear_native(&self) -> Result<(), HResult> {
        self.native_cell.cast::<IPanel>()?.children()?.ok_or(HResult::POINTER)?.clear()
    }

    /// The rasterization scale of the content of the window (its DPI over 96).
    pub fn rasterization_scale(&self) -> Option<f64> {
        let xaml_root = self.native_cell.cast::<IUIElement>().ok()?.xaml_root().ok()??;
        xaml_root.rasterization_scale().ok()
    }

    /// The bounds of the native cell in the client area of the window, in device-independent
    /// pixels.
    pub fn native_bounds(&self) -> Result<Rect, HResult> {
        Self::bounds_of(&self.native_cell)
    }

    /// The bounds of the panel in the client area of the window, in device-independent pixels.
    pub fn panel_bounds(&self) -> Result<Rect, HResult> {
        Self::bounds_of(&self.panel_element)
    }

    fn bounds_of(element: &ComPtr<IFrameworkElement>) -> Result<Rect, HResult> {
        let transform = element.cast::<IUIElement>()?.transform_to_visual(None)?.ok_or(HResult::POINTER)?;
        let size = Rect { x: 0.0, y: 0.0, width: element.actual_width()? as f32, height: element.actual_height()? as f32 };
        transform.transform_bounds(size)
    }
}
