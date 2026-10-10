//! `VARIANT` of the system (the port of `Marshalling/ComVariant.cs`, which
//! is a simplified copy of the type of the reference's runtime).
//!
//! The reference converts between a variant and an object of any type by
//! testing the type at run time. A value that crosses the boundary here is
//! a [`VariantValue`]: one case per type the reference converts.

use super::ole_aut;
use super::safe_array_ref::{SafeArrayElement, SafeArrayLockError, SafeArrayRef};
use ferroui_base::utilities::Decimal;
use ferroui_base::AnyValue;
use ferroui_controls::automation::provider::ToggleState;
use ferroui_controls::automation::AutomationLiveSetting;
use ferroui_microcom::{ComPtr, IUnknown};
use std::ffi::c_void;

/// The type of the value of a variant (`VARENUM` of the system: the members
/// the marshalling names).
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct VarEnum(pub u16);

#[allow(missing_docs)]
impl VarEnum {
    pub const VT_EMPTY: VarEnum = VarEnum(0);
    pub const VT_NULL: VarEnum = VarEnum(1);
    pub const VT_I2: VarEnum = VarEnum(2);
    pub const VT_I4: VarEnum = VarEnum(3);
    pub const VT_R4: VarEnum = VarEnum(4);
    pub const VT_R8: VarEnum = VarEnum(5);
    pub const VT_CY: VarEnum = VarEnum(6);
    pub const VT_DATE: VarEnum = VarEnum(7);
    pub const VT_BSTR: VarEnum = VarEnum(8);
    pub const VT_DISPATCH: VarEnum = VarEnum(9);
    pub const VT_ERROR: VarEnum = VarEnum(10);
    pub const VT_BOOL: VarEnum = VarEnum(11);
    pub const VT_VARIANT: VarEnum = VarEnum(12);
    pub const VT_UNKNOWN: VarEnum = VarEnum(13);
    pub const VT_DECIMAL: VarEnum = VarEnum(14);
    pub const VT_I1: VarEnum = VarEnum(16);
    pub const VT_UI1: VarEnum = VarEnum(17);
    pub const VT_UI2: VarEnum = VarEnum(18);
    pub const VT_UI4: VarEnum = VarEnum(19);
    pub const VT_I8: VarEnum = VarEnum(20);
    pub const VT_UI8: VarEnum = VarEnum(21);
    pub const VT_INT: VarEnum = VarEnum(22);
    pub const VT_UINT: VarEnum = VarEnum(23);
    pub const VT_LPSTR: VarEnum = VarEnum(30);
    pub const VT_LPWSTR: VarEnum = VarEnum(31);
    pub const VT_STREAM: VarEnum = VarEnum(66);
    pub const VT_STORAGE: VarEnum = VarEnum(67);
    pub const VT_STREAMED_OBJECT: VarEnum = VarEnum(68);
    pub const VT_STORED_OBJECT: VarEnum = VarEnum(69);
    pub const VT_CLSID: VarEnum = VarEnum(72);
    pub const VT_ARRAY: VarEnum = VarEnum(0x2000);

    /// `HasFlag`.
    pub fn has_flag(self, flag: VarEnum) -> bool {
        self.0 & flag.0 == flag.0
    }
}

/// The values of an array a variant carries.
#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum VariantArray {
    I1(Vec<i8>),
    I2(Vec<i16>),
    I4(Vec<i32>),
    I8(Vec<i64>),
    UI1(Vec<u8>),
    UI2(Vec<u16>),
    UI4(Vec<u32>),
    UI8(Vec<u64>),
    R4(Vec<f32>),
    R8(Vec<f64>),
    Bool(Vec<bool>),
    Bstr(Vec<String>),
    Unknown(Vec<ComPtr<IUnknown>>),
}

/// A value the marshalling carries in a variant: what the reference passes
/// as an object.
#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum VariantValue {
    I2(i16),
    I4(i32),
    R4(f32),
    R8(f64),
    Decimal(Decimal),
    /// A date of OLE automation: days since 30 December 1899. The
    /// reference converts it to and from the date type of its runtime.
    Date(f64),
    Bstr(String),
    Bool(bool),
    I1(i8),
    UI1(u8),
    UI2(u16),
    UI4(u32),
    I8(i64),
    UI8(u64),
    Array(VariantArray),
    Unknown(ComPtr<IUnknown>),
}

