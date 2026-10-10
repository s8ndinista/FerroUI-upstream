//! The functions of the automation library of the system (`oleaut32.dll`)
//! the marshalling calls: safe arrays and strings with a length prefix.
//!
//! The reference imports four of them in `SafeArrayRef.cs` and reaches the
//! strings through its runtime. Here they are in one place, behind safe
//! names, so that the marshalling is the same code on every host: on
//! Windows the functions are the ones of the system; on any other host a
//! small stand-in keeps the same memory layout (a `SAFEARRAY` with one
//! bound and its data, a string after its length in bytes), which is what
//! the tests of the marshalling run against there. The stand-in is no part
//! of a build for Windows.

use super::safe_array_ref::{SAFEARRAY, SAFEARRAYBOUND};

#[cfg(windows)]
mod system {
    use super::{SAFEARRAY, SAFEARRAYBOUND};

    #[cfg_attr(not(target_arch = "x86"), link(name = "oleaut32", kind = "raw-dylib"))]
    #[cfg_attr(target_arch = "x86", link(name = "oleaut32", kind = "raw-dylib", import_name_type = "undecorated"))]
    unsafe extern "system" {
        pub fn SafeArrayCreate(vt: u16, c_dims: u32, rgsabound: *const SAFEARRAYBOUND) -> *mut SAFEARRAY;
        pub fn SafeArrayDestroy(array: *mut SAFEARRAY) -> i32;
        pub fn SafeArrayLock(array: *mut SAFEARRAY) -> i32;
        pub fn SafeArrayUnlock(array: *mut SAFEARRAY) -> i32;
        pub fn SysAllocStringLen(str_in: *const u16, ui: u32) -> *mut u16;
        pub fn SysFreeString(bstr: *mut u16);
        pub fn SysStringLen(bstr: *const u16) -> u32;
    }
}

/// The stand-in of the library on a host that is not Windows.
#[cfg(not(windows))]
#[allow(non_snake_case)]
mod system {
    use super::super::com_variant::VarEnum;
    use super::{SAFEARRAY, SAFEARRAYBOUND};
    use ferroui_microcom::IUnknown;
    use std::ffi::c_void;

    /// An array with what the stand-in has to remember of it: the type of
    /// its elements, which the system keeps before the descriptor too.
    #[repr(C)]
    struct Block {
        vt: u16,
        data: Vec<u64>,
        array: SAFEARRAY,
    }

    fn element_size(vt: u16) -> Option<u32> {
        Some(match VarEnum(vt) {
            VarEnum::VT_I1 | VarEnum::VT_UI1 => 1,
            VarEnum::VT_I2 | VarEnum::VT_UI2 | VarEnum::VT_BOOL => 2,
            VarEnum::VT_I4 | VarEnum::VT_UI4 | VarEnum::VT_R4 | VarEnum::VT_INT | VarEnum::VT_UINT => 4,
            VarEnum::VT_I8 | VarEnum::VT_UI8 | VarEnum::VT_R8 => 8,
            VarEnum::VT_BSTR | VarEnum::VT_UNKNOWN => std::mem::size_of::<usize>() as u32,
            _ => return None,
        })
    }

    unsafe fn block_of(array: *mut SAFEARRAY) -> *mut Block {
        // SAFETY (of the caller): the array is the field of a block.
        unsafe { array.byte_sub(std::mem::offset_of!(Block, array)).cast::<Block>() }
    }

    pub unsafe fn SafeArrayCreate(vt: u16, c_dims: u32, rgsabound: *const SAFEARRAYBOUND) -> *mut SAFEARRAY {
        let Some(size) = element_size(vt) else {
            return std::ptr::null_mut();
        };
        if c_dims != 1 || rgsabound.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY (of the caller): one bound is readable.
        let bound = unsafe { *rgsabound };
        let bytes = bound.c_elements as usize * size as usize;
        // Words of eight bytes: aligned for every element type, and zeroed
        // as the system zeroes a new array.
        let mut block = Box::new(Block {
            vt,
            data: vec![0u64; bytes.div_ceil(8)],
            array: SAFEARRAY {
                c_dims: 1,
                f_features: 0,
                cb_elements: size,
                c_locks: 0,
                pv_data: std::ptr::null_mut(),
                rgsabound: [bound],
            },
        });
        block.array.pv_data = block.data.as_mut_ptr().cast::<c_void>();
        let block = Box::into_raw(block);
        // SAFETY: the block that was just leaked.
        unsafe { &raw mut (*block).array }
    }

    pub unsafe fn SafeArrayDestroy(array: *mut SAFEARRAY) -> i32 {
        if array.is_null() {
            return 0;
        }
        // SAFETY (of the caller): an array of `SafeArrayCreate` above that
        // was not destroyed yet.
        let block = unsafe { Box::from_raw(block_of(array)) };
        let count = block.array.rgsabound[0].c_elements as usize;
        let pointers = block.array.pv_data.cast::<*mut c_void>();
        for index in 0..count {
            match VarEnum(block.vt) {
                // SAFETY: the elements are strings of `SysAllocStringLen`
                // or null, which the array owns.
                VarEnum::VT_BSTR => unsafe { SysFreeString((*pointers.add(index)).cast::<u16>()) },
                VarEnum::VT_UNKNOWN => {
                    // SAFETY: the elements are interface pointers the
                    // array holds a reference of, or null.
                    unsafe {
                        if let Some(unknown) = (*pointers.add(index)).cast::<IUnknown>().as_ref() {
                            unknown.release();
                        }
                    }
                }
                _ => {}
            }
        }
        0
    }

