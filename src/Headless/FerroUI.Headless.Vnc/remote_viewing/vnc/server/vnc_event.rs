//! An event of the server session: what an `event EventHandler<T>` of C#
//! is. Not a file of RemoteViewing.
//!
//! The session raises its events on its own threads, so the handlers are
//! `Send` and `Sync`; a handler gets the session that raised the event and
//! the arguments (`sender` and `e` of the original).

use super::VncServerSession;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

/// A handler of an event with the arguments `A`.
pub type VncEventHandler<A> = Arc<dyn Fn(&VncServerSession, &A) + Send + Sync>;

/// An event with the arguments `A`: handlers are added (`+=`) and removed
/// (`-=`), and called in the order they were added.
pub struct VncEvent<A> {
    handlers: Mutex<Vec<(u64, VncEventHandler<A>)>>,
    next_token: AtomicU64,
}

impl<A> Default for VncEvent<A> {
    fn default() -> Self {
        Self {
            handlers: Mutex::new(Vec::new()),
            next_token: AtomicU64::new(1),
        }
    }
}

impl<A> VncEvent<A> {
    /// Adds a handler. Returns the token that removes it.
    pub fn add(&self, handler: impl Fn(&VncServerSession, &A) + Send + Sync + 'static) -> u64 {
        let token = self.next_token.fetch_add(1, Ordering::Relaxed);
        self.handlers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((token, Arc::new(handler)));
        token
    }

    /// Removes the handler of `token`. Returns whether there was one.
    pub fn remove(&self, token: u64) -> bool {
        let mut handlers = self.handlers.lock().unwrap_or_else(PoisonError::into_inner);
        let count = handlers.len();
        handlers.retain(|(t, _)| *t != token);
        handlers.len() != count
    }

    /// Whether the event has handlers.
    pub fn has_handlers(&self) -> bool {
        !self
            .handlers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_empty()
    }

    /// Raises the event: calls the handlers that are added now, without
    /// holding the list (a handler may add or remove handlers).
    pub(crate) fn invoke(&self, sender: &VncServerSession, args: &A) {
        let handlers: Vec<VncEventHandler<A>> = self
            .handlers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .map(|(_, handler)| handler.clone())
            .collect();
        for handler in handlers {
            handler(sender, args);
        }
    }
}
