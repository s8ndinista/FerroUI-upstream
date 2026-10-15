//! A lock that the thread holding it can take again: what the `lock`
//! statement of C# is (a monitor). Not a file of RemoteViewing; its
//! session locks `FramebufferUpdateRequestLock` and calls, with the lock
//! held, methods that take it again.

use std::sync::{Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, ThreadId};

#[derive(Default)]
struct Owner {
    thread: Option<ThreadId>,
    count: usize,
}

/// A reentrant lock without data.
#[derive(Default)]
pub struct ReentrantLock {
    owner: Mutex<Owner>,
    released: Condvar,
}

fn guard<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl ReentrantLock {
    /// A new lock that no thread holds.
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes the lock, waiting while another thread holds it. The thread
    /// that holds it takes it again at once.
    pub fn lock(&self) -> ReentrantLockGuard<'_> {
        let current = thread::current().id();
        let mut owner = guard(&self.owner);
        loop {
            match owner.thread {
                None => {
                    owner.thread = Some(current);
                    owner.count = 1;
                    break;
                }
                Some(thread) if thread == current => {
                    owner.count += 1;
                    break;
                }
                Some(_) => {
                    owner = self
                        .released
                        .wait(owner)
                        .unwrap_or_else(PoisonError::into_inner)
                }
            }
        }
        ReentrantLockGuard { lock: self }
    }

    /// Whether the current thread holds the lock.
    pub fn is_held_by_current_thread(&self) -> bool {
        guard(&self.owner).thread == Some(thread::current().id())
    }

    fn unlock(&self) {
        let mut owner = guard(&self.owner);
        owner.count -= 1;
        if owner.count == 0 {
            owner.thread = None;
            drop(owner);
            self.released.notify_one();
        }
    }
}

/// The lock held: released when it is dropped.
pub struct ReentrantLockGuard<'a> {
    lock: &'a ReentrantLock,
}

impl Drop for ReentrantLockGuard<'_> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn the_holder_takes_the_lock_again_and_another_thread_waits() {
        let lock = Arc::new(ReentrantLock::new());
        let entered = Arc::new(AtomicBool::new(false));

        let outer = lock.lock();
        let inner = lock.lock();
        assert!(lock.is_held_by_current_thread());

        let other = {
            let lock = lock.clone();
            let entered = entered.clone();
            thread::spawn(move || {
                let _guard = lock.lock();
                entered.store(true, Ordering::SeqCst);
            })
        };

        drop(inner);
        thread::sleep(Duration::from_millis(50));
        assert!(
            !entered.load(Ordering::SeqCst),
            "the lock is still held once"
        );

        drop(outer);
        other.join().unwrap();
        assert!(entered.load(Ordering::SeqCst));
        assert!(!lock.is_held_by_current_thread());
    }
}
