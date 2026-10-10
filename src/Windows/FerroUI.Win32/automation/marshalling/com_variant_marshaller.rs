//! The conversions of a variant at the boundary (the port of
//! `Marshalling/ComVariantMarshaller.cs`).

use super::com_variant::{ComVariant, VariantError, VariantValue};

/// The conversions between a value and a variant.
pub struct ComVariantMarshaller;

impl ComVariantMarshaller {
    /// A variant with the value, which whoever takes it frees.
    pub fn convert_to_unmanaged(managed: Option<&VariantValue>) -> Result<ComVariant, VariantError> {
        ComVariant::create(managed)
    }

    /// The value of a variant.
    ///
    /// # Safety
    /// As [`ComVariant::as_object`].
    pub unsafe fn convert_to_managed(unmanaged: &ComVariant) -> Result<Option<VariantValue>, VariantError> {
        // SAFETY: the contract of the function.
        unsafe { unmanaged.as_object() }
    }

    /// Releases what a variant owns.
    ///
    /// # Safety
    /// As [`ComVariant::dispose`].
    pub unsafe fn free(mut unmanaged: ComVariant) {
        // SAFETY: the contract of the function.
        unsafe { unmanaged.dispose() }
    }
}
