//! Port of `Embedding/Win32WindowControlHandle.cs`.

use super::win_api;
use ferroui_controls::platform::{INativeControlHostDestroyableControlHandle, IPlatformHandle};
use std::any::Any;

/// The handle of a native window the sample created, which the native control
/// host destroys.
pub struct Win32WindowControlHandle {
    handle: isize,
    descriptor: String,
}

impl Win32WindowControlHandle {
    pub fn new(handle: isize, descriptor: &str) -> Self {
        Self { handle, descriptor: descriptor.to_owned() }
    }
}

impl IPlatformHandle for Win32WindowControlHandle {
    fn handle(&self) -> isize {
        self.handle
    }

    fn handle_descriptor(&self) -> Option<&str> {
        Some(&self.descriptor)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_native_control_host_destroyable_control_handle(
        &self,
    ) -> Option<&dyn INativeControlHostDestroyableControlHandle> {
        Some(self)
    }
}

impl INativeControlHostDestroyableControlHandle for Win32WindowControlHandle {
    fn destroy(&self) {
        win_api::destroy_window(self.handle as _);
    }
}
