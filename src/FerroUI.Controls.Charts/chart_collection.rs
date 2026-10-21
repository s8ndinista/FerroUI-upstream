use crate::{Axis, CartesianSeries, ChartError, Samples};
use ferroui_base::collections::{CollectionChangedHandler, FerroList};
use ferroui_base::{IntoRef, ObjectType, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An item that can be retained by one of the three chart collections.
#[doc(hidden)]
pub trait ChartCollectionItem: ObjectType {
    const EXCLUSIVE: bool;
    fn chart_owner(&self) -> Option<u64>;
    fn set_chart_owner(&self, owner: Option<u64>);
    fn validate_chart(&self, _owner: u64) -> Result<(), ChartError> { Ok(()) }
}

impl ChartCollectionItem for Axis {
    const EXCLUSIVE: bool = true;
    fn chart_owner(&self) -> Option<u64> { self.owner_id() }
    fn set_chart_owner(&self, owner: Option<u64>) { self.set_owner_id(owner); }
}

impl ChartCollectionItem for CartesianSeries {
    const EXCLUSIVE: bool = true;
    fn chart_owner(&self) -> Option<u64> { self.owner_id() }
    fn set_chart_owner(&self, owner: Option<u64>) { self.set_owner_id(owner); }
    fn validate_chart(&self, owner: u64) -> Result<(), ChartError> {
        for axis in [self.x_axis(), self.y_axis()].into_iter().flatten() {
            if axis.owner_id() != Some(owner) { return Err(ChartError::AxisNotInChart); }
        }
        Ok(())
    }
}

impl ChartCollectionItem for Samples {
    const EXCLUSIVE: bool = false;
    fn chart_owner(&self) -> Option<u64> { None }
    fn set_chart_owner(&self, _owner: Option<u64>) {}
}

pub(crate) struct CollectionOwner {
    pub(crate) id: Cell<Option<u64>>,
    pub(crate) changed: RefCell<Option<Rc<dyn Fn()>>>,
}

impl CollectionOwner {
    pub(crate) fn new(id: u64) -> Rc<Self> {
        Rc::new(Self { id: Cell::new(Some(id)), changed: RefCell::new(None) })
    }
    fn notify(&self) {
        let changed = self.changed.borrow().clone();
        if let Some(changed) = changed { changed(); }
    }
}

struct CollectionData<T: ChartCollectionItem> {
    owner: Rc<CollectionOwner>,
    items: FerroList<Ref<T>>,
}

/// An observable collection with transactional ownership checks. Its storage is
/// shared by handles; snapshots remain valid during reentrant notifications.
pub struct ChartCollection<T: ChartCollectionItem>(Rc<CollectionData<T>>);

impl<T: ChartCollectionItem> Clone for ChartCollection<T> {
    fn clone(&self) -> Self { Self(self.0.clone()) }
}
impl<T: ChartCollectionItem> PartialEq for ChartCollection<T> {
    fn eq(&self, other: &Self) -> bool { Rc::ptr_eq(&self.0, &other.0) }
}

impl<T: ChartCollectionItem> ChartCollection<T> {
    pub(crate) fn new(owner: Rc<CollectionOwner>) -> Self {
        let items = FerroList::new();
        let notify_owner = owner.clone();
        items.add_collection_changed(Rc::new(move |_| notify_owner.notify()));
        Self(Rc::new(CollectionData { owner, items }))
    }
    pub fn count(&self) -> usize { self.0.items.count() }
    pub fn is_empty(&self) -> bool { self.0.items.is_empty() }
    pub fn snapshot(&self) -> Rc<Vec<Ref<T>>> { self.0.items.snapshot() }
    pub fn get(&self, index: usize) -> Result<Ref<T>, ChartError> {
        self.snapshot().get(index).cloned().ok_or(ChartError::IndexOutOfBounds)
    }
    pub fn contains(&self, value: impl IntoRef<T>) -> bool {
        self.0.items.contains(&value.into_ref())
    }
    pub fn index_of(&self, value: impl IntoRef<T>) -> Option<usize> {
        let value = value.into_ref();
        self.snapshot().iter().position(|item| *item == value)
    }
    fn validate(&self, value: &Ref<T>) -> Result<(), ChartError> {
        if T::EXCLUSIVE {
            if self.0.items.contains(value) { return Err(ChartError::DuplicateItem); }
            if value.chart_owner().is_some() { return Err(ChartError::AlreadyOwned); }
        }
        if let Some(owner) = self.0.owner.id.get() { value.validate_chart(owner)?; }
        Ok(())
    }
    pub fn add(&self, value: impl IntoRef<T>) -> Result<(), ChartError> {
        self.insert(self.count(), value)
    }
    pub fn insert(&self, index: usize, value: impl IntoRef<T>) -> Result<(), ChartError> {
        if index > self.count() { return Err(ChartError::IndexOutOfBounds); }
        let value = value.into_ref();
        self.validate(&value)?;
        value.set_chart_owner(self.0.owner.id.get());
        // State and ownership are committed before observers run. The internal
        // listener was registered before any public listener by the chart.
        self.0.items.insert(index, value);
        Ok(())
    }
    pub fn set(&self, index: usize, value: impl IntoRef<T>) -> Result<(), ChartError> {
        let old = self.get(index)?;
        let value = value.into_ref();
        if old == value { return Ok(()); }
        self.validate(&value)?;
        old.set_chart_owner(None);
        value.set_chart_owner(self.0.owner.id.get());
        self.0.items.set(index, value);
        Ok(())
    }
    pub fn remove_at(&self, index: usize) -> Result<Ref<T>, ChartError> {
        let old = self.get(index)?;
        old.set_chart_owner(None);
        self.0.items.remove_at(index);
        Ok(old)
    }
    pub fn remove(&self, value: impl IntoRef<T>) -> bool {
        let value = value.into_ref();
        let Some(index) = self.snapshot().iter().position(|item| item == &value) else { return false; };
        self.remove_at(index).is_ok()
    }
    pub fn clear(&self) {
        if self.is_empty() { return; }
        for item in self.snapshot().iter() { item.set_chart_owner(None); }
        self.0.items.clear();
    }
    pub fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<Ref<T>>>) -> u64 {
        self.0.items.add_collection_changed(handler)
    }
    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.0.items.remove_collection_changed(token)
    }
    pub(crate) fn release_ownership(&self) {
        for item in self.snapshot().iter() { item.set_chart_owner(None); }
    }
}

