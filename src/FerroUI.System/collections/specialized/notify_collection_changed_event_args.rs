//! `System.Collections.Specialized.NotifyCollectionChangedEventArgs`: the description of a change
//! to a collection, borrowed by the handlers of the change and shareable as an owned handle.

use std::rc::Rc;

/// Describes the action that caused a collection changed notification.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NotifyCollectionChangedAction {
    Add,
    Remove,
    Replace,
    Move,
    Reset,
}

/// Describes a change to a collection.
///
/// Indexes are `-1` when not applicable to the action, as in the notification
/// model this mirrors.
pub struct NotifyCollectionChangedEventArgs<'a, T> {
    pub action: NotifyCollectionChangedAction,
    pub new_items: &'a [T],
    pub old_items: &'a [T],
    pub new_starting_index: i32,
    pub old_starting_index: i32,
}

impl<T: Clone> NotifyCollectionChangedEventArgs<'_, T> {
    /// The change as an owned, shareable value: what untyped code (an
    /// event handler wired up by markup) is given, since the borrowed form
    /// cannot be held in an untyped value.
    pub fn share(&self) -> SharedNotifyCollectionChangedEventArgs<T> {
        SharedNotifyCollectionChangedEventArgs(Rc::new(SharedNotifyCollectionChangedData {
            action: self.action,
            new_items: self.new_items.to_vec(),
            old_items: self.old_items.to_vec(),
            new_starting_index: self.new_starting_index,
            old_starting_index: self.old_starting_index,
        }))
    }
}

struct SharedNotifyCollectionChangedData<T> {
    action: NotifyCollectionChangedAction,
    new_items: Vec<T>,
    old_items: Vec<T>,
    new_starting_index: i32,
    old_starting_index: i32,
}

/// The owned form of [`NotifyCollectionChangedEventArgs`]: a shared handle
/// to the description of a change. Clones refer to the same description and
/// handles compare by identity, as the event args object this mirrors.
pub struct SharedNotifyCollectionChangedEventArgs<T>(Rc<SharedNotifyCollectionChangedData<T>>);

impl<T> Clone for SharedNotifyCollectionChangedEventArgs<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> PartialEq for SharedNotifyCollectionChangedEventArgs<T> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<T> SharedNotifyCollectionChangedEventArgs<T> {
    /// The action that caused the notification.
    pub fn action(&self) -> NotifyCollectionChangedAction {
        self.0.action
    }

    /// The items involved in the change that are new.
    pub fn new_items(&self) -> &[T] {
        &self.0.new_items
    }

    /// The items affected by a replace, remove or move.
    pub fn old_items(&self) -> &[T] {
        &self.0.old_items
    }

    /// The index at which the change occurred; `-1` when not applicable.
    pub fn new_starting_index(&self) -> i32 {
        self.0.new_starting_index
    }

    /// The index at which a move, remove or replace occurred; `-1` when not
    /// applicable.
    pub fn old_starting_index(&self) -> i32 {
        self.0.old_starting_index
    }

    /// The change in the borrowed form handlers take.
    pub fn as_args(&self) -> NotifyCollectionChangedEventArgs<'_, T> {
        NotifyCollectionChangedEventArgs {
            action: self.0.action,
            new_items: &self.0.new_items,
            old_items: &self.0.old_items,
            new_starting_index: self.0.new_starting_index,
            old_starting_index: self.0.old_starting_index,
        }
    }
}
