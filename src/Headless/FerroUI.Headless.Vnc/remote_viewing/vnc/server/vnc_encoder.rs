//! Port of `Vnc/Server/VncEncoder.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

use crate::remote_viewing::vnc::{VncEncoding, VncPixelFormat, VncRectangle};
use std::io::{self, Write};

/// A common base for VNC encoders.
pub trait VncEncoder: Send + Sync {
    /// Gets the encoding protocol implemented by this encoder.
    fn encoding(&self) -> VncEncoding;

    /// The name of the type of the encoder, for the log of the session
    /// (`GetType().Name` of the original).
    fn type_name(&self) -> &'static str;

    /// Sends the contents of a rectangle to the client.
    ///
    /// `stream` is the connection to the VNC client, `pixel_format` the
    /// pixel format being used, `region` the dimensions of the rectangle
    /// and `contents` its contents, in raw pixel format.
    ///
    /// Returns the total number of bytes written to the wire, used for
    /// bookkeeping.
    fn send(
        &self,
        stream: &mut dyn Write,
        pixel_format: &VncPixelFormat,
        region: VncRectangle,
        contents: &[u8],
    ) -> io::Result<usize>;
}
