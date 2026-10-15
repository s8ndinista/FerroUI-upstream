//! Collections of the Windows Runtime the integration hands out: the list of
//! the children of an automation peer (`IVector<AutomationPeer>`, which
//! `GetChildrenCore` returns) and the items of a drag
//! (`IIterable<IStorageItem>`, which `DataPackage.SetStorageItems` takes).
//! The reference passes a `List<T>`, which its runtime projects as these
//! interfaces.
//!
//! The objects are of the UI thread, as the objects of WinUI that ask them
//! are. An iterator reads a copy of the items taken when it was created.

use super::win_rt::{iids, runtime_class_name, to_abi};
use super::*;
use ferroui_microcom::{make_com_with, ComPtr, Guid, HResult, Interface, InterfaceEntry, E_INVALIDARG};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The index of `value` among `items`: the same object (its identity).
fn position<T: Interface>(items: &[ComPtr<T>], value: Option<&T>) -> Option<usize> {
    let value = value?;
    let identity = value.cast_identity()?;
    items.iter().position(|item| item.cast_identity() == Some(identity))
}

trait Identity {
    /// The pointer of the object's `IUnknown`: what tells two references to
    /// one object.
    fn cast_identity(&self) -> Option<usize>;
}

impl<T: Interface> Identity for T {
    fn cast_identity(&self) -> Option<usize> {
        let mut out = std::ptr::null_mut();
        // SAFETY: a live interface; `out` is a place of this frame.
        let result = unsafe { self.as_unknown().query_interface_raw(&ferroui_microcom::IUnknown::IID, &mut out) };
        if HResult::check(result).is_err() || out.is_null() {
            return None;
        }
        // SAFETY: the query succeeded; the reference it added is given back.
        unsafe { (*(out as *const ferroui_microcom::IUnknown)).release() };
        Some(out as usize)
    }
}

/// Copies items to the array of the caller (the "fill" array of the ABI),
/// from `start`; the count copied.
///
/// # Safety
/// `items` must be valid for `capacity` writes of a pointer.
unsafe fn fill<T: Interface>(source: &[ComPtr<T>], start: usize, capacity: u32, items: *mut *mut T) -> u32 {
    let mut count = 0u32;
    for item in source.iter().skip(start).take(capacity as usize) {
        // SAFETY: within the capacity of the array; the caller owns the
        // reference written to it.
        unsafe { items.add(count as usize).write(item.clone().into_raw()) };
        count += 1;
    }
    count
}

/// `IVector<AutomationPeer>` (with its `IIterable<AutomationPeer>`).
pub(crate) struct AutomationPeerVector {
    items: Rc<RefCell<Vec<ComPtr<IAutomationPeer>>>>,
}

impl AutomationPeerVector {
    pub(crate) fn new(items: Vec<ComPtr<IAutomationPeer>>) -> ComPtr<IVectorAutomationPeer> {
        make_com_with(
            AutomationPeerVector { items: Rc::new(RefCell::new(items)) },
            [InterfaceEntry::of::<IIterableAutomationPeer>()],
        )
    }
}

impl IInspectableImpl for AutomationPeerVector {
    fn get_iids(&self, iid_count: *mut u32) -> Result<*mut Guid, HResult> {
        // SAFETY: the out parameter of the call.
        unsafe { iids(&[IVectorAutomationPeer::IID, IIterableAutomationPeer::IID], iid_count) }
    }

    fn get_runtime_class_name(&self) -> Result<isize, HResult> {
        runtime_class_name("Windows.Foundation.Collections.IVector`1<Microsoft.UI.Xaml.Automation.Peers.AutomationPeer>")
    }

    fn get_trust_level(&self) -> Result<TrustLevel, HResult> {
        Ok(TrustLevel::BaseTrust)
    }
}

impl IVectorAutomationPeerImpl for AutomationPeerVector {
    fn get_at(&self, index: u32) -> Result<Option<ComPtr<IAutomationPeer>>, HResult> {
        self.items.borrow().get(index as usize).cloned().map(Some).ok_or(HResult(E_BOUNDS))
    }

    fn size(&self) -> Result<u32, HResult> {
        Ok(self.items.borrow().len() as u32)
    }

    fn get_view(&self) -> Result<Option<ComPtr<IVectorViewAutomationPeer>>, HResult> {
        Ok(Some(make_com_with(
            AutomationPeerVectorView { items: Rc::new(self.items.borrow().clone()) },
            [InterfaceEntry::of::<IIterableAutomationPeer>()],
        )))
    }

    fn index_of(&self, value: Option<&IAutomationPeer>, index: *mut u32) -> Result<u8, HResult> {
        let found = position(&self.items.borrow(), value);
        // SAFETY: the out parameter of the call.
        unsafe { index.write(found.unwrap_or(0) as u32) };
        Ok(to_abi(found.is_some()))
    }

    fn set_at(&self, index: u32, value: Option<&IAutomationPeer>) -> Result<(), HResult> {
        let value = value.map(ComPtr::from_ref).ok_or(HResult(E_INVALIDARG))?;
        let mut items = self.items.borrow_mut();
        let slot = items.get_mut(index as usize).ok_or(HResult(E_BOUNDS))?;
        *slot = value;
        Ok(())
    }

