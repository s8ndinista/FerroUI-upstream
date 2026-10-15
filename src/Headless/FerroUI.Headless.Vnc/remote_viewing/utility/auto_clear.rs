//! Port of `Utility/AutoClear.cs` of RemoteViewing (BSD-2-Clause, Copyright
//! (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

/// Wraps around an array and clears the array if this object is disposed
/// (dropped).
pub(crate) struct AutoClear<'a> {
    array: Option<&'a mut [u8]>,
}

impl<'a> AutoClear<'a> {
    /// Initializes a new instance of the [`AutoClear`] struct: `array` is
    /// cleared when this object is disposed of.
    pub(crate) fn new(array: &'a mut [u8]) -> Self {
        Self { array: Some(array) }
    }

    /// The array, while it is not cleared.
    pub(crate) fn array(&self) -> &[u8] {
        self.array.as_deref().unwrap_or(&[])
    }

    /// `Dispose`: clears the array.
    pub(crate) fn dispose(&mut self) {
        let Some(array) = self.array.take() else {
            return;
        };

        array.fill(0);
    }
}

impl Drop for AutoClear<'_> {
    fn drop(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn the_array_is_cleared_when_the_scope_ends() {
        let mut secret = [1u8, 2, 3];
        {
            let clear = AutoClear::new(&mut secret);
            assert_eq!(&[1, 2, 3], clear.array());
        }
        assert_eq!([0, 0, 0], secret);
    }
}
