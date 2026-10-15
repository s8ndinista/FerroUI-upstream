//! An asynchronous operation of the Windows Runtime as a future: what the
//! reference awaits (`await view.GetTextAsync()`, `await StartDragAsync`).
//!
//! The future gives the operation a completion handler when it is first
//! polled. The handler is agile: the operation calls it on the thread it
//! completes on, and all it does there is record the status and wake the
//! task. The task of the integration runs on the dispatcher of the UI thread
//! (`Dispatcher::to_task_scheduler().start_local`), whose waker posts to
//! that thread, so the results are read on the UI thread, where the await
//! of the reference resumes too (its synchronization context is the one of
//! the dispatcher queue of WinUI).

use super::*;
use ferroui_microcom::{make_com_with, ComPtr, HResult, InterfaceEntry, E_ABORT, E_FAIL};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::task::{Context, Poll, Waker};

/// The status of an operation as the completion handler reports it, and the
/// waker of the task that waits for it.
#[derive(Default)]
pub(crate) struct Completion {
    status: Option<AsyncStatus>,
    waker: Option<Waker>,
}

pub(crate) type SharedCompletion = Arc<Mutex<Completion>>;

fn complete(completion: &SharedCompletion, status: AsyncStatus) {
    let waker = {
        let mut completion = completion.lock().unwrap_or_else(PoisonError::into_inner);
        completion.status = Some(status);
        completion.waker.take()
    };
    if let Some(waker) = waker {
        waker.wake();
    }
}

/// An interface `IAsyncOperation<T>` of the bindings.
pub(crate) trait AsyncOperationInterface: ferroui_microcom::Interface {
    type Output;

    /// Gives the operation a completion handler that records the status.
    fn set_completion(&self, completion: SharedCompletion) -> Result<(), HResult>;

    /// `GetResults`.
    fn results(&self) -> Result<Self::Output, HResult>;
}

macro_rules! async_operation {
    ($operation:ident, $handler:ident, $handler_impl:ident, $output:ty) => {
        impl AsyncOperationInterface for $operation {
            type Output = $output;

            fn set_completion(&self, completion: SharedCompletion) -> Result<(), HResult> {
                struct Handler(SharedCompletion);

                impl $handler_impl for Handler {
                    fn invoke(&self, _async_info: Option<&$operation>, async_status: AsyncStatus) -> Result<(), HResult> {
                        complete(&self.0, async_status);
                        Ok(())
                    }
                }

                impl IAgileObjectImpl for Handler {}

                let handler: ComPtr<$handler> = make_com_with(Handler(completion), [InterfaceEntry::of::<IAgileObject>()]);
                self.set_completed(Some(&handler))
            }

            fn results(&self) -> Result<$output, HResult> {
                self.get_results()
            }
        }
    };
}

async_operation!(
    IAsyncOperationDataPackageOperation,
    AsyncOperationCompletedHandlerDataPackageOperation,
    AsyncOperationCompletedHandlerDataPackageOperationImpl,
    DataPackageOperation
);
async_operation!(
    IAsyncOperationStorageItems,
    AsyncOperationCompletedHandlerStorageItems,
    AsyncOperationCompletedHandlerStorageItemsImpl,
    Option<ComPtr<IVectorViewStorageItem>>
);
async_operation!(IAsyncOperationString, AsyncOperationCompletedHandlerString, AsyncOperationCompletedHandlerStringImpl, isize);
async_operation!(
    IAsyncOperationStorageFile,
    AsyncOperationCompletedHandlerStorageFile,
    AsyncOperationCompletedHandlerStorageFileImpl,
    Option<ComPtr<IStorageFile>>
);
async_operation!(
    IAsyncOperationStorageFolder,
    AsyncOperationCompletedHandlerStorageFolder,
    AsyncOperationCompletedHandlerStorageFolderImpl,
    Option<ComPtr<IStorageFolder>>
);

/// Awaits an operation: its results, or the error it ended with (the
/// exception the await of the reference throws).
pub(crate) fn wait<O: AsyncOperationInterface>(operation: ComPtr<O>) -> AsyncOperationFuture<O> {
    AsyncOperationFuture { operation, completion: None }
}

pub(crate) struct AsyncOperationFuture<O: AsyncOperationInterface> {
    operation: ComPtr<O>,
    completion: Option<SharedCompletion>,
}

impl<O: AsyncOperationInterface> Unpin for AsyncOperationFuture<O> {}

impl<O: AsyncOperationInterface> Future for AsyncOperationFuture<O> {
    type Output = Result<O::Output, HResult>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let completion = match &self.completion {
            Some(completion) => completion.clone(),
            None => {
                let completion: SharedCompletion = Arc::default();
                completion.lock().unwrap_or_else(PoisonError::into_inner).waker = Some(cx.waker().clone());
                self.completion = Some(completion.clone());
                // An operation that has completed already calls the handler
                // before this returns.
                if let Err(error) = self.operation.set_completion(completion.clone()) {
                    return Poll::Ready(Err(error));
                }
                completion
            }
        };

        let status = {
            let mut state = completion.lock().unwrap_or_else(PoisonError::into_inner);
            if state.status.is_none() {
                state.waker = Some(cx.waker().clone());
            }
            state.status
        };
        match status {
            None => Poll::Pending,
            Some(AsyncStatus::Completed) => Poll::Ready(self.operation.results()),
            Some(AsyncStatus::Canceled) => Poll::Ready(Err(HResult(E_ABORT))),
            Some(_) => {
                let error = self
                    .operation
                    .cast::<IAsyncInfo>()
                    .and_then(|info| info.error_code())
                    .map(|code| if code < 0 { HResult(code as u32) } else { HResult(E_FAIL) })
                    .unwrap_or(HResult(E_FAIL));
                Poll::Ready(Err(error))
            }
        }
    }
}
