//! Port of `Vnc/Server/TightEncoder.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).
//!
//! zlib is `flate2` (in place of SharpCompress) and JPEG is `jpeg-encoder`
//! (in place of libjpeg-turbo, which the original calls through a wrapper):
//! both are written in Rust, so the encoder needs no native library.
//!
//! One correction: the protocol sends data of fewer than 12 bytes (after
//! the pixels are reduced to three bytes) as it is, without a length and
//! uncompressed. The original compares the size before that reduction and
//! writes a length in front of the bytes, which a client does not expect;
//! here the size after the reduction decides, and no length is written.

use super::{IVncServerSession, TightCompression, TightCompressionControl, VncEncoder};
use crate::remote_viewing::vnc::{VncEncoding, VncPixelFormat, VncRectangle};
use flate2::{Compress, Compression, FlushCompress, Status};
use jpeg_encoder::{ColorType, Encoder as JpegEncoder, SamplingFactor};
use std::io::{self, Write};
use std::sync::{Mutex, PoisonError, Weak};

/// Mapping of Tight quality levels to JPEG quality levels.
const QUALITY_LEVELS: [i32; 11] = [5, 10, 15, 25, 37, 50, 60, 70, 75, 80, 0];

/// The default zlib compression level (`CompressionLevel.Default` of the
/// original, level 6).
const DEFAULT_COMPRESSION_LEVEL: i32 = 6;

/// Tight encoding provides efficient compression for pixel data, using zlib
/// compression (over 4 multiple zlib streams), JPEG compression or fill
/// compression, and optionally applying filters to the raw pixel data.
///
/// The encoder currently only implements zlib compression over a single
/// zlib stream, which is reset after every rectangle.
///
/// See <https://github.com/rfbproto/rfbproto/blob/master/rfbproto.rst#tight-encoding>
/// and <https://virtualgl.org/pmwiki/uploads/About/tighttoturbo.pdf>.
pub struct TightEncoder {
    vnc_server_session: Weak<dyn IVncServerSession>,
    compression: Mutex<TightCompression>,
}

impl TightEncoder {
    /// Initializes a new instance of the [`TightEncoder`] class, linked to
    /// the parent session `vnc_server_session`.
    pub fn new(vnc_server_session: Weak<dyn IVncServerSession>) -> Self {
        Self {
            vnc_server_session,
            compression: Mutex::new(TightCompression::Jpeg),
        }
    }

