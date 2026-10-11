//! An output that renders with OpenGL (the port of `IGlOutputBackend.cs`).

use super::IOutputBackend;
use ferroui_base::platform::IPlatformGraphics;
use std::sync::Arc;

pub trait IGlOutputBackend: IOutputBackend {
    fn platform_graphics(&self) -> Arc<dyn IPlatformGraphics>;
}
