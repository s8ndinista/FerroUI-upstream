//! The element tree: a description of controls, their properties, handlers
//! and children, and how it becomes controls ([`Element::mount`]).
//!
//! An element is data: the class to create, a list of property values (the
//! property and its value, compared by value), handlers, children and a key.
//! Mounting creates the controls once; what changes afterwards changes through
//! the reactive values the element holds (a property set from a signal or a
//! closure, a list of children that follows a signal). Because an element is
//! data, a later reconciler can compare two trees of elements and patch the
//! controls of the first into the second; nothing here assumes that the tree
//! is mounted only once.

use crate::reactive::{effect, on_cleanup, untrack, Owner};
use ferroui_base::data::BindingBase;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{BoxedValue, FerroProperty, Ref};
use ferroui_base::data::BindingPriority;
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::{Control, ContentControl, Decorator, ItemsControl, Panel, TextBlock};
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

/// The key of an element among its siblings: an element of a dynamic list
/// keeps its control while its key stays.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Key {
    /// A number.
    Int(i64),
    /// A string.
    Str(Rc<str>),
}

impl From<i64> for Key {
    fn from(value: i64) -> Self {
        Key::Int(value)
    }
}

impl From<i32> for Key {
    fn from(value: i32) -> Self {
        Key::Int(i64::from(value))
    }
}

impl From<u32> for Key {
    fn from(value: u32) -> Self {
        Key::Int(i64::from(value))
    }
}

impl From<usize> for Key {
    fn from(value: usize) -> Self {
        Key::Int(value as i64)
    }
}

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Key::Str(Rc::from(value))
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Key::Str(Rc::from(value))
    }
}

/// The value an element gives a property.
#[derive(Clone)]
pub enum PropValue {
    /// A value, set once. Values compare by value.
    Value(BoxedValue),
    /// A value computed by a reactive closure: the property is set again
    /// whenever what the closure read changes.
    Dynamic(Rc<dyn Fn() -> BoxedValue>),
    /// A value that goes both ways: the property follows `get`, and a change
    /// of the property is given to `set`.
    TwoWay {
        /// The value of the source, read reactively.
        get: Rc<dyn Fn() -> BoxedValue>,
        /// Takes a new value of the property.
        set: Rc<dyn Fn(&dyn Any)>,
    },
    /// An element, mounted with its parent: the property holds its control
    /// (`Content`, `Tag`, a header).
    Element(Rc<RefCell<Option<Element>>>),
    /// A binding of the property system (`Binding`, a path on the data
    /// context, a compiled binding).
    Binding(Rc<dyn BindingBase>),
}

impl PartialEq for PropValue {
    /// Values compare by value; anything else is never equal (a reconciler
    /// sets it again).
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (PropValue::Value(a), PropValue::Value(b)) => a == b,
            _ => false,
        }
    }
}

impl fmt::Debug for PropValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PropValue::Value(_) => f.write_str("Value"),
            PropValue::Dynamic(_) => f.write_str("Dynamic"),
            PropValue::TwoWay { .. } => f.write_str("TwoWay"),
            PropValue::Element(_) => f.write_str("Element"),
            PropValue::Binding(_) => f.write_str("Binding"),
        }
    }
}

/// A property and the value an element gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct PropEntry {
    /// The property.
    pub property: &'static FerroProperty,
    /// Its value.
    pub value: PropValue,
}

/// What attaches a handler (a routed event, a change of a property) to the
/// control of an element.
pub type Attach = Rc<dyn Fn(&Ref<Control>)>;

/// The class of an element: how its control is made.
#[derive(Clone, Copy)]
pub struct ElementClass {
    /// The name of the class, for messages.
    pub name: &'static str,
    /// The identity of the class.
    pub type_id: fn() -> std::any::TypeId,
    /// Creates a control of the class.
    pub create: fn() -> Ref<Control>,
}

impl PartialEq for ElementClass {
    fn eq(&self, other: &Self) -> bool {
        (self.type_id)() == (other.type_id)()
    }
}

impl fmt::Debug for ElementClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// An element of a control class.
#[derive(Clone)]
pub struct ControlElement {
    /// The class of the control.
    pub class: ElementClass,
    /// The key among its siblings.
    pub key: Option<Key>,
    /// `Name` of the control.
    pub name: Option<String>,
    /// Style classes of the control.
    pub classes: Vec<String>,
    /// Values of properties, in the order they were given.
    pub props: Vec<PropEntry>,
    /// Handlers.
    pub handlers: Vec<Attach>,
    /// Children, placed into the content of the control (see
    /// [`Element::mount`]).
    pub children: Vec<Element>,
    /// Called with the control once it was made and given its values.
    pub on_mount: Vec<Attach>,
}