    /// Gets the compression method used.
    pub fn compression(&self) -> TightCompression {
        *self
            .compression
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Sets the compression method used.
    pub fn set_compression(&self, value: TightCompression) {
        *self
            .compression
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = value;
    }

    /// Gets the session to which this encoder is linked.
    fn vnc_server_session(&self) -> Option<std::sync::Arc<dyn IVncServerSession>> {
        self.vnc_server_session.upgrade()
    }

    /// Encodes `value` into `buffer`, starting at `start_index`, 7 bits at a
    /// time: the high bit of a byte, when on, tells the reader to continue
    /// reading more bytes.
    ///
    /// Returns the index of the first byte after the encoded value.
    pub fn write_encoded_value(buffer: &mut [u8], start_index: usize, value: i32) -> usize {
        let mut index = start_index;
        let mut v = value as u32;
        while v >= 0x80 {
            buffer[index] = (v | 0x80) as u8;
            v >>= 7;
            index += 1;
        }

        buffer[index] = v as u8;
        index + 1
    }

    /// Gets the zlib compression level to use for the session.
    pub fn get_compression_level(vnc_server_session: Option<&dyn IVncServerSession>) -> i32 {
        Self::get_level(
            vnc_server_session,
            VncEncoding::TightCompressionLevel0,
            VncEncoding::TightCompressionLevel9,
            DEFAULT_COMPRESSION_LEVEL,
        )
    }

    /// Gets the JPEG quality level to use for the session, or 0 when JPEG
    /// compression should be disabled.
    pub fn get_quality_level(vnc_server_session: Option<&dyn IVncServerSession>) -> i32 {
        QUALITY_LEVELS[Self::get_level(
            vnc_server_session,
            VncEncoding::TightQualityLevel0,
            VncEncoding::TightQualityLevel9,
            10,
        ) as usize]
    }

    /// Sends a rectangle using JPEG compression. `pixel_format` must be
    /// [`VncPixelFormat::RGB32`].
    pub fn send_with_jpeg_compression(
        &self,
        stream: &mut dyn Write,
        _pixel_format: &VncPixelFormat,
        region: VncRectangle,
        contents: &[u8],
        jpeg_quality_level: i32,
    ) -> io::Result<usize> {
        let mut jpeg = Vec::new();
        let mut encoder = JpegEncoder::new(&mut jpeg, jpeg_quality_level.clamp(1, 100) as u8);
        encoder.set_sampling_factor(SamplingFactor::F_2_2);
        encoder
            .encode(
                contents,
                region.width as u16,
                region.height as u16,
                ColorType::Bgra,
            )
            .map_err(io::Error::other)?;

        // The compression control byte and the size of the JPEG buffer.
        let mut header = [0u8; 5];
        header[0] = TightCompressionControl::JPEG_COMPRESSION.bits();
        let length = Self::write_encoded_value(&mut header, 1, jpeg.len() as i32);

        stream.write_all(&header[..length])?;
        stream.write_all(&jpeg)?;

        Ok(jpeg.len() + length)
    }

    /// Sends a rectangle using basic compression.
    pub fn send_with_basic_compression(
        &self,
        stream: &mut dyn Write,
        pixel_format: &VncPixelFormat,
        _region: VncRectangle,
        contents: &[u8],
    ) -> io::Result<usize> {
        // The Tight encoding makes use of a new type TPIXEL (Tight pixel). This is the same as a
        // PIXEL for the agreed pixel format, except where true-colour-flag is non-zero,
        // bits-per-pixel is 32, depth is 24 and all of the bits making up the red, green and
        // blue intensities are exactly 8 bits wide. In this case a TPIXEL is only 3 bytes long,
        // where the first byte is the red component, the second byte is the green component,
        // and the third byte is the blue component of the pixel color value.
        let tight_pixels = pixel_format.bits_per_pixel() == 32
            && pixel_format.bit_depth() == 24
            && pixel_format.blue_bits() == 8
            && pixel_format.red_bits() == 8
            && pixel_format.green_bits() == 8
            && !pixel_format.is_palettized();

        let data = if tight_pixels {
            debug_assert!(
                contents.len() % 4 == 0,
                "The size of the raw pixel data must be a multiple of 4 when using a 32bpp pixel format."
            );

            // The byte of a colour in a pixel of four bytes: its shift in the value, and the
            // order of the bytes.
            let offset = |shift: i32| {
                let byte = (shift / 8) as usize;
                if pixel_format.is_little_endian() {
                    byte
                } else {
                    3 - byte
                }
            };
            let (red_offset, green_offset, blue_offset) = (
                offset(pixel_format.red_shift()),
                offset(pixel_format.green_shift()),
                offset(pixel_format.blue_shift()),
            );

            let mut data = Vec::with_capacity(contents.len() / 4 * 3);
            for pixel in contents.chunks_exact(4) {
                data.push(pixel[red_offset]);
                data.push(pixel[green_offset]);
                data.push(pixel[blue_offset]);
            }
            data
        } else {
            contents.to_vec()
        };

        if data.len() < 12 {
            // If the data size after applying the filter but before the compression is less
            // than 12, then the data is sent as is, uncompressed.
            stream.write_all(&[TightCompressionControl::BASIC_COMPRESSION.bits()])?;
            stream.write_all(&data)?;
            return Ok(1 + data.len());
        }

        let compression_control = TightCompressionControl::BASIC_COMPRESSION
            | TightCompressionControl::RESET_STREAM_0
            | TightCompressionControl::USE_STREAM_0;

        let compression_level =
            Self::get_compression_level(self.vnc_server_session().as_deref()).clamp(0, 9) as u32;

        stream.write_all(&[compression_control.bits()])?;

        let flush = if tight_pixels {
            FlushCompress::Full
        } else {
            FlushCompress::Finish
        };
        let buffer = Self::deflate(&data, compression_level, flush)?;

        let mut encoded_buffer = [0u8; 4];
        let length = Self::write_encoded_value(&mut encoded_buffer, 0, buffer.len() as i32);
        stream.write_all(&encoded_buffer[..length])?;
        stream.write_all(&buffer)?;

        Ok(1 + length + buffer.len())
    }

    /// Compresses `data` into a new zlib stream, ending with `flush`.
    fn deflate(data: &[u8], level: u32, flush: FlushCompress) -> io::Result<Vec<u8>> {
        let mut deflater = Compress::new(Compression::new(level), true);
        let mut buffer = Vec::with_capacity(data.len() / 2 + 64);
        loop {
            let consumed = deflater.total_in() as usize;
            if buffer.len() == buffer.capacity() {
                buffer.reserve(buffer.capacity().max(64));
            }
            let status = deflater
                .compress_vec(&data[consumed..], &mut buffer, flush)
                .map_err(io::Error::other)?;
            let done =
                deflater.total_in() as usize == data.len() && buffer.len() < buffer.capacity();
            if status == Status::StreamEnd || done {
                return Ok(buffer);
            }
        }
    }

    fn get_level(
        vnc_server_session: Option<&dyn IVncServerSession>,
        lower: VncEncoding,
        upper: VncEncoding,
        default_value: i32,
    ) -> i32 {
        let Some(vnc_server_session) = vnc_server_session else {
            return default_value;
        };

        for encoding in vnc_server_session.client_encodings() {
            if encoding >= lower && encoding <= upper {
                return encoding.0 - lower.0;
            }
        }

        default_value
    }
}

impl VncEncoder for TightEncoder {
    fn encoding(&self) -> VncEncoding {
        VncEncoding::Tight
    }

