//! Not from the reference, which has no tests of the variant.

use super::com_variant::{ComVariant, VarEnum, VariantArray, VariantValue};
use super::ComVariantMarshaller;
use crate::automation::interop::{IToggleProvider, IToggleProviderImpl};
use ferroui_base::utilities::Decimal;
use ferroui_base::BoxedValue;
use ferroui_controls::automation::provider::ToggleState;
use ferroui_controls::automation::ExpandCollapseState;
use ferroui_microcom::{ComObject, HResult, IUnknown};
use std::ffi::c_void;
use std::rc::Rc;

fn round_trip(value: VariantValue, var_type: VarEnum) {
    let variant = ComVariantMarshaller::convert_to_unmanaged(Some(&value)).unwrap();
    assert_eq!(var_type, variant.var_type(), "{value:?}");
    // SAFETY: the variant of the line above.
    let back = unsafe { ComVariantMarshaller::convert_to_managed(&variant) }.unwrap();
    assert_eq!(Some(value), back);
    // SAFETY: the variant owns what it carries and is used no more.
    unsafe { ComVariantMarshaller::free(variant) };
}

#[test]
fn a_variant_has_the_size_of_the_system() {
    // Variant size is the size of 4 pointers (16 bytes) on a 32-bit processor,
    // and 3 pointers (24 bytes) on a 64-bit processor.
    let pointer = std::mem::size_of::<usize>();
    let expected = if pointer == 4 { 4 * pointer } else { 3 * pointer };
    assert_eq!(expected, std::mem::size_of::<ComVariant>());
}

#[test]
fn no_value_is_the_null_variant_and_an_empty_variant_is_no_value() {
    let variant = ComVariant::create(None).unwrap();
    assert_eq!(VarEnum::VT_NULL, variant.var_type());
    // SAFETY: variants without anything to own.
    unsafe {
        assert_eq!(None, variant.as_object().unwrap());
        assert_eq!(VarEnum::VT_EMPTY, ComVariant::default().var_type());
        assert_eq!(None, ComVariant::default().as_object().unwrap());
    }
}

#[test]
fn the_values_round_trip_with_the_types_of_the_system() {
    round_trip(VariantValue::I2(-2), VarEnum::VT_I2);
    round_trip(VariantValue::I4(i32::MIN), VarEnum::VT_I4);
    round_trip(VariantValue::R4(1.5), VarEnum::VT_R4);
    round_trip(VariantValue::R8(-0.125), VarEnum::VT_R8);
    round_trip(VariantValue::Date(45000.5), VarEnum::VT_DATE);
    round_trip(VariantValue::Bstr("a text of several words".to_owned()), VarEnum::VT_BSTR);
    round_trip(VariantValue::Bstr(String::new()), VarEnum::VT_BSTR);
    round_trip(VariantValue::Bool(true), VarEnum::VT_BOOL);
    round_trip(VariantValue::Bool(false), VarEnum::VT_BOOL);
    round_trip(VariantValue::I1(-1), VarEnum::VT_I1);
    round_trip(VariantValue::UI1(255), VarEnum::VT_UI1);
    round_trip(VariantValue::UI2(65535), VarEnum::VT_UI2);
    round_trip(VariantValue::UI4(u32::MAX), VarEnum::VT_UI4);
    round_trip(VariantValue::I8(i64::MIN), VarEnum::VT_I8);
    round_trip(VariantValue::UI8(u64::MAX), VarEnum::VT_UI8);
}

#[test]
fn a_boolean_is_the_boolean_of_the_system() {
    let variant = ComVariant::create(Some(&VariantValue::Bool(true))).unwrap();
    // SAFETY: a variant is plain bytes; the value follows the eight bytes
    // of the type and the reserved words.
    let bytes = unsafe { std::slice::from_raw_parts((&raw const variant).cast::<u8>(), 16) };
    assert_eq!([11, 0], bytes[0..2]);
    assert_eq!([0xff, 0xff], bytes[8..10]);
}

#[test]
fn a_decimal_overlaps_the_variant_and_keeps_its_type() {
    for text in ["0", "1", "-1.5", "79228162514264337593543950335", "-0.0000000000000000000000000001", "12345.678"] {
        let value = Decimal::parse(text).unwrap();
        round_trip(VariantValue::Decimal(value), VarEnum::VT_DECIMAL);
    }

    // The layout of `DECIMAL` of the system: the scale, the sign, the high
    // 32 bits and the low 64.
    let variant = ComVariant::create(Some(&VariantValue::Decimal(Decimal::parse("-1.5").unwrap()))).unwrap();
    // SAFETY: a variant is plain bytes.
    let bytes = unsafe { std::slice::from_raw_parts((&raw const variant).cast::<u8>(), 16) };
    assert_eq!([14, 0], bytes[0..2]);
    assert_eq!(1, bytes[2]);
    assert_eq!(0x80, bytes[3]);
    assert_eq!([0, 0, 0, 0], bytes[4..8]);
    assert_eq!(15u64.to_le_bytes(), bytes[8..16]);
}

