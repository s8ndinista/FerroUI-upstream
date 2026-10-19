//! `System.Collections.Specialized.INotifyCollectionChanged`: the change notification of a
//! collection, and the untyped description of a change that untyped listeners receive.

use super::NotifyCollectionChangedAction;
use crate::events::Event;

/// An untyped description of a change to a collection, as seen by bindings
/// with an indexer in their path.
///
/// Indexes are `-1` when not applicable to the action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CollectionChange {
    pub action: NotifyCollectionChangedAction,
    pub new_starting_index: i32,
    pub new_count: usize,
    pub old_starting_index: i32,
    pub old_count: usize,
}

/// Notifies clients of changes to a collection, such as when items are added
/// and removed or the whole collection is reset.
pub trait INotifyCollectionChanged {
    /// The event raised when the collection changes.
    fn collection_changed(&self) -> &Event<CollectionChange>;
}
