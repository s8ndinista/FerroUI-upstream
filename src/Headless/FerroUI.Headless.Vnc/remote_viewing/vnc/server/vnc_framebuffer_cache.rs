//! Port of `Vnc/Server/VncFramebufferCache.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).
//!
//! One correction: the regions of an incremental update are the runs of
//! changed lines. The original offsets them by the top of the requested
//! region twice and makes each one line longer than its run, which only
//! matches the changes for a request from the top of the framebuffer.

use super::{IVncFramebufferCache, IVncServerSession};
use crate::remote_viewing::logging::{ILogger, LoggerExtensions};
use crate::remote_viewing::vnc::{VncFramebuffer, VncRectangle};
use std::sync::{Arc, Mutex, PoisonError};

/// The size of the tiles which will be invalidated. Not used: the cache
/// compares the framebuffer line by line.
#[allow(dead_code)]
const TILE_SIZE: i32 = 64;

/// Caches the framebuffer pixel data and updates them as new framebuffer
/// commands are received.
pub(crate) struct VncFramebufferCache {
    logger: Option<Arc<dyn ILogger>>,

    /// Whether each line of the region of the last request is invalid.
    is_line_invalid: Mutex<Vec<bool>>,

    /// We cache the latest framebuffer data as it was sent to the client.
    /// When looking for changes, we compare with the framebuffer which is
    /// cached here and send the deltas (for each line which was
    /// invalidated) to the client.
    cached_framebuffer: Arc<VncFramebuffer>,

    framebuffer: Arc<VncFramebuffer>,
}

impl VncFramebufferCache {
    /// Initializes a new instance of the [`VncFramebufferCache`] class for
    /// `framebuffer`, logging with `logger`.
    pub(crate) fn new(framebuffer: Arc<VncFramebuffer>, logger: Option<Arc<dyn ILogger>>) -> Self {
        let cached_framebuffer = VncFramebuffer::new(
            framebuffer.name(),
            framebuffer.width(),
            framebuffer.height(),
            *framebuffer.pixel_format(),
        );
        let is_line_invalid = vec![false; framebuffer.height().max(0) as usize];

        Self {
            logger,
            is_line_invalid: Mutex::new(is_line_invalid),
            cached_framebuffer,
            framebuffer,
        }
    }
}

impl IVncFramebufferCache for VncFramebufferCache {
    fn framebuffer(&self) -> Arc<VncFramebuffer> {
        self.framebuffer.clone()
    }

    fn respond_to_update_request(&self, session: &dyn IVncServerSession) -> bool {
        let fb = &self.framebuffer;
        let Some(fbr) = session.framebuffer_update_request() else {
            return false;
        };

        let incremental = fbr.incremental();
        // The request was checked against the framebuffer of its time, which a capture may have
        // replaced by a smaller one since.
        let region = VncRectangle::intersect(
            fbr.region(),
            VncRectangle::new(0, 0, fb.width(), fb.height()),
        );
        let bpp = fb.pixel_format().bytes_per_pixel() as usize;

        if let Some(logger) = &self.logger {
            logger.log_debug(&format!(
                "Responding to an update request for region {region}."
            ));
        }

        session.framebuffer_manual_begin_update();

        let mut is_line_invalid = self
            .is_line_invalid
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        // Take the locks here, as we will modify both buffers heavily in the next block.
        {
            let actual_buffer = fb.get_buffer();
            let mut buffered_buffer = self.cached_framebuffer.get_buffer();

            // In this block, we will determine which rectangles need updating. Right now, we
            // consider each line at once. It's not a very efficient algorithm, but it works.
            // We're going to start at the upper-left position of the region, and then we will
            // work our way down, on a line by line basis, to determine if each line is still
            // valid. `is_line_invalid` will indicate, on a line-by-line basis, whether a line is
            // still valid or not.
            for y in region.y..region.y + region.height {
                // For a given y, the x pixels are stored sequentially in the array starting at
                // y * stride (number of bytes per row); for each x value there are bpp bytes of
                // data (4 for a 32-bit integer); we are looking for pixels between x and x + w
                // so this translates to y * stride + bpp * x and y * stride + bpp * (x + w).
                let src_offset = (y as usize * fb.stride() as usize) + (bpp * region.x as usize);
                let length = bpp * region.width as usize;
                let range = src_offset..src_offset + length;

                let is_valid = actual_buffer[range.clone()] == buffered_buffer[range.clone()];
                if !is_valid {
                    buffered_buffer[range.clone()].copy_from_slice(&actual_buffer[range]);
                }

                is_line_invalid[(y - region.y) as usize] = !is_valid;
            }
        }

        if incremental {
            // Determine logical groups of lines which are invalid. We find the first line which
            // is invalid, and create a new region which contains all invalid lines which
            // immediately follow it. If we find a valid line, we'll create a new region.
            let mut first: Option<i32> = None;

            for line in 0..region.height {
                let invalid = is_line_invalid[line as usize];
                if first.is_none() && invalid {
                    first = Some(line);
                }

                if let Some(start) = first {
                    let last_line = line == region.height - 1;
                    if !invalid || last_line {
                        // Flush
                        let end = if invalid { line + 1 } else { line };
                        let subregion = VncRectangle::new(
                            region.x,
                            region.y + start,
                            region.width,
                            end - start,
                        );
                        session.framebuffer_manual_invalidate(subregion);
                        first = None;
                    }
                }
            }
        } else {
            session.framebuffer_manual_invalidate(region);
        }

        drop(is_line_invalid);
        session.framebuffer_manual_end_update()
    }
}
