//! Port of `Vnc/Server/TightCompressionControl.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

bitflags::bitflags! {
    /// Control flags used by the Tight compression protocol.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub(crate) struct TightCompressionControl: u8 {
        /// Informs the client zlib compression stream at index 0 should be
        /// reset before decoding the rectangle.
        const RESET_STREAM_0 = 0b_0001;

        /// Informs the client zlib compression stream at index 1 should be
        /// reset before decoding the rectangle.
        const RESET_STREAM_1 = 0b_0010;

        /// Informs the client zlib compression stream at index 2 should be
        /// reset before decoding the rectangle.
        const RESET_STREAM_2 = 0b_0100;

        /// Informs the client zlib compression stream at index 3 should be
        /// reset before decoding the rectangle. (The original has the value
        /// `0b_10100` here, which is not the bit of stream 3; the bit of the
        /// protocol is used.)
        const RESET_STREAM_3 = 0b_1000;

        /// Informs the client zlib compression stream at index 0 should be
        /// used to decode the rectangle.
        const USE_STREAM_0 = 0b_0000_0000;

        /// Informs the client zlib compression stream at index 1 should be
        /// used to decode the rectangle.
        const USE_STREAM_1 = 0b_0001_0000;

        /// Informs the client zlib compression stream at index 2 should be
        /// used to decode the rectangle.
        const USE_STREAM_2 = 0b_0010_0000;

        /// Informs the client zlib compression stream at index 3 should be
        /// used to decode the rectangle.
        const USE_STREAM_3 = 0b_0011_0000;

        /// Informs the client the next byte specifies the `filter-id` value
        /// which tells the decoder what filter type was used by the encoder
        /// to pre-process pixel data before the compression.
        const READ_FILTER_ID = 0b_0100_0000;

        /// Informs the client basic compression has been used.
        const BASIC_COMPRESSION = 0b_0000_0000;

        /// Informs the client fill compression has been used.
        const FILL_COMPRESSION = 0b_1000_0000;

        /// Informs the client JPEG compression has been used.
        const JPEG_COMPRESSION = 0b_1001_0000;
    }
}
