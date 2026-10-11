//! The result of work another thread does, awaited where it was asked for.
//!
//! The reference returns a `Task` from `WaylandWorker.InvokeOobAsync` and
//! from a read or a write of a pipe on a pool thread, and awaits it on the
//! UI thread or blocks on it (`GetAwaiter().GetResult()`). This is that
//! task: a value one thread gives and another one awaits or waits for.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};

/// The result of work another thread does.
///
/// The future resolves to `None` when the work was dropped before it gave a
/// result: a command the worker never ran, or one that panicked.
pub struct CrossThreadResult<T> {
    shared: Arc<Mutex<Shared<T>>>,
}

/// The other end of a [`CrossThreadResult`]: what the thread that does the work holds.
pub struct CrossThreadCompletion<T> {
    shared: Arc<Mutex<Shared<T>>>,
}

struct Shared<T> {
    value: Option<T>,
    closed: bool,
    waker: Option<Waker>,
}

/// A result and its completion.
pub fn cross_thread_result<T>() -> (CrossThreadCompletion<T>, CrossThreadResult<T>) {
    let shared = Arc::new(Mutex::new(Shared { value: None, closed: false, waker: None }));
    (CrossThreadCompletion { shared: shared.clone() }, CrossThreadResult { shared })
}

impl<T> CrossThreadCompletion<T> {
    /// Gives the result and wakes whoever waits for it.
    pub fn complete(self, value: T) {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner).value = Some(value);
        // The drop that follows closes the result and wakes.
    }
}

impl<T> Drop for CrossThreadCompletion<T> {
    fn drop(&mut self) {
        let waker = {
            let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
            shared.closed = true;
            shared.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<T> CrossThreadResult<T> {
    /// A result that is there already.
    pub fn ready(value: Option<T>) -> Self {
        Self { shared: Arc::new(Mutex::new(Shared { value, closed: true, waker: None })) }
    }

    /// Runs `work` on a thread of its own and gives its result (`Task.Run`).
    pub fn run_on_thread(work: impl FnOnce() -> T + Send + 'static) -> Self
    where
        T: Send + 'static,
    {
        let (completion, result) = cross_thread_result();
        let spawned = std::thread::Builder::new().name("FerroWaylandTransfer".to_string()).spawn(move || {
            completion.complete(work());
        });
        // A thread that cannot be started drops the completion with the closure: the result
        // is `None`.
        drop(spawned);
        result
    }

    /// Waits for the result on the calling thread (`GetAwaiter().GetResult()`).
    pub fn wait(self) -> Option<T> {
        struct ThreadWaker(std::thread::Thread);
        impl std::task::Wake for ThreadWaker {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Waker::from(Arc::new(ThreadWaker(std::thread::current())));
        let mut context = Context::from_waker(&waker);
        let mut this = self;
        loop {
            match Pin::new(&mut this).poll(&mut context) {
                Poll::Ready(value) => return value,
                Poll::Pending => std::thread::park(),
            }
        }
    }
}

impl<T> Future for CrossThreadResult<T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(value) = shared.value.take() {
            return Poll::Ready(Some(value));
        }
        if shared.closed {
            return Poll::Ready(None);
        }
        shared.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn a_result_given_on_another_thread_is_waited_for() {
        let result = CrossThreadResult::run_on_thread(|| {
            std::thread::sleep(std::time::Duration::from_millis(20));
            42
        });
        assert_eq!(result.wait(), Some(42));
    }

    #[test]
    fn a_completion_that_is_dropped_gives_no_result() {
        let (completion, result) = cross_thread_result::<u32>();
        let thread = std::thread::spawn(move || drop(completion));
        assert_eq!(result.wait(), None);
        thread.join().unwrap();
        assert_eq!(CrossThreadResult::<u32>::ready(None).wait(), None);
        assert_eq!(CrossThreadResult::ready(Some(7)).wait(), Some(7));
    }

    #[test]
    fn a_result_that_is_polled_first_is_woken_when_it_is_given() {
        struct Flag(AtomicBool);
        impl std::task::Wake for Flag {
            fn wake(self: Arc<Self>) {
                self.0.store(true, Ordering::SeqCst);
            }
        }
        let flag = Arc::new(Flag(AtomicBool::new(false)));
        let waker = Waker::from(flag.clone());
        let mut context = Context::from_waker(&waker);
        let (completion, mut result) = cross_thread_result::<&str>();
        assert!(Pin::new(&mut result).poll(&mut context).is_pending());
        completion.complete("done");
        assert!(flag.0.load(Ordering::SeqCst));
        assert_eq!(Pin::new(&mut result).poll(&mut context), Poll::Ready(Some("done")));
    }
}
