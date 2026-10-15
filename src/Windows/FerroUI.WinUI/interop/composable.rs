//! Classes of the integration that derive from a class of WinUI.
//!
//! The reference derives in C# (`class FerroSwapChainPanel : SwapChainPanel`,
//! a peer that derives from `FrameworkElementAutomationPeer`), which its
//! runtime turns into composition, the way the Windows Runtime derives from
//! a class: the object of the derived class is the outer object, the
//! factory of the base class creates the base object as its inner object,
//! and an overridable method of the base class is called through an
//! interface of the outer object (`IUIElementOverrides`,
//! `IAutomationPeerOverrides`), which hands the methods it does not override
//! to the inner object's.
//!
//! Here an outer object is a COM object of the MicroCom runtime made with
//! the interfaces it overrides; [`ferroui_microcom::set_inner`] makes it ask
//! the inner object for every other interface and keeps the inner object
//! alive. Its value keeps the pointer of the inner object, uncounted (the
//! inner object lives as long as the outer object), to call the base
//! implementation of a method and the protected members of the base class.

use super::IInspectable;
use ferroui_microcom::{make_com_with, set_inner, ComObject, ComPtr, HResult, ImplementedBy, Interface, InterfaceEntry, E_NOINTERFACE};
use std::cell::Cell;
use std::ffi::c_void;

/// The inner object of an outer object: the base object of the class it
/// derives from.
pub(crate) struct ComposedBase {
    inner: Cell<*mut c_void>,
}

impl Default for ComposedBase {
    fn default() -> Self {
        ComposedBase { inner: Cell::new(std::ptr::null_mut()) }
    }
}

impl ComposedBase {
    /// An interface of the base object: of the inner object itself, so that
    /// a method of an overridable interface is the implementation of the base
    /// class and not the outer object's override. The reference counts of
    /// the interface are the outer object's.
    pub(crate) fn base<U: Interface>(&self) -> Result<ComPtr<U>, HResult> {
        let inner = self.inner.get();
        if inner.is_null() {
            return Err(HResult(E_NOINTERFACE));
        }
        let mut out = std::ptr::null_mut();
        // SAFETY: `inner` is the non-delegating `IUnknown` of the inner
        // object, which lives as long as the outer object whose value holds
        // this; `out` is a place of this frame.
        let result = unsafe { (*(inner as *const ferroui_microcom::IUnknown)).query_interface_raw(&U::IID, &mut out) };
        HResult::check(result)?;
        // SAFETY: the query succeeded: `out` is the interface, with a
        // reference this function owns.
        unsafe { ComPtr::from_raw(out.cast::<U>()) }.ok_or(HResult(E_NOINTERFACE))
    }
}

/// The value of an outer object.
pub(crate) trait Composed {
    fn composed_base(&self) -> &ComposedBase;
}

/// Makes an outer object with `value` and the interfaces `I` and `more`,
/// and has `create` create the base object for it: `create` is the
/// composable factory of the base class, given the outer object and the
/// place of the inner object. The object of the class (its default
/// interface, which the factory returns) is returned as `I` of the outer
/// object.
pub(crate) fn compose<I, T, const N: usize>(
    value: T,
    more: [InterfaceEntry<T>; N],
    create: impl FnOnce(&IInspectable, *mut *mut IInspectable) -> Result<(), HResult>,
) -> Result<ComPtr<I>, HResult>
where
    I: ImplementedBy<T>,
    T: Composed + 'static,
{
    let outer: ComPtr<I> = make_com_with(value, more);
    let raw = outer.as_ptr().cast::<c_void>();
    // SAFETY: every interface of the integration derives from
    // `IInspectable`, so the outer object's first interface is one.
    let outer_inspectable = unsafe { &*(raw as *const IInspectable) };
    let mut inner: *mut IInspectable = std::ptr::null_mut();
    create(outer_inspectable, &mut inner)?;
    if inner.is_null() {
        return Err(HResult(E_NOINTERFACE));
    }
    // SAFETY: `raw` is an interface of the object `make_com_with` made with
    // a `T`, alive through `outer`; `inner` is the owned non-delegating
    // `IUnknown` the factory wrote, whose reference the object now holds.
    unsafe {
        ComObject::<T>::value(raw).composed_base().inner.set(inner.cast());
        set_inner::<T>(raw, inner.cast());
    }
    Ok(outer)
}
