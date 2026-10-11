use std::ffi::{c_char, CString};

/// The strings of a list as an array of pointers to zero-terminated UTF-8
/// strings, the form in which Vulkan takes the names of layers and
/// extensions. The array is valid for as long as the value lives.
pub struct Utf8BufferArray {
    buffers: Vec<CString>,
    buffer_array: Vec<*const c_char>,
}

impl Utf8BufferArray {
    /// Copies the strings. A string is cut at its first zero character,
    /// which is where a reader of the zero-terminated copy stops.
    pub fn new<S: AsRef<str>>(strings: impl IntoIterator<Item = S>) -> Self {
        let buffers: Vec<CString> = strings
            .into_iter()
            .map(|s| {
                let s = s.as_ref();
                let end = s.find('\0').unwrap_or(s.len());
                CString::new(&s[..end]).expect("the string was cut at its first zero character")
            })
            .collect();
        // The pointers are into the heap blocks of the strings, which do not
        // move when the vector that holds the strings does.
        let buffer_array = buffers.iter().map(|buffer| buffer.as_ptr()).collect();
        Self { buffers, buffer_array }
    }

    /// The array of pointers.
    pub fn as_slice(&self) -> &[*const c_char] {
        &self.buffer_array
    }

    /// The number of strings.
    pub fn count(&self) -> i32 {
        self.buffers.len() as i32
    }

    /// The number of strings, as Vulkan counts.
    pub fn u_count(&self) -> u32 {
        self.count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn the_array_points_at_zero_terminated_copies() {
        let array = Utf8BufferArray::new(["VK_KHR_surface", "VK_KHR_xlib_surface", "zażółć"]);
        assert_eq!(array.count(), 3);
        assert_eq!(array.u_count(), 3);
        let read: Vec<String> = array
            .as_slice()
            .iter()
            // SAFETY: each pointer is to a zero-terminated string the array owns.
            .map(|pointer| unsafe { CStr::from_ptr(*pointer) }.to_str().unwrap().to_string())
            .collect();
        assert_eq!(read, ["VK_KHR_surface", "VK_KHR_xlib_surface", "zażółć"]);
    }

    #[test]
    fn an_empty_list_is_an_empty_array() {
        let array = Utf8BufferArray::new(Vec::<String>::new());
        assert_eq!(array.count(), 0);
        assert!(array.as_slice().is_empty());
    }

    #[test]
    fn a_string_ends_at_its_first_zero_character() {
        let array = Utf8BufferArray::new(["ab\0cd"]);
        // SAFETY: as above.
        let read = unsafe { CStr::from_ptr(array.as_slice()[0]) };
        assert_eq!(read.to_str().unwrap(), "ab");
    }
}
