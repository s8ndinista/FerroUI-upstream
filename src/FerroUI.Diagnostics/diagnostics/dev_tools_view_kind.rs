//! Port of `Diagnostics/DevToolsViewKind.cs`.

/// Kinds of diagnostic views available in DevTools
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DevToolsViewKind {
    /// The Logical Tree diagnostic view
    #[default]
    LogicalTree = 0,
    /// The Visual Tree diagnostic view
    VisualTree = 1,
    /// Events diagnostic view
    Events = 2,
}
