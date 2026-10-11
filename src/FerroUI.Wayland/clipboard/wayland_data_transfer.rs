//! The data of an offer of another client (the port of
//! `WaylandDataTransfer.cs`).
//!
//! The reference reads a pipe with the asynchronous calls of its runtime on
//! a pool thread, and blocks on that task where the contract is synchronous.
//! Here an asynchronous read is a read on a thread of its own, awaited on
//! the UI thread, and a synchronous read is a read on the calling thread.

use super::wayland_mime_mapper;
use crate::server::cross_thread_result::CrossThreadResult;
use crate::server::interop::pipe2_stream::{Pipe2Stream, PipeDirection};
use crate::server::transient::clipboard::wayland_offer_cookie::WaylandOfferCookie;
use ferroui_base::input::platform::{ClipboardError, PlatformDataTransferItem};
use ferroui_base::input::{
    DataFormat, IAsyncDataTransfer, IAsyncDataTransferItem, IDataTransfer, IDataTransferItem, LocalBoxFuture,
};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::storage::IStorageItem;
use ferroui_base::reactive::IDisposable;
use ferroui_x11::selections::uri_list_helper;
use std::any::{Any, TypeId};
use std::cell::{OnceCell, RefCell};
use std::future::ready;
use std::io::Cursor;
use std::rc::Rc;
use std::sync::Arc;

/// An item under both of its contracts.
#[derive(Clone)]
struct Item {
    sync: Rc<dyn IDataTransferItem>,
    asynchronous: Rc<dyn IAsyncDataTransferItem>,
}

impl Item {
    fn of<T: IDataTransferItem + IAsyncDataTransferItem + 'static>(item: Rc<T>) -> Self {
        Self { sync: item.clone(), asynchronous: item }
    }
}

struct Items {
    sync: Rc<[Rc<dyn IDataTransferItem>]>,
    asynchronous: Rc<[Rc<dyn IAsyncDataTransferItem>]>,
}

impl Items {
    fn of(items: Vec<Item>) -> Self {
        Self {
            sync: items.iter().map(|item| item.sync.clone()).collect(),
            asynchronous: items.iter().map(|item| item.asynchronous.clone()).collect(),
        }
    }
}

struct Formats {
    formats: Rc<[DataFormat]>,
    format_to_mime: Rc<[(DataFormat, String)]>,
}

/// Data transfer backed by a Wayland `wl_data_offer`, implementing both sync and async interfaces.
/// The offer cookie is validated on each read to detect stale offers.
/// Reused by both clipboard and DnD.
pub struct WaylandDataTransfer {
    mime_types: Vec<String>,
    offer_cookie: Arc<WaylandOfferCookie>,
    formats: OnceCell<Formats>,
    items: RefCell<Option<Items>>,
}

impl WaylandDataTransfer {
    pub fn new(mime_types: Vec<String>, offer_cookie: Arc<WaylandOfferCookie>) -> Rc<Self> {
        Rc::new(Self { mime_types, offer_cookie, formats: OnceCell::new(), items: RefCell::new(None) })
    }

    fn build_formats(&self) -> &Formats {
        self.formats.get_or_init(|| {
            let mut format_to_mime: Vec<(DataFormat, String)> = Vec::new();
            for mime in &self.mime_types {
                if let Some(format) = wayland_mime_mapper::from_mime_type(mime) {
                    if !format_to_mime.iter().any(|(known, _)| *known == format) {
                        format_to_mime.push((format, mime.clone()));
                    }
                }
            }
            Formats {
                formats: format_to_mime.iter().map(|(format, _)| format.clone()).collect(),
                format_to_mime: format_to_mime.into(),
            }
        })
    }

    fn get_formats(&self) -> Rc<[DataFormat]> {
        self.build_formats().formats.clone()
    }

    /// Pre-reads the file list asynchronously and builds the item layout. Used by the clipboard read
    /// path so the (cross-process) pipe read doesn't block the caller's thread.
    pub(crate) fn preload_async(self: &Rc<Self>) -> LocalBoxFuture<()> {
        if self.items.borrow().is_some() {
            return Box::pin(ready(()));
        }
        let this = self.clone();
        Box::pin(async move {
            let files = match this.file_mime() {
                Some(mime) => files_of(WaylandDataTransferItem::read_raw_bytes_async(&this.offer_cookie, &mime).await),
                None => None,
            };
            this.set_items(this.build_items(files));
        })
    }

    fn ensure_items_blocking(&self) {
        if self.items.borrow().is_some() {
            return;
        }
        // Synchronous IDataTransfer contract — blocking here is allowed (DnD reads this path at drop
        // on the UI thread). The offer is received on the Wayland thread, which never waits for
        // the UI thread, so it can't deadlock. wl_data_offer transfers are always cross-process —
        // in-process drags use the original IDataTransfer.
        let files = match self.file_mime() {
            Some(mime) => files_of(WaylandDataTransferItem::read_raw_bytes_blocking(&self.offer_cookie, &mime)),
            None => None,
        };
        self.set_items(self.build_items(files));
    }

