//! Port of `Vnc/VncUtility.cs` of RemoteViewing (BSD-2-Clause, Copyright
//! (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

/// Provides utility methods.
pub(crate) struct VncUtility;

#[allow(dead_code)] // the methods of the original that the client of the library uses as well
impl VncUtility {
    /// Allocates a byte array of a given size: `scratch` is re-used if it
    /// has enough size.
    pub(crate) fn allocate_scratch(bytes: usize, scratch: &mut Vec<u8>) -> &mut Vec<u8> {
        if scratch.len() < bytes {
            *scratch = vec![0; bytes];
        }

        scratch
    }

    /// Decodes a `u16` from a byte array, in big-endian encoding.
    pub(crate) fn decode_uint16_be(buffer: &[u8], offset: usize) -> u16 {
        u16::from(buffer[offset]) << 8 | u16::from(buffer[offset + 1])
    }

    /// Encodes a `u16` as a byte array in big-endian encoding.
    pub(crate) fn encode_uint16_be(value: u16) -> [u8; 2] {
        let mut buffer = [0u8; 2];
        Self::encode_uint16_be_into(&mut buffer, 0, value);
        buffer
    }

    /// Encodes a `u16` in big-endian encoding into `buffer` at `offset`.
    pub(crate) fn encode_uint16_be_into(buffer: &mut [u8], offset: usize, value: u16) {
        buffer[offset] = (value >> 8) as u8;
        buffer[offset + 1] = value as u8;
    }

    /// Decodes a `u32` from a byte array, in big-endian encoding.
    pub(crate) fn decode_uint32_be(buffer: &[u8], offset: usize) -> u32 {
        u32::from(buffer[offset]) << 24
            | u32::from(buffer[offset + 1]) << 16
            | u32::from(buffer[offset + 2]) << 8
            | u32::from(buffer[offset + 3])
    }

    /// Encodes a `u32` as a byte array in big-endian encoding.
    pub(crate) fn encode_uint32_be(value: u32) -> [u8; 4] {
        let mut buffer = [0u8; 4];
        Self::encode_uint32_be_into(&mut buffer, 0, value);
        buffer
    }

    /// Encodes a `u32` in big-endian encoding into `buffer` at `offset`.
    pub(crate) fn encode_uint32_be_into(buffer: &mut [u8], offset: usize, value: u32) {
        buffer[offset] = (value >> 24) as u8;
        buffer[offset + 1] = (value >> 16) as u8;
        buffer[offset + 2] = (value >> 8) as u8;
        buffer[offset + 3] = value as u8;
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn values_are_big_endian() {
        assert_eq!([0x12, 0x34], VncUtility::encode_uint16_be(0x1234));
        assert_eq!(
            [0x12, 0x34, 0x56, 0x78],
            VncUtility::encode_uint32_be(0x1234_5678)
        );
        assert_eq!(0x1234, VncUtility::decode_uint16_be(&[0, 0x12, 0x34], 1));
        assert_eq!(
            0x1234_5678,
            VncUtility::decode_uint32_be(&[0x12, 0x34, 0x56, 0x78], 0)
        );

        let mut scratch = vec![1u8; 2];
        assert_eq!(4, VncUtility::allocate_scratch(4, &mut scratch).len());
        assert_eq!(4, VncUtility::allocate_scratch(3, &mut scratch).len());
    }
}
