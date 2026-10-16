//! The real application of the sample, headless (`sample_testing::Shell`): the application
//! populates itself from `App.xaml`, the main window is created and shown as the desktop
//! lifetime shows it, and its frames are rendered through the compositor with Skia.
//!
//! The compositor of the tests renders with Skia on the processor and has no GPU interop, so
//! the demo shows why it does not draw, as upstream does where the backend has none.

use crate::vulkan_demo::VulkanDemoControl;
use crate::{as_gpu_demo, App, DemoType, DrawingSurfaceDemoBase, GpuDemo, MainWindow, SAMPLE};
use ferroui_base::Ref;
use ferroui_controls::primitives::RangeBase;
use ferroui_controls::{Application, Control, Slider, TextBlock};
use sample_testing::{assert_accepted, Shell};

/// The size of the window of the test.
const SIZE: (f64, f64) = (800.0, 600.0);

/// The binding errors the sample is accepted to report: none.
const ACCEPTED: &[(&str, &str, usize)] = &[];

fn start(demo_type: DemoType) -> (Shell, Ref<MainWindow>) {
    let shell = Shell::start(&SAMPLE, SIZE.0, SIZE.1, || App::new().upcast());
    let window = MainWindow::with_demo_type(demo_type);
    window.show();
    shell.settle();
    (shell, window)
}

fn reports(shell: &Shell, place: &str) -> Vec<(String, String, usize)> {
    shell.take_binding_reports().into_iter().map(|(report, count)| (place.to_string(), report.line(), count)).collect()
}

fn gpu_demo(window: &Ref<MainWindow>) -> Ref<GpuDemo> {
    let content = window.content().expect("the content of the window");
    Control::from_boxed(&content).and_then(|c| c.cast::<GpuDemo>()).expect("the content is the demo control")
}

#[test]
fn the_application_has_the_theme_its_document_declares() {
    let (shell, window) = start(DemoType::Vulkan);

    let application = Application::current().expect("the application of the test");
    assert!(application.cast::<App>().is_some(), "the application is the application of the sample");
    assert_eq!(application.styles().count(), 1, "App.xaml declares one style: the Fluent theme");

    window.close();
    drop(shell);
}

#[test]
fn the_main_window_shows_the_vulkan_demo_in_the_demo_control() {
    let (shell, window) = start(DemoType::Vulkan);

    assert_eq!(window.title().as_deref(), Some("Vulkan"), "the title names the demo");
    let gpu_demo = gpu_demo(&window);
    let demo = gpu_demo.demo().expect("the demo of the demo control");
    assert!(demo.cast::<VulkanDemoControl>().is_some(), "the demo is the Vulkan demo");
    let demo = as_gpu_demo(&demo).expect("the demo is a demo of the sample");

    // The content control of `GpuDemo.xaml` shows the demo through its binding.
    let shown = gpu_demo.find_descendant_of_type::<DrawingSurfaceDemoBase>(false).expect("the demo is in the visual tree");
    assert!(shown == demo, "the visual tree has the demo of the property");

    // The sliders start at the values of the demo control, and the disco slider is shown.
    assert_eq!((gpu_demo.yaw(), gpu_demo.pitch(), gpu_demo.roll(), gpu_demo.disco()), (5.0, 5.0, 5.0, 0.0));
    let sliders: Vec<Ref<Slider>> =
        gpu_demo.get_visual_descendants().filter_map(|v| v.cast::<Slider>()).collect();
    assert_eq!(sliders.len(), 4, "yaw, pitch, roll and disco");
    assert_eq!(sliders[0].value(), 5.0, "the yaw slider is bound to the yaw");
    assert!(gpu_demo.disco_visible(), "the demo supports the disco mode");

    // No GPU interop in the compositor of the tests: the demo says so, through the demo control.
    assert_eq!(demo.info(), "Compositor doesn't support interop for the current backend");
    assert_eq!(gpu_demo.info(), demo.info());
    assert!(!demo.is_initialized());
    let texts: Vec<String> =
        gpu_demo.get_visual_descendants().filter_map(|v| v.cast::<TextBlock>()).filter_map(|t| t.text()).collect();
    assert!(texts.iter().any(|t| t == &demo.info()), "the text of the demo is shown: {texts:?}");

    assert!(shell.frames(0) > 0, "the compositor drew the window");
    assert_accepted(&reports(&shell, "MainWindow"), ACCEPTED);

    window.close();
    drop(shell);
}

#[test]
fn a_slider_passes_its_value_to_the_demo() {
    let (shell, window) = start(DemoType::Vulkan);

    let gpu_demo = gpu_demo(&window);
    let demo = gpu_demo.demo().as_ref().and_then(as_gpu_demo).expect("the demo of the demo control");
    gpu_demo.set_yaw(2.5);
    gpu_demo.set_disco(0.5);
    assert_eq!((demo.yaw(), demo.disco()), (2.5, 0.5));

    let sliders: Vec<Ref<Slider>> =
        gpu_demo.get_visual_descendants().filter_map(|v| v.cast::<Slider>()).collect();
    sliders[1].set_value(RangeBase::value_property(), 7.0);
    assert_eq!(gpu_demo.pitch(), 7.0, "the pitch slider is bound both ways");
    assert_eq!(demo.pitch(), 7.0);

    window.close();
    drop(shell);
}

#[test]
fn the_other_demos_say_why_they_do_not_draw_here() {
    for (demo_type, title) in [(DemoType::D3D11, "D3D11"), (DemoType::VulkanDmaBuf, "VulkanDmaBuf")] {
        let (shell, window) = start(demo_type);
        assert_eq!(window.title().as_deref(), Some(title));
        let demo = gpu_demo(&window).demo().as_ref().and_then(as_gpu_demo).expect("the demo of the demo control");
        assert_eq!(demo.info(), "Compositor doesn't support interop for the current backend");
        window.close();
        drop(shell);
    }
}
