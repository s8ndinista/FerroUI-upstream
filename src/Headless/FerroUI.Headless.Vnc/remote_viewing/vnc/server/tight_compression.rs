//! Port of `Vnc/Server/TightCompression.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

/// Determines the compression used by the Tight encoder.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TightCompression {
    /// Use Basic compression, and compress all data using zlib.
    Basic,

    /// Use JPEG compression, and compress all images as a JPEG file.
    #[default]
    Jpeg,
}
