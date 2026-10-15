//! `MainWindow` of the reference's sample: a window of WinUI whose content is
//! a grid with the swap chain panel, which shows the main view of the
//! lifetime (the markup of the reference, `MainWindow.xaml`, built in code).

use crate::app::App;
use ferroui_controls::application_lifetimes::ISingleViewApplicationLifetime;
use ferroui_microcom::HResult;
use ferroui_winui::{FerroSwapChainPanel, XamlGrid, XamlWindow};
use std::rc::Rc;

pub struct MainWindow {
    window: XamlWindow,
    #[allow(dead_code)] // Held: the window's content.
    grid: XamlGrid,
    ferro_panel: Rc<FerroSwapChainPanel>,
}

impl MainWindow {
    pub fn new() -> Result<Rc<MainWindow>, HResult> {
        let window = XamlWindow::new()?;
        window.set_title("ControlGallery.WinUI")?;
        let grid = XamlGrid::new()?;
        let ferro_panel = FerroSwapChainPanel::new()?;
        grid.add_child(&ferro_panel.as_ui_element())?;
        window.set_content(&*grid.as_ui_element()?)?;

        ferro_panel.set_content(App::lifetime().main_view());
        Ok(Rc::new(MainWindow { window, grid, ferro_panel }))
    }

    pub fn activate(&self) -> Result<(), HResult> {
        self.window.activate()
    }

    pub fn window(&self) -> &XamlWindow {
        &self.window
    }

    /// The panel the markup of the reference names.
    pub fn ferro_panel(&self) -> &Rc<FerroSwapChainPanel> {
        &self.ferro_panel
    }
}