pub type ChartSeriesCollection = ChartCollection<CartesianSeries>;
pub type ChartAxisCollection = ChartCollection<Axis>;
pub type ChartDataCollection = ChartCollection<Samples>;

// Explicit declarations also make the collection adapters visible to the
// build-time source scanner; generated applications call their public helpers.
ferroui_base::ferro_markup_type!(class ChartSeriesCollection {
    handles: [ChartSeriesCollection], this: ChartSeriesCollection,
    properties: [Count: i32 { get: |x: &ChartSeriesCollection| x.count() as i32 }],
    methods: [try fn Add(Ref<CartesianSeries>) => |x: &ChartSeriesCollection, item: Ref<CartesianSeries>| x.add(item)],
    indexers: [(i32) -> Ref<CartesianSeries> {
        try_get: |x: &ChartSeriesCollection, i: i32| x.get(i as usize),
        try_set: |x: &ChartSeriesCollection, i: i32, item: Ref<CartesianSeries>| x.set(i as usize, item)
    }],
});
ferroui_base::ferro_markup_type!(class ChartAxisCollection {
    handles: [ChartAxisCollection], this: ChartAxisCollection,
    properties: [Count: i32 { get: |x: &ChartAxisCollection| x.count() as i32 }],
    methods: [try fn Add(Ref<Axis>) => |x: &ChartAxisCollection, item: Ref<Axis>| x.add(item)],
    indexers: [(i32) -> Ref<Axis> {
        try_get: |x: &ChartAxisCollection, i: i32| x.get(i as usize),
        try_set: |x: &ChartAxisCollection, i: i32, item: Ref<Axis>| x.set(i as usize, item)
    }],
});
ferroui_base::ferro_markup_type!(class ChartDataCollection {
    handles: [ChartDataCollection], this: ChartDataCollection,
    properties: [Count: i32 { get: |x: &ChartDataCollection| x.count() as i32 }],
    methods: [try fn Add(Ref<Samples>) => |x: &ChartDataCollection, item: Ref<Samples>| x.add(item)],
    indexers: [(i32) -> Ref<Samples> {
        try_get: |x: &ChartDataCollection, i: i32| x.get(i as usize),
        try_set: |x: &ChartDataCollection, i: i32, item: Ref<Samples>| x.set(i as usize, item)
    }],
});
