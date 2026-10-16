//! The resource database of the server (the port of `XResources.cs`): the
//! `RESOURCE_MANAGER` property of the root window, which `xrdb` writes.

use crate::event::Event;
use crate::x11_info::X11Info;
use crate::x11_platform::FerroX11Platform;
use crate::xlib;
use ferroui_base::platform::surfaces::{PlatformTextRenderingParameters, SubpixelGeometry};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicU8, Ordering};

/// The resources of the server, with a notification per resource that
/// changed.
pub struct XResources {
    resources: RefCell<HashMap<String, String>>,
    x11: Rc<X11Info>,
    pub resource_changed: Event<String>,
}

/// The resources of a resource string (`name: value` lines).
pub fn parse_resources(res: &str) -> HashMap<String, String> {
    let mut new_resources = HashMap::new();
    for item in res.split('\n') {
        let Some((key, value)) = item.split_once(':') else {
            continue;
        };
        new_resources.insert(key.to_string(), value.trim_start().to_string());
    }
    new_resources
}

/// The keys to notify when `old` is replaced by `new`: first the ones
/// that are gone, then the ones that are new or have another value. (The
/// reference enumerates two hash sets; the order within each group is not
/// defined there, and is sorted here.)
pub fn changed_resources(old: &HashMap<String, String>, new: &HashMap<String, String>) -> Vec<String> {
    let mut missing_resources: HashSet<&String> = old.keys().collect();
    let mut changed_resources = Vec::new();
    for (key, value) in new {
        if !missing_resources.remove(key) || old.get(key) != Some(value) {
            changed_resources.push(key.clone());
        }
    }
    let mut missing: Vec<String> = missing_resources.into_iter().cloned().collect();
    missing.sort();
    changed_resources.sort();
    missing.extend(changed_resources);
    missing
}

impl XResources {
    pub fn new(plat: &Rc<FerroX11Platform>) -> Rc<Self> {
        let this = Rc::new(Self {
            resources: RefCell::new(HashMap::new()),
            x11: plat.info().clone(),
            resource_changed: Event::new(),
        });
        let weak = Rc::downgrade(&this);
        plat.globals().root_property_changed.subscribe(move |atom| {
            if let Some(this) = weak.upgrade() {
                this.on_root_property_changed(atom);
            }
        });
        this.update_resources();
        this
    }

    fn update_resources(&self) {
        let res = self.read_resources_string().unwrap_or_default();
        let new_resources = parse_resources(&res);
        set_subpixel_order(new_resources.get("Xft.rgba").map(String::as_str));
        let changed = changed_resources(&self.resources.borrow(), &new_resources);
        *self.resources.borrow_mut() = new_resources;
        for key in changed {
            self.resource_changed.invoke(key);
        }
    }

    pub fn get_resource(&self, key: &str) -> Option<String> {
        self.resources.borrow().get(key).cloned()
    }

    fn read_resources_string(&self) -> Option<String> {
        let atoms = self.x11.atoms();
        let property = xlib::x_get_window_property(
            self.x11.display(),
            self.x11.root_window(),
            atoms.RESOURCE_MANAGER,
            0,
            0x7fffffff,
            false,
            atoms.STRING,
        );
        if property.actual_format != 8 {
            return None;
        }
        // The bytes are Latin-1 for the `STRING` type; the reference
        // reads them with the ANSI code page of the process, which is
        // UTF-8 on the systems it runs on.
        Some(String::from_utf8_lossy(&property.data).into_owned())
    }

    fn on_root_property_changed(&self, atom: xlib::Atom) {
        if atom == self.x11.atoms().RESOURCE_MANAGER {
            self.update_resources();
        }
    }
}

/// The subpixel order of `Xft.rgba` as the resources last said, for the
/// thread that renders: 0 when the resource is not set, else 1 plus the
/// index of the order in [`SUBPIXEL_ORDERS`].
///
/// FerroUI's own addition (docs/porting/text-rendering.md): the desktop
/// (GNOME, KDE, Xfce) writes the font settings of the user to `Xft.rgba`,
/// which fontconfig and the toolkits of the desktop follow.
static SUBPIXEL_ORDER: AtomicU8 = AtomicU8::new(0);

