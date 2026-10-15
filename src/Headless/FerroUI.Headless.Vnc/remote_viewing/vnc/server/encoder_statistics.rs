//! Port of `Vnc/Server/EncoderStatistics.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

/// Tracks statistics of an encoder: the settable properties of the original
/// are fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EncoderStatistics {
    /// The number of rectangles which were sent with this encoder.
    pub rectangles: u32,

    /// The size of the raw pixel data of those rectangles, in bytes.
    pub raw_bytes: u64,

    /// The number of bytes those rectangles took on the wire.
    pub encoded_bytes: u64,
}