    fn set_items(&self, items: Vec<Item>) {
        *self.items.borrow_mut() = Some(Items::of(items));
    }

    /// Builds the item layout: one item per file (each carrying a single storage item,
    /// matching the single-value contract of the file format) plus one shared item for all
    /// remaining formats. Mirrors the other platforms so file copy/paste and drag-drop work.
    fn build_items(&self, files: Option<Vec<Rc<dyn IStorageItem>>>) -> Vec<Item> {
        let Formats { formats, format_to_mime } = self.build_formats();
        let mut items = Vec::new();
        let mut non_file_formats: Vec<DataFormat> = Vec::new();

        for format in formats.iter() {
            if DataFormat::file() == *format {
                if let Some(files) = &files {
                    for file in files {
                        items.push(Item::of(PlatformDataTransferItem::create(&DataFormat::file(), file.clone())));
                    }
                }
            } else {
                non_file_formats.push(format.clone());
            }
        }

        if !non_file_formats.is_empty() {
            items.push(Item::of(Rc::new(WaylandDataTransferItem::new(
                non_file_formats.into(),
                format_to_mime.clone(),
                self.offer_cookie.clone(),
            ))));
        }

        items
    }

    /// The MIME type the file format is read with, or `None` if no File format.
    fn file_mime(&self) -> Option<String> {
        self.build_formats().format_to_mime.iter().find(|(format, _)| DataFormat::file() == *format).map(|(_, mime)| mime.clone())
    }
}

/// Parses the `text/uri-list` into storage items.
fn files_of(bytes: Option<Vec<u8>>) -> Option<Vec<Rc<dyn IStorageItem>>> {
    let bytes = bytes.filter(|bytes| !bytes.is_empty())?;
    Some(uri_list_helper::utf8_bytes_to_file_uri_list(&bytes))
}

impl IDataTransfer for WaylandDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.get_formats()
    }

    // Synchronous interface: may block to resolve the file list (allowed by the IDataTransfer contract).
    fn items(&self) -> Rc<[Rc<dyn IDataTransferItem>]> {
        self.ensure_items_blocking();
        self.items.borrow().as_ref().map(|items| items.sync.clone()).unwrap_or_else(|| Rc::from([]))
    }

    fn as_async_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IAsyncDataTransfer>> {
        Some(self)
    }
}

impl IAsyncDataTransfer for WaylandDataTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.get_formats()
    }

    // The fallback here isn't supposed to be called since clipboard calls `preload_async`, but we do a
    // non-blocking fallback just in case
    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        match self.items.borrow().as_ref() {
            Some(items) => items.asynchronous.clone(),
            None => Items::of(self.build_items(None)).asynchronous,
        }
    }

    fn as_data_transfer(self: Rc<Self>) -> Option<Rc<dyn IDataTransfer>> {
        Some(self)
    }
}

impl IDisposable for WaylandDataTransfer {
    fn dispose(&self) {
        // The offer is owned by the data device — nothing to dispose here
    }
}

/// A single data transfer item that reads from a Wayland data offer via pipe.
/// The offer cookie is sent back to the Wayland thread on each read;
/// the cookie validates the offer is still live and performs the receive internally.
/// Implements both the synchronous contract (blocks) and the asynchronous one
/// (true async via pipe).
pub struct WaylandDataTransferItem {
    formats: Rc<[DataFormat]>,
    format_to_mime: Rc<[(DataFormat, String)]>,
    offer_cookie: Arc<WaylandOfferCookie>,
}

impl WaylandDataTransferItem {
    pub fn new(formats: Rc<[DataFormat]>, format_to_mime: Rc<[(DataFormat, String)]>, offer_cookie: Arc<WaylandOfferCookie>) -> Self {
        Self { formats, format_to_mime, offer_cookie }
    }

    /// Reads the raw bytes offered for a MIME type through a pipe: the cookie is sent to the
    /// Wayland thread, which gives the reading end, and the pipe is read on a thread of its
    /// own.
    pub(crate) fn read_raw_bytes_async(offer_cookie: &Arc<WaylandOfferCookie>, mime_type: &str) -> LocalBoxFuture<Option<Vec<u8>>> {
        // Send cookie + MIME to Wayland thread; the cookie posts and returns the fd.
        let read_fd = offer_cookie.try_receive_async(mime_type);
        Box::pin(async move {
            let read_fd = read_fd.await.flatten()?;
            CrossThreadResult::run_on_thread(move || Pipe2Stream::new(read_fd, PipeDirection::In).read_to_end().ok())
                .await
                .flatten()
        })
    }

    /// Reads the raw bytes offered for a MIME type through a pipe, on the calling thread.
    pub(crate) fn read_raw_bytes_blocking(offer_cookie: &Arc<WaylandOfferCookie>, mime_type: &str) -> Option<Vec<u8>> {
        let read_fd = offer_cookie.try_receive_async(mime_type).wait().flatten()?;
        Pipe2Stream::new(read_fd, PipeDirection::In).read_to_end().ok()
    }

