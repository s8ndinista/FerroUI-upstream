//! A minimal RFB (VNC) client for the tests: version 3.8, the security type
//! None, the raw encoding (with the desktop size pseudo-encodings), and the
//! key and pointer events. It keeps a copy of the framebuffer of the server
//! in the pixel format the server announces (32 bits, blue first).

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

/// The raw encoding.
pub const RAW: i32 = 0;
/// The desktop size pseudo-encoding.
pub const DESKTOP_SIZE: i32 = -223;
/// The extended desktop size pseudo-encoding.
pub const EXTENDED_DESKTOP_SIZE: i32 = -308;

/// A rectangle of a framebuffer update.
#[derive(Clone, Debug)]
pub struct UpdateRectangle {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub encoding: i32,
}

/// A connection to a VNC server.
pub struct RfbClient {
    stream: TcpStream,
    /// The size of the framebuffer.
    pub width: u16,
    pub height: u16,
    /// The pixel format of the server: bits per pixel, depth, big endian,
    /// true colour, the maxima and the shifts of red, green and blue.
    pub pixel_format: [u8; 16],
    /// The name of the desktop.
    pub name: String,
    /// The pixels, `width * height * 4` bytes.
    pub framebuffer: Vec<u8>,
}

fn read_exact(stream: &mut TcpStream, count: usize) -> io::Result<Vec<u8>> {
    let mut buffer = vec![0u8; count];
    stream.read_exact(&mut buffer)?;
    Ok(buffer)
}

fn u16_at(buffer: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([buffer[offset], buffer[offset + 1]])
}

fn u32_at(buffer: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        buffer[offset],
        buffer[offset + 1],
        buffer[offset + 2],
        buffer[offset + 3],
    ])
}

impl RfbClient {
    /// Connects to `address` and runs the handshake: the protocol version,
    /// the security type None, a shared desktop, the server init message.
    pub fn connect(address: &str) -> io::Result<RfbClient> {
        let mut stream = TcpStream::connect(address)?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_nodelay(true)?;

        let version = read_exact(&mut stream, 12)?;
        if &version != b"RFB 003.008\n" {
            return Err(io::Error::other(format!(
                "unexpected version {:?}",
                String::from_utf8_lossy(&version)
            )));
        }
        stream.write_all(b"RFB 003.008\n")?;

        let count = read_exact(&mut stream, 1)?[0];
        let types = read_exact(&mut stream, usize::from(count))?;
        if !types.contains(&1) {
            return Err(io::Error::other(format!(
                "the server does not offer the security type None: {types:?}"
            )));
        }
        stream.write_all(&[1])?;
        let result = read_exact(&mut stream, 4)?;
        if result != [0, 0, 0, 0] {
            return Err(io::Error::other(format!(
                "the security handshake failed: {result:?}"
            )));
        }

        // ClientInit: shared.
        stream.write_all(&[1])?;
        let init = read_exact(&mut stream, 24)?;
        let width = u16_at(&init, 0);
        let height = u16_at(&init, 2);
        let mut pixel_format = [0u8; 16];
        pixel_format.copy_from_slice(&init[4..20]);
        let name_length = u32_at(&init, 20) as usize;
        let name = String::from_utf8_lossy(&read_exact(&mut stream, name_length)?).into_owned();

        Ok(RfbClient {
            stream,
            width,
            height,
            pixel_format,
            name,
            framebuffer: vec![0; usize::from(width) * usize::from(height) * 4],
        })
    }

    /// Says which encodings the client supports.
    pub fn set_encodings(&mut self, encodings: &[i32]) -> io::Result<()> {
        let mut message = vec![2, 0];
        message.extend_from_slice(&(encodings.len() as u16).to_be_bytes());
        for encoding in encodings {
            message.extend_from_slice(&encoding.to_be_bytes());
        }
        self.stream.write_all(&message)
    }

    /// Asks for an update of a region of the framebuffer.
    pub fn request_update(
        &mut self,
        incremental: bool,
        x: u16,
        y: u16,
        width: u16,
        height: u16,
    ) -> io::Result<()> {
        let mut message = vec![3, u8::from(incremental)];
        for value in [x, y, width, height] {
            message.extend_from_slice(&value.to_be_bytes());
        }
        self.stream.write_all(&message)
    }

    /// Sends a pointer event: the mask of the pressed buttons (1 left, 2
    /// middle, 4 right) and the position.
    pub fn pointer(&mut self, buttons: u8, x: u16, y: u16) -> io::Result<()> {
        let mut message = vec![5, buttons];
        message.extend_from_slice(&x.to_be_bytes());
        message.extend_from_slice(&y.to_be_bytes());
        self.stream.write_all(&message)
    }

