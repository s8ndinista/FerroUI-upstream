//! Mounting elements as controls, and the reconciler that patches mounted
//! controls into a new element tree.
//!
//! A mounted element is an [`Instance`]: the control it made (or the text,
//! the list, the component) with what it was made from, so that the next tree
//! can be compared with it. Each instance places its controls into the
//! content of its parent control through a [`Slot`]: a list of placed
//! children, or a group of slots (the children of a control, the items of a
//! dynamic list, the output of a component). A slot that changes on its own
//! (a dynamic list whose signal changed, a component that ran again) replaces
//! its part of the parent's children, and only the part that differs.
//!
//! The rules of the reconciler (`docs/declarative-ui.md`, section 8):
//!
//! 1. Siblings are matched by key, and siblings without a key by their place
//!    among the siblings without a key. A match of the same kind (the same
//!    control class, the same component) keeps its control; anything else is
//!    mounted anew and the old one is taken out.
//! 2. A property value equal to the old one is left alone; any other value is
//!    applied again (a reactive value, a binding, a two-way value: the old one
//!    is stopped first); a property that is gone is cleared.
//! 3. Handlers are attached again: closures are never equal.
//! 4. The children are compared by the same rules.
//! 5. A component is run again with what its parent gave it; a dynamic list
//!    takes the new closure and runs again, keeping its items by key.

