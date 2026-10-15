//! Port of `Vnc/Server/IVncServerSession.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

use super::{
    CreateFramebufferCache, FramebufferUpdateRequest, PasswordProvidedEventArgs, VncEvent,
    VncServerSessionOptions,
};
use crate::remote_viewing::logging::ILogger;
use crate::remote_viewing::utility::ReentrantLock;
use crate::remote_viewing::vnc::{
    IVncFramebufferSource, IVncPasswordChallenge, KeyChangedEventArgs, PointerChangedEventArgs,
    VncEncoding, VncRectangle, VncTransport,
};
use std::sync::Arc;

/// Serves a VNC client with framebuffer information and receives keyboard
/// and mouse interactions.
pub trait IVncServerSession: Send + Sync {
    /// Occurs when a key has been pressed or released.
    fn key_changed(&self) -> &VncEvent<KeyChangedEventArgs>;

    /// Occurs on a mouse movement, button click, etc.
    fn pointer_changed(&self) -> &VncEvent<PointerChangedEventArgs>;

    /// Occurs when the VNC client has successfully connected to the server.
    fn connected(&self) -> &VncEvent<()>;

    /// Occurs when the VNC client has failed to connect to the server.
    fn connection_failed(&self) -> &VncEvent<()>;

    /// Occurs when the VNC client is disconnected.
    fn closed(&self) -> &VncEvent<()>;

    /// Occurs when the VNC client provides a password. Respond to this
    /// event by accepting or rejecting the password.
    fn password_provided(&self) -> &VncEvent<PasswordProvidedEventArgs>;

    /// Gets the logger to use when logging.
    fn logger(&self) -> Option<Arc<dyn ILogger>>;

    /// Sets the logger to use when logging.
    fn set_logger(&self, value: Option<Arc<dyn ILogger>>);

    /// Gets the password challenge to use when authenticating clients.
    fn password_challenge(&self) -> Arc<dyn IVncPasswordChallenge>;

    /// Sets the password challenge to use when authenticating clients.
    ///
    /// # Panics
    /// Once the security has been negotiated (`InvalidOperationException`).
    fn set_password_challenge(&self, value: Arc<dyn IVncPasswordChallenge>);

    /// Gets the max rate to send framebuffer updates at, in frames per
    /// second.
    fn max_update_rate(&self) -> f64;

    /// Sets the max rate to send framebuffer updates at, in frames per
    /// second.
    ///
    /// # Panics
    /// When the rate is not positive (`ArgumentOutOfRangeException`).
    fn set_max_update_rate(&self, value: f64);

    /// Gets the function which initializes a new framebuffer cache for use
    /// by this session.
    fn create_framebuffer_cache(&self) -> CreateFramebufferCache;

    /// Sets the function which initializes a new framebuffer cache for use
    /// by this session.
    fn set_create_framebuffer_cache(&self, value: CreateFramebufferCache);

    /// Gets the update request the session is answering, if any.
    fn framebuffer_update_request(&self) -> Option<FramebufferUpdateRequest>;

    /// Gets a lock which should be used before performing any framebuffer
    /// updates.
    fn framebuffer_update_request_lock(&self) -> &ReentrantLock;

    /// Gets the list of encodings supported by the client.
    fn client_encodings(&self) -> Vec<VncEncoding>;

    /// Starts a session with a VNC client over `stream`, with `options` if
    /// any.
    fn connect(&self, stream: Arc<dyn VncTransport>, options: Option<VncServerSessionOptions>);

    /// Closes the connection with the remote client.
    fn close(&self);

    /// Sets the framebuffer source; `None` if you intend to handle the
    /// framebuffer manually.
    fn set_framebuffer_source(&self, source: Option<Arc<dyn IVncFramebufferSource>>);

    /// Begins a manual update of the framebuffer.
    fn framebuffer_manual_begin_update(&self);

    /// Queues an update for the specified region.
    ///
    /// Do not call this method without holding
    /// [`framebuffer_update_request_lock`](Self::framebuffer_update_request_lock).
    fn framebuffer_manual_invalidate(&self, region: VncRectangle);

    /// Queues an update for each of the specified regions.
    ///
    /// Do not call this method without holding
    /// [`framebuffer_update_request_lock`](Self::framebuffer_update_request_lock).
    fn framebuffer_manual_invalidate_regions(&self, regions: &[VncRectangle]);

    /// Queues an update for the entire framebuffer.
    ///
    /// Do not call this method without holding
    /// [`framebuffer_update_request_lock`](Self::framebuffer_update_request_lock).
    fn framebuffer_manual_invalidate_all(&self);

    /// Completes a manual update of the framebuffer: sends what was queued.
    ///
    /// Do not call this method without holding
    /// [`framebuffer_update_request_lock`](Self::framebuffer_update_request_lock).
    ///
    /// Returns true if updates were sent.
    fn framebuffer_manual_end_update(&self) -> bool;
}