impl VariantValue {
    /// The value of an untyped value of the framework, for the types the
    /// reference converts (`IConvertible` and the enumerations: an
    /// enumeration is its number). `None` for any other type.
    pub fn from_any(value: &dyn AnyValue) -> Option<VariantValue> {
        macro_rules! cases {
            ($($type:ty => $case:ident),* $(,)?) => {
                $(
                    if let Some(value) = value.downcast_ref::<$type>() {
                        return Some(VariantValue::$case(value.clone()));
                    }
                )*
            };
        }
        cases! {
            i16 => I2, i32 => I4, f32 => R4, f64 => R8, Decimal => Decimal, String => Bstr, bool => Bool,
            i8 => I1, u8 => UI1, u16 => UI2, u32 => UI4, i64 => I8, u64 => UI8,
        }
        if let Some(value) = value.downcast_ref::<&'static str>() {
            return Some(VariantValue::Bstr((*value).to_owned()));
        }
        // The enumerations of the automation contracts that are values of
        // properties.
        if let Some(value) = value.downcast_ref::<ToggleState>() {
            return Some(VariantValue::I4(*value as i32));
        }
        if let Some(value) = value.downcast_ref::<AutomationLiveSetting>() {
            return Some(VariantValue::I4(*value as i32));
        }
        None
    }
}

/// The two pointers of a record, the largest member of the union of a
/// variant.
#[repr(C)]
#[derive(Clone, Copy)]
struct Record {
    record: *mut c_void,
    record_info: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
union UnionTypes {
    i1: i8,
    i2: i16,
    i4: i32,
    i8_: i64,
    ui1: u8,
    ui2: u16,
    ui4: u32,
    ui8: u64,
    bool_: i16,
    r4: f32,
    r8: f64,
    date: f64,
    bstr: *mut u16,
    unknown: *mut c_void,
    byref: *mut c_void,
    record: Record,
    parray: SafeArrayRef,
}

/// A variant with the layout of the system: sixteen bytes on a 32-bit
/// target, twenty-four on a 64-bit one. A decimal overlaps all of it but
/// the type: its scale and sign are the first reserved word, the high part
/// of its number the other two, the low part the first eight bytes of the
/// union.
///
/// Nothing is freed when a variant goes away: [`ComVariant::dispose`]
/// releases what it owns, as in the reference (a variant is copied into the
/// memory of a caller, which then owns it).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ComVariant {
    vt: u16,
    w_reserved1: u16,
    w_reserved2: u16,
    w_reserved3: u16,
    union_types: UnionTypes,
}

/// A value the marshalling does not convert (the `ArgumentException` of the
/// reference).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantError(pub String);

impl std::fmt::Display for VariantError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for VariantError {}

impl From<SafeArrayLockError> for VariantError {
    fn from(error: SafeArrayLockError) -> Self {
        VariantError(error.to_string())
    }
}

impl Default for ComVariant {
    /// The empty variant (`VT_EMPTY`), all zero.
    fn default() -> Self {
        ComVariant {
            vt: 0,
            w_reserved1: 0,
            w_reserved2: 0,
            w_reserved3: 0,
            union_types: UnionTypes { record: Record { record: std::ptr::null_mut(), record_info: std::ptr::null_mut() } },
        }
    }
}

impl std::fmt::Debug for ComVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ComVariant({:?})", self.var_type())
    }
}

impl ComVariant {
    // VARIANT_BOOL constants.
    pub(crate) const VARIANT_TRUE: i16 = -1;
    pub(crate) const VARIANT_FALSE: i16 = 0;

    fn of(var_type: VarEnum, union_types: UnionTypes) -> ComVariant {
        let mut variant = ComVariant::default();
        variant.union_types = union_types;
        variant.vt = var_type.0;
        variant
    }

    /// `Dispose`: releases what the variant owns and makes it empty.
    ///
    /// # Safety
    /// The variant owns what it carries (a string, an array, a reference
    /// of an object), and no copy of it is used afterwards.
    pub unsafe fn dispose(&mut self) {
        let var_type = self.var_type();
        // SAFETY: the member of the union the type names is the one that
        // was written; the contract of the function for the rest.
        unsafe {
            if var_type == VarEnum::VT_BSTR {
                ole_aut::free_bstr(self.union_types.bstr);
            } else if var_type.has_flag(VarEnum::VT_ARRAY) {
                self.union_types.parray.destroy();
            } else if matches!(
                var_type,
                VarEnum::VT_UNKNOWN
                    | VarEnum::VT_DISPATCH
                    | VarEnum::VT_STREAM
                    | VarEnum::VT_STREAMED_OBJECT
                    | VarEnum::VT_STORAGE
                    | VarEnum::VT_STORED_OBJECT
            ) {
                if let Some(unknown) = self.union_types.unknown.cast::<IUnknown>().as_ref() {
                    unknown.release();
                }
            }
            // The reference also frees the strings and the class
            // identifiers of a property variant (`VT_LPSTR`, `VT_LPWSTR`,
            // `VT_CLSID`) with the task allocator; no variant of UI
            // Automation carries one, and none is created here.
        }

        // Clear out this ComVariant instance.
        *self = ComVariant::default();
    }