impl fmt::Debug for ControlElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ControlElement")
            .field("class", &self.class)
            .field("key", &self.key)
            .field("name", &self.name)
            .field("classes", &self.classes)
            .field("props", &self.props)
            .field("children", &self.children)
            .finish()
    }
}

impl ControlElement {
    /// An element of `class` with nothing set.
    pub fn new(class: ElementClass) -> Self {
        Self {
            class,
            key: None,
            name: None,
            classes: Vec::new(),
            props: Vec::new(),
            handlers: Vec::new(),
            children: Vec::new(),
            on_mount: Vec::new(),
        }
    }
}

/// A list of children that follows reactive state: the closure is run in an
/// effect, and its elements replace the ones it gave before; elements with a
/// key that stays keep their control (see [`for_each`](crate::for_each)).
#[derive(Clone)]
pub struct DynamicChildren {
    pub(crate) render: Rc<dyn Fn() -> Vec<(Option<Key>, Option<Rc<dyn AnyValueEq>>, Rc<dyn Fn() -> Element>)>>,
}

/// A value an item of a dynamic list is compared by, to decide whether its
/// element is made again.
pub trait AnyValueEq: Any {
    /// Whether `other` holds an equal value.
    fn item_equals(&self, other: &dyn AnyValueEq) -> bool;
    /// The value as `Any`.
    fn item_as_any(&self) -> &dyn Any;
}

impl<T: PartialEq + 'static> AnyValueEq for T {
    fn item_equals(&self, other: &dyn AnyValueEq) -> bool {
        other.item_as_any().downcast_ref::<T>().is_some_and(|other| other == self)
    }

    fn item_as_any(&self) -> &dyn Any {
        self
    }
}

/// A description of user interface: a control with its properties and
/// children, text, several elements, a list that follows state, or a control
/// that exists already.
#[derive(Clone, Default)]
pub enum Element {
    /// Nothing.
    #[default]
    Empty,
    /// A control.
    Control(Box<ControlElement>),
    /// Text: the text of the content of its parent, or a `TextBlock` where a
    /// control is needed.
    Text(Rc<str>),
    /// Several elements in place of one.
    Fragment(Vec<Element>),
    /// Children that follow reactive state.
    Dynamic(DynamicChildren),
    /// A control made elsewhere (a XAML user control, a control kept by the
    /// application).
    Existing(Ref<Control>),
}

impl fmt::Debug for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Element::Empty => f.write_str("Empty"),
            Element::Control(control) => control.fmt(f),
            Element::Text(text) => write!(f, "Text({text:?})"),
            Element::Fragment(items) => f.debug_tuple("Fragment").field(items).finish(),
            Element::Dynamic(_) => f.write_str("Dynamic"),
            Element::Existing(_) => f.write_str("Existing"),
        }
    }
}

/// What turns into an element: an element, a builder of one, a control.
pub trait IntoElement {
    /// The element.
    fn into_element(self) -> Element;
}

impl IntoElement for Element {
    fn into_element(self) -> Element {
        self
    }
}

impl<T: ferroui_base::ObjectType + ferroui_base::Upcast<Control>> IntoElement for Ref<T> {
    fn into_element(self) -> Element {
        Element::Existing(self.upcast())
    }
}

impl<T: IntoElement> IntoElement for Option<T> {
    fn into_element(self) -> Element {
        self.map_or(Element::Empty, IntoElement::into_element)
    }
}

impl<T: IntoElement> IntoElement for Vec<T> {
    fn into_element(self) -> Element {
        Element::Fragment(self.into_iter().map(IntoElement::into_element).collect())
    }
}

/// What can be a child of an element: an element, a builder, text, a list of
/// them.
pub trait IntoChild {
    /// The child as an element.
    fn into_child(self) -> Element;
}

impl<T: IntoElement> IntoChild for T {
    fn into_child(self) -> Element {
        self.into_element()
    }
}

impl IntoChild for &str {
    fn into_child(self) -> Element {
        Element::Text(Rc::from(self))
    }
}

impl IntoChild for String {
    fn into_child(self) -> Element {
        Element::Text(Rc::from(self))
    }
}

/// What can be several children: a vector, an iterator collected, a tuple.
pub trait IntoChildren {
    /// The children.
    fn into_children(self) -> Vec<Element>;
}

