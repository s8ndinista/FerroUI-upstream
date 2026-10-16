//! The platforms without an automation backend: the tests compile and the
//! runner reports that it has nothing to run.

use super::super::keys::Key;
use super::super::session::{By, ElementRect, MouseButton, Screenshot, WebDriverError};
use std::path::Path;

const UNSUPPORTED: &str = "the integration tests drive the application through UI Automation on Windows and the \
    accessibility API on macOS; this platform has no automation backend";

/// No element exists on this platform.
#[derive(Clone)]
pub enum NativeElement {}

/// No backend exists on this platform.
pub enum Backend {}

impl Backend {
    pub fn check() -> Result<(), String> {
        Err(UNSUPPORTED.to_owned())
    }

    pub fn launch(_app: &Path, _arguments: &[&str]) -> Result<Backend, String> {
        Err(UNSUPPORTED.to_owned())
    }

    pub fn desktop() -> Result<Backend, String> {
        Err(UNSUPPORTED.to_owned())
    }

    pub fn quit(&self) {
        match *self {}
    }

    pub fn settle(&self) {
        match *self {}
    }

    pub fn session_root(&self, _window: Option<&str>) -> Result<NativeElement, WebDriverError> {
        match *self {}
    }

    pub fn window_handles(&self) -> Vec<String> {
        match *self {}
    }

    pub fn activate_window(&self, _handle: &str) {
        match *self {}
    }

    pub fn close_window(&self, _handle: &str) {
        match *self {}
    }

    pub fn children(&self, _element: &NativeElement) -> Vec<NativeElement> {
        match *self {}
    }

    pub fn same(&self, _a: &NativeElement, _b: &NativeElement) -> bool {
        match *self {}
    }

    pub fn tag_name(&self, _element: &NativeElement) -> String {
        match *self {}
    }

    pub fn automation_id(&self, _element: &NativeElement) -> String {
        match *self {}
    }

    pub fn name(&self, _element: &NativeElement) -> String {
        match *self {}
    }

    pub fn class_name(&self, _element: &NativeElement) -> String {
        match *self {}
    }

    pub fn text(&self, _element: &NativeElement) -> String {
        match *self {}
    }

    pub fn attribute(&self, _element: &NativeElement, _name: &str) -> Option<String> {
        match *self {}
    }

    pub fn is_enabled(&self, _element: &NativeElement) -> bool {
        match *self {}
    }

    pub fn is_selected(&self, _element: &NativeElement) -> bool {
        match *self {}
    }

    pub fn has_focus(&self, _element: &NativeElement) -> bool {
        match *self {}
    }

    pub fn rect(&self, _element: &NativeElement) -> ElementRect {
        match *self {}
    }

    pub fn press(&self, _element: &NativeElement) -> bool {
        match *self {}
    }

    pub fn focus(&self, _element: &NativeElement) {
        match *self {}
    }

    pub fn clear(&self, _element: &NativeElement) {
        match *self {}
    }

    pub fn find_all(&self, _root: &NativeElement, _include_root: bool, _by: &By) -> Vec<NativeElement> {
        match *self {}
    }

    pub fn screenshot(&self, _rect: &ElementRect) -> Result<Screenshot, String> {
        match *self {}
    }

    pub fn mouse_move(&self, _x: f64, _y: f64) {
        match *self {}
    }

    pub fn mouse_button(&self, _button: MouseButton, _down: bool, _x: f64, _y: f64, _click_count: u32) {
        match *self {}
    }

    pub fn key(&self, _key: Key, _down: bool) {
        match *self {}
    }
}