    /// `Create`: a variant with `value`; the null variant for `None`.
    pub fn create(value: Option<&VariantValue>) -> Result<ComVariant, VariantError> {
        let Some(value) = value else {
            return Ok(Self::null());
        };

        Ok(match value {
            VariantValue::I2(value) => Self::of(VarEnum::VT_I2, UnionTypes { i2: *value }),
            VariantValue::I4(value) => Self::of(VarEnum::VT_I4, UnionTypes { i4: *value }),
            VariantValue::R4(value) => Self::of(VarEnum::VT_R4, UnionTypes { r4: *value }),
            VariantValue::R8(value) => Self::of(VarEnum::VT_R8, UnionTypes { r8: *value }),
            VariantValue::Decimal(value) => {
                // The decimal storage overlaps the whole variant but the
                // type. The value of a numeric box is a decimal, so its
                // property changed events arrive here.
                let mantissa = value.mantissa();
                let mut variant = Self::of(VarEnum::VT_DECIMAL, UnionTypes { ui8: mantissa as u64 });
                let high = (mantissa >> 64) as u32;
                let sign: u16 = if value.is_sign_negative() { 0x80 } else { 0 };
                variant.w_reserved1 = u16::from(value.scale()) | sign << 8;
                variant.w_reserved2 = high as u16;
                variant.w_reserved3 = (high >> 16) as u16;
                variant
            }
            VariantValue::Date(value) => Self::of(VarEnum::VT_DATE, UnionTypes { date: *value }),
            VariantValue::Bstr(value) => Self::of(VarEnum::VT_BSTR, UnionTypes { bstr: ole_aut::string_to_bstr(value) }),
            // bool values in OLE VARIANTs are VARIANT_BOOL values.
            VariantValue::Bool(value) => Self::of(
                VarEnum::VT_BOOL,
                UnionTypes { bool_: if *value { Self::VARIANT_TRUE } else { Self::VARIANT_FALSE } },
            ),
            VariantValue::I1(value) => Self::of(VarEnum::VT_I1, UnionTypes { i1: *value }),
            VariantValue::UI1(value) => Self::of(VarEnum::VT_UI1, UnionTypes { ui1: *value }),
            VariantValue::UI2(value) => Self::of(VarEnum::VT_UI2, UnionTypes { ui2: *value }),
            VariantValue::UI4(value) => Self::of(VarEnum::VT_UI4, UnionTypes { ui4: *value }),
            VariantValue::I8(value) => Self::of(VarEnum::VT_I8, UnionTypes { i8_: *value }),
            VariantValue::UI8(value) => Self::of(VarEnum::VT_UI8, UnionTypes { ui8: *value }),
            VariantValue::Array(array) => {
                fn array_of<T: SafeArrayElement>(values: &[T]) -> Result<ComVariant, VariantError> {
                    let (array, var_enum) =
                        SafeArrayRef::try_create(Some(values))?.expect("an array is created for values that are given");
                    Ok(ComVariant::of(VarEnum(var_enum.0 | VarEnum::VT_ARRAY.0), UnionTypes { parray: array }))
                }
                match array {
                    VariantArray::I1(values) => array_of(values)?,
                    VariantArray::I2(values) => array_of(values)?,
                    VariantArray::I4(values) => array_of(values)?,
                    VariantArray::I8(values) => array_of(values)?,
                    VariantArray::UI1(values) => array_of(values)?,
                    VariantArray::UI2(values) => array_of(values)?,
                    VariantArray::UI4(values) => array_of(values)?,
                    VariantArray::UI8(values) => array_of(values)?,
                    VariantArray::R4(values) => array_of(values)?,
                    VariantArray::R8(values) => array_of(values)?,
                    VariantArray::Bool(values) => array_of(values)?,
                    VariantArray::Bstr(values) => array_of(values)?,
                    VariantArray::Unknown(values) => array_of(values)?,
                }
            }
            // The variant holds a reference of the object.
            VariantValue::Unknown(unknown) => {
                Self::of(VarEnum::VT_UNKNOWN, UnionTypes { unknown: unknown.clone().into_raw().cast::<c_void>() })
            }
        })
    }

    /// A variant that represents a null value with the type `VT_NULL`.
    pub fn null() -> ComVariant {
        Self::of(VarEnum::VT_NULL, UnionTypes { ui8: 0 })
    }