    /// Finds the MIME type to use for the given format: the preserved mapping from the
    /// original offer (round-trippable).
    fn find_mime_type(&self, format: &DataFormat) -> Option<String> {
        self.format_to_mime.iter().find(|(known, _)| known == format).map(|(_, mime)| mime.clone())
    }

    pub(crate) fn convert_data(format: &DataFormat, data: Vec<u8>) -> Option<Rc<dyn Any>> {
        if data.is_empty() {
            return None;
        }

        if DataFormat::text() == *format {
            return Some(Rc::new(String::from_utf8_lossy(&data).into_owned()));
        }

        // Files are never read through an item of this type: the transfer splits them
        // into one item per file (the file format is a single-value format).

        if DataFormat::bitmap() == *format {
            // The reference lets the exception of the bitmap constructor go on; the contract
            // of an item here has no error on the synchronous side, so data that is no image
            // is no value.
            return Bitmap::from_stream(&mut Cursor::new(data)).ok().map(|bitmap| Rc::new(Rc::new(bitmap)) as Rc<dyn Any>);
        }

        if format.data_type() == Some(TypeId::of::<String>()) {
            return Some(Rc::new(String::from_utf8_lossy(&data).into_owned()));
        }

        if format.data_type() == Some(TypeId::of::<Rc<[u8]>>()) {
            let bytes: Rc<[u8]> = Rc::from(data);
            return Some(Rc::new(bytes));
        }

        None
    }
}

impl IDataTransferItem for WaylandDataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.formats.clone()
    }

    /// Synchronous read — blocks the calling thread while the pipe is read.
    /// Used for DnD (sync data access during event handling).
    fn try_get_raw(&self, format: &DataFormat) -> Option<Rc<dyn Any>> {
        if !self.formats.contains(format) {
            return None;
        }
        let mime_type = self.find_mime_type(format)?;
        let data = Self::read_raw_bytes_blocking(&self.offer_cookie, &mime_type)?;
        Self::convert_data(format, data)
    }
}

impl IAsyncDataTransferItem for WaylandDataTransferItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.formats.clone()
    }

    /// Asynchronous read — sends the offer cookie to the Wayland thread,
    /// which validates and creates a pipe; data is read on a thread of its own.
    /// The data is converted on the dispatcher thread after resuming from the await.
    fn try_get_raw_async(&self, format: &DataFormat) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        if !self.formats.contains(format) {
            return Box::pin(ready(Ok(None)));
        }
        let Some(mime_type) = self.find_mime_type(format) else {
            return Box::pin(ready(Ok(None)));
        };
        let read = Self::read_raw_bytes_async(&self.offer_cookie, &mime_type);
        let format = format.clone();
        Box::pin(async move { Ok(read.await.and_then(|data| Self::convert_data(&format, data))) })
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn bytes_become_the_value_of_their_format() {
        let text: DataFormat = DataFormat::text().into();
        let value = WaylandDataTransferItem::convert_data(&text, "h\u{e9}".as_bytes().to_vec()).unwrap();
        assert_eq!(value.downcast_ref::<String>().unwrap(), "h\u{e9}");
        assert!(WaylandDataTransferItem::convert_data(&text, Vec::new()).is_none());

        let bytes: DataFormat = DataFormat::create_bytes_platform_format("application/x-thing").into();
        let value = WaylandDataTransferItem::convert_data(&bytes, vec![1, 2, 3]).unwrap();
        assert_eq!(&**value.downcast_ref::<Rc<[u8]>>().unwrap(), [1, 2, 3]);

        let string: DataFormat = DataFormat::create_string_platform_format("text/html").into();
        let value = WaylandDataTransferItem::convert_data(&string, b"<b>x</b>".to_vec()).unwrap();
        assert_eq!(value.downcast_ref::<String>().unwrap(), "<b>x</b>");

        // A format of the process has no value that could come through a pipe.
        let private: DataFormat = DataFormat::create_in_process_format::<u32>("private").into();
        assert!(WaylandDataTransferItem::convert_data(&private, vec![1, 2, 3]).is_none());
    }

    #[test]
    fn a_list_of_uris_is_a_list_of_files() {
        assert!(files_of(None).is_none());
        assert!(files_of(Some(Vec::new())).is_none());
        // Only what exists is an item: this test binary and its directory, not a third path.
        let binary = std::env::current_exe().unwrap();
        let directory = binary.parent().unwrap();
        let list = format!("# a comment\r\nfile://{}\r\nfile://{}\r\nfile:///nonexistent/ferroui-wayland-test\r\n", binary.display(), directory.display());
        let files = files_of(Some(list.into_bytes())).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name(), binary.file_name().unwrap().to_string_lossy());
    }
}
