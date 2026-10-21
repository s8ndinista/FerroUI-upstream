//! Interactive inking sample; works through the same controls on every backend.
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, ContentPage, CoreInputDeviceTypes, InkCanvas, InkToolbar, TextBlock};
use std::cell::RefCell;

#[repr(C)]
pub struct InkingPage {
    base: ContentPage,
    snapshot: RefCell<Option<Vec<u8>>>,
}
content_page_class!(InkingPage);
ferro_class_info!(InkingPage { new: InkingPage::new });
xaml_class!(InkingPage, "/Pages/InkingPage.xaml");

impl InkingPage {
    pub fn construct() -> Self { Self { base: ContentPage::construct(), snapshot: RefCell::new(None) } }
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let canvas = this.find_control::<InkCanvas>("InkSurface").expect("InkingPage InkSurface");
        let presenter = canvas.ink_presenter();
        presenter.set_input_device_types(CoreInputDeviceTypes::PEN | CoreInputDeviceTypes::MOUSE | CoreInputDeviceTypes::TOUCH);
        this.find_control::<InkToolbar>("Toolbar").expect("InkingPage Toolbar").set_target_ink_canvas(Some(canvas));
        for (name, action) in [("ClearInk", 0), ("SaveInk", 1), ("RestoreInk", 2)] {
            let weak = this.downgrade();
            this.find_control::<Button>(name).expect("InkingPage action").click(move |_, _| {
                if let Some(this) = weak.upgrade() { this.run_action(action); }
            });
        }
        let weak = this.downgrade();
        presenter.stroke_container().changed(move || {
            if let Some(this) = weak.upgrade() { this.update_status(None); }
        });
        this
    }
    fn run_action(&self, action: i32) {
        let Some(canvas) = self.find_control::<InkCanvas>("InkSurface") else { return; };
        let container = canvas.ink_presenter().stroke_container();
        let result = match action {
            0 => { container.clear(); Ok(()) }
            1 => container.save().map(|bytes| { *self.snapshot.borrow_mut() = Some(bytes); }),
            _ => {
                let snapshot = self.snapshot.borrow().clone();
                match snapshot { Some(bytes) => container.load(&bytes), None => { self.update_status(Some("Save a snapshot first")); return; } }
            }
        };
        match result { Ok(()) => self.update_status(None), Err(error) => self.update_status(Some(&error.to_string())) }
    }
    fn update_status(&self, message: Option<&str>) {
        let Some(status) = self.find_control::<TextBlock>("InkStatus") else { return; };
        let count = self.find_control::<InkCanvas>("InkSurface").map_or(0, |c| c.ink_presenter().stroke_container().get_strokes().len());
        status.set_text(Some(&message.map(str::to_owned).unwrap_or_else(|| format!("{count} strokes"))));
    }
}