    /// `AsObject`: the value of the variant; `None` for an empty or a null
    /// one. The variant keeps what it owns.
    ///
    /// # Safety
    /// The variant is one of the system or of [`ComVariant::create`]: what
    /// its type names is what it carries, alive.
    pub unsafe fn as_object(&self) -> Result<Option<VariantValue>, VariantError> {
        let var_type = self.var_type();
        let u = &self.union_types;
        // SAFETY: the member of the union the type names is the one that
        // was written; the contract of the function for the pointers.
        unsafe {
            Ok(Some(match var_type {
                VarEnum::VT_EMPTY | VarEnum::VT_NULL => return Ok(None),
                // integer
                VarEnum::VT_I1 => VariantValue::I1(u.i1),
                VarEnum::VT_I2 => VariantValue::I2(u.i2),
                VarEnum::VT_I4 | VarEnum::VT_INT | VarEnum::VT_ERROR => VariantValue::I4(u.i4),
                VarEnum::VT_I8 => VariantValue::I8(u.i8_),
                // unsigned integer
                VarEnum::VT_UI1 => VariantValue::UI1(u.ui1),
                VarEnum::VT_UI2 => VariantValue::UI2(u.ui2),
                VarEnum::VT_UI4 | VarEnum::VT_UINT => VariantValue::UI4(u.ui4),
                VarEnum::VT_UI8 => VariantValue::UI8(u.ui8),
                // floating
                VarEnum::VT_R4 => VariantValue::R4(u.r4),
                VarEnum::VT_R8 => VariantValue::R8(u.r8),
                // decimal
                VarEnum::VT_DECIMAL => VariantValue::Decimal(self.get_decimal()),
                // date
                VarEnum::VT_DATE => VariantValue::Date(u.date),
                // string
                VarEnum::VT_BSTR => match ole_aut::ptr_to_string_bstr(u.bstr) {
                    Some(text) => VariantValue::Bstr(text),
                    None => return Ok(None),
                },
                // bool
                VarEnum::VT_BOOL => VariantValue::Bool(u.bool_ != Self::VARIANT_FALSE),
                // unknown
                VarEnum::VT_UNKNOWN => match ComPtr::from_raw_add_ref(u.unknown.cast::<IUnknown>()) {
                    Some(unknown) => VariantValue::Unknown(unknown),
                    None => return Ok(None),
                },
                // array
                var_enum if var_enum.has_flag(VarEnum::VT_ARRAY) => {
                    fn values<T: SafeArrayElement>(array: SafeArrayRef) -> Result<Vec<T>, VariantError> {
                        // SAFETY: the array of a variant whose type names
                        // the type of its elements.
                        Ok(unsafe { SafeArrayRef::to_array::<T>(Some(array)) }?.unwrap_or_default())
                    }
                    let array = u.parray;
                    VariantValue::Array(match VarEnum(var_enum.0 ^ VarEnum::VT_ARRAY.0) {
                        // integer
                        VarEnum::VT_I1 => VariantArray::I1(values(array)?),
                        VarEnum::VT_I2 => VariantArray::I2(values(array)?),
                        VarEnum::VT_I4 | VarEnum::VT_INT => VariantArray::I4(values(array)?),
                        VarEnum::VT_I8 => VariantArray::I8(values(array)?),
                        // unsigned integer
                        VarEnum::VT_UI1 => VariantArray::UI1(values(array)?),
                        VarEnum::VT_UI2 => VariantArray::UI2(values(array)?),
                        VarEnum::VT_UI4 | VarEnum::VT_UINT => VariantArray::UI4(values(array)?),
                        VarEnum::VT_UI8 => VariantArray::UI8(values(array)?),
                        // floating
                        VarEnum::VT_R4 => VariantArray::R4(values(array)?),
                        VarEnum::VT_R8 => VariantArray::R8(values(array)?),
                        // string
                        VarEnum::VT_BSTR => VariantArray::Bstr(values(array)?),
                        // The reference has no case for an array of
                        // `VARIANT_BOOL`, which it creates.
                        VarEnum::VT_BOOL => VariantArray::Bool(values(array)?),
                        // variant
                        VarEnum::VT_UNKNOWN => VariantArray::Unknown(values(array)?),
                        _ => return Err(VariantError(format!("Unknown variant type: {var_enum:?}"))),
                    })
                }
                _ => return Err(VariantError(format!("Unknown variant type: {var_type:?}"))),
            }))
        }
    }

    /// The decimal stored in a `VT_DECIMAL` variant.
    fn get_decimal(&self) -> Decimal {
        // SAFETY: every bit pattern is a number.
        let low = unsafe { self.union_types.ui8 };
        let high = u32::from(self.w_reserved2) | u32::from(self.w_reserved3) << 16;
        let scale = (self.w_reserved1 & 0xff) as u8;
        let negative = self.w_reserved1 & 0x8000 != 0;
        Decimal::from_parts(u128::from(high) << 64 | u128::from(low), scale, negative)
    }

    /// The type of the data stored in this variant.
    pub fn var_type(&self) -> VarEnum {
        VarEnum(self.vt)
    }
}
