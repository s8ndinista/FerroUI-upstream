//! The seats of a connection and their devices (the port of
//! `WaylandInputDispatcher.cs`).
//!
//! The reference keeps the events of a pointer or touch frame as closures and
//! runs them at `frame`. Here they are values (`PointerAction`,
//! `TouchAction`) that are applied at `frame`, in their order: the handlers
//! of the port cannot be captured by closures while the state of the worker
//! is borrowed for the dispatch.
//!
//! The text input facade belongs to the last part of stage 2 of
//! `docs/porting/wayland-platform.md`.

use ferroui_base::input::raw::RawPointerEventType;
use ferroui_base::input::RawInputModifiers;
use ferroui_base::Vector;

// Linux input-event-codes.h button constants
const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;
const BTN_MIDDLE: u32 = 0x112;
const BTN_SIDE: u32 = 0x113;
const BTN_EXTRA: u32 = 0x114;

/// The raw event of a button of a pointer, `None` for a button the framework has no event for.
pub fn map_button(button: u32, pressed: bool) -> Option<RawPointerEventType> {
    Some(match (button, pressed) {
        (BTN_LEFT, true) => RawPointerEventType::LeftButtonDown,
        (BTN_LEFT, false) => RawPointerEventType::LeftButtonUp,
        (BTN_RIGHT, true) => RawPointerEventType::RightButtonDown,
        (BTN_RIGHT, false) => RawPointerEventType::RightButtonUp,
        (BTN_MIDDLE, true) => RawPointerEventType::MiddleButtonDown,
        (BTN_MIDDLE, false) => RawPointerEventType::MiddleButtonUp,
        (BTN_SIDE, true) => RawPointerEventType::XButton1Down,
        (BTN_SIDE, false) => RawPointerEventType::XButton1Up,
        (BTN_EXTRA, true) => RawPointerEventType::XButton2Down,
        (BTN_EXTRA, false) => RawPointerEventType::XButton2Up,
        _ => return None,
    })
}

/// The modifier a button of a pointer is while it is down.
pub fn button_to_modifier(button: u32) -> RawInputModifiers {
    match button {
        BTN_LEFT => RawInputModifiers::LEFT_MOUSE_BUTTON,
        BTN_RIGHT => RawInputModifiers::RIGHT_MOUSE_BUTTON,
        BTN_MIDDLE => RawInputModifiers::MIDDLE_MOUSE_BUTTON,
        BTN_SIDE => RawInputModifiers::X_BUTTON_1_MOUSE_BUTTON,
        BTN_EXTRA => RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON,
        _ => RawInputModifiers::NONE,
    }
}

// Standard XKB modifier bit positions
const MOD_SHIFT: u32 = 1 << 0;
const MOD_CONTROL: u32 = 1 << 2;
const MOD_ALT: u32 = 1 << 3; // Mod1
const MOD_META: u32 = 1 << 6; // Mod4 / Super

/// The keyboard modifiers of the effective mask of `wl_keyboard.modifiers`.
pub fn modifiers_from_xkb_mask(mask: u32) -> RawInputModifiers {
    let mut mods = RawInputModifiers::NONE;
    if mask & MOD_SHIFT != 0 {
        mods |= RawInputModifiers::SHIFT;
    }
    if mask & MOD_CONTROL != 0 {
        mods |= RawInputModifiers::CONTROL;
    }
    if mask & MOD_ALT != 0 {
        mods |= RawInputModifiers::ALT;
    }
    if mask & MOD_META != 0 {
        mods |= RawInputModifiers::META;
    }
    mods
}

/// The axis of a scroll event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollAxis {
    Vertical,
    Horizontal,
}

/// The scroll events of one pointer frame (protocol mandates combining within a frame).
///
/// Wayland sends two related events for scrolling:
///  - axis_value120 (since v8) — discrete-step indicator: 120 == one wheel detent.
///    Replaces the deprecated axis_discrete in v8+. Sent for wheel sources.
///  - axis — continuous magnitude in surface-local pixel-equivalent units. Sent for
///    every scroll source (wheel + touchpad/continuous), regardless of version.
///
/// For the framework (1.0 == one detent), value120/120 is the right unit when present.
/// For continuous sources (touchpad), there is no value120 and we scale the raw axis
/// value by the units per detent. We accumulate the two streams independently and combine at frame
/// flush time, preferring v120 per axis when present. This is robust to either event order.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AxisFrame {
    has_axis: bool,
    timestamp: u64,
    raw_x: f64,
    raw_y: f64,
    v120_x: f64,
    v120_y: f64,
    v120_seen_x: bool,
    v120_seen_y: bool,
}

impl AxisFrame {
    // Logical px per detent for continuous sources, as assumed by GTK3, Qt, SDL and Chromium
    const AXIS_UNITS_PER_DETENT: f64 = 10.0;

