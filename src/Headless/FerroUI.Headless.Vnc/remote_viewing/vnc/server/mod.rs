//! Port of the `Vnc/Server` directory of RemoteViewing: the server session,
//! its options and events, the framebuffer cache that finds what changed
//! and the encoders that send it.

mod creating_desktop_event_args;
mod encoder_statistics;
mod framebuffer_update_request;
mod framebuffer_updating_event_args;
mod i_vnc_framebuffer_cache;
mod i_vnc_server_session;
mod password_provided_event_args;
mod raw_encoder;
mod tight_compression;
mod tight_compression_control;
mod tight_encoder;
mod vnc_encoder;
mod vnc_event;
mod vnc_framebuffer_cache;
mod vnc_message_type;
mod vnc_server_session;
mod vnc_server_session_options;

pub use creating_desktop_event_args::CreatingDesktopEventArgs;
pub use encoder_statistics::EncoderStatistics;
pub use framebuffer_update_request::FramebufferUpdateRequest;
pub use framebuffer_updating_event_args::FramebufferUpdatingEventArgs;
pub use i_vnc_framebuffer_cache::IVncFramebufferCache;
pub use i_vnc_server_session::IVncServerSession;
pub use password_provided_event_args::PasswordProvidedEventArgs;
pub(crate) use raw_encoder::RawEncoder;
pub use tight_compression::TightCompression;
pub(crate) use tight_compression_control::TightCompressionControl;
pub use tight_encoder::TightEncoder;
pub use vnc_encoder::VncEncoder;
pub use vnc_event::{VncEvent, VncEventHandler};
pub(crate) use vnc_framebuffer_cache::VncFramebufferCache;
pub(crate) use vnc_message_type::VncMessageType;
pub use vnc_server_session::{CreateFramebufferCache, VncServerSession};
pub use vnc_server_session_options::VncServerSessionOptions;
