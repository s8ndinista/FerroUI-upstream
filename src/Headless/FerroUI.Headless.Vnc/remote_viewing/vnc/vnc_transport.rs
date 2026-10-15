//! The connection a session reads from and writes to: what the `Stream`
//! of .NET is for RemoteViewing. Not a file of RemoteViewing.
//!
//! A session reads on its own thread while the thread of its framebuffer
//! updates writes, as the network stream of .NET allows; a transport is
//! therefore used through shared references, as `&TcpStream` is.

use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpStream};

/// A duplex byte stream a VNC server session runs over.
pub trait VncTransport: Send + Sync {
    /// Reads into `buffer`; 0 at the end of the stream.
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize>;

    /// Writes all of `buffer`.
    fn write_all(&self, buffer: &[u8]) -> io::Result<()>;

    /// Closes the stream in both directions: a read that waits returns.
    fn close(&self);
}

impl VncTransport for TcpStream {
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut stream: &TcpStream = self;
        Read::read(&mut stream, buffer)
    }

    fn write_all(&self, buffer: &[u8]) -> io::Result<()> {
        let mut stream: &TcpStream = self;
        Write::write_all(&mut stream, buffer)
    }

    fn close(&self) {
        let _ = self.shutdown(Shutdown::Both);
    }
}

#[cfg(unix)]
impl VncTransport for std::os::unix::net::UnixStream {
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut stream: &std::os::unix::net::UnixStream = self;
        Read::read(&mut stream, buffer)
    }

    fn write_all(&self, buffer: &[u8]) -> io::Result<()> {
        let mut stream: &std::os::unix::net::UnixStream = self;
        Write::write_all(&mut stream, buffer)
    }

    fn close(&self) {
        let _ = self.shutdown(Shutdown::Both);
    }
}

/// A transport as a writer, for the encoders.
pub(crate) struct TransportWriter<'a>(pub(crate) &'a dyn VncTransport);

impl Write for TransportWriter<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.write_all(buffer)?;
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
