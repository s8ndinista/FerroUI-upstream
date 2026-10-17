//! The tests of `WaylandPropertiesTests.cs`.

use crate::platform::{IWaylandOptionsToplevelImplFeature, WaylandProperties};
use crate::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
use crate::Window;
use std::cell::RefCell;
use std::rc::Rc;

/// Records the calls of the feature (the `Mock<IWaylandOptionsToplevelImplFeature>` of the
/// reference).
#[derive(Default)]
struct TestWaylandOptions {
    app_ids: RefCell<Vec<Option<String>>>,
}

impl TestWaylandOptions {
    fn times(&self, app_id: Option<&str>) -> usize {
        self.app_ids.borrow().iter().filter(|value| value.as_deref() == app_id).count()
    }
}

impl IWaylandOptionsToplevelImplFeature for TestWaylandOptions {
    fn set_app_id(&self, app_id: Option<&str>) {
        self.app_ids.borrow_mut().push(app_id.map(str::to_owned));
    }
}

#[test]
fn wayland_properties_app_id_attached_property_works() {
    let _app = UnitTestApplication::start(TestServices::mock_windowing_platform());
    let window = Window::new();
    assert_eq!(None, WaylandProperties::get_app_id(&window));

    WaylandProperties::set_app_id(&window, Some("org.test.app".to_owned()));
    assert_eq!(Some("org.test.app".to_owned()), WaylandProperties::get_app_id(&window));

    WaylandProperties::set_app_id(&window, None);
    assert_eq!(None, WaylandProperties::get_app_id(&window));
}

#[test]
fn wayland_properties_app_id_notifies_feature() {
    let feature = Rc::new(TestWaylandOptions::default());
    let window_impl = MockWindowingPlatform::create_window_mock();
    window_impl.render_scaling.set(1.0);
    window_impl.setup_feature::<dyn IWaylandOptionsToplevelImplFeature>(feature.clone());

    let platform_impl = window_impl.clone();
    let services = TestServices::styled_window()
        .with_windowing_platform(MockWindowingPlatform::with_window_impl(move || platform_impl.clone()));

    let _app = UnitTestApplication::start(services);
    let window = Window::new();

    WaylandProperties::set_app_id(&window, Some("my.custom.appid".to_owned()));
    assert_eq!(1, feature.times(Some("my.custom.appid")));

    WaylandProperties::set_app_id(&window, None);
    assert_eq!(1, feature.times(None));
}