    /// `wl_pointer.axis`. Returns whether this is the first scroll event of the frame: the
    /// place of the wheel event among the events of the frame.
    pub fn axis(&mut self, time: u32, axis: ScrollAxis, value: f64) -> bool {
        self.timestamp = u64::from(time);
        match axis {
            ScrollAxis::Vertical => self.raw_y += value,
            ScrollAxis::Horizontal => self.raw_x += value,
        }
        self.mark()
    }

    /// `wl_pointer.axis_discrete`: wl_pointer v5–v7 discrete-step indicator. 1 == one wheel detent.
    /// Deprecated and not sent in v8+ (replaced by axis_value120). Folded into the v120
    /// accumulator (since their semantics align: 1 detent each).
    pub fn axis_discrete(&mut self, axis: ScrollAxis, discrete: i32) -> bool {
        self.add_steps(axis, f64::from(discrete));
        self.mark()
    }

    /// `wl_pointer.axis_value120`: higher-resolution scroll, 120 units = one notch. Multiple
    /// value120 events for the same axis in a frame must be summed (per spec example: a single
    /// hardware event can produce -240 = two negative notches).
    pub fn axis_value120(&mut self, axis: ScrollAxis, value120: i32) -> bool {
        self.add_steps(axis, f64::from(value120) / 120.0);
        self.mark()
    }

    fn add_steps(&mut self, axis: ScrollAxis, steps: f64) {
        match axis {
            ScrollAxis::Vertical => {
                self.v120_y += steps;
                self.v120_seen_y = true;
            }
            ScrollAxis::Horizontal => {
                self.v120_x += steps;
                self.v120_seen_x = true;
            }
        }
    }

    fn mark(&mut self) -> bool {
        !std::mem::replace(&mut self.has_axis, true)
    }

    /// The time of the last `axis` event of the frame.
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    /// The wheel delta of the frame, `None` when it is zero.
    ///
    /// Combine v120 (notches) with raw axis (continuous). Per axis, prefer v120
    /// if any v120 event fired this frame; otherwise scale the raw axis.
    pub fn delta(&self) -> Option<Vector> {
        let delta_x = if self.v120_seen_x { self.v120_x } else { self.raw_x / Self::AXIS_UNITS_PER_DETENT };
        let delta_y = if self.v120_seen_y { self.v120_y } else { self.raw_y / Self::AXIS_UNITS_PER_DETENT };
        if delta_x != 0.0 || delta_y != 0.0 {
            // Wayland: positive = scroll down/right. The framework: positive Y = scroll up.
            Some(Vector::new(-delta_x, -delta_y))
        } else {
            None
        }
    }
}

