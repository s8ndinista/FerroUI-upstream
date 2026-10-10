//! The conversions of a safe array at the boundary (the port of
//! `Marshalling/SafeArrayMarshaller.cs`).

use super::safe_array_ref::{SafeArrayElement, SafeArrayLockError, SafeArrayRef};
use std::marker::PhantomData;

/// The conversions between the values of an array and a safe array of the
/// element type `T`.
pub struct SafeArrayMarshaller<T>(PhantomData<T>);

impl<T: SafeArrayElement> SafeArrayMarshaller<T> {
    /// A safe array with the values, which whoever takes it destroys; the
    /// reference to no array for `None`.
    ///
    /// The reference fails here for an element type it does not convert;
    /// such a type does not implement [`SafeArrayElement`].
    pub fn convert_to_unmanaged(managed: Option<&[T]>) -> Result<SafeArrayRef, SafeArrayLockError> {
        Ok(match SafeArrayRef::try_create(managed)? {
            Some((array, _)) => array,
            None => SafeArrayRef::null(),
        })
    }

    /// The values of a safe array.
    ///
    /// # Safety
    /// As [`SafeArrayRef::to_array`].
    pub unsafe fn convert_to_managed(unmanaged: SafeArrayRef) -> Result<Option<Vec<T>>, SafeArrayLockError> {
        // SAFETY: the contract of the function.
        unsafe { SafeArrayRef::to_array::<T>(Some(unmanaged)) }
    }

    /// Destroys a safe array.
    ///
    /// # Safety
    /// As [`SafeArrayRef::destroy`].
    pub unsafe fn free(unmanaged: SafeArrayRef) {
        // SAFETY: the contract of the function.
        unsafe { unmanaged.destroy() }
    }
}
