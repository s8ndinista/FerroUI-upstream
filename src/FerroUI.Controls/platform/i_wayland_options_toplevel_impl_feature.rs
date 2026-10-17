/// Wayland-specific options of a top-level.
pub trait IWaylandOptionsToplevelImplFeature {
    fn set_app_id(&self, app_id: Option<&str>);
}