    fn insert_at(&self, index: u32, value: Option<&IAutomationPeer>) -> Result<(), HResult> {
        let value = value.map(ComPtr::from_ref).ok_or(HResult(E_INVALIDARG))?;
        let mut items = self.items.borrow_mut();
        if index as usize > items.len() {
            return Err(HResult(E_BOUNDS));
        }
        items.insert(index as usize, value);
        Ok(())
    }

    fn remove_at(&self, index: u32) -> Result<(), HResult> {
        let mut items = self.items.borrow_mut();
        if index as usize >= items.len() {
            return Err(HResult(E_BOUNDS));
        }
        items.remove(index as usize);
        Ok(())
    }

    fn append(&self, value: Option<&IAutomationPeer>) -> Result<(), HResult> {
        let value = value.map(ComPtr::from_ref).ok_or(HResult(E_INVALIDARG))?;
        self.items.borrow_mut().push(value);
        Ok(())
    }

    fn remove_at_end(&self) -> Result<(), HResult> {
        self.items.borrow_mut().pop().map(|_| ()).ok_or(HResult(E_BOUNDS))
    }

    fn clear(&self) -> Result<(), HResult> {
        self.items.borrow_mut().clear();
        Ok(())
    }

    fn get_many(&self, start_index: u32, items_size: u32, items: *mut *mut IAutomationPeer) -> Result<u32, HResult> {
        // SAFETY: the array of the caller, of `items_size` places.
        Ok(unsafe { fill(&self.items.borrow(), start_index as usize, items_size, items) })
    }

    fn replace_all(&self, items_size: u32, items: *mut *mut IAutomationPeer) -> Result<(), HResult> {
        let mut replacement = Vec::with_capacity(items_size as usize);
        for index in 0..items_size as usize {
            // SAFETY: the array of the caller, of `items_size` places, each
            // an interface the call borrows.
            let item = unsafe { ComPtr::from_raw_add_ref(*items.add(index)) }.ok_or(HResult(E_INVALIDARG))?;
            replacement.push(item);
        }
        *self.items.borrow_mut() = replacement;
        Ok(())
    }
}

impl IIterableAutomationPeerImpl for AutomationPeerVector {
    fn first(&self) -> Result<Option<ComPtr<IIteratorAutomationPeer>>, HResult> {
        Ok(Some(AutomationPeerIterator::new(Rc::new(self.items.borrow().clone()))))
    }
}

/// `IVectorView<AutomationPeer>`: a copy of the items of the vector.
struct AutomationPeerVectorView {
    items: Rc<Vec<ComPtr<IAutomationPeer>>>,
}

impl IInspectableImpl for AutomationPeerVectorView {
    fn get_iids(&self, iid_count: *mut u32) -> Result<*mut Guid, HResult> {
        // SAFETY: the out parameter of the call.
        unsafe { iids(&[IVectorViewAutomationPeer::IID, IIterableAutomationPeer::IID], iid_count) }
    }

    fn get_runtime_class_name(&self) -> Result<isize, HResult> {
        runtime_class_name("Windows.Foundation.Collections.IVectorView`1<Microsoft.UI.Xaml.Automation.Peers.AutomationPeer>")
    }

    fn get_trust_level(&self) -> Result<TrustLevel, HResult> {
        Ok(TrustLevel::BaseTrust)
    }
}

impl IVectorViewAutomationPeerImpl for AutomationPeerVectorView {
    fn get_at(&self, index: u32) -> Result<Option<ComPtr<IAutomationPeer>>, HResult> {
        self.items.get(index as usize).cloned().map(Some).ok_or(HResult(E_BOUNDS))
    }

    fn size(&self) -> Result<u32, HResult> {
        Ok(self.items.len() as u32)
    }

    fn index_of(&self, value: Option<&IAutomationPeer>, index: *mut u32) -> Result<u8, HResult> {
        let found = position(&self.items, value);
        // SAFETY: the out parameter of the call.
        unsafe { index.write(found.unwrap_or(0) as u32) };
        Ok(to_abi(found.is_some()))
    }

    fn get_many(&self, start_index: u32, items_size: u32, items: *mut *mut IAutomationPeer) -> Result<u32, HResult> {
        // SAFETY: the array of the caller, of `items_size` places.
        Ok(unsafe { fill(&self.items, start_index as usize, items_size, items) })
    }
}

impl IIterableAutomationPeerImpl for AutomationPeerVectorView {
    fn first(&self) -> Result<Option<ComPtr<IIteratorAutomationPeer>>, HResult> {
        Ok(Some(AutomationPeerIterator::new(self.items.clone())))
    }
}

/// `IIterator<AutomationPeer>`.
struct AutomationPeerIterator {
    items: Rc<Vec<ComPtr<IAutomationPeer>>>,
    index: Cell<usize>,
}

