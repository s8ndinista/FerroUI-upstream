//! Port of `Diagnostics/ViewModels/EventsPageViewModel.cs`.

use super::{EventOwnerTreeNode, EventTreeNodeBase, FilterViewModel, FiredEvent, MainViewModel, ViewModelBase};
use crate::diagnostics::models::EventChainLink;
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::ferro_markup_type;
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventRegistry};
use ferroui_base::TypeInfo;
use ferroui_controls::{Button, Control};
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::{Rc, Weak};

/// The events recorded when the page is created, and again with "Enable
/// default".
fn default_events() -> HashSet<RoutedEvent> {
    HashSet::from([
        Button::click_event().as_routed_event(),
        InputElement::key_down_event().as_routed_event(),
        InputElement::key_up_event().as_routed_event(),
        InputElement::text_input_event().as_routed_event(),
        InputElement::pointer_released_event().as_routed_event(),
        InputElement::pointer_pressed_event().as_routed_event(),
    ])
}

/// The view model of the events page: the tree of the registered routed
/// events grouped by the type that owns them, whose check boxes choose the
/// events to record, and the recorded events.
pub struct EventsPageViewModel {
    base: ViewModelBase,
    main_view_model: Weak<MainViewModel>,
    selected_event: RefCell<Option<Rc<FiredEvent>>>,
    selected_node: RefCell<Option<Rc<EventTreeNodeBase>>>,
    nodes: FerroList<Rc<EventTreeNodeBase>>,
    recorded_events: FerroList<Rc<FiredEvent>>,
    events_filter: Rc<FilterViewModel>,
}

