//! `System.Threading`: the cancellation of work.

mod cancellation_token;

pub use cancellation_token::{CancellationToken, CancellationTokenRegistration, CancellationTokenSource};