    fn type_name(&self) -> &'static str {
        "TightEncoder"
    }

    fn send(
        &self,
        stream: &mut dyn Write,
        pixel_format: &VncPixelFormat,
        region: VncRectangle,
        contents: &[u8],
    ) -> io::Result<usize> {
        let jpeg_quality_level = Self::get_quality_level(self.vnc_server_session().as_deref());

        // The JPEG compression currently assumes a RGB32 pixel format, fall back to basic
        // compression when this pixel format is not available. Additionally, a minimal JPEG
        // image is at least ~128 bytes in size, so only compress to JPEG when the uncompressed
        // file is significantly larger.
        if self.compression() != TightCompression::Jpeg
            || VncPixelFormat::RGB32 != *pixel_format
            || region.is_empty()
            || contents.len() < 256
            || jpeg_quality_level == 0
        {
            self.send_with_basic_compression(stream, pixel_format, region, contents)
        } else {
            self.send_with_jpeg_compression(
                stream,
                pixel_format,
                region,
                contents,
                jpeg_quality_level,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use flate2::{Decompress, FlushDecompress};

    fn encoder() -> TightEncoder {
        let session: Weak<dyn IVncServerSession> =
            Weak::<crate::remote_viewing::vnc::server::VncServerSession>::new();
        TightEncoder::new(session)
    }

    #[test]
    fn values_are_written_seven_bits_at_a_time() {
        let mut buffer = [0u8; 4];
        assert_eq!(1, TightEncoder::write_encoded_value(&mut buffer, 0, 0x7f));
        assert_eq!(0x7f, buffer[0]);
        assert_eq!(2, TightEncoder::write_encoded_value(&mut buffer, 0, 0x80));
        assert_eq!([0x80, 0x01], buffer[..2]);
        assert_eq!(
            3,
            TightEncoder::write_encoded_value(&mut buffer, 0, 100_000)
        );
    }

    #[test]
    fn without_a_session_the_levels_are_the_defaults() {
        assert_eq!(6, TightEncoder::get_compression_level(None));
        assert_eq!(0, TightEncoder::get_quality_level(None));
    }

    #[test]
    fn basic_compression_sends_three_bytes_a_pixel_through_zlib() {
        // 16 pixels of RGB32: blue, green, red, unused.
        let contents: Vec<u8> = (0..16u8).flat_map(|i| [i, 100 + i, 200 + i, 0]).collect();
        let mut wire = Vec::new();
        let written = encoder()
            .send_with_basic_compression(
                &mut wire,
                &VncPixelFormat::RGB32,
                VncRectangle::new(0, 0, 4, 4),
                &contents,
            )
            .unwrap();
        assert_eq!(written, wire.len());
        assert_eq!(
            0b0001, wire[0],
            "basic compression, stream 0 reset and used"
        );

        // The compact length, then the zlib data.
        let (mut length, mut index, mut shift) = (0usize, 1usize, 0);
        loop {
            let byte = wire[index];
            length |= usize::from(byte & 0x7f) << shift;
            index += 1;
            shift += 7;
            if byte & 0x80 == 0 {
                break;
            }
        }
        assert_eq!(wire.len() - index, length);

        let mut inflated = Vec::with_capacity(64);
        Decompress::new(true)
            .decompress_vec(&wire[index..], &mut inflated, FlushDecompress::Sync)
            .unwrap();
        let expected: Vec<u8> = (0..16u8).flat_map(|i| [200 + i, 100 + i, i]).collect();
        assert_eq!(expected, inflated);
    }

    #[test]
    fn small_rectangles_are_sent_as_they_are() {
        let contents = [1u8, 2, 3, 0, 4, 5, 6, 0];
        let mut wire = Vec::new();
        encoder()
            .send_with_basic_compression(
                &mut wire,
                &VncPixelFormat::RGB32,
                VncRectangle::new(0, 0, 2, 1),
                &contents,
            )
            .unwrap();
        assert_eq!(vec![0, 3, 2, 1, 6, 5, 4], wire);
    }

    #[test]
    fn jpeg_compression_writes_a_jpeg_image() {
        let contents: Vec<u8> = (0..64 * 64)
            .flat_map(|i| [(i % 256) as u8, 128, 64, 0])
            .collect();
        let mut wire = Vec::new();
        encoder()
            .send_with_jpeg_compression(
                &mut wire,
                &VncPixelFormat::RGB32,
                VncRectangle::new(0, 0, 64, 64),
                &contents,
                75,
            )
            .unwrap();
        assert_eq!(0b1001_0000, wire[0]);
        let start = if wire[1] & 0x80 == 0 {
            2
        } else if wire[2] & 0x80 == 0 {
            3
        } else {
            4
        };
        assert_eq!(
            &[0xff, 0xd8],
            &wire[start..start + 2],
            "the image starts with the marker of JPEG"
        );
    }
}
