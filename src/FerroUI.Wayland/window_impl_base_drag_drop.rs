//! The drags that enter a surface (the port of `WindowImplBase.DragDrop.cs`):
//! the events of the data device become the raw drag events of the
//! framework, and what the framework answers goes back to the offer.
//!
//! The reference guards the pending motion with a lock, from a time when the
//! worker wrote to it. Every member of a sink runs on the UI thread (the
//! proxy posts them), so the port has plain cells.

use crate::clipboard::wayland_data_transfer::WaylandDataTransfer;
use crate::clipboard::wayland_outgoing_transfer::WaylandOutgoingTransfer;
use crate::server::transient::clipboard::wayland_offer_cookie::WaylandOfferCookie;
use crate::window_impl_base::{RawEvent, Sink};
use ferroui_base::input::raw::{IDragDropDevice, IRawInputEventArgs, RawDragEvent, RawDragEventType, RawInputEventArgs};
use ferroui_base::input::{DragDropDevice, DragDropEffects, IDataTransfer, IInputRoot, RawInputModifiers};
use ferroui_base::Point;
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// DnD session state of a sink.
#[derive(Default)]
pub(crate) struct DragDropState {
    current_drag_data_transfer: RefCell<Option<Rc<dyn IDataTransfer>>>,
    current_drag_source_effects: Cell<DragDropEffects>,
    pending_drag_motion: RefCell<Option<Rc<WaylandDragMotionArgs>>>,
    current_dnd_cookie: RefCell<Option<Arc<WaylandOfferCookie>>>,
}

fn drag_drop_device() -> Rc<dyn IDragDropDevice> {
    DragDropDevice::instance()
}

impl Sink {
    /// Handles DnD-related dispatch in `dispatch_input`.
    /// Returns true if the event was DnD and was fully handled.
    pub(crate) fn handle_drag_drop_dispatch(&self, args: &RawEvent, input_root: &Rc<dyn IInputRoot>) -> bool {
        // DnD motion debouncing: consume latest position, create the raw drag event
        if let Some(drag_motion) = args.downcast_ref::<WaylandDragMotionArgs>() {
            let position = drag_motion.position.get();
            let modifiers = drag_motion.modifiers.get();
            drag_motion.consumed.set(true);

            let over_event = Rc::new(RawDragEvent::new(
                drag_drop_device(),
                RawDragEventType::DragOver,
                input_root.clone(),
                position,
                drag_motion.data_transfer.clone(),
                drag_motion.source_effects,
                modifiers,
            ));
            if let Some(parent) = self.parent() {
                parent.base().raise_input(over_event.clone());
            }

            let effects = over_event.effects();
            if let Some(offer_cookie) = &drag_motion.offer_cookie {
                offer_cookie.post_update_feedback(effects);
            }
            return true;
        }

        false
    }

    /// Post-dispatch feedback for DnD enter/drop events.
    /// Called after the input callback of the window has processed the event.
    /// Returns true if the event was DnD and feedback was sent.
    pub(crate) fn handle_drag_drop_post_dispatch(&self, args: &RawEvent) -> bool {
        let Some(drag_event) = args.downcast_ref::<WaylandRawDragEvent>() else {
            return false;
        };

        // Route DnD feedback through the session cookie, which resolves
        // the correct data device on the Wayland thread.
        match drag_event.type_() {
            RawDragEventType::DragEnter => {
                let effects = drag_event.effects();
                if let Some(offer_cookie) = &drag_event.offer_cookie {
                    offer_cookie.post_update_feedback(effects);
                }
            }
            RawDragEventType::Drop => {
                if let Some(offer_cookie) = &drag_event.offer_cookie {
                    offer_cookie.post_finish();
                }
            }
            _ => {}
        }

        true
    }

    pub(crate) fn on_drag_enter(
        &self,
        position: Point,
        mime_types: Vec<String>,
        offer_cookie: Arc<WaylandOfferCookie>,
        source_effects: DragDropEffects,
        modifiers: RawInputModifiers,
    ) {
        let Some(input_root) = self.input_root() else {
            return;
        };

        let state = &self.drag_drop;
        *state.current_dnd_cookie.borrow_mut() = Some(offer_cookie.clone());

        // Check for in-process drag source to avoid deadlock: if the offer contains
        // our sentinel MIME type, use the original data transfer directly instead of
        // creating a pipe-based one (whose reads would deadlock because
        // the source's send handler dispatches to the same UI thread).
        let data_transfer: Rc<dyn IDataTransfer> = WaylandOutgoingTransfer::try_get_in_process_transfer(&mime_types)
            .unwrap_or_else(|| WaylandDataTransfer::new(mime_types, offer_cookie.clone()));
        *state.current_drag_data_transfer.borrow_mut() = Some(data_transfer.clone());
        state.current_drag_source_effects.set(source_effects);
        *state.pending_drag_motion.borrow_mut() = None;

        self.schedule_input(Rc::new(WaylandRawDragEvent::new(
            RawDragEventType::DragEnter,
            input_root,
            position,
            data_transfer,
            source_effects,
            modifiers,
            Some(offer_cookie),
        )));
    }

