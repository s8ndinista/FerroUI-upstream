//! Port of `Vnc/Server/VncMessageType.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

use std::fmt;

/// The types of the messages a client sends. A transparent wrapper of the
/// byte, as a message type of the original may be any byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub(crate) struct VncMessageType(pub(crate) u8);

#[allow(non_upper_case_globals, dead_code)] // the names of the original; not every type is handled
impl VncMessageType {
    /// Sets the format in which pixel values should be sent in framebuffer
    /// update messages.
    pub(crate) const SetPixelFormat: VncMessageType = VncMessageType(0);

    /// Sets the encoding types in which pixel data can be sent by the
    /// server.
    pub(crate) const SetEncodings: VncMessageType = VncMessageType(2);

    /// Notifies the server that the client is interested in the area of the
    /// framebuffer specified by x-position, y-position, width and height.
    pub(crate) const FrameBufferUpdateRequest: VncMessageType = VncMessageType(3);

    /// A key press or release.
    pub(crate) const KeyEvent: VncMessageType = VncMessageType(4);

    /// Indicates either pointer movement or a pointer button press or
    /// release.
    pub(crate) const PointerEvent: VncMessageType = VncMessageType(5);

    /// The client has new ISO 8859-1 (Latin-1) text in its cut buffer.
    pub(crate) const ClientCutText: VncMessageType = VncMessageType(6);

    /// This message informs the server to switch between only sending
    /// framebuffer update messages as a result of a framebuffer update
    /// request message, or sending them continuously.
    pub(crate) const EnableContinuousUpdates: VncMessageType = VncMessageType(150);

    /// A fence message of the client.
    pub(crate) const ClientFence: VncMessageType = VncMessageType(248);

    /// A message of the XVP extension.
    pub(crate) const Xvp: VncMessageType = VncMessageType(250);

    /// Requests a change of desktop size.
    pub(crate) const SetDesktopSize: VncMessageType = VncMessageType(251);

    /// An unknown message type.
    pub(crate) const Unknown: VncMessageType = VncMessageType(0xFF);
}

impl fmt::Display for VncMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self.0 {
            0 => "SetPixelFormat",
            2 => "SetEncodings",
            3 => "FrameBufferUpdateRequest",
            4 => "KeyEvent",
            5 => "PointerEvent",
            6 => "ClientCutText",
            150 => "EnableContinuousUpdates",
            248 => "ClientFence",
            250 => "Xvp",
            251 => "SetDesktopSize",
            255 => "Unknown",
            other => return write!(f, "{other}"),
        };
        f.write_str(name)
    }
}
