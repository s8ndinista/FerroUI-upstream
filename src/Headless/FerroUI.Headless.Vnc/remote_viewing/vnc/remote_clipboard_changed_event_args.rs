//! Port of `Vnc/RemoteClipboardChangedEventArgs.cs` of RemoteViewing
//! (BSD-2-Clause, Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of
//! this crate).

/// Provides data for the remote clipboard changed event of the server
/// session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteClipboardChangedEventArgs {
    contents: String,
}

impl RemoteClipboardChangedEventArgs {
    /// Initializes a new instance of the
    /// [`RemoteClipboardChangedEventArgs`] class with the clipboard
    /// `contents`.
    pub fn new(contents: String) -> Self {
        Self { contents }
    }

    /// Gets the text contents of the clipboard.
    pub fn contents(&self) -> &str {
        &self.contents
    }
}