/// Reference equality, as the class of the original.
impl PartialEq for EventsPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for EventsPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl EventsPageViewModel {
    /// C# `new EventsPageViewModel(MainViewModel mainViewModel)`: the tree of
    /// the routed events registered on this thread, with the default events
    /// enabled.
    ///
    /// The main view model is held weakly: it holds the page.
    pub fn new(main_view_model: Weak<MainViewModel>) -> Rc<Self> {
        // Deviation (DEVIATIONS.md, Developer tools): the static `DefaultEvents` of the original
        // is initialized before the constructor runs, which registers its events; here they
        // register on first access, so the set is made before the registered events are read.
        let _ = default_events();
        let this = Rc::new_cyclic(|this: &Weak<EventsPageViewModel>| {
            // `GroupBy(e => e.OwnerType)`: the groups in the order of their first event.
            let mut groups: Vec<(&'static TypeInfo, Vec<RoutedEvent>)> = Vec::new();
            for e in RoutedEventRegistry::instance().get_all_registered() {
                let owner_type = e.owner_type();
                match groups.iter_mut().find(|(key, _)| std::ptr::eq(*key, owner_type)) {
                    Some((_, events)) => events.push(e),
                    None => groups.push((owner_type, vec![e])),
                }
            }
            // `OrderBy(e => e.Key.Name)` is a stable sort.
            groups.sort_by(|(x, _), (y, _)| x.name().cmp(y.name()));

            Self {
                base: ViewModelBase::new(),
                main_view_model,
                selected_event: RefCell::new(None),
                selected_node: RefCell::new(None),
                nodes: FerroList::from_items(
                    groups.into_iter().map(|(key, events)| EventOwnerTreeNode::new(key, events, this)),
                ),
                recorded_events: FerroList::new(),
                events_filter: FilterViewModel::new(),
            }
        });

        let weak = Rc::downgrade(&this);
        // As upstream, the handler is never removed: the filter belongs to the page.
        let _ = this.events_filter.refresh_filter(move || {
            if let Some(this) = weak.upgrade() {
                this.update_event_filters();
            }
        });

        this.enable_default();
        this
    }

    /// The title of the page.
    pub fn name(&self) -> String {
        "Events".to_string()
    }

    /// The nodes of the types that own routed events, ordered by the names
    /// of the types.
    pub fn nodes(&self) -> FerroList<Rc<EventTreeNodeBase>> {
        self.nodes.clone()
    }

    /// The recorded events, the oldest first.
    pub fn recorded_events(&self) -> FerroList<Rc<FiredEvent>> {
        self.recorded_events.clone()
    }

    pub fn selected_event(&self) -> Option<Rc<FiredEvent>> {
        self.selected_event.borrow().clone()
    }

    pub fn set_selected_event(&self, value: Option<Rc<FiredEvent>>) {
        self.base.raise_and_set_if_changed(&self.selected_event, value, "SelectedEvent");
    }

    pub fn selected_node(&self) -> Option<Rc<EventTreeNodeBase>> {
        self.selected_node.borrow().clone()
    }

    pub fn set_selected_node(&self, value: Option<Rc<EventTreeNodeBase>>) {
        self.base.raise_and_set_if_changed(&self.selected_node, value, "SelectedNode");
    }

    /// The filter of the tree of the events.
    pub fn events_filter(&self) -> Rc<FilterViewModel> {
        self.events_filter.clone()
    }

    /// Removes the recorded events.
    pub fn clear(&self) {
        self.recorded_events.clear();
    }

    /// Stops recording every event.
    pub fn disable_all(&self) {
        self.evaluate_node_enabled(&|_: &EventTreeNodeBase| false);
    }

    /// Records the default events and no other.
    pub fn enable_default(&self) {
        let default_events = default_events();
        self.evaluate_node_enabled(&|node: &EventTreeNodeBase| {
            let event = node.as_event_tree_node().expect("a node of an event").event();
            default_events.contains(&event)
        });
    }

    /// Shows the element of a link of the chain of an event in the visual
    /// tree page.
    pub fn request_tree_navigate_to(&self, nav_target: &EventChainLink) {
        if let Some(control) = ValueTypes::as_object(&*nav_target.handler()).and_then(|handler| handler.cast::<Control>())
        {
            if let Some(main_view_model) = self.main_view() {
                main_view_model.request_tree_navigate_to(&control, true);
            }
        }
    }

    /// Selects the node of `evt` in the tree, if it is visible.
    pub fn select_event_by_type(&self, evt: RoutedEvent) {
        fn find_node(node: &Rc<EventTreeNodeBase>, event_type: RoutedEvent) -> Option<Rc<EventTreeNodeBase>> {
            if node.as_event_tree_node().is_some_and(|event_node| event_node.event() == event_type) {
                return Some(node.clone());
            }

            if let Some(children) = node.children() {
                for child in children.to_vec() {
                    let result = find_node(&child, event_type);

                    if result.is_some() {
                        return result;
                    }
                }
            }

            None
        }

        for node in self.nodes.to_vec() {
            let result = find_node(&node, evt);

            if let Some(result) = result.filter(|result| result.is_visible()) {
                self.set_selected_node(Some(result));

                break;
            }
        }
    }

    /// Sets `IsEnabled` of every node of an event to `eval` of the node.
    fn evaluate_node_enabled(&self, eval: &dyn Fn(&EventTreeNodeBase) -> bool) {
        fn process_node(node: &Rc<EventTreeNodeBase>, eval: &dyn Fn(&EventTreeNodeBase) -> bool) {
            if node.as_event_tree_node().is_some() {
                node.set_is_enabled(Some(eval(&**node)));
            }

            if let Some(children) = node.children() {
                for child_node in children.to_vec() {
                    process_node(&child_node, eval);
                }
            }
        }

        for node in self.nodes.to_vec() {
            process_node(&node, eval);
        }
    }

    /// Shows the nodes whose text matches the filter, their ancestors and
    /// their descendants.
    fn update_event_filters(&self) {
        fn filter_node(filter: &FilterViewModel, node: &Rc<EventTreeNodeBase>, is_parent_visible: bool) -> bool {
            let matches_filter = filter.filter(node.text());
            let mut has_visible_child = false;

            if let Some(children) = node.children() {
                for child_node in children.to_vec() {
                    has_visible_child |= filter_node(filter, &child_node, matches_filter);
                }
            }

            node.set_is_visible(has_visible_child || matches_filter || is_parent_visible);

            node.is_visible()
        }

        for node in self.nodes.to_vec() {
            filter_node(&self.events_filter, &node, false);
        }
    }

    /// The main view model the page belongs to.
    pub fn main_view(&self) -> Option<Rc<MainViewModel>> {
        self.main_view_model.upgrade()
    }
}

ferro_markup_type!(class EventsPageViewModel {
    this: Rc<EventsPageViewModel>,
    handles: [EventsPageViewModel, Rc<EventsPageViewModel>, Option<Rc<EventsPageViewModel>>],
    base: ViewModelBase,
    properties: [
        Name: String { get: EventsPageViewModel::name },
        Nodes: FerroList<Rc<EventTreeNodeBase>> { get: EventsPageViewModel::nodes },
        RecordedEvents: FerroList<Rc<FiredEvent>> { get: EventsPageViewModel::recorded_events },
        SelectedEvent: Option<Rc<FiredEvent>> {
            get: EventsPageViewModel::selected_event,
            set: EventsPageViewModel::set_selected_event
        },
        SelectedNode: Option<Rc<EventTreeNodeBase>> {
            get: EventsPageViewModel::selected_node,
            set: EventsPageViewModel::set_selected_node
        },
        EventsFilter: Rc<FilterViewModel> { get: EventsPageViewModel::events_filter },
        MainView: Option<Rc<MainViewModel>> { get: EventsPageViewModel::main_view },
    ],
    methods: [
        fn Clear() => EventsPageViewModel::clear,
        fn DisableAll() => EventsPageViewModel::disable_all,
        fn EnableDefault() => EventsPageViewModel::enable_default,
        fn RequestTreeNavigateTo(Rc<EventChainLink>) =>
            (|this: &Rc<EventsPageViewModel>, nav_target: Rc<EventChainLink>| this.request_tree_navigate_to(&nav_target)),
        fn SelectEventByType(RoutedEvent) => EventsPageViewModel::select_event_by_type,
    ],
    notify_property_changed: EventsPageViewModel,
});
