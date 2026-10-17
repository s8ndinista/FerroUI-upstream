use crate::platform::{IGeometryContext, IGeometryImpl};
use std::sync::Arc;

/// Builds a geometry using drawing commands.
pub trait IStreamGeometryBuilder: IGeometryContext {
    /// Creates the geometry described by the commands issued so far.
    ///
    /// Returns the resulting geometry, which is immutable.
    ///
    /// This method can only be called once: the context can't be used
    /// afterwards, except to be disposed.
    fn to_geometry(&mut self) -> Arc<dyn IGeometryImpl>;
}
