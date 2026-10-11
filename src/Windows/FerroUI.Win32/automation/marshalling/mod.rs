//! The values that cross the boundary of UI Automation: the variant and
//! the safe array of the system, and the conversions between them and the
//! values of the port (the `Marshalling` directory of the reference, whose
//! two marshallers the generator of its runtime calls; here the node calls
//! them where a value crosses).

mod com_variant;
mod com_variant_marshaller;
mod ole_aut;
mod safe_array_marshaller;
mod safe_array_ref;

pub use com_variant::{ComVariant, VarEnum, VariantArray, VariantError, VariantValue};
pub use com_variant_marshaller::ComVariantMarshaller;
pub use safe_array_marshaller::SafeArrayMarshaller;
pub use safe_array_ref::{
    SafeArrayElement, SafeArrayLockError, SafeArrayRef, ADVANCED_FEATURE_FLAGS, SAFEARRAY, SAFEARRAYBOUND,
};

#[allow(unused_imports)] // The tests read and free the strings the nodes make.
pub(crate) use ole_aut::{free_bstr, ptr_to_string_bstr, string_to_bstr};

#[cfg(test)]
mod com_variant_tests;
#[cfg(test)]
mod safe_array_ref_tests;
