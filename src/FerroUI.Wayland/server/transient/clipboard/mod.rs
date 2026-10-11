//! The worker half of the clipboard and of drag and drop (the directory
//! `Server/Transient/Clipboard` of the reference): the data device of a
//! seat, the offers of other clients, the sources of this one, and the token
//! of an offer the UI thread holds.

pub mod wayland_data_device;
pub mod wayland_data_offer;
pub mod wayland_data_source;
pub mod wayland_offer_cookie;