/// The values of `Xft.rgba` and the geometry each means; `none` is
/// greyscale text.
const SUBPIXEL_ORDERS: [(&str, SubpixelGeometry); 5] = [
    ("none", SubpixelGeometry::Unknown),
    ("rgb", SubpixelGeometry::RgbHorizontal),
    ("bgr", SubpixelGeometry::BgrHorizontal),
    ("vrgb", SubpixelGeometry::RgbVertical),
    ("vbgr", SubpixelGeometry::BgrVertical),
];

fn set_subpixel_order(value: Option<&str>) {
    let index = value
        .and_then(|value| SUBPIXEL_ORDERS.iter().position(|(name, _)| value.trim().eq_ignore_ascii_case(name)))
        .map_or(0, |index| index as u8 + 1);
    SUBPIXEL_ORDER.store(index, Ordering::Release);
}

/// The text rendering parameters of the desktop: subpixel text for the
/// subpixel order of `Xft.rgba`, greyscale for `none`, and nothing (the
/// defaults of the render backend: greyscale) when the resource is not set
/// or has a value that is not known. Contrast and gamma are left to the
/// backend.
pub fn text_rendering_parameters() -> Option<PlatformTextRenderingParameters> {
    let index = SUBPIXEL_ORDER.load(Ordering::Acquire);
    let (_, geometry) = SUBPIXEL_ORDERS.get(usize::from(index).checked_sub(1)?)?;
    Some(PlatformTextRenderingParameters {
        subpixel_antialiasing: *geometry != SubpixelGeometry::Unknown,
        subpixel_geometry: *geometry,
        ..PlatformTextRenderingParameters::default()
    })
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn xft_rgba_gives_the_text_rendering_parameters() {
        // One test owns the value of the process.
        set_subpixel_order(None);
        assert_eq!(None, text_rendering_parameters());

        set_subpixel_order(Some("rgb"));
        let parameters = text_rendering_parameters().unwrap();
        assert!(parameters.subpixel_antialiasing);
        assert_eq!(SubpixelGeometry::RgbHorizontal, parameters.subpixel_geometry);
        assert!(!parameters.composited);
        assert_eq!(None, parameters.text_contrast);

        set_subpixel_order(Some("VBGR"));
        assert_eq!(SubpixelGeometry::BgrVertical, text_rendering_parameters().unwrap().subpixel_geometry);

        set_subpixel_order(Some("none"));
        let parameters = text_rendering_parameters().unwrap();
        assert!(!parameters.subpixel_antialiasing);

        set_subpixel_order(Some("unknown"));
        assert_eq!(None, text_rendering_parameters());
        set_subpixel_order(None);
    }

    #[test]
    fn resources_are_lines_of_name_and_value() {
        let resources = parse_resources("Xft.dpi:\t144\nXcursor.size: 24\n! a comment\n\nXft.rgba:rgb\nURL: http://x:1\n");
        assert_eq!(resources.get("Xft.dpi").map(String::as_str), Some("144"));
        assert_eq!(resources.get("Xcursor.size").map(String::as_str), Some("24"));
        assert_eq!(resources.get("Xft.rgba").map(String::as_str), Some("rgb"));
        // Only the first colon separates.
        assert_eq!(resources.get("URL").map(String::as_str), Some("http://x:1"));
        assert_eq!(resources.len(), 4);
        assert!(parse_resources("").is_empty());
    }

    #[test]
    fn the_last_line_of_a_name_wins() {
        let resources = parse_resources("Xft.dpi: 96\nXft.dpi: 192");
        assert_eq!(resources.get("Xft.dpi").map(String::as_str), Some("192"));
    }

    #[test]
    fn removed_new_and_changed_resources_are_notified() {
        let old = parse_resources("a: 1\nb: 2\nc: 3");
        let new = parse_resources("b: 2\nc: 4\nd: 5");
        assert_eq!(changed_resources(&old, &new), vec!["a".to_string(), "c".to_string(), "d".to_string()]);
        assert!(changed_resources(&new, &new).is_empty());
        assert_eq!(changed_resources(&HashMap::new(), &old), vec!["a".to_string(), "b".to_string(), "c".to_string()]);
    }
}