impl<T: IntoChild> IntoChildren for Vec<T> {
    fn into_children(self) -> Vec<Element> {
        self.into_iter().map(IntoChild::into_child).collect()
    }
}

impl<T: IntoChild, const N: usize> IntoChildren for [T; N] {
    fn into_children(self) -> Vec<Element> {
        self.into_iter().map(IntoChild::into_child).collect()
    }
}

macro_rules! tuple_children {
    ($($name:ident),+) => {
        impl<$($name: IntoChild),+> IntoChildren for ($($name,)+) {
            #[allow(non_snake_case)]
            fn into_children(self) -> Vec<Element> {
                let ($($name,)+) = self;
                vec![$($name.into_child()),+]
            }
        }
    };
}

tuple_children!(A);
tuple_children!(A, B);
tuple_children!(A, B, C);
tuple_children!(A, B, C, D);
tuple_children!(A, B, C, D, E);
tuple_children!(A, B, C, D, E, F);
tuple_children!(A, B, C, D, E, F, G);
tuple_children!(A, B, C, D, E, F, G, H);
tuple_children!(A, B, C, D, E, F, G, H, I);
tuple_children!(A, B, C, D, E, F, G, H, I, J);
tuple_children!(A, B, C, D, E, F, G, H, I, J, K);
tuple_children!(A, B, C, D, E, F, G, H, I, J, K, L);