    pub unsafe fn SafeArrayLock(array: *mut SAFEARRAY) -> i32 {
        if array.is_null() {
            return 0x8007_0057u32 as i32;
        }
        // SAFETY (of the caller): a live array.
        unsafe { (*array).c_locks += 1 };
        0
    }

    pub unsafe fn SafeArrayUnlock(array: *mut SAFEARRAY) -> i32 {
        if array.is_null() {
            return 0x8007_0057u32 as i32;
        }
        // SAFETY (of the caller): a live array.
        unsafe { (*array).c_locks -= 1 };
        0
    }

    /// A string as the system lays it out: the length in bytes, the code
    /// units, a terminator. The pointer is the one of the first code unit.
    pub unsafe fn SysAllocStringLen(str_in: *const u16, ui: u32) -> *mut u16 {
        let length = ui as usize;
        // Words of four bytes hold the prefix and keep the units aligned.
        let mut words = vec![0u32; 1 + (length + 1).div_ceil(2)].into_boxed_slice();
        words[0] = ui * 2;
        let units = words.as_mut_ptr().wrapping_add(1).cast::<u16>();
        if !str_in.is_null() {
            // SAFETY (of the caller): `ui` code units are readable; the
            // block has room for them and the terminator.
            unsafe { std::ptr::copy_nonoverlapping(str_in, units, length) };
        }
        let _ = Box::into_raw(words);
        units
    }

    pub unsafe fn SysFreeString(bstr: *mut u16) {
        if bstr.is_null() {
            return;
        }
        // SAFETY (of the caller): a string of `SysAllocStringLen` above.
        unsafe {
            let length = SysStringLen(bstr) as usize;
            let words = bstr.cast::<u32>().sub(1);
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(words, 1 + (length + 1).div_ceil(2))));
        }
    }

    pub unsafe fn SysStringLen(bstr: *const u16) -> u32 {
        if bstr.is_null() {
            return 0;
        }
        // SAFETY (of the caller): the prefix lies before the units.
        unsafe { *bstr.cast::<u32>().sub(1) / 2 }
    }
}

/// `SafeArrayCreate`: an array of one dimension of `count` elements of the
/// type `vt`, from zero; null when the system cannot create it.
pub(crate) fn safe_array_create(vt: u16, count: u32) -> *mut SAFEARRAY {
    let bound = SAFEARRAYBOUND { c_elements: count, l_lbound: 0 };
    // SAFETY: one dimension with its one bound, which lives through the
    // call.
    unsafe { system::SafeArrayCreate(vt, 1, &bound) }
}

/// `SafeArrayDestroy`: frees an array with what its elements own.
///
/// # Safety
/// `array` is null or an array of the library that nothing uses after the
/// call.
pub(crate) unsafe fn safe_array_destroy(array: *mut SAFEARRAY) {
    // SAFETY: the contract of the function.
    unsafe {
        system::SafeArrayDestroy(array);
    }
}

/// `SafeArrayLock`: the result code of the system (zero for success).
///
/// # Safety
/// `array` is null or a live array of the library.
pub(crate) unsafe fn safe_array_lock(array: *mut SAFEARRAY) -> i32 {
    // SAFETY: the contract of the function.
    unsafe { system::SafeArrayLock(array) }
}

/// `SafeArrayUnlock`.
///
/// # Safety
/// `array` is a live array of the library that was locked.
pub(crate) unsafe fn safe_array_unlock(array: *mut SAFEARRAY) {
    // SAFETY: the contract of the function.
    unsafe {
        system::SafeArrayUnlock(array);
    }
}

/// A string of the system with the text of `value` (`Marshal.StringToBSTR`
/// of the reference). It is freed with [`free_bstr`], or by whoever it is
/// handed to.
pub(crate) fn string_to_bstr(value: &str) -> *mut u16 {
    let units: Vec<u16> = value.encode_utf16().collect();
    // SAFETY: the code units are readable for their count.
    unsafe { system::SysAllocStringLen(units.as_ptr(), units.len() as u32) }
}

/// The text of a string of the system (`Marshal.PtrToStringBSTR`); `None`
/// for null.
///
/// # Safety
/// `bstr` is null or a live string of the library.
pub(crate) unsafe fn ptr_to_string_bstr(bstr: *const u16) -> Option<String> {
    if bstr.is_null() {
        return None;
    }
    // SAFETY: the contract of the function: the string has the length the
    // library says it has.
    unsafe {
        let length = system::SysStringLen(bstr) as usize;
        Some(String::from_utf16_lossy(std::slice::from_raw_parts(bstr, length)))
    }
}

/// `SysFreeString` (`Marshal.FreeBSTR`).
///
/// # Safety
/// `bstr` is null or a string of the library that nothing uses after the
/// call.
pub(crate) unsafe fn free_bstr(bstr: *mut u16) {
    // SAFETY: the contract of the function.
    unsafe { system::SysFreeString(bstr) }
}