    pub(crate) fn on_drag_motion(&self, position: Point, modifiers: RawInputModifiers) {
        let state = &self.drag_drop;
        if let Some(pending) = state.pending_drag_motion.borrow().as_ref() {
            if !pending.consumed.get() {
                pending.position.set(position);
                pending.modifiers.set(modifiers);
                return;
            }
        }

        let (Some(input_root), Some(data_transfer)) = (self.input_root(), state.current_drag_data_transfer.borrow().clone()) else {
            return;
        };

        let args = Rc::new(WaylandDragMotionArgs {
            base: RawInputEventArgs::new(drag_drop_device(), 0, input_root),
            position: Cell::new(position),
            modifiers: Cell::new(modifiers),
            consumed: Cell::new(false),
            data_transfer,
            source_effects: state.current_drag_source_effects.get(),
            offer_cookie: state.current_dnd_cookie.borrow().clone(),
        });
        *state.pending_drag_motion.borrow_mut() = Some(args.clone());
        self.schedule_input(args);
    }

    pub(crate) fn on_drag_leave(&self) {
        let state = &self.drag_drop;
        let (Some(input_root), Some(data_transfer)) = (self.input_root(), state.current_drag_data_transfer.borrow().clone()) else {
            return;
        };

        *state.pending_drag_motion.borrow_mut() = None;
        self.schedule_input(Rc::new(RawDragEvent::new(
            drag_drop_device(),
            RawDragEventType::DragLeave,
            input_root,
            Point::default(),
            data_transfer,
            DragDropEffects::NONE,
            RawInputModifiers::NONE,
        )));
        *state.current_drag_data_transfer.borrow_mut() = None;
        *state.current_dnd_cookie.borrow_mut() = None;
    }

    pub(crate) fn on_drop(&self, position: Point, modifiers: RawInputModifiers) {
        let state = &self.drag_drop;
        let (Some(input_root), Some(data_transfer)) = (self.input_root(), state.current_drag_data_transfer.borrow().clone()) else {
            return;
        };

        *state.pending_drag_motion.borrow_mut() = None;
        let offer_cookie = state.current_dnd_cookie.borrow().clone();
        self.schedule_input(Rc::new(WaylandRawDragEvent::new(
            RawDragEventType::Drop,
            input_root,
            position,
            data_transfer,
            state.current_drag_source_effects.get(),
            modifiers,
            offer_cookie,
        )));
        *state.current_drag_data_transfer.borrow_mut() = None;
        *state.current_dnd_cookie.borrow_mut() = None;
    }
}

/// A raw drag event that carries the offer cookie
/// through the event queue so the UI thread can route DnD feedback.
pub(crate) struct WaylandRawDragEvent {
    base: RawDragEvent,
    offer_cookie: Option<Arc<WaylandOfferCookie>>,
}

impl WaylandRawDragEvent {
    #[allow(clippy::too_many_arguments)]
    fn new(
        type_: RawDragEventType,
        root: Rc<dyn IInputRoot>,
        location: Point,
        data_transfer: Rc<dyn IDataTransfer>,
        effects: DragDropEffects,
        modifiers: RawInputModifiers,
        offer_cookie: Option<Arc<WaylandOfferCookie>>,
    ) -> Self {
        Self {
            base: RawDragEvent::new(drag_drop_device(), type_, root, location, data_transfer, effects, modifiers),
            offer_cookie,
        }
    }
}

impl std::ops::Deref for WaylandRawDragEvent {
    type Target = RawDragEvent;

    fn deref(&self) -> &RawDragEvent {
        &self.base
    }
}

impl IRawInputEventArgs for WaylandRawDragEvent {
    fn as_raw_input_event_args(&self) -> &RawInputEventArgs {
        IRawInputEventArgs::as_raw_input_event_args(&self.base)
    }

    fn query_args(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<WaylandRawDragEvent>() {
            Some(self)
        } else {
            IRawInputEventArgs::query_args(&self.base, type_id)
        }
    }
}

/// Specialized event args for DnD motion debouncing.
///
/// While an instance is in the dispatch queue and not consumed, a later motion updates
/// its position and modifiers in place. When the event is dispatched, the latest values
/// are read and `consumed` is set: a motion after that makes a new instance.
pub(crate) struct WaylandDragMotionArgs {
    base: RawInputEventArgs,
    position: Cell<Point>,
    modifiers: Cell<RawInputModifiers>,
    consumed: Cell<bool>,
    data_transfer: Rc<dyn IDataTransfer>,
    source_effects: DragDropEffects,
    offer_cookie: Option<Arc<WaylandOfferCookie>>,
}

impl IRawInputEventArgs for WaylandDragMotionArgs {
    fn as_raw_input_event_args(&self) -> &RawInputEventArgs {
        &self.base
    }

    fn query_args(&self, type_id: TypeId) -> Option<&dyn Any> {
        if type_id == TypeId::of::<WaylandDragMotionArgs>() {
            Some(self)
        } else {
            IRawInputEventArgs::query_args(&self.base, type_id)
        }
    }
}