/// What holds the children of a control.
enum Host {
    /// `Panel.Children`.
    Panel(Ref<Panel>),
    /// `ItemsControl.Items`.
    Items(Ref<ItemsControl>),
    /// One child: `ContentControl.Content`, `ContentPresenter.Content`.
    Content(Ref<Control>, &'static FerroProperty),
    /// `Decorator.Child`.
    Decorator(Ref<Decorator>),
    /// `TextBlock.Text`.
    Text(Ref<TextBlock>),
}

/// A child in its host: a control or a value (text in a content, an item).
#[derive(Clone)]
enum Placed {
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
}

impl Host {
    fn of(control: &Ref<Control>) -> Option<Host> {
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

    fn is_single(&self) -> bool {
        !matches!(self, Host::Panel(_) | Host::Items(_))
    }

    /// Whether text is held as text (a content, an item, the text of a
    /// `TextBlock`) rather than as a `TextBlock`.
    fn takes_text(&self) -> bool {
        matches!(self, Host::Content(..) | Host::Items(_) | Host::Text(_))
    }

    /// Replaces the children from `start` on (`removed` of them) with `items`.
    fn splice(&self, start: usize, removed: usize, items: &[Placed]) {
        match self {
            Host::Panel(panel) => {
                let children = panel.children();
                for _ in 0..removed {
                    children.remove_at(start);
                }
                for (offset, item) in items.iter().enumerate() {
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
                for (offset, item) in items.iter().enumerate() {
                    collection.insert(start + offset, Some(item.boxed()));
                }
            }
            Host::Content(control, property) => {
                assert!(start + items.len() <= 1, "a content holds one child");
                let value: Option<BoxedValue> = items.first().map(Placed::boxed);
                control.set_value_untyped(property, &value, BindingPriority::LocalValue);
            }
            Host::Decorator(decorator) => {
                assert!(start + items.len() <= 1, "a decorator holds one child");
                match items.first() {
                    Some(Placed::Control(control)) => decorator.set_child(control.clone()),
                    Some(Placed::Value(_)) => panic!("a decorator holds a control; text is not its child (use a TextBlock)"),
                    None => decorator.set_child(None::<Ref<Control>>),
                }
            }
            Host::Text(text_block) => {
                let text: String = items
                    .iter()
                    .map(|item| match item {
                        Placed::Value(value) => value.downcast_ref::<String>().cloned().unwrap_or_default(),
                        Placed::Control(_) => panic!("a TextBlock holds text, not controls"),
                    })
                    .collect();
                text_block.set_text(Some(text.as_str()));
            }
        }
    }
}

/// A slot of the children of a host: one placed child, or the children of a
/// dynamic list.
enum Slot {
    Fixed(Placed),
    Dynamic(Rc<RefCell<Vec<Placed>>>),
}

impl Element {
    /// Mounts the element as a control: creates the controls of the tree,
    /// gives them their values and children, attaches their handlers and
    /// starts the effects of their reactive values. The effects belong to
    /// the current owner (see [`Owner`]): disposing it stops them.
    ///
    /// Text becomes a `TextBlock`, and several elements a `StackPanel`.
    ///
    /// # Panics
    /// Panics for an empty element or a dynamic list, which have no control
    /// of their own.
    pub fn mount(self) -> Ref<Control> {
        match self {
            Element::Control(element) => mount_control(*element),
            Element::Existing(control) => control,
            Element::Text(text) => {
                let text_block = TextBlock::new();
                text_block.set_text(Some(&*text));
                text_block.upcast()
            }
            Element::Fragment(items) => {
                let panel = ferroui_controls::StackPanel::new();
                place_children(&panel.clone().upcast(), items);
                panel.upcast()
            }
            Element::Empty | Element::Dynamic(_) => panic!("an empty element or a dynamic list has no control of its own"),
        }
    }

    /// Mounts the element under a new owner of its own, which is disposed
    /// when the returned owner is: the root of a window or of a piece of user
    /// interface that ends on its own.
    pub fn mount_root(self) -> (Ref<Control>, Owner) {
        let owner = Owner::new_root();
        let control = owner.with(|| self.mount());
        (control, owner)
    }

    /// The key of the element, if it has one.
    pub fn key(&self) -> Option<&Key> {
        match self {
            Element::Control(element) => element.key.as_ref(),
            _ => None,
        }
    }
}

fn mount_control(element: ControlElement) -> Ref<Control> {
    let control = (element.class.create)();
    if let Some(name) = element.name {
        control.set_name(Some(name));
    }
    if !element.classes.is_empty() {
        let classes = control.classes();
        for class in &element.classes {
            classes.add(class);
        }
    }
    for entry in element.props {
        apply_prop(&control, entry.property, entry.value);
    }
    for attach in &element.handlers {
        attach(&control);
    }
    place_children(&control, element.children);
    for attach in &element.on_mount {
        attach(&control);
    }
    control
}

fn apply_prop(control: &Ref<Control>, property: &'static FerroProperty, value: PropValue) {
    match value {
        PropValue::Value(value) => {
            set_untyped(control, property, value);
        }
        PropValue::Dynamic(compute) => {
            let weak = control.downgrade();
            effect(move || {
                let value = compute();
                if let Some(control) = weak.upgrade() {
                    untrack(|| set_untyped(&control, property, value));
                }
            });
        }
        PropValue::TwoWay { get, set } => {
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
        PropValue::Element(element) => {
            let element = element.borrow_mut().take().unwrap_or_default();
            let value: Option<BoxedValue> = match element {
                Element::Empty => None,
                Element::Text(text) => Some(Rc::new(text.to_string())),
                element => Some(Control::boxed(&element.mount())),
            };
            control.set_value_untyped(property, &value, BindingPriority::LocalValue);
        }
        PropValue::Binding(binding) => {
            control.bind_binding(property, &*binding);
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

/// Places the children of an element into the content of its control.
fn place_children(control: &Ref<Control>, children: Vec<Element>) {
    let children: Vec<Element> = flatten(children);
    if children.is_empty() {
        return;
    }
    let host = Host::of(control).unwrap_or_else(|| {
        panic!("{} has no content to place children into", control.get_type().full_name())
    });
    let host = Rc::new(host);
    let slots: Rc<RefCell<Vec<Slot>>> = Rc::new(RefCell::new(Vec::new()));
    let mut fixed = Vec::new();
    for child in children {
        match child {
            Element::Dynamic(dynamic) => {
                let items = Rc::new(RefCell::new(Vec::new()));
                slots.borrow_mut().push(Slot::Dynamic(items.clone()));
                mount_dynamic(host.clone(), slots.clone(), items, dynamic);
            }
            child => {
                let placed = place(child, host.takes_text());
                fixed.push(placed.clone());
                slots.borrow_mut().push(Slot::Fixed(placed));
            }
        }
    }
    if host.is_single() && slots.borrow().len() > 1 {
        panic!("{} holds one child; it was given {}", control.get_type().full_name(), slots.borrow().len());
    }
    // The fixed children in order, with the dynamic lists (mounted above)
    // where they are.
    let all: Vec<Placed> = slots
        .borrow()
        .iter()
        .flat_map(|slot| match slot {
            Slot::Fixed(placed) => vec![placed.clone()],
            Slot::Dynamic(items) => items.borrow().clone(),
        })
        .collect();
    if !all.is_empty() || !fixed.is_empty() {
        host.splice(0, 0, &all);
    }
}

fn flatten(children: Vec<Element>) -> Vec<Element> {
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

/// A child element as what its host holds: text stays text in a content and
/// an item, and becomes a `TextBlock` elsewhere.
fn place(child: Element, takes_text: bool) -> Placed {
    match child {
        Element::Text(text) if takes_text => Placed::Value(Rc::new(text.to_string())),
        child => Placed::Control(child.mount()),
    }
}

/// The mounted item of a dynamic list, by key.
struct MountedItem {
    value: Option<Rc<dyn AnyValueEq>>,
    placed: Vec<Placed>,
    owner: Owner,
}

fn mount_dynamic(host: Rc<Host>, slots: Rc<RefCell<Vec<Slot>>>, items: Rc<RefCell<Vec<Placed>>>, dynamic: DynamicChildren) {
    let takes_text = host.takes_text();
    let mounted: Rc<RefCell<HashMap<Key, MountedItem>>> = Rc::new(RefCell::new(HashMap::new()));
    let unkeyed: Rc<RefCell<Vec<Owner>>> = Rc::new(RefCell::new(Vec::new()));
    let first_run = Rc::new(std::cell::Cell::new(true));
    {
        let mounted = mounted.clone();
        let unkeyed = unkeyed.clone();
        on_cleanup(move || {
            for (_, item) in mounted.borrow_mut().drain() {
                item.owner.dispose();
            }
            for owner in unkeyed.borrow_mut().drain(..) {
                owner.dispose();
            }
        });
    }
    let parent = Owner::current();
    effect(move || {
        let entries = (dynamic.render)();
        untrack(|| {
            let make = |render: &Rc<dyn Fn() -> Element>| -> (Vec<Placed>, Owner) {
                let owner = parent.map_or_else(Owner::new_root, |parent| parent.with(Owner::new));
                let placed = owner.with(|| {
                    flatten(vec![render()])
                        .into_iter()
                        .map(|element| match element {
                            Element::Dynamic(_) => panic!("a dynamic list gives elements, not other dynamic lists"),
                            element => place(element, takes_text),
                        })
                        .collect()
                });
                (placed, owner)
            };
            for owner in unkeyed.borrow_mut().drain(..) {
                owner.dispose();
            }
            let mut previous = std::mem::take(&mut *mounted.borrow_mut());
            let mut next_mounted = HashMap::new();
            let mut next = Vec::with_capacity(entries.len());
            for (key, value, render) in &entries {
                match key {
                    Some(key) => {
                        let reuse = previous.remove(key).filter(|item| match (&item.value, value) {
                            (Some(old), Some(new)) => old.item_equals(&**new),
                            _ => true,
                        });
                        let item = match reuse {
                            Some(item) => item,
                            None => {
                                if let Some(stale) = previous.remove(key) {
                                    stale.owner.dispose();
                                }
                                let (placed, owner) = make(render);
                                MountedItem { value: value.clone(), placed, owner }
                            }
                        };
                        next.extend(item.placed.iter().cloned());
                        next_mounted.insert(key.clone(), item);
                    }
                    None => {
                        let (placed, owner) = make(render);
                        unkeyed.borrow_mut().push(owner);
                        next.extend(placed);
                    }
                }
            }
            for (_, item) in previous {
                item.owner.dispose();
            }
            *mounted.borrow_mut() = next_mounted;

            let old_len = items.borrow().len();
            *items.borrow_mut() = next.clone();
            if first_run.replace(false) {
                // The first run places nothing: the parent places every
                // child at once.
                return;
            }
            let start = slot_start(&slots, &items);
            host.splice(start, old_len, &next);
        });
    });
}

/// Where the children of a dynamic slot begin among the children of the
/// host.
fn slot_start(slots: &Rc<RefCell<Vec<Slot>>>, items: &Rc<RefCell<Vec<Placed>>>) -> usize {
    let mut start = 0;
    for slot in slots.borrow().iter() {
        match slot {
            Slot::Fixed(_) => start += 1,
            Slot::Dynamic(other) => {
                if Rc::ptr_eq(other, items) {
                    return start;
                }
                start += other.borrow().len();
            }
        }
    }
    start
}

/// A subscription of the property system ended with its owner.
#[allow(dead_code)]
pub(crate) fn dispose_with_owner(subscription: Rc<dyn IDisposable>) {
    on_cleanup(move || subscription.dispose());
}

/// The element with `key` (an element of a control keeps it; any other
/// element has no key and is returned as it is).
pub fn with_key(element: Element, key: Key) -> Element {
    match element {
        Element::Control(mut control) => {
            control.key = Some(key);
            Element::Control(control)
        }
        element => element,
    }
}
