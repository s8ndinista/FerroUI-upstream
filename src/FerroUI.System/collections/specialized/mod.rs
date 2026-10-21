//! `System.Collections.Specialized`: the change notification of collections.

mod i_notify_collection_changed;
mod notify_collection_changed_event_args;

pub use i_notify_collection_changed::{CollectionChange, INotifyCollectionChanged};
pub use notify_collection_changed_event_args::{
    NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs, SharedNotifyCollectionChangedEventArgs,
};