impl AutomationPeerIterator {
    fn new(items: Rc<Vec<ComPtr<IAutomationPeer>>>) -> ComPtr<IIteratorAutomationPeer> {
        make_com_with(AutomationPeerIterator { items, index: Cell::new(0) }, [])
    }
}

impl IInspectableImpl for AutomationPeerIterator {
    fn get_iids(&self, iid_count: *mut u32) -> Result<*mut Guid, HResult> {
        // SAFETY: the out parameter of the call.
        unsafe { iids(&[IIteratorAutomationPeer::IID], iid_count) }
    }

    fn get_runtime_class_name(&self) -> Result<isize, HResult> {
        runtime_class_name("Windows.Foundation.Collections.IIterator`1<Microsoft.UI.Xaml.Automation.Peers.AutomationPeer>")
    }

    fn get_trust_level(&self) -> Result<TrustLevel, HResult> {
        Ok(TrustLevel::BaseTrust)
    }
}

impl IIteratorAutomationPeerImpl for AutomationPeerIterator {
    fn current(&self) -> Result<Option<ComPtr<IAutomationPeer>>, HResult> {
        self.items.get(self.index.get()).cloned().map(Some).ok_or(HResult(E_BOUNDS))
    }

    fn has_current(&self) -> Result<u8, HResult> {
        Ok(to_abi(self.index.get() < self.items.len()))
    }

    fn move_next(&self) -> Result<u8, HResult> {
        if self.index.get() < self.items.len() {
            self.index.set(self.index.get() + 1);
        }
        Ok(to_abi(self.index.get() < self.items.len()))
    }

    fn get_many(&self, items_size: u32, items: *mut *mut IAutomationPeer) -> Result<u32, HResult> {
        // SAFETY: the array of the caller, of `items_size` places.
        let count = unsafe { fill(&self.items, self.index.get(), items_size, items) };
        self.index.set(self.index.get() + count as usize);
        Ok(count)
    }
}

/// `IIterable<IStorageItem>`.
pub(crate) struct StorageItems {
    items: Rc<Vec<ComPtr<IStorageItem>>>,
}

impl StorageItems {
    pub(crate) fn new(items: Vec<ComPtr<IStorageItem>>) -> ComPtr<IIterableStorageItem> {
        make_com_with(StorageItems { items: Rc::new(items) }, [])
    }
}

impl IInspectableImpl for StorageItems {
    fn get_iids(&self, iid_count: *mut u32) -> Result<*mut Guid, HResult> {
        // SAFETY: the out parameter of the call.
        unsafe { iids(&[IIterableStorageItem::IID], iid_count) }
    }

    fn get_runtime_class_name(&self) -> Result<isize, HResult> {
        runtime_class_name("Windows.Foundation.Collections.IIterable`1<Windows.Storage.IStorageItem>")
    }

    fn get_trust_level(&self) -> Result<TrustLevel, HResult> {
        Ok(TrustLevel::BaseTrust)
    }
}

impl IIterableStorageItemImpl for StorageItems {
    fn first(&self) -> Result<Option<ComPtr<IIteratorStorageItem>>, HResult> {
        Ok(Some(make_com_with(StorageItemIterator { items: self.items.clone(), index: Cell::new(0) }, [])))
    }
}

/// `IIterator<IStorageItem>`.
struct StorageItemIterator {
    items: Rc<Vec<ComPtr<IStorageItem>>>,
    index: Cell<usize>,
}

impl IInspectableImpl for StorageItemIterator {
    fn get_iids(&self, iid_count: *mut u32) -> Result<*mut Guid, HResult> {
        // SAFETY: the out parameter of the call.
        unsafe { iids(&[IIteratorStorageItem::IID], iid_count) }
    }

    fn get_runtime_class_name(&self) -> Result<isize, HResult> {
        runtime_class_name("Windows.Foundation.Collections.IIterator`1<Windows.Storage.IStorageItem>")
    }

    fn get_trust_level(&self) -> Result<TrustLevel, HResult> {
        Ok(TrustLevel::BaseTrust)
    }
}

impl IIteratorStorageItemImpl for StorageItemIterator {
    fn current(&self) -> Result<Option<ComPtr<IStorageItem>>, HResult> {
        self.items.get(self.index.get()).cloned().map(Some).ok_or(HResult(E_BOUNDS))
    }

    fn has_current(&self) -> Result<u8, HResult> {
        Ok(to_abi(self.index.get() < self.items.len()))
    }

    fn move_next(&self) -> Result<u8, HResult> {
        if self.index.get() < self.items.len() {
            self.index.set(self.index.get() + 1);
        }
        Ok(to_abi(self.index.get() < self.items.len()))
    }

    fn get_many(&self, items_size: u32, items: *mut *mut IStorageItem) -> Result<u32, HResult> {
        // SAFETY: the array of the caller, of `items_size` places.
        let count = unsafe { fill(&self.items, self.index.get(), items_size, items) };
        self.index.set(self.index.get() + count as usize);
        Ok(count)
    }
}

/// `E_BOUNDS`: an index past the end of a collection.
const E_BOUNDS: u32 = 0x8000_000B;