#[cfg(target_os = "linux")]
pub use imp::*;

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use crate::server::persistent::i_w_surface_event_sink::{PlatformInputEventCookie, WSurfaceEventSinkProxy};
    use crate::server::persistent::w_surface::{shell_surface_of, WSurfaceId, WXdgPopup, WXdgShellSurface, WXdgTopLevel, WlSurfaceData};
    use crate::server::persistent::wayland_cursor::{resolve_cursor, WaylandCursorId, WaylandCursors};
    use crate::server::persistent::wayland_input_event_cookie::WaylandInputEventCookie;
    use crate::server::transient::clipboard::wayland_data_device::WaylandDataDevice;
    use crate::server::transient::clipboard::wayland_data_offer::WaylandDataOffer;
    use crate::server::transient::wayland_cursor_manager::WaylandCursorManager;
    use crate::server::transient::wayland_globals::WaylandGlobals;
    use crate::server::transient::wayland_input_dispatcher_keyboard::KeyboardHandler;
    use crate::server::wayland_worker::{WaylandWorker, WaylandWorkerState};
    use ferroui_base::input::StandardCursorType;
    use ferroui_base::Point;
    use std::collections::HashMap;
    use std::sync::Arc;
    use wayland_client::backend::ObjectId;
    use wayland_client::protocol::wl_data_device_manager::WlDataDeviceManager;
    use wayland_client::protocol::wl_pointer::{self, WlPointer};
    use wayland_client::protocol::wl_registry::WlRegistry;
    use wayland_client::protocol::wl_seat::{self, WlSeat};
    use wayland_client::protocol::wl_surface::WlSurface;
    use wayland_client::protocol::wl_touch::{self, WlTouch};
    use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};

    /// The shell surface a `wl_surface` belongs to, by the user data it was created with.
    pub(crate) fn find_surface_for_wl_surface(surface: &WlSurface) -> Option<WSurfaceId> {
        match surface.data::<WlSurfaceData>() {
            Some(WlSurfaceData::Shell(id)) => Some(*id),
            _ => None,
        }
    }

    /// What the handlers of a seat need of the rest of the worker while they hand out events.
    pub(crate) struct InputContext<'a> {
        pub top_levels: &'a HashMap<WSurfaceId, WXdgTopLevel>,
        pub popups: &'a HashMap<WSurfaceId, WXdgPopup>,
        pub cursors: &'a WaylandCursors,
        pub cursor_manager: &'a WaylandCursorManager,
        pub connection_id: u64,
        pub worker: &'a Arc<WaylandWorker>,
    }

    impl InputContext<'_> {
        /// The shell surface of a top-level or of a popup.
        pub fn shell(&self, surface: WSurfaceId) -> Option<&WXdgShellSurface> {
            shell_surface_of(self.top_levels, self.popups, surface)
        }

        pub fn sink_of(&self, surface: WSurfaceId) -> Option<WSurfaceEventSinkProxy> {
            self.shell(surface).map(|shell| shell.event_sink().clone())
        }

        fn cursor_of(&self, surface: WSurfaceId) -> Option<WaylandCursorId> {
            self.shell(surface).and_then(|shell| shell.surface().current_cursor())
        }
    }

    pub struct WaylandInputDispatcher {
        seats: HashMap<u32, Seat>,
    }

    impl WaylandInputDispatcher {
        pub fn new() -> Self {
            Self { seats: HashMap::new() }
        }

        pub(crate) fn notify_cursor_changed(&self, surface: WSurfaceId, cx: &InputContext<'_>) {
            for seat in self.seats.values() {
                seat.notify_cursor_changed(surface, cx);
            }
        }

        /// Sets the pointer cursor to provide visual DnD action feedback.
        /// Called from the Wayland thread when `wl_data_source.action` fires during an active drag.
        pub(crate) fn set_dnd_cursor(&self, cursor_type: StandardCursorType, cursor_manager: &WaylandCursorManager) {
            for seat in self.seats.values() {
                seat.set_dnd_cursor(cursor_type, cursor_manager);
            }
        }

        /// Returns the `wl_surface` that currently has pointer focus.
        /// Used as the origin surface for `wl_data_device.start_drag`.
        pub(crate) fn find_origin_surface(&self) -> Option<WlSurface> {
            self.seats_in_order().find_map(|seat| seat.pointer_focused_wl_surface().cloned())
        }

        /// The seats in the order of their registry names (the reference has them in the
        /// order they were announced).
        fn seats_in_order(&self) -> impl Iterator<Item = &Seat> {
            let mut names: Vec<u32> = self.seats.keys().copied().collect();
            names.sort_unstable();
            names.into_iter().filter_map(|name| self.seats.get(&name))
        }

        /// Returns the first available data device, or `None` if no seat has one.
        /// Used by the clipboard implementation to access the active selection.
        pub(crate) fn get_data_device(&self) -> Option<&WaylandDataDevice> {
            self.seats_in_order().find_map(|seat| seat.data_device.as_ref())
        }

        /// The first available data device, to change it.
        pub(crate) fn get_data_device_mut(&mut self) -> Option<&mut WaylandDataDevice> {
            let seat = self.get_data_device()?.seat();
            self.data_device_of_mut(seat)
        }

        /// The data device of a seat.
        pub(crate) fn data_device_of_mut(&mut self, seat: u32) -> Option<&mut WaylandDataDevice> {
            self.seats.get_mut(&seat)?.data_device.as_mut()
        }

        /// The offer of the identity `id`, whichever data device has it.
        pub(crate) fn find_data_offer_mut(&mut self, id: &ObjectId) -> Option<&mut WaylandDataOffer> {
            self.seats.values_mut().find_map(|seat| seat.data_device.as_mut()?.find_offer_mut(id))
        }

        #[allow(clippy::too_many_arguments)]
        pub(crate) fn on_seat_added(
            &mut self,
            global_name: u32,
            registry: &WlRegistry,
            version: u32,
            queue_handle: &QueueHandle<WaylandWorkerState>,
            data_device_manager: Option<&WlDataDeviceManager>,
            worker: &Arc<WaylandWorker>,
        ) {
            let mut seat = Seat::new(registry, global_name, version, queue_handle);
            seat.ensure_data_device(data_device_manager, worker, queue_handle);
            self.seats.insert(global_name, seat);
        }

        /// Called once after the globals have been bound. Backfills the data devices for
        /// any seats that were announced before this point. The reference also makes the
        /// text input facade here, which belongs to the last part of stage 2.
        pub(crate) fn on_initial_globals_bound(
            &mut self,
            data_device_manager: Option<&WlDataDeviceManager>,
            worker: &Arc<WaylandWorker>,
            queue_handle: &QueueHandle<WaylandWorkerState>,
        ) {
            for seat in self.seats.values_mut() {
                seat.ensure_data_device(data_device_manager, worker, queue_handle);
            }
        }

        /// Releases a seat that was taken away. The source its data device had set as the
        /// selection is returned for the caller to dispose.
        pub(crate) fn on_seat_removed(&mut self, global_name: u32) -> Option<ObjectId> {
            self.seats.remove(&global_name).and_then(Seat::dispose)
        }

        /// Releases every seat. The data sources are disposed by the globals, which keep them.
        pub fn dispose(&mut self) {
            for (_, seat) in self.seats.drain() {
                seat.dispose();
            }
        }

        pub(crate) fn seat_mut(&mut self, global_name: u32) -> Option<&mut Seat> {
            self.seats.get_mut(&global_name)
        }
    }

    impl Default for WaylandInputDispatcher {
        fn default() -> Self {
            Self::new()
        }
    }

    pub(crate) struct Seat {
        global_name: u32,
        wl_seat: WlSeat,
        pointer_handler: Option<PointerHandler>,
        touch_handler: Option<TouchHandler>,
        pub(crate) keyboard_handler: Option<KeyboardHandler>,
        pub(crate) data_device: Option<WaylandDataDevice>,
        pub(crate) keyboard_modifiers: RawInputModifiers,
    }

    impl Seat {
        fn new(registry: &WlRegistry, global_name: u32, version: u32, queue_handle: &QueueHandle<WaylandWorkerState>) -> Self {
            let wl_seat: WlSeat = registry.bind(global_name, version, queue_handle, global_name);
            Self {
                global_name,
                wl_seat,
                pointer_handler: None,
                touch_handler: None,
                keyboard_handler: None,
                data_device: None,
                keyboard_modifiers: RawInputModifiers::NONE,
            }
        }

        /// The `wl_surface` currently under pointer focus, exposed for DnD origin lookup.
        fn pointer_focused_wl_surface(&self) -> Option<&WlSurface> {
            self.pointer_handler.as_ref()?.focused_wl_surface.as_ref()
        }

        /// Creates the data device for this seat if a `wl_data_device_manager` is available
        /// and the device hasn't been created yet. Called when the seat is made and also after
        /// the manager global becomes available (seats can be announced before the manager).
        fn ensure_data_device(
            &mut self,
            data_device_manager: Option<&WlDataDeviceManager>,
            worker: &Arc<WaylandWorker>,
            queue_handle: &QueueHandle<WaylandWorkerState>,
        ) {
            if self.data_device.is_some() {
                return;
            }

            let Some(manager) = data_device_manager else {
                return;
            };

            let wl_data_device = manager.get_data_device(&self.wl_seat, queue_handle, self.global_name);
            self.data_device = Some(WaylandDataDevice::new(wl_data_device, self.global_name, worker.clone()));
        }

        /// Records the serial of an input event that can authorize `set_selection`.
        pub(crate) fn note_input_serial(&mut self, serial: u32) {
            if let Some(data_device) = &mut self.data_device {
                data_device.last_input_serial = serial;
            }
        }

        fn set_dnd_cursor(&self, cursor_type: StandardCursorType, cursor_manager: &WaylandCursorManager) {
            if let Some(handler) = &self.pointer_handler {
                handler.set_dnd_cursor(cursor_type, cursor_manager);
            }
        }

        fn on_capabilities(&mut self, capabilities: wl_seat::Capability, queue_handle: &QueueHandle<WaylandWorkerState>) {
            let has_pointer = capabilities.contains(wl_seat::Capability::Pointer);
            if has_pointer && self.pointer_handler.is_none() {
                self.pointer_handler = Some(PointerHandler::new(&self.wl_seat, self.global_name, queue_handle));
            } else if !has_pointer {
                if let Some(handler) = self.pointer_handler.take() {
                    handler.dispose();
                }
            }

            let has_touch = capabilities.contains(wl_seat::Capability::Touch);
            if has_touch && self.touch_handler.is_none() {
                self.touch_handler = Some(TouchHandler::new(&self.wl_seat, self.global_name, queue_handle));
            } else if !has_touch {
                if let Some(handler) = self.touch_handler.take() {
                    handler.dispose();
                }
            }

            let has_keyboard = capabilities.contains(wl_seat::Capability::Keyboard);
            if has_keyboard && self.keyboard_handler.is_none() {
                self.keyboard_handler = Some(KeyboardHandler::new(&self.wl_seat, self.global_name, queue_handle));
            } else if !has_keyboard {
                if let Some(handler) = self.keyboard_handler.take() {
                    handler.dispose();
                }
            }
        }

        /// Releases the seat and its devices. The source the data device had set as the
        /// selection is returned for the caller to dispose.
        fn dispose(self) -> Option<ObjectId> {
            if let Some(handler) = self.pointer_handler {
                handler.dispose();
            }
            if let Some(handler) = self.touch_handler {
                handler.dispose();
            }
            if let Some(handler) = self.keyboard_handler {
                handler.dispose();
            }
            let active_source = self.data_device.and_then(WaylandDataDevice::dispose);
            // `wl_seat.release` exists from version 5, the least the seat is bound with.
            self.wl_seat.release();
            active_source
        }

        fn notify_cursor_changed(&self, surface: WSurfaceId, cx: &InputContext<'_>) {
            if let Some(handler) = &self.pointer_handler {
                handler.notify_cursor_changed(surface, cx);
            }
        }

    }

    /// An event of a pointer frame, kept until the frame ends.
    enum PointerAction {
        Enter { serial: u32, surface: WlSurface, shell_surface: Option<WSurfaceId>, position: Point },
        Leave { serial: u32 },
        Motion { time: u32, position: Point, modifiers: RawInputModifiers },
        Button {
            time: u32,
            serial: u32,
            type_: RawPointerEventType,
            modifiers: RawInputModifiers,
            position: Point,
            cookie: Option<PlatformInputEventCookie>,
        },
        /// The place of the combined wheel event of the frame.
        Axis,
    }

    struct PointerHandler {
        pointer: WlPointer,

        // Persistent pointer state (survives across frames)
        focused_sink: Option<WSurfaceEventSinkProxy>,
        focused_surface: Option<WSurfaceId>,
        focused_wl_surface: Option<WlSurface>,
        pointer_position: Point,
        modifiers: RawInputModifiers,
        last_enter_serial: u32,

        // Per-frame: ordered event actions dispatched during Frame
        frame_actions: Vec<PointerAction>,
        // Per-frame: axis accumulation
        frame_axis: AxisFrame,
    }

    impl PointerHandler {
        fn new(wl_seat: &WlSeat, seat_name: u32, queue_handle: &QueueHandle<WaylandWorkerState>) -> Self {
            Self {
                pointer: wl_seat.get_pointer(queue_handle, seat_name),
                focused_sink: None,
                focused_surface: None,
                focused_wl_surface: None,
                pointer_position: Point::default(),
                modifiers: RawInputModifiers::NONE,
                last_enter_serial: 0,
                frame_actions: Vec::new(),
                frame_axis: AxisFrame::default(),
            }
        }

        fn update_cursor(&self, cx: &InputContext<'_>) {
            let Some(focused_surface) = self.focused_surface else {
                return;
            };

            match resolve_cursor(cx.cursor_of(focused_surface), cx.cursors, cx.cursor_manager) {
                Some(image) => {
                    self.pointer.set_cursor(self.last_enter_serial, Some(&image.surface), image.hotspot_x, image.hotspot_y);
                }
                None => self.pointer.set_cursor(self.last_enter_serial, None, 0, 0),
            }
        }

        fn notify_cursor_changed(&self, surface: WSurfaceId, cx: &InputContext<'_>) {
            if self.focused_surface == Some(surface) {
                self.update_cursor(cx);
            }
        }

        /// Sets the pointer cursor to reflect the current DnD action.
        /// Called from the `action` callback of a data source during an active drag source operation.
        ///
        /// TODO: Weston does not honor `wl_pointer.set_cursor` during drag.
        /// To support Weston, we'd need to attach cursor buffers to the drag icon surface
        /// passed to `wl_data_device.start_drag`. Our current approach matches Qt behavior
        /// and works correctly with KWin/Mutter. Weston compat is low priority.
        fn set_dnd_cursor(&self, cursor_type: StandardCursorType, cursor_manager: &WaylandCursorManager) {
            match cursor_manager.get_cursor(cursor_type) {
                Some(cursor) => {
                    self.pointer.set_cursor(self.last_enter_serial, Some(&cursor.surface), cursor.hotspot_x, cursor.hotspot_y);
                }
                None => self.pointer.set_cursor(self.last_enter_serial, None, 0, 0),
            }
        }

        fn dispose(self) {
            self.pointer.release();
        }

        fn enqueue_axis_dispatch(&mut self, first: bool) {
            if first {
                self.frame_actions.push(PointerAction::Axis);
            }
        }

        fn on_event(&mut self, event: wl_pointer::Event, seat: &SeatInfo<'_>, cx: &InputContext<'_>) {
            match event {
                wl_pointer::Event::Enter { serial, surface, surface_x, surface_y } => {
                    let shell_surface = find_surface_for_wl_surface(&surface);
                    let position = Point::new(surface_x, surface_y);
                    self.pointer_position = position;
                    self.frame_actions.push(PointerAction::Enter { serial, surface, shell_surface, position });
                }
                wl_pointer::Event::Leave { serial, .. } => {
                    self.frame_actions.push(PointerAction::Leave { serial });
                }
                wl_pointer::Event::Motion { time, surface_x, surface_y } => {
                    let position = Point::new(surface_x, surface_y);
                    self.pointer_position = position;
                    let modifiers = self.modifiers | seat.keyboard_modifiers;
                    self.frame_actions.push(PointerAction::Motion { time, position, modifiers });
                }
                wl_pointer::Event::Button { serial, time, button, state } => {
                    let pressed = state == WEnum::Value(wl_pointer::ButtonState::Pressed);
                    let Some(type_) = map_button(button, pressed) else {
                        return;
                    };

                    let modifier = button_to_modifier(button);
                    if pressed {
                        self.modifiers |= modifier;
                    } else {
                        self.modifiers &= !modifier;
                    }

                    let modifiers = self.modifiers | seat.keyboard_modifiers;
                    let position = self.pointer_position;
                    let cookie = pressed.then(|| {
                        let cookie: PlatformInputEventCookie =
                            Arc::new(WaylandInputEventCookie::new(seat.wl_seat.clone(), serial, cx.connection_id, cx.worker.clone()));
                        cookie
                    });
                    self.frame_actions.push(PointerAction::Button { time, serial, type_, modifiers, position, cookie });
                }
                wl_pointer::Event::Axis { time, axis, value } => {
                    let first = match axis {
                        WEnum::Value(wl_pointer::Axis::VerticalScroll) => self.frame_axis.axis(time, ScrollAxis::Vertical, value),
                        WEnum::Value(wl_pointer::Axis::HorizontalScroll) => {
                            self.frame_axis.axis(time, ScrollAxis::Horizontal, value)
                        }
                        _ => return,
                    };
                    self.enqueue_axis_dispatch(first);
                }
                wl_pointer::Event::AxisDiscrete { axis, discrete } => {
                    let first = match axis {
                        WEnum::Value(wl_pointer::Axis::VerticalScroll) => {
                            self.frame_axis.axis_discrete(ScrollAxis::Vertical, discrete)
                        }
                        WEnum::Value(wl_pointer::Axis::HorizontalScroll) => {
                            self.frame_axis.axis_discrete(ScrollAxis::Horizontal, discrete)
                        }
                        _ => return,
                    };
                    self.enqueue_axis_dispatch(first);
                }
                wl_pointer::Event::AxisValue120 { axis, value120 } => {
                    let first = match axis {
                        WEnum::Value(wl_pointer::Axis::VerticalScroll) => {
                            self.frame_axis.axis_value120(ScrollAxis::Vertical, value120)
                        }
                        WEnum::Value(wl_pointer::Axis::HorizontalScroll) => {
                            self.frame_axis.axis_value120(ScrollAxis::Horizontal, value120)
                        }
                        _ => return,
                    };
                    self.enqueue_axis_dispatch(first);
                }
                wl_pointer::Event::Frame => self.on_frame(seat, cx),
                // The source and the stop of a scroll are not used.
                _ => {}
            }
        }

        fn on_frame(&mut self, seat: &SeatInfo<'_>, cx: &InputContext<'_>) {
            let actions = std::mem::take(&mut self.frame_actions);
            let frame_axis = std::mem::take(&mut self.frame_axis);
            for action in actions {
                match action {
                    PointerAction::Enter { serial, surface, shell_surface, position } => {
                        self.focused_sink = shell_surface.and_then(|surface| cx.sink_of(surface));
                        self.focused_surface = shell_surface;
                        self.focused_wl_surface = Some(surface);
                        self.last_enter_serial = serial;
                        self.update_cursor(cx);
                        if let Some(sink) = &self.focused_sink {
                            sink.on_pointer_enter(0, serial, position);
                        }
                    }
                    PointerAction::Leave { serial } => {
                        let leave_sink = self.focused_sink.take();
                        self.focused_surface = None;
                        self.focused_wl_surface = None;
                        if let Some(sink) = leave_sink {
                            sink.on_pointer_leave(serial);
                        }
                    }
                    PointerAction::Motion { time, position, modifiers } => {
                        if let Some(sink) = &self.focused_sink {
                            sink.on_pointer_motion(u64::from(time), position, modifiers);
                        }
                    }
                    PointerAction::Button { time, serial, type_, modifiers, position, cookie } => {
                        if let Some(sink) = &self.focused_sink {
                            sink.on_pointer_button(u64::from(time), serial, type_, modifiers, position, cookie);
                        }
                    }
                    PointerAction::Axis => {
                        if let (Some(delta), Some(sink)) = (frame_axis.delta(), &self.focused_sink) {
                            sink.on_pointer_axis(
                                frame_axis.timestamp(),
                                delta,
                                self.modifiers | seat.keyboard_modifiers,
                                self.pointer_position,
                            );
                        }
                    }
                }
            }
        }
    }

    /// What a device handler reads of its seat.
    struct SeatInfo<'a> {
        wl_seat: &'a WlSeat,
        keyboard_modifiers: RawInputModifiers,
    }

    /// An event of a touch frame, kept until the frame ends.
    enum TouchAction {
        Down { time: u32, id: i32, shell_surface: Option<WSurfaceId>, position: Point, cookie: PlatformInputEventCookie },
        Motion { time: u32, id: i32, position: Point },
        Up { time: u32, id: i32 },
        Cancel,
    }

    struct TouchHandler {
        touch: WlTouch,

        // Track active touch point ID → (surface sink, last position)
        active_touch_points: HashMap<i32, (WSurfaceEventSinkProxy, Point)>,

        // Per-frame: ordered event actions dispatched during Frame
        frame_actions: Vec<TouchAction>,
    }

    impl TouchHandler {
        fn new(wl_seat: &WlSeat, seat_name: u32, queue_handle: &QueueHandle<WaylandWorkerState>) -> Self {
            Self { touch: wl_seat.get_touch(queue_handle, seat_name), active_touch_points: HashMap::new(), frame_actions: Vec::new() }
        }

        fn dispose(self) {
            self.touch.release();
        }

        fn on_event(&mut self, event: wl_touch::Event, seat: &SeatInfo<'_>, cx: &InputContext<'_>) {
            match event {
                wl_touch::Event::Down { serial, time, surface, id, x, y } => {
                    let shell_surface = find_surface_for_wl_surface(&surface);
                    let position = Point::new(x, y);
                    let cookie: PlatformInputEventCookie =
                        Arc::new(WaylandInputEventCookie::new(seat.wl_seat.clone(), serial, cx.connection_id, cx.worker.clone()));
                    self.frame_actions.push(TouchAction::Down { time, id, shell_surface, position, cookie });
                }
                wl_touch::Event::Motion { time, id, x, y } => {
                    self.frame_actions.push(TouchAction::Motion { time, id, position: Point::new(x, y) });
                }
                wl_touch::Event::Up { time, id, .. } => self.frame_actions.push(TouchAction::Up { time, id }),
                wl_touch::Event::Cancel => self.frame_actions.push(TouchAction::Cancel),
                wl_touch::Event::Frame => self.on_frame(cx),
                // Shape and orientation info are not currently used
                _ => {}
            }
        }

        fn on_frame(&mut self, cx: &InputContext<'_>) {
            for action in std::mem::take(&mut self.frame_actions) {
                match action {
                    TouchAction::Down { time, id, shell_surface, position, cookie } => {
                        if let Some(sink) = shell_surface.and_then(|surface| cx.sink_of(surface)) {
                            sink.on_touch_down(u64::from(time), id, position, Some(cookie));
                            self.active_touch_points.insert(id, (sink, position));
                        }
                    }
                    TouchAction::Motion { time, id, position } => {
                        if let Some(entry) = self.active_touch_points.get_mut(&id) {
                            entry.1 = position;
                            entry.0.on_touch_move(u64::from(time), id, position);
                        }
                    }
                    TouchAction::Up { time, id } => {
                        if let Some((sink, position)) = self.active_touch_points.remove(&id) {
                            sink.on_touch_up(u64::from(time), id, position);
                        }
                    }
                    TouchAction::Cancel => {
                        for (id, (sink, position)) in self.active_touch_points.drain() {
                            sink.on_touch_cancel(id, position);
                        }
                    }
                }
            }
        }
    }

    /// Splits the state of the worker into the seat of an event and what its handlers need
    /// of the rest.
    pub(crate) fn with_seat<R>(
        state: &mut WaylandWorkerState,
        seat_name: u32,
        f: impl FnOnce(&mut Seat, &InputContext<'_>) -> R,
    ) -> Option<R> {
        let WaylandWorkerState { globals, top_levels, popups, cursors, worker, .. } = state;
        let WaylandGlobals { input_dispatcher, cursor_manager, connection_id, .. } = globals.as_mut()?;
        let cx = InputContext { top_levels, popups, cursors, cursor_manager, connection_id: *connection_id, worker };
        let seat = input_dispatcher.seat_mut(seat_name)?;
        Some(f(seat, &cx))
    }

    /// The listener of a seat; the user data of a seat and of its devices is the registry name
    /// of the seat.
    impl Dispatch<WlSeat, u32> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            _proxy: &WlSeat,
            event: wl_seat::Event,
            data: &u32,
            _conn: &Connection,
            qhandle: &QueueHandle<Self>,
        ) {
            if let wl_seat::Event::Capabilities { capabilities: WEnum::Value(capabilities) } = event {
                with_seat(state, *data, |seat, _| seat.on_capabilities(capabilities, qhandle));
            }
        }
    }

    impl Dispatch<WlPointer, u32> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            _proxy: &WlPointer,
            event: wl_pointer::Event,
            data: &u32,
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            with_seat(state, *data, |seat, cx| {
                if let wl_pointer::Event::Button { serial, .. } = &event {
                    seat.note_input_serial(*serial);
                }
                let Seat { wl_seat, pointer_handler, keyboard_modifiers, .. } = seat;
                if let Some(handler) = pointer_handler {
                    handler.on_event(event, &SeatInfo { wl_seat, keyboard_modifiers: *keyboard_modifiers }, cx);
                }
            });
        }
    }

    impl Dispatch<WlTouch, u32> for WaylandWorkerState {
        fn event(
            state: &mut Self,
            _proxy: &WlTouch,
            event: wl_touch::Event,
            data: &u32,
            _conn: &Connection,
            _qhandle: &QueueHandle<Self>,
        ) {
            with_seat(state, *data, |seat, cx| {
                if let wl_touch::Event::Down { serial, .. } = &event {
                    seat.note_input_serial(*serial);
                }
                let Seat { wl_seat, touch_handler, keyboard_modifiers, .. } = seat;
                if let Some(handler) = touch_handler {
                    handler.on_event(event, &SeatInfo { wl_seat, keyboard_modifiers: *keyboard_modifiers }, cx);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn the_buttons_of_the_kernel_are_the_buttons_of_the_framework() {
        assert_eq!(map_button(0x110, true), Some(RawPointerEventType::LeftButtonDown));
        assert_eq!(map_button(0x110, false), Some(RawPointerEventType::LeftButtonUp));
        assert_eq!(map_button(0x111, true), Some(RawPointerEventType::RightButtonDown));
        assert_eq!(map_button(0x112, false), Some(RawPointerEventType::MiddleButtonUp));
        assert_eq!(map_button(0x113, true), Some(RawPointerEventType::XButton1Down));
        assert_eq!(map_button(0x114, false), Some(RawPointerEventType::XButton2Up));
        assert_eq!(map_button(0x115, true), None);
        assert_eq!(button_to_modifier(0x110), RawInputModifiers::LEFT_MOUSE_BUTTON);
        assert_eq!(button_to_modifier(0x114), RawInputModifiers::X_BUTTON_2_MOUSE_BUTTON);
        assert_eq!(button_to_modifier(1), RawInputModifiers::NONE);
    }

    #[test]
    fn the_modifiers_of_a_keyboard_are_read_from_the_mask() {
        assert_eq!(modifiers_from_xkb_mask(0), RawInputModifiers::NONE);
        assert_eq!(modifiers_from_xkb_mask(1), RawInputModifiers::SHIFT);
        // Caps lock and num lock are not modifiers of the framework.
        assert_eq!(modifiers_from_xkb_mask(2 | 16), RawInputModifiers::NONE);
        assert_eq!(
            modifiers_from_xkb_mask(1 | 4 | 8 | 64),
            RawInputModifiers::SHIFT | RawInputModifiers::CONTROL | RawInputModifiers::ALT | RawInputModifiers::META
        );
    }

    #[test]
    fn a_wheel_step_is_one_unit_upwards_for_a_negative_value() {
        // A wheel on a compositor of version 8: value120 and the continuous value together.
        let mut frame = AxisFrame::default();
        assert!(frame.axis(1000, ScrollAxis::Vertical, 15.0));
        assert!(!frame.axis_value120(ScrollAxis::Vertical, 120));
        assert_eq!(frame.delta(), Some(Vector::new(0.0, -1.0)));
        assert_eq!(frame.timestamp(), 1000);

        // The other order, and two notches in one event.
        let mut frame = AxisFrame::default();
        assert!(frame.axis_value120(ScrollAxis::Vertical, -240));
        assert!(!frame.axis(5, ScrollAxis::Vertical, -30.0));
        assert_eq!(frame.delta(), Some(Vector::new(0.0, 2.0)));
    }

    #[test]
    fn a_discrete_step_counts_as_a_notch_and_a_touchpad_scrolls_by_tenths() {
        let mut frame = AxisFrame::default();
        assert!(frame.axis_discrete(ScrollAxis::Horizontal, 1));
        assert!(!frame.axis(1, ScrollAxis::Horizontal, 15.0));
        assert_eq!(frame.delta(), Some(Vector::new(-1.0, 0.0)));

        // Continuous: no steps, both axes in one frame.
        let mut frame = AxisFrame::default();
        assert!(frame.axis(1, ScrollAxis::Vertical, 5.0));
        assert!(!frame.axis(1, ScrollAxis::Horizontal, -2.5));
        assert!(!frame.axis(2, ScrollAxis::Vertical, 5.0));
        assert_eq!(frame.delta(), Some(Vector::new(0.25, -1.0)));

        // Steps on one axis and a continuous value on the other.
        let mut frame = AxisFrame::default();
        frame.axis_value120(ScrollAxis::Vertical, 120);
        frame.axis(1, ScrollAxis::Vertical, 15.0);
        frame.axis(1, ScrollAxis::Horizontal, 20.0);
        assert_eq!(frame.delta(), Some(Vector::new(-2.0, -1.0)));
    }

    #[test]
    fn a_frame_without_movement_has_no_wheel_event() {
        let mut frame = AxisFrame::default();
        assert_eq!(frame.delta(), None);
        frame.axis(1, ScrollAxis::Vertical, 0.0);
        assert_eq!(frame.delta(), None);
        frame.axis_value120(ScrollAxis::Vertical, 0);
        assert_eq!(frame.delta(), None);
    }
}
