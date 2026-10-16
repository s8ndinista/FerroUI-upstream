//! Port of `MainWindow.xaml.cs`: the class of the document `MainWindow.xaml`.

use crate::d3d_demo::D3D11DemoControl;
use crate::demo_type::DemoType;
use crate::drawing_surface_demo_base::DrawingSurfaceDemoBase;
use crate::gpu_demo::{GpuDemo, IGpuDemo};
use crate::markup::xaml_class;
use crate::vulkan_demo::{VulkanDemoControl, VulkanDmaBufDemoControl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::rendering::RendererDebugOverlays;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Control, ContentControlImpl, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl};
use std::rc::Rc;

#[repr(C)]
pub struct MainWindow {
    base: Window,
}

ferro_class!(MainWindow: Window);
ferro_impl_classes!(
    MainWindow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(MainWindow { new: MainWindow::new });
xaml_class!(MainWindow, "/MainWindow.xaml");

impl MainWindow {
    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()) }
    }

    /// The window of the Vulkan demo.
    pub fn new() -> Ref<Self> {
        Self::with_demo_type(DemoType::Vulkan)
    }

    /// The window of the demo `demo_type`.
    pub fn with_demo_type(demo_type: DemoType) -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.set_title(Some(format!("{demo_type:?}")));

        let demo: Rc<dyn IGpuDemo> = match demo_type {
            DemoType::Vulkan => DrawingSurfaceDemoBase::as_gpu_demo(VulkanDemoControl::new()),
            DemoType::D3D11 => DrawingSurfaceDemoBase::as_gpu_demo(D3D11DemoControl::new()),
            DemoType::VulkanDmaBuf => DrawingSurfaceDemoBase::as_gpu_demo(VulkanDmaBufDemoControl::new()),
        };
        let content = GpuDemo::new();
        content.set_demo(Some(demo));
        this.set_content(Some(Control::boxed(content)));

        this.renderer_diagnostics().set_debug_overlays(RendererDebugOverlays::FPS);
        this
    }
}