use crate::element::{AnyValueEq, ComponentElement, ControlElement, DynamicChildren, Element, ElementClass, Key, PropValue};
use crate::reactive::{effect, on_cleanup, signal, untrack, Owner, Signal};
use ferroui_base::data::BindingPriority;
use ferroui_base::{BoxedValue, FerroProperty, Ref};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::{ContentControl, Control, Decorator, ItemsControl, Panel, TextBlock};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// What holds the children of a control.
pub(crate) enum Host {
    /// `Panel.Children`.
    Panel(Ref<Panel>),
    /// `ItemsControl.Items`.
    Items(Ref<ItemsControl>),
    /// One child in a property: `ContentControl.Content`,
    /// `ContentPresenter.Content`, a property that holds an element.
    Content(Ref<Control>, &'static FerroProperty),
    /// `Decorator.Child`.
    Decorator(Ref<Decorator>),
    /// `TextBlock.Text`.
    Text(Ref<TextBlock>),
    /// One control in a property of a control class (`Popup.Child`, a
    /// property that holds a control: `ContextMenu`): `wrap` makes the value.
    Typed(Ref<Control>, &'static FerroProperty, fn(Option<Ref<Control>>) -> BoxedValue),
    /// No parent: the root of a mounted tree, whose control is handed to the
    /// caller and cannot be replaced.
    Root,
}

/// A child in its host: a control or a value (text in a content, an item).
#[derive(Clone)]
pub(crate) enum Placed {
    Control(Ref<Control>),
    Value(BoxedValue),
}

impl Placed {
    fn boxed(&self) -> BoxedValue {
        match self {
            Placed::Control(control) => Control::boxed(control),
            Placed::Value(value) => value.clone(),
        }
    }

    fn same(&self, other: &Placed) -> bool {
        match (self, other) {
            (Placed::Control(a), Placed::Control(b)) => a.ptr_eq(b),
            (Placed::Value(a), Placed::Value(b)) => Rc::ptr_eq(a, b) || **a == **b,
            _ => false,
        }
    }
}

impl Host {
    fn of(control: &Ref<Control>) -> Option<Host> {
        // A popup shows its child in a root of its own, above the window: the
        // child is in the logical tree and under the owner of the popup, but
        // outside its visual tree (what a portal is in React).
        if control.cast::<ferroui_controls::primitives::Popup>().is_some() {
            fn wrap(child: Option<Ref<Control>>) -> BoxedValue {
                Rc::new(child)
            }
            return Some(Host::Typed(
                control.clone(),
                ferroui_controls::primitives::Popup::child_property().as_property(),
                wrap,
            ));
        }
        if let Some(panel) = control.cast::<Panel>() {
            return Some(Host::Panel(panel));
        }
        if let Some(items) = control.cast::<ItemsControl>() {
            return Some(Host::Items(items));
        }
        if control.cast::<ContentControl>().is_some() {
            return Some(Host::Content(control.clone(), ContentControl::content_property().as_property()));
        }
        if control.cast::<ContentPresenter>().is_some() {
            return Some(Host::Content(control.clone(), ContentPresenter::content_property().as_property()));
        }
        if let Some(decorator) = control.cast::<Decorator>() {
            return Some(Host::Decorator(decorator));
        }
        if let Some(text) = control.cast::<TextBlock>() {
            return Some(Host::Text(text));
        }
        None
    }

    fn of_control(control: &Ref<Control>) -> Host {
        Host::of(control)
            .unwrap_or_else(|| panic!("{} has no content to place children into", control.get_type().full_name()))
    }

    /// The control whose content this is: what a component mounted here is
    /// in (its data context, for [`use_data_context`](crate::hooks::use_data_context)).
    fn control(&self) -> Option<Ref<Control>> {
        match self {
            Host::Panel(panel) => Some(panel.clone().upcast()),
            Host::Items(items) => Some(items.clone().upcast()),
            Host::Content(control, _) => Some(control.clone()),
            Host::Decorator(decorator) => Some(decorator.clone().upcast()),
            Host::Text(text) => Some(text.clone().upcast()),
            Host::Typed(control, ..) => Some(control.clone()),
            Host::Root => None,
        }
    }

    fn is_single(&self) -> bool {
        !matches!(self, Host::Panel(_) | Host::Items(_))
    }

    /// Whether text is held as text (a content, an item, the text of a
    /// `TextBlock`) rather than as a `TextBlock`.
    fn takes_text(&self) -> bool {
        matches!(self, Host::Content(..) | Host::Items(_) | Host::Text(_))
    }

    /// Replaces `removed` children from `start` on with `inserted`; `all` is
    /// the whole list after the change.
    fn splice(&self, start: usize, removed: usize, inserted: &[Placed], all: &[Placed]) {
        match self {
            Host::Panel(panel) => {
                let children = panel.children();
                for _ in 0..removed {
                    children.remove_at(start);
                }
                for (offset, item) in inserted.iter().enumerate() {
                    match item {
                        Placed::Control(control) => children.insert(start + offset, control.clone()),
                        Placed::Value(_) => panic!("a panel holds controls; text is not a child of a panel (use a TextBlock)"),
                    }
                }
            }
            Host::Items(items_control) => {
                let collection = items_control.items();
                for _ in 0..removed {
                    collection.remove_at(start);
                }
                for (offset, item) in inserted.iter().enumerate() {
                    collection.insert(start + offset, Some(item.boxed()));
                }
            }
            Host::Content(control, property) => {
                assert!(all.len() <= 1, "a content holds one child; it was given {}", all.len());
                let value: Option<BoxedValue> = all.first().map(Placed::boxed);
                control.set_value_untyped(property, &value, BindingPriority::LocalValue);
            }
            Host::Decorator(decorator) => {
                assert!(all.len() <= 1, "a decorator holds one child; it was given {}", all.len());
                match all.first() {
                    Some(Placed::Control(control)) => decorator.set_child(control.clone()),
                    Some(Placed::Value(_)) => panic!("a decorator holds a control; text is not its child (use a TextBlock)"),
                    None => decorator.set_child(None::<Ref<Control>>),
                }
            }
            Host::Text(text_block) => {
                let text: String = all
                    .iter()
                    .map(|item| match item {
                        Placed::Value(value) => value.downcast_ref::<String>().cloned().unwrap_or_default(),
                        Placed::Control(_) => panic!("a TextBlock holds text, not controls"),
                    })
                    .collect();
                text_block.set_text(Some(text.as_str()));
            }
            Host::Typed(control, property, wrap) => {
                assert!(all.len() <= 1, "a property that holds a control holds one; it was given {}", all.len());
                let child = match all.first() {
                    Some(Placed::Control(child)) => Some(child.clone()),
                    Some(Placed::Value(_)) => panic!("this property holds a control, not text (use a TextBlock)"),
                    None => None,
                };
                set_untyped(control, property, wrap(child));
            }
            Host::Root => panic!(
                "the root of a mounted tree changed its control: a component at the root keeps the class of the element it returns"
            ),
        }
    }
}

enum SlotContent {
    Leaf(Vec<Placed>),
    Group(Vec<Rc<Slot>>),
}

/// A part of the children of a host (see the module documentation).
pub(crate) struct Slot {
    host: Rc<Host>,
    parent: RefCell<Weak<Slot>>,
    /// On the top slot of a host: whether its children were placed into the
    /// host. Until then a change places nothing; the parent places every
    /// child at once.
    attached: Cell<bool>,
    content: RefCell<SlotContent>,
}

impl Slot {
    fn leaf(host: &Rc<Host>, items: Vec<Placed>) -> Rc<Slot> {
        Rc::new(Slot {
            host: host.clone(),
            parent: RefCell::new(Weak::new()),
            attached: Cell::new(false),
            content: RefCell::new(SlotContent::Leaf(items)),
        })
    }

    fn group(host: &Rc<Host>) -> Rc<Slot> {
        Rc::new(Slot {
            host: host.clone(),
            parent: RefCell::new(Weak::new()),
            attached: Cell::new(false),
            content: RefCell::new(SlotContent::Group(Vec::new())),
        })
    }

    fn placed(&self) -> Vec<Placed> {
        let mut out = Vec::new();
        self.collect(&mut out);
        out
    }

    fn collect(&self, out: &mut Vec<Placed>) {
        match &*self.content.borrow() {
            SlotContent::Leaf(items) => out.extend(items.iter().cloned()),
            SlotContent::Group(slots) => {
                for slot in slots {
                    slot.collect(out);
                }
            }
        }
    }

    fn len(&self) -> usize {
        match &*self.content.borrow() {
            SlotContent::Leaf(items) => items.len(),
            SlotContent::Group(slots) => slots.iter().map(|slot| slot.len()).sum(),
        }
    }

    /// The top slot of the host and where this slot begins among its
    /// children.
    fn top_and_offset(self: &Rc<Self>) -> (Rc<Slot>, usize) {
        let mut node = self.clone();
        let mut offset = 0;
        loop {
            let parent = node.parent.borrow().upgrade();
            let Some(parent) = parent else { return (node, offset) };
            if let SlotContent::Group(slots) = &*parent.content.borrow() {
                for sibling in slots {
                    if Rc::ptr_eq(sibling, &node) {
                        break;
                    }
                    offset += sibling.len();
                }
            }
            node = parent;
        }
    }

    fn set_leaf(self: &Rc<Self>, items: Vec<Placed>) {
        let old = self.placed();
        *self.content.borrow_mut() = SlotContent::Leaf(items);
        self.publish(old);
    }

    fn set_group(self: &Rc<Self>, slots: Vec<Rc<Slot>>) {
        let old = self.placed();
        for slot in &slots {
            *slot.parent.borrow_mut() = Rc::downgrade(self);
        }
        *self.content.borrow_mut() = SlotContent::Group(slots);
        self.publish(old);
    }

    /// Places the change from `old` to what the slot holds now into the host,
    /// once the host's children were placed.
    fn publish(self: &Rc<Self>, old: Vec<Placed>) {
        let (top, offset) = self.top_and_offset();
        if !top.attached.get() {
            return;
        }
        let new = self.placed();
        let prefix = old.iter().zip(&new).take_while(|(a, b)| a.same(b)).count();
        let max_suffix = old.len().min(new.len()) - prefix;
        let suffix = old.iter().rev().zip(new.iter().rev()).take(max_suffix).take_while(|(a, b)| a.same(b)).count();
        if prefix == old.len() && prefix == new.len() {
            return;
        }
        let all = top.placed();
        self.host.splice(offset + prefix, old.len() - prefix - suffix, &new[prefix..new.len() - suffix], &all);
    }

    /// Places the children of a top slot into the host for the first time.
    fn attach(&self) {
        let all = self.placed();
        if self.host.is_single() && all.len() > 1 {
            panic!("a control that holds one child was given {}", all.len());
        }
        if !all.is_empty() {
            self.host.splice(0, 0, &all, &all);
        }
        self.attached.set(true);
    }
}

/// A mounted element.
#[derive(Clone)]
pub(crate) enum Instance {
    Control(Rc<ControlInstance>),
    Text(Rc<TextInstance>),
    Existing(Ref<Control>, Rc<Slot>),
    Dynamic(Rc<DynamicInstance>),
    Component(Rc<ComponentInstance>),
}

impl Instance {
    fn slot(&self) -> Rc<Slot> {
        match self {
            Instance::Control(control) => control.slot.clone(),
            Instance::Text(text) => text.slot.clone(),
            Instance::Existing(_, slot) => slot.clone(),
            Instance::Dynamic(dynamic) => dynamic.slot.clone(),
            Instance::Component(component) => component.slot.clone(),
        }
    }

    fn key(&self) -> Option<&Key> {
        match self {
            Instance::Control(control) => control.key.as_ref(),
            Instance::Component(component) => component.key.as_ref(),
            _ => None,
        }
    }

    /// Takes the instance out: its owner is disposed, which stops its
    /// effects and detaches its handlers.
    fn dispose(&self) {
        match self {
            Instance::Control(control) => control.owner.dispose(),
            Instance::Dynamic(dynamic) => dynamic.owner.dispose(),
            Instance::Component(component) => component.owner.dispose(),
            Instance::Text(_) | Instance::Existing(..) => {}
        }
    }
}

/// The mounted children of a control (or of a property that holds an
/// element): the host, its top slot and the instances.
pub(crate) struct Children {
    host: Rc<Host>,
    top: Rc<Slot>,
    instances: Vec<Instance>,
}

impl Children {
    fn mount(host: Host, elements: Vec<Element>) -> Children {
        let host = Rc::new(host);
        let top = Slot::group(&host);
        let instances: Vec<Instance> = flatten(elements).into_iter().map(|element| mount_instance(element, &host)).collect();
        top.set_group(instances.iter().map(Instance::slot).collect());
        top.attach();
        Children { host, top, instances }
    }

    fn reconcile(&mut self, elements: Vec<Element>) {
        let old = std::mem::take(&mut self.instances);
        self.instances = reconcile_list(&self.host, &self.top, old, elements);
    }

    fn dispose(&self) {
        for instance in &self.instances {
            instance.dispose();
        }
    }
}

/// The parts of an element of a control that change.
struct ControlState {
    name: Option<String>,
    classes: Vec<String>,
    props: Vec<MountedProp>,
    handlers: Owner,
    children: Option<Children>,
}

struct MountedProp {
    property: &'static FerroProperty,
    value: PropValue,
    /// What the value started (the effect of a reactive value, a binding),
    /// stopped when the value changes.
    owner: Option<Owner>,
    /// The element a property holds, mounted.
    element: Option<Children>,
}

pub(crate) struct ControlInstance {
    class: ElementClass,
    key: Option<Key>,
    control: Ref<Control>,
    owner: Owner,
    slot: Rc<Slot>,
    state: RefCell<ControlState>,
}

pub(crate) struct TextInstance {
    text: RefCell<Rc<str>>,
    slot: Rc<Slot>,
}

/// A list of children that follows reactive state.
pub(crate) struct DynamicInstance {
    owner: Owner,
    slot: Rc<Slot>,
    render: RefCell<DynamicChildren>,
    /// Changed by the reconciler when the list was given a new closure: the
    /// list runs again, and the items it keeps are compared with what they
    /// render now.
    version: Signal<u64>,
    refresh: Cell<bool>,
}

/// An item of a dynamic list, by key.
struct MountedItem {
    value: Option<Rc<dyn AnyValueEq>>,
    slot: Rc<Slot>,
    instances: Vec<Instance>,
    owner: Owner,
    /// The owner of what the item's elements made on their last render.
    scope: Owner,
}

/// A component that runs again.
pub(crate) struct ComponentInstance {
    type_id: TypeId,
    key: Option<Key>,
    owner: Owner,
    /// The owner of the instances of what the component rendered.
    tree: Owner,
    slot: Rc<Slot>,
    host: Rc<Host>,
    render: RefCell<Rc<dyn Fn() -> Element>>,
    props: RefCell<Option<Rc<dyn AnyValueEq>>>,
    version: Signal<u64>,
    hooks: Rc<RefCell<Vec<Rc<dyn Any>>>>,
    instances: RefCell<Vec<Instance>>,
}

/// Elements in a list, with fragments taken apart and nothing left out.
pub(crate) fn flatten(children: Vec<Element>) -> Vec<Element> {
    let mut flat = Vec::with_capacity(children.len());
    for child in children {
        match child {
            Element::Fragment(items) => flat.extend(flatten(items)),
            Element::Empty => {}
            child => flat.push(child),
        }
    }
    flat
}

/// Mounts an element that is not a fragment or empty, for `host`.
fn mount_instance(element: Element, host: &Rc<Host>) -> Instance {
    match element {
        Element::Control(element) => Instance::Control(mount_control(*element, host)),
        Element::Text(text) => {
            let placed = if host.takes_text() {
                Placed::Value(Rc::new(text.to_string()))
            } else {
                let text_block = TextBlock::new();
                text_block.set_text(Some(&*text));
                Placed::Control(text_block.upcast())
            };
            Instance::Text(Rc::new(TextInstance { text: RefCell::new(text), slot: Slot::leaf(host, vec![placed]) }))
        }
        Element::Existing(control) => {
            let slot = Slot::leaf(host, vec![Placed::Control(control.clone())]);
            Instance::Existing(control, slot)
        }
        Element::Dynamic(dynamic) => Instance::Dynamic(mount_dynamic(dynamic, host)),
        Element::Component(component) => Instance::Component(mount_component(component, host)),
        Element::Fragment(_) | Element::Empty => unreachable!("fragments are flattened"),
    }
}

/// Mounts the root of a tree: an element that makes one control.
pub(crate) fn mount_root(element: Element) -> Ref<Control> {
    let element = match element {
        Element::Fragment(items) => {
            let mut panel = ControlElement::new(ElementClass {
                name: "StackPanel",
                type_id: TypeId::of::<ferroui_controls::StackPanel>,
                create: || ferroui_controls::StackPanel::new().upcast(),
            });
            panel.children = items;
            Element::Control(Box::new(panel))
        }
        Element::Empty | Element::Dynamic(_) => panic!("an empty element or a dynamic list has no control of its own"),
        element => element,
    };
    if let Element::Existing(control) = element {
        return control;
    }
    let host = Rc::new(Host::Root);
    let top = Slot::group(&host);
    let instance = mount_instance(element, &host);
    top.set_group(vec![instance.slot()]);
    top.attached.set(true);
    keep_alive(top.clone());
    let placed = top.placed();
    match placed.as_slice() {
        [Placed::Control(control)] => control.clone(),
        _ => panic!("the root element made {} controls; it makes one", placed.len()),
    }
}

fn mount_control(element: ControlElement, host: &Rc<Host>) -> Rc<ControlInstance> {
    let owner = Owner::new();
    owner.with(|| {
        let control = (element.class.create)();
        // As the XAML loader does: the control is told that it is being
        // initialised while it gets its values and children, so that what it
        // decides from both (a selected index of an items control) is decided
        // once both are there.
        control.begin_init();
        if let Some(name) = &element.name {
            control.set_name(Some(name.clone()));
        }
        if !element.classes.is_empty() {
            let classes = control.classes();
            for class in &element.classes {
                classes.add(class);
            }
        }
        let props = element.props.into_iter().map(|entry| mount_prop(&control, entry.property, entry.value)).collect();
        let handlers = Owner::new();
        handlers.with(|| {
            for attach in &element.handlers {
                attach(&control);
            }
        });
        let children = flatten(element.children);
        let children = (!children.is_empty()).then(|| Children::mount(Host::of_control(&control), children));
        control.end_init();
        for attach in &element.on_mount {
            attach(&control);
        }
        let slot = Slot::leaf(host, vec![Placed::Control(control.clone())]);
        let instance = Rc::new(ControlInstance {
            class: element.class,
            key: element.key,
            control,
            owner,
            slot,
            state: RefCell::new(ControlState { name: element.name, classes: element.classes, props, handlers, children }),
        });
        keep_alive(instance.clone());
        instance
    })
}

fn mount_prop(control: &Ref<Control>, property: &'static FerroProperty, value: PropValue) -> MountedProp {
    match &value {
        PropValue::Value(boxed) => {
            set_untyped(control, property, boxed.clone());
            MountedProp { property, value, owner: None, element: None }
        }
        PropValue::Element(element) => {
            let element = element.borrow_mut().take().unwrap_or_default();
            let owner = Owner::new();
            let children = owner.with(|| Children::mount(Host::Content(control.clone(), property), vec![element]));
            MountedProp { property, value, owner: Some(owner), element: Some(children) }
        }
        PropValue::TypedElement { element, wrap } => {
            let element = element.borrow_mut().take().unwrap_or_default();
            let owner = Owner::new();
            let wrap = *wrap;
            let children = owner.with(|| Children::mount(Host::Typed(control.clone(), property, wrap), vec![element]));
            MountedProp { property, value, owner: Some(owner), element: Some(children) }
        }
        _ => {
            let owner = Owner::new();
            owner.with(|| start_prop(control, property, &value));
            MountedProp { property, value, owner: Some(owner), element: None }
        }
    }
}

/// Starts a reactive value, a two-way value or a binding of a property; it
/// belongs to the current owner.
fn start_prop(control: &Ref<Control>, property: &'static FerroProperty, value: &PropValue) {
    match value {
        PropValue::Dynamic(compute) => {
            let compute = compute.clone();
            let weak = control.downgrade();
            effect(move || {
                let value = compute();
                if let Some(control) = weak.upgrade() {
                    untrack(|| set_untyped(&control, property, value));
                }
            });
        }
        PropValue::TwoWay { get, set } => {
            let get = get.clone();
            let set = set.clone();
            let weak = control.downgrade();
            effect(move || {
                let value = get();
                if let Some(control) = weak.upgrade() {
                    untrack(|| set_untyped(&control, property, value));
                }
            });
            let subscription = control.property_changed(move |e| {
                if std::ptr::eq(e.property(), property) {
                    set(e.new_value());
                }
            });
            on_cleanup(move || subscription.dispose());
        }
        PropValue::Binding(binding) => {
            let expression = control.bind_binding(property, &**binding);
            on_cleanup(move || expression.dispose());
        }
        PropValue::Value(_) | PropValue::Element(_) | PropValue::TypedElement { .. } => {
            unreachable!("handled by mount_prop")
        }
    }
}

/// Sets a property from a value of its type held untyped.
fn set_untyped(control: &Ref<Control>, property: &'static FerroProperty, value: BoxedValue) {
    // The value inside the box, not the box: `as_any` of the handle itself
    // would give the `Rc`.
    let inner: &dyn ferroui_base::AnyValue = &*value;
    let value: &dyn Any = inner.as_any();
    control.set_value_untyped(property, value, BindingPriority::LocalValue);
}

fn mount_dynamic(dynamic: DynamicChildren, host: &Rc<Host>) -> Rc<DynamicInstance> {
    let owner = Owner::new();
    let instance = owner.with(|| {
        Rc::new(DynamicInstance {
            owner,
            slot: Slot::group(host),
            render: RefCell::new(dynamic),
            version: signal(0),
            refresh: Cell::new(false),
        })
    });
    let mounted: Rc<RefCell<HashMap<Key, MountedItem>>> = Rc::new(RefCell::new(HashMap::new()));
    let unkeyed: Rc<RefCell<Vec<MountedItem>>> = Rc::new(RefCell::new(Vec::new()));
    let state = instance.clone();
    let host = host.clone();
    owner.with(|| {
        effect(move || {
            state.version.get();
            let render = state.render.borrow().render.clone();
            let entries = render();
            untrack(|| update_dynamic(&state, &host, &mounted, &unkeyed, entries));
        })
    });
    instance
}

type Entry = (Option<Key>, Option<Rc<dyn AnyValueEq>>, Rc<dyn Fn() -> Element>);

fn update_dynamic(
    state: &DynamicInstance,
    host: &Rc<Host>,
    mounted: &RefCell<HashMap<Key, MountedItem>>,
    unkeyed: &RefCell<Vec<MountedItem>>,
    entries: Vec<Entry>,
) {
    let refresh = state.refresh.replace(false);
    let parent = state.owner;
    let make = |render: &Rc<dyn Fn() -> Element>, value: Option<Rc<dyn AnyValueEq>>| -> MountedItem {
        let owner = parent.with(Owner::new);
        let scope = owner.with(Owner::new);
        let elements = scope.with(|| flatten(vec![render()]));
        let slot = Slot::group(host);
        let instances: Vec<Instance> = owner.with(|| elements.into_iter().map(|element| mount_instance(element, host)).collect());
        slot.set_group(instances.iter().map(Instance::slot).collect());
        MountedItem { value, slot, instances, owner, scope }
    };
    // An item that is kept renders again when the list was given a new
    // closure (its parent ran again): its controls are patched.
    let rerender = |item: &mut MountedItem, render: &Rc<dyn Fn() -> Element>| {
        let scope = item.owner.with(Owner::new);
        let elements = scope.with(|| vec![render()]);
        let old = std::mem::take(&mut item.instances);
        item.instances = item.owner.with(|| reconcile_list(host, &item.slot, old, elements));
        std::mem::replace(&mut item.scope, scope).dispose();
    };

    let mut previous = std::mem::take(&mut *mounted.borrow_mut());
    let mut previous_unkeyed = std::mem::take(&mut *unkeyed.borrow_mut()).into_iter();
    let mut next_mounted = HashMap::new();
    let mut next_unkeyed = Vec::new();
    let mut slots = Vec::with_capacity(entries.len());
    for (key, value, render) in &entries {
        match key {
            Some(key) => {
                let item = match previous.remove(key) {
                    Some(mut item) => {
                        let same = match (&item.value, value) {
                            (Some(old), Some(new)) => old.item_equals(&**new),
                            _ => true,
                        };
                        if same {
                            if refresh {
                                rerender(&mut item, render);
                            }
                            item
                        } else {
                            item.owner.dispose();
                            make(render, value.clone())
                        }
                    }
                    None => make(render, value.clone()),
                };
                slots.push(item.slot.clone());
                next_mounted.insert(key.clone(), item);
            }
            None => {
                // An item without a key is compared with the one at its place
                // when the list was given a new closure, and made again
                // otherwise (what it read changed).
                let item = match previous_unkeyed.next() {
                    Some(mut item) if refresh => {
                        rerender(&mut item, render);
                        item
                    }
                    Some(item) => {
                        item.owner.dispose();
                        make(render, None)
                    }
                    None => make(render, None),
                };
                slots.push(item.slot.clone());
                next_unkeyed.push(item);
            }
        }
    }
    state.slot.set_group(slots);
    for (_, item) in previous {
        item.owner.dispose();
    }
    for item in previous_unkeyed {
        item.owner.dispose();
    }
    *mounted.borrow_mut() = next_mounted;
    *unkeyed.borrow_mut() = next_unkeyed;
}

fn mount_component(element: ComponentElement, host: &Rc<Host>) -> Rc<ComponentInstance> {
    let owner = Owner::new();
    let instance = owner.with(|| {
        Rc::new(ComponentInstance {
            type_id: element.type_id,
            key: element.key,
            owner,
            tree: Owner::new(),
            slot: Slot::group(host),
            host: host.clone(),
            render: RefCell::new(element.render),
            props: RefCell::new(element.props),
            version: signal(0),
            hooks: Rc::new(RefCell::new(Vec::new())),
            instances: RefCell::new(Vec::new()),
        })
    });
    let state = instance.clone();
    owner.with(|| {
        effect(move || {
            state.version.get();
            let render = state.render.borrow().clone();
            let parent = state.host.control();
            let (element, after) = crate::hooks::render_with(&state.hooks, state.owner, parent, || render());
            untrack(|| {
                let old = std::mem::take(&mut *state.instances.borrow_mut());
                let next = state.tree.with(|| reconcile_list(&state.host, &state.slot, old, vec![element]));
                *state.instances.borrow_mut() = next;
                for f in after {
                    f();
                }
            });
        })
    });
    instance
}

/// Patches the instances `old` (the content of `slot`) into `elements`, and
/// returns the instances of the new list.
fn reconcile_list(host: &Rc<Host>, slot: &Rc<Slot>, old: Vec<Instance>, elements: Vec<Element>) -> Vec<Instance> {
    let elements = flatten(elements);
    let mut keyed: HashMap<Key, Instance> = HashMap::new();
    let mut unkeyed = Vec::new();
    for instance in old {
        match instance.key().cloned() {
            Some(key) => {
                if let Some(duplicate) = keyed.insert(key, instance) {
                    unkeyed.push(duplicate);
                }
            }
            None => unkeyed.push(instance),
        }
    }
    let mut unkeyed = unkeyed.into_iter();
    let mut removed = Vec::new();
    let mut next = Vec::with_capacity(elements.len());
    for element in elements {
        let candidate = match element.key() {
            Some(key) => keyed.remove(key),
            None => unkeyed.next(),
        };
        let instance = match candidate {
            Some(instance) => match patch(instance, element) {
                Ok(instance) => instance,
                Err((old, element)) => {
                    removed.push(old);
                    mount_instance(element, host)
                }
            },
            None => mount_instance(element, host),
        };
        next.push(instance);
    }
    removed.extend(keyed.into_values());
    removed.extend(unkeyed);
    slot.set_group(next.iter().map(Instance::slot).collect());
    for instance in removed {
        instance.dispose();
    }
    next
}

/// Patches `instance` into `element` when they are of the same kind; gives
/// both back otherwise.
fn patch(instance: Instance, element: Element) -> Result<Instance, (Instance, Element)> {
    match (instance, element) {
        (Instance::Control(control), Element::Control(element)) if control.class == element.class => {
            patch_control(&control, *element);
            Ok(Instance::Control(control))
        }
        (Instance::Text(text), Element::Text(new)) => {
            if **text.text.borrow() != *new {
                let placed = text.slot.placed();
                match placed.first() {
                    Some(Placed::Control(control)) => {
                        if let Some(text_block) = control.cast::<TextBlock>() {
                            text_block.set_text(Some(&*new));
                        }
                    }
                    _ => text.slot.set_leaf(vec![Placed::Value(Rc::new(new.to_string()))]),
                }
                *text.text.borrow_mut() = new;
            }
            Ok(Instance::Text(text))
        }
        (Instance::Existing(control, slot), Element::Existing(new)) if control.ptr_eq(&new) => {
            Ok(Instance::Existing(control, slot))
        }
        (Instance::Dynamic(dynamic), Element::Dynamic(new)) => {
            *dynamic.render.borrow_mut() = new;
            dynamic.refresh.set(true);
            dynamic.version.update(|version| *version += 1);
            Ok(Instance::Dynamic(dynamic))
        }
        (Instance::Component(component), Element::Component(new)) if component.type_id == new.type_id => {
            let same_props = match (&*component.props.borrow(), &new.props) {
                (Some(old), Some(props)) => old.item_equals(&**props),
                _ => false,
            };
            *component.render.borrow_mut() = new.render;
            *component.props.borrow_mut() = new.props;
            if !same_props {
                component.version.update(|version| *version += 1);
            }
            Ok(Instance::Component(component))
        }
        (instance, element) => Err((instance, element)),
    }
}

fn patch_control(instance: &ControlInstance, element: ControlElement) {
    let control = &instance.control;
    let mut state = instance.state.borrow_mut();
    if state.name != element.name {
        control.set_name(element.name.clone());
        state.name = element.name;
    }
    if state.classes != element.classes {
        let classes = control.classes();
        for class in &state.classes {
            if !element.classes.contains(class) {
                classes.remove(class);
            }
        }
        for class in &element.classes {
            if !state.classes.contains(class) {
                classes.add(class);
            }
        }
        state.classes = element.classes;
    }

    // Properties: the old values by property; what is equal stays.
    let mut old: Vec<Option<MountedProp>> = std::mem::take(&mut state.props).into_iter().map(Some).collect();
    let mut props = Vec::with_capacity(element.props.len());
    instance.owner.with(|| {
        for entry in element.props {
            let previous = old.iter_mut().rev().find(|prop| prop.as_ref().is_some_and(|prop| std::ptr::eq(prop.property, entry.property)));
            let previous = previous.and_then(Option::take);
            let prop = match previous {
                Some(previous) if previous.value == entry.value => previous,
                Some(MountedProp { element: Some(mut children), owner, .. })
                    if matches!(entry.value, PropValue::Element(_) | PropValue::TypedElement { .. }) =>
                {
                    let (PropValue::Element(cell) | PropValue::TypedElement { element: cell, .. }) = &entry.value else {
                        unreachable!()
                    };
                    let new = cell.borrow_mut().take().unwrap_or_default();
                    match owner {
                        Some(owner) => owner.with(|| children.reconcile(vec![new])),
                        None => children.reconcile(vec![new]),
                    }
                    MountedProp { property: entry.property, value: entry.value, owner, element: Some(children) }
                }
                Some(previous) => {
                    stop_prop(&previous);
                    mount_prop(control, entry.property, entry.value)
                }
                None => mount_prop(control, entry.property, entry.value),
            };
            props.push(prop);
        }
        for prop in old.into_iter().flatten() {
            stop_prop(&prop);
            control.clear_value_untyped(prop.property);
        }
    });
    state.props = props;

    // Handlers: closures are never equal, so they are attached again.
    let handlers = instance.owner.with(Owner::new);
    handlers.with(|| {
        for attach in &element.handlers {
            attach(control);
        }
    });
    std::mem::replace(&mut state.handlers, handlers).dispose();

    // Children.
    let children = flatten(element.children);
    instance.owner.with(|| match &mut state.children {
        Some(mounted) => mounted.reconcile(children),
        None if children.is_empty() => {}
        None => state.children = Some(Children::mount(Host::of_control(control), children)),
    });
}

/// Stops what a property value started.
fn stop_prop(prop: &MountedProp) {
    if let Some(children) = &prop.element {
        children.dispose();
    }
    if let Some(owner) = prop.owner {
        owner.dispose();
    }
}

/// The children an element mounted into a control that holds one child (a
/// decorator, a content control): what `ComponentHost` keeps.
pub(crate) struct MountedChild(Children);

impl MountedChild {
    /// Mounts `element` as the child of `control`, under the current owner.
    pub(crate) fn mount(control: &Ref<Control>, element: Element) -> MountedChild {
        MountedChild(Children::mount(Host::of_control(control), vec![element]))
    }

    /// Takes the child out of the control and disposes what it mounted.
    pub(crate) fn unmount(self) {
        let mut children = self.0;
        children.reconcile(Vec::new());
    }
}

/// Keeps `value` until the current owner is disposed: the instances of a
/// mounted tree live as long as their owners, whether or not a parent keeps
/// them.
fn keep_alive<T: 'static>(value: Rc<T>) {
    on_cleanup(move || drop(value));
}

/// A subscription of the property system ended with its owner.
#[allow(dead_code)]
pub(crate) fn dispose_with_owner(subscription: Rc<dyn ferroui_base::reactive::IDisposable>) {
    on_cleanup(move || subscription.dispose());
}
