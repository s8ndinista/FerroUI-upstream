//! Port of `VulkanDemo/ByteString.cs`: zero-terminated strings for the names Vulkan takes, and
//! a list of them as the array of pointers Vulkan takes.

use std::ffi::{c_char, CString};

/// A zero-terminated string of bytes (`Marshal.StringToHGlobalAnsi` upstream).
pub struct ByteString {
    value: CString,
}

impl ByteString {
    /// # Panics
    /// Panics when `s` contains a zero byte, which a name of Vulkan never does.
    pub fn new(s: &str) -> Self {
        Self { value: CString::new(s).unwrap_or_else(|e| panic!("{s:?}: {e}")) }
    }

    /// The string, valid while this value lives.
    pub fn pointer(&self) -> *const c_char {
        self.value.as_ptr()
    }
}

/// Strings and the array of their pointers.
pub struct ByteStringList {
    inner: Vec<ByteString>,
    ptr: Vec<*const c_char>,
}

impl ByteStringList {
    pub fn new<'a>(items: impl IntoIterator<Item = &'a String>) -> Self {
        let inner: Vec<ByteString> = items.into_iter().map(|x| ByteString::new(x)).collect();
        let ptr = inner.iter().map(ByteString::pointer).collect();
        Self { inner, ptr }
    }

    pub fn count(&self) -> usize {
        self.inner.len()
    }

    pub fn u_count(&self) -> u32 {
        self.inner.len() as u32
    }

    /// The array of the pointers, valid while this value lives.
    pub fn as_ptrs(&self) -> &[*const c_char] {
        &self.ptr
    }
}