#[test]
fn arrays_round_trip_with_the_array_flag() {
    let array = |var_type: VarEnum| VarEnum(var_type.0 | VarEnum::VT_ARRAY.0);
    round_trip(VariantValue::Array(VariantArray::I4(vec![3, 42])), array(VarEnum::VT_I4));
    round_trip(VariantValue::Array(VariantArray::R8(vec![1.5, 2.5])), array(VarEnum::VT_R8));
    round_trip(VariantValue::Array(VariantArray::Bstr(vec!["a".to_owned(), "b".to_owned()])), array(VarEnum::VT_BSTR));
    round_trip(VariantValue::Array(VariantArray::Bool(vec![true, false])), array(VarEnum::VT_BOOL));
    round_trip(VariantValue::Array(VariantArray::UI1(vec![1, 2, 3])), array(VarEnum::VT_UI1));
    round_trip(VariantValue::Array(VariantArray::I2(vec![-1])), array(VarEnum::VT_I2));
    round_trip(VariantValue::Array(VariantArray::I8(vec![-1])), array(VarEnum::VT_I8));
    round_trip(VariantValue::Array(VariantArray::UI2(vec![1])), array(VarEnum::VT_UI2));
    round_trip(VariantValue::Array(VariantArray::UI4(vec![1])), array(VarEnum::VT_UI4));
    round_trip(VariantValue::Array(VariantArray::UI8(vec![1])), array(VarEnum::VT_UI8));
    round_trip(VariantValue::Array(VariantArray::R4(vec![1.0])), array(VarEnum::VT_R4));
    round_trip(VariantValue::Array(VariantArray::I1(vec![-1])), array(VarEnum::VT_I1));
}

struct Provider;

impl IToggleProviderImpl for Provider {
    fn toggle(&self) -> Result<(), HResult> {
        Ok(())
    }

    fn get_toggle_state(&self) -> Result<i32, HResult> {
        Ok(7)
    }
}

#[test]
fn an_object_is_held_by_the_variant_until_it_is_disposed() {
    let provider = IToggleProvider::from_impl(Provider);
    let unknown = provider.cast::<IUnknown>().unwrap();
    let raw = unknown.as_ptr() as *mut c_void;
    // SAFETY: the object of this test.
    let count = || unsafe { ComObject::<Provider>::ref_count(raw) };
    assert_eq!(2, count());

    let variant = ComVariant::create(Some(&VariantValue::Unknown(unknown.clone()))).unwrap();
    assert_eq!(VarEnum::VT_UNKNOWN, variant.var_type());
    assert_eq!(3, count());

    // SAFETY: the variant of this test.
    let back = unsafe { variant.as_object() }.unwrap();
    assert_eq!(Some(VariantValue::Unknown(unknown.clone())), back);
    drop(back);
    assert_eq!(3, count());

    let mut variant = variant;
    // SAFETY: the variant owns its reference.
    unsafe { variant.dispose() };
    assert_eq!(VarEnum::VT_EMPTY, variant.var_type());
    assert_eq!(2, count());

    // An array of objects holds a reference of each.
    let array = ComVariant::create(Some(&VariantValue::Array(VariantArray::Unknown(vec![unknown.clone()])))).unwrap();
    assert_eq!(VarEnum(VarEnum::VT_UNKNOWN.0 | VarEnum::VT_ARRAY.0), array.var_type());
    assert_eq!(3, count());
    // SAFETY: the variant owns its array.
    unsafe { ComVariantMarshaller::free(array) };
    assert_eq!(2, count());
}

#[test]
fn the_untyped_values_of_the_framework_that_convert() {
    let boxed = |value: BoxedValue| VariantValue::from_any(&*value);
    assert_eq!(Some(VariantValue::R8(0.5)), boxed(Rc::new(0.5f64)));
    assert_eq!(Some(VariantValue::Bool(true)), boxed(Rc::new(true)));
    assert_eq!(Some(VariantValue::Bstr("text".to_owned())), boxed(Rc::new("text".to_owned())));
    assert_eq!(Some(VariantValue::I4(1)), boxed(Rc::new(ToggleState::On)));
    assert_eq!(Some(VariantValue::I4(2)), boxed(Rc::new(ToggleState::Indeterminate)));
    assert_eq!(Some(VariantValue::Decimal(Decimal::ONE)), boxed(Rc::new(Decimal::ONE)));
    // The node converts the states of expanding itself: their numbers are
    // not the ones of the system.
    assert_eq!(None, boxed(Rc::new(ExpandCollapseState::LeafNode)));
    assert_eq!(None, boxed(Rc::new(vec![1i32])));
}