    /// Sends a key event of an X11 key symbol.
    pub fn key(&mut self, pressed: bool, keysym: u32) -> io::Result<()> {
        let mut message = vec![4, u8::from(pressed), 0, 0];
        message.extend_from_slice(&keysym.to_be_bytes());
        self.stream.write_all(&message)
    }

    /// Presses and releases the left button at a position.
    pub fn click(&mut self, x: u16, y: u16) -> io::Result<()> {
        self.pointer(0, x, y)?;
        self.pointer(1, x, y)?;
        self.pointer(0, x, y)
    }

    /// Presses and releases a key.
    pub fn type_key(&mut self, keysym: u32) -> io::Result<()> {
        self.key(true, keysym)?;
        self.key(false, keysym)
    }

    /// Reads the next framebuffer update message (and skips the bells and
    /// the clipboard messages before it), applies its rectangles to the
    /// framebuffer and returns them.
    pub fn read_update(&mut self) -> io::Result<Vec<UpdateRectangle>> {
        loop {
            let message_type = read_exact(&mut self.stream, 1)?[0];
            match message_type {
                0 => break,
                // Bell.
                2 => continue,
                // Server cut text.
                3 => {
                    let header = read_exact(&mut self.stream, 7)?;
                    read_exact(&mut self.stream, u32_at(&header, 3) as usize)?;
                    continue;
                }
                other => return Err(io::Error::other(format!("unexpected message type {other}"))),
            }
        }

        let header = read_exact(&mut self.stream, 3)?;
        let count = u16_at(&header, 1);
        let mut rectangles = Vec::with_capacity(usize::from(count));
        for _ in 0..count {
            let header = read_exact(&mut self.stream, 12)?;
            let rectangle = UpdateRectangle {
                x: u16_at(&header, 0),
                y: u16_at(&header, 2),
                width: u16_at(&header, 4),
                height: u16_at(&header, 6),
                encoding: u32_at(&header, 8) as i32,
            };
            match rectangle.encoding {
                RAW => {
                    let row = usize::from(rectangle.width) * 4;
                    let pixels = read_exact(&mut self.stream, row * usize::from(rectangle.height))?;
                    let stride = usize::from(self.width) * 4;
                    for line in 0..usize::from(rectangle.height) {
                        let target = (usize::from(rectangle.y) + line) * stride
                            + usize::from(rectangle.x) * 4;
                        if target + row <= self.framebuffer.len() {
                            self.framebuffer[target..target + row]
                                .copy_from_slice(&pixels[line * row..(line + 1) * row]);
                        }
                    }
                }
                DESKTOP_SIZE => self.resize(rectangle.width, rectangle.height),
                EXTENDED_DESKTOP_SIZE => {
                    let screens = read_exact(&mut self.stream, 4)?;
                    read_exact(&mut self.stream, 16 * usize::from(screens[0]))?;
                    self.resize(rectangle.width, rectangle.height);
                }
                other => return Err(io::Error::other(format!("unexpected encoding {other}"))),
            }
            rectangles.push(rectangle);
        }

        Ok(rectangles)
    }

    fn resize(&mut self, width: u16, height: u16) {
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            self.framebuffer = vec![0; usize::from(width) * usize::from(height) * 4];
        }
    }

    /// Asks for the whole framebuffer and reads updates until pixels
    /// arrived.
    pub fn fetch_frame(&mut self) -> io::Result<()> {
        loop {
            self.request_update(false, 0, 0, self.width, self.height)?;
            let rectangles = self.read_update()?;
            if rectangles.iter().any(|rectangle| rectangle.encoding == RAW) {
                return Ok(());
            }
        }
    }

    /// Writes the framebuffer as a PNG file (the pixels are blue, green,
    /// red and unused, as the server announces them).
    pub fn save_png(&self, file: &Path) -> io::Result<()> {
        let mut rgb = Vec::with_capacity(self.framebuffer.len() / 4 * 3);
        for pixel in self.framebuffer.chunks_exact(4) {
            rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        }
        let writer = io::BufWriter::new(std::fs::File::create(file)?);
        let mut encoder = png::Encoder::new(writer, u32::from(self.width), u32::from(self.height));
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(io::Error::other)?;
        writer.write_image_data(&rgb).map_err(io::Error::other)?;
        writer.finish().map_err(io::Error::other)
    }
}
