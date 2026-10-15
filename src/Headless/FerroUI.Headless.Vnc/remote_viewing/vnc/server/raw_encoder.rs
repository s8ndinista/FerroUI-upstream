//! Port of `Vnc/Server/RawEncoder.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

use super::VncEncoder;
use crate::remote_viewing::vnc::{VncEncoding, VncPixelFormat, VncRectangle};
use std::io::{self, Write};

/// Represents the raw VNC encoding protocol.
///
/// The simplest encoding type is raw pixel data. In this case the data
/// consists of width * height pixel values (where width and height are the
/// width and height of the rectangle). The values simply represent each
/// pixel in left-to-right scanline order. See
/// <https://github.com/rfbproto/rfbproto/blob/master/rfbproto.rst#raw-encoding>.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RawEncoder;

impl VncEncoder for RawEncoder {
    fn encoding(&self) -> VncEncoding {
        VncEncoding::Raw
    }

    fn type_name(&self) -> &'static str {
        "RawEncoder"
    }

    fn send(
        &self,
        stream: &mut dyn Write,
        _pixel_format: &VncPixelFormat,
        _region: VncRectangle,
        contents: &[u8],
    ) -> io::Result<usize> {
        stream.write_all(contents)?;
        Ok(contents.len())
    }
}
