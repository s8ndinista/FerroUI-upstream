//! Port of `HeadlessVncFramebufferSource.cs`: the framebuffer source of a
//! VNC server session, which shows a headless window and passes the input
//! of the client to it.
//!
//! The session calls the source on its own threads, while the window lives
//! on the UI thread. As upstream, the input is posted to the UI thread at
//! input priority. Upstream reads the last rendered frame of the window on
//! the thread of the session; here the window can only be reached on the UI
//! thread, so [`capture`](HeadlessVncFramebufferSource::capture) does its
//! work there and waits for it (the frame itself is kept under a lock by
//! the window, as upstream).
//!
//! One difference: the input events reach the window in the order the
//! client sent them. Upstream posts each event on its own; the input
//! extensions of a headless window run the jobs of the dispatcher before
//! they deliver an event, so an event posted while another is delivered
//! (a release right after a press, as a client that is not a person sends
//! them) is delivered first, and the press after it. Here the events wait
//! in a queue of the source, and each post delivers what is queued, in
//! order, unless a delivery is running already.

use crate::remote_viewing::vnc::server::VncServerSession;
use crate::remote_viewing::vnc::{
    ExtendedDesktopSizeStatus, IVncFramebufferSource, KeyChangedEventArgs, KeySym, VncFramebuffer,
    VncPixelFormat,
};
use ferroui_base::input::{Key, MouseButton, PhysicalKey, RawInputModifiers};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::{NumberFormatInfo, ThreadBound};
use ferroui_base::{PixelRect, Point, Ref, Vector};
use ferroui_controls::Window;
use ferroui_headless::HeadlessWindowExtensions;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

bitflags::bitflags! {
    /// The buttons of a pointer event of the protocol.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    struct VncButton: i32 {
        const LEFT = 1;
        const MIDDLE = 2;
        const RIGHT = 4;
        const SCROLL_UP = 8;
        const SCROLL_DOWN = 16;
    }
}

/// The buttons whose presses and releases are passed on.
const CHECKED_BUTTONS: [VncButton; 3] = [VncButton::LEFT, VncButton::MIDDLE, VncButton::RIGHT];

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// An input event that waits for the UI thread.
type PendingInput = Box<dyn FnOnce(&HeadlessVncFramebufferSource) + Send>;

/// The framebuffer source of a VNC server session that shows a headless
/// window.
pub struct HeadlessVncFramebufferSource {
    this: Weak<HeadlessVncFramebufferSource>,
    /// `Window`: reached on the UI thread only.
    window: ThreadBound<RefCell<Option<Ref<Window>>>>,
    /// `_framebuffer`, under `_lock`.
    framebuffer: Mutex<Arc<VncFramebuffer>>,
    previous_buttons: Mutex<VncButton>,
    key_state: Mutex<RawInputModifiers>,
    /// The input events that wait for the UI thread, in the order they
    /// came.
    pending_input: Mutex<VecDeque<PendingInput>>,
    /// Whether the UI thread delivers input events.
    delivering_input: ThreadBound<Cell<bool>>,
}

impl HeadlessVncFramebufferSource {
    /// Creates the source of `session` for `window`, and handles the
    /// pointer and key events of the session.
    ///
    /// Called on the UI thread, which the window belongs to.
    pub fn new(
        session: &VncServerSession,
        window: Ref<Window>,
    ) -> Arc<HeadlessVncFramebufferSource> {
        let source = Arc::new_cyclic(|this| HeadlessVncFramebufferSource {
            this: this.clone(),
            window: ThreadBound::new(RefCell::new(Some(window))),
            framebuffer: Mutex::new(VncFramebuffer::new("FerroUI", 1, 1, VncPixelFormat::RGB32)),
            previous_buttons: Mutex::new(VncButton::empty()),
            key_state: Mutex::new(RawInputModifiers::NONE),
            pending_input: Mutex::new(VecDeque::new()),
            delivering_input: ThreadBound::new(Cell::new(false)),
        });

        let weak = Arc::downgrade(&source);
        session.pointer_changed().add(move |_, args| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let pt = Point::new(f64::from(args.x()), f64::from(args.y()));

            let buttons = VncButton::from_bits_retain(args.pressed_buttons());

            fn translate_button(vnc_button: VncButton) -> MouseButton {
                if vnc_button == VncButton::LEFT {
                    MouseButton::Left
                } else if vnc_button == VncButton::MIDDLE {
                    MouseButton::Middle
                } else if vnc_button == VncButton::RIGHT {
                    MouseButton::Right
                } else {
                    MouseButton::None
                }
            }

            let modifiers = RawInputModifiers::from_bits_retain((buttons.bits() & 7) << 4);

            this.post_input(move |this| {
                let window = this.window();
                let Some(window) = window else {
                    *lock(&this.previous_buttons) = buttons;
                    return;
                };
                let previous_buttons = *lock(&this.previous_buttons);
                window.mouse_move(pt, RawInputModifiers::NONE);
                for btn in CHECKED_BUTTONS {
                    if previous_buttons.contains(btn) && !buttons.contains(btn) {
                        window.mouse_up(pt, translate_button(btn), modifiers);
                    }
                }

                for btn in CHECKED_BUTTONS {
                    if !previous_buttons.contains(btn) && buttons.contains(btn) {
                        window.mouse_down(pt, translate_button(btn), modifiers);
                    }
                }

                if buttons == VncButton::SCROLL_UP {
                    window.mouse_wheel(pt, Vector::ONE, modifiers);
                } else if buttons == VncButton::SCROLL_DOWN {
                    window.mouse_wheel(pt, Vector::ONE.negate(), modifiers);
                }

                *lock(&this.previous_buttons) = buttons;
            });
        });

        let weak = Arc::downgrade(&source);
        session.key_changed().add(move |_, args| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let is_modifier_key = this.check_key_is_input_modifier(args);
            if is_modifier_key {
                return;
            }

            let (key, key_symbol) = Self::translate_key(args.keysym());
            if key == Key::None {
                return;
            }

            // We only care about text input on key up if not using Ctrl or Alt.
            let key_state = *lock(&this.key_state);
            let input_text = if args.pressed()
                || key_state.contains(RawInputModifiers::CONTROL)
                || key_state.contains(RawInputModifiers::ALT)
            {
                None
            } else {
                key_symbol.clone()
            };

            let pressed = args.pressed();
            this.post_input(move |this| {
                let Some(window) = this.window() else {
                    return;
                };
                let key_state = *lock(&this.key_state);
                if pressed {
                    window.key_press(key, key_state, PhysicalKey::None, key_symbol.as_deref());
                } else {
                    window.key_release(key, key_state, PhysicalKey::None, key_symbol.as_deref());
                }

                if let Some(input_text) = &input_text {
                    window.key_text_input(input_text);
                }
            });
        });

        source
    }

    /// Queues an input event and posts its delivery to the UI thread, at
    /// input priority.
    fn post_input(&self, input: impl FnOnce(&HeadlessVncFramebufferSource) + Send + 'static) {
        lock(&self.pending_input).push_back(Box::new(input));
        let this = self.arc();
        Dispatcher::ui_thread().post(move || this.deliver_input(), DispatcherPriority::INPUT);
    }

    /// Delivers the queued input events in order, on the UI thread. A
    /// delivery that starts while one runs (from the jobs the input
    /// extensions of the window run) leaves the queue to it.
    fn deliver_input(&self) {
        struct Delivering<'a>(&'a Cell<bool>);
        impl Drop for Delivering<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }

        let delivering = self.delivering_input.get();
        if delivering.replace(true) {
            return;
        }
        let _delivering = Delivering(delivering);

        loop {
            let input = lock(&self.pending_input).pop_front();
            let Some(input) = input else {
                return;
            };
            input(self);
        }
    }

    fn arc(&self) -> Arc<HeadlessVncFramebufferSource> {
        self.this
            .upgrade()
            .expect("the source is alive while it is used")
    }

    /// The window the source shows.
    ///
    /// # Panics
    /// On a thread other than the UI thread.
    pub fn window(&self) -> Option<Ref<Window>> {
        self.window.get().borrow().clone()
    }

    /// Sets the window the source shows.
    ///
    /// # Panics
    /// On a thread other than the UI thread.
    pub fn set_window(&self, value: Option<Ref<Window>>) {
        let old = self.window.get().replace(value);
        drop(old);
    }

    /// The framebuffer the source captured last (`_framebuffer`).
    pub fn framebuffer(&self) -> Arc<VncFramebuffer> {
        lock(&self.framebuffer).clone()
    }

    /// Keeps the state of the modifier keys: whether the key of `args` is
    /// one (Shift, Control, Alt), in which case it changes the state.
    fn check_key_is_input_modifier(&self, args: &KeyChangedEventArgs) -> bool {
        let toggle_modifier = match args.keysym() {
            KeySym::ShiftLeft | KeySym::ShiftRight => Some(RawInputModifiers::SHIFT),
            KeySym::ControlLeft | KeySym::ControlRight => Some(RawInputModifiers::CONTROL),
            KeySym::AltLeft | KeySym::AltRight => Some(RawInputModifiers::ALT),
            _ => None,
        };

        let Some(toggle_modifier) = toggle_modifier else {
            return false;
        };

        let mut key_state = lock(&self.key_state);
        if args.pressed() {
            *key_state |= toggle_modifier;
        } else {
            *key_state &= !toggle_modifier;
        }

        true
    }

    /// The key of the framework and the text a key symbol stands for.
    fn translate_key(key: KeySym) -> (Key, Option<String>) {
        let (key, symbol): (Key, Option<&str>) = match key {
            KeySym::Backspace => (Key::Back, Some("\u{8}")),
            KeySym::Tab => (Key::Tab, Some("\t")),
            KeySym::LineFeed => (Key::LineFeed, None),
            KeySym::Clear => (Key::Clear, None),
            KeySym::Return => (Key::Return, Some("\r")),
            KeySym::Pause => (Key::Pause, None),
            KeySym::Escape => (Key::Escape, Some("\u{1B}")),
            KeySym::Delete => (Key::Delete, None),
            KeySym::Home => (Key::Home, None),
            KeySym::Left => (Key::Left, None),
            KeySym::Up => (Key::Up, None),
            KeySym::Right => (Key::Right, None),
            KeySym::Down => (Key::Down, None),
            KeySym::PageUp => (Key::PageUp, None),
            KeySym::PageDown => (Key::PageDown, None),
            KeySym::End => (Key::End, None),
            KeySym::Begin => (Key::Home, None),
            KeySym::Select => (Key::Select, None),
            KeySym::Print => (Key::Print, None),
            KeySym::Execute => (Key::Execute, None),
            KeySym::Insert => (Key::Insert, None),
            KeySym::Cancel => (Key::Cancel, None),
            KeySym::Help => (Key::Help, None),
            KeySym::Break => (Key::Pause, None),
            KeySym::Num_Lock => (Key::NumLock, None),
            KeySym::NumPadSpace => (Key::Space, None),
            KeySym::NumPadTab => (Key::Tab, None),
            KeySym::NumPadEnter => (Key::Enter, None),
            KeySym::NumPadF1 => (Key::F1, None),
            KeySym::NumPadF2 => (Key::F2, None),
            KeySym::NumPadF3 => (Key::F3, None),
            KeySym::NumPadF4 => (Key::F4, None),
            KeySym::NumPadHome => (Key::Home, None),
            KeySym::NumPadLeft => (Key::Left, None),
            KeySym::NumPadUp => (Key::Up, None),
            KeySym::NumPadRight => (Key::Right, None),
            KeySym::NumPadDown => (Key::Down, None),
            KeySym::NumPadPageUp => (Key::PageUp, None),
            KeySym::NumPadPageDown => (Key::PageDown, None),
            KeySym::NumPadEnd => (Key::End, None),
            KeySym::NumPadBegin => (Key::Home, None),
            KeySym::NumPadInsert => (Key::Insert, None),
            KeySym::NumPadDelete => (Key::Delete, None),
            KeySym::NumPadEqual => (Key::Enter, Some("=")),
            KeySym::NumPadMultiply => (Key::Multiply, Some("*")),
            KeySym::NumPadAdd => (Key::Add, Some("+")),
            KeySym::NumPadSeparator => {
                return (
                    Key::Separator,
                    Some(
                        NumberFormatInfo::current_info()
                            .number_group_separator()
                            .to_owned(),
                    ),
                )
            }
            KeySym::NumPadSubtract => (Key::Subtract, Some("-")),
            KeySym::NumPadDecimal => {
                return (
                    Key::Decimal,
                    Some(
                        NumberFormatInfo::current_info()
                            .number_decimal_separator()
                            .to_owned(),
                    ),
                )
            }
            KeySym::NumPadDivide => (Key::Divide, Some("/")),
            KeySym::NumPad0 => (Key::NumPad0, Some("0")),
            KeySym::NumPad1 => (Key::NumPad1, Some("1")),
            KeySym::NumPad2 => (Key::NumPad2, Some("2")),
            KeySym::NumPad3 => (Key::NumPad3, Some("3")),
            KeySym::NumPad4 => (Key::NumPad4, Some("4")),
            KeySym::NumPad5 => (Key::NumPad5, Some("5")),
            KeySym::NumPad6 => (Key::NumPad6, Some("6")),
            KeySym::NumPad7 => (Key::NumPad7, Some("7")),
            KeySym::NumPad8 => (Key::NumPad8, Some("8")),
            KeySym::NumPad9 => (Key::NumPad9, Some("9")),
            KeySym::F1 => (Key::F1, None),
            KeySym::F2 => (Key::F2, None),
            KeySym::F3 => (Key::F3, None),
            KeySym::F4 => (Key::F4, None),
            KeySym::F5 => (Key::F5, None),
            KeySym::F6 => (Key::F6, None),
            KeySym::F7 => (Key::F7, None),
            KeySym::F8 => (Key::F8, None),
            KeySym::F9 => (Key::F9, None),
            KeySym::F10 => (Key::F10, None),
            KeySym::F11 => (Key::F11, None),
            KeySym::F12 => (Key::F12, None),
            KeySym::F13 => (Key::F13, None),
            KeySym::F14 => (Key::F14, None),
            KeySym::F15 => (Key::F15, None),
            KeySym::F16 => (Key::F16, None),
            KeySym::F17 => (Key::F17, None),
            KeySym::F18 => (Key::F18, None),
            KeySym::F19 => (Key::F19, None),
            KeySym::F20 => (Key::F20, None),
            KeySym::F21 => (Key::F21, None),
            KeySym::F22 => (Key::F22, None),
            KeySym::F23 => (Key::F23, None),
            KeySym::F24 => (Key::F24, None),
            KeySym::ShiftLeft => (Key::LeftShift, None),
            KeySym::ShiftRight => (Key::RightShift, None),
            KeySym::ControlLeft => (Key::LeftCtrl, None),
            KeySym::ControlRight => (Key::RightCtrl, None),
            KeySym::CapsLock => (Key::CapsLock, None),
            KeySym::AltLeft => (Key::LeftAlt, None),
            KeySym::AltRight => (Key::RightAlt, None),
            KeySym::Space => (Key::Space, Some(" ")),
            KeySym::Exclamation => (Key::D1, Some("!")),
            KeySym::Quote => (Key::D2, Some("\"")),
            KeySym::NumberSign => (Key::D3, Some("#")),
            KeySym::Dollar => (Key::D4, Some("$")),
            KeySym::Percent => (Key::D5, Some("%")),
            KeySym::Ampersand => (Key::D7, Some("&")),
            KeySym::Apostrophe => (Key::Oem3, Some("'")),
            KeySym::ParenthesisLeft => (Key::D9, Some("(")),
            KeySym::ParenthesisRight => (Key::D0, Some(")")),
            KeySym::Asterisk => (Key::D8, Some("*")),
            KeySym::Plus => (Key::OemPlus, Some("+")),
            KeySym::Comma => (Key::OemComma, Some(",")),
            KeySym::Minus => (Key::OemMinus, Some("-")),
            KeySym::Period => (Key::OemPeriod, Some(".")),
            KeySym::Slash => (Key::OemQuestion, Some("/")),
            KeySym::D0 => (Key::D0, Some("0")),
            KeySym::D1 => (Key::D1, Some("1")),
            KeySym::D2 => (Key::D2, Some("2")),
            KeySym::D3 => (Key::D3, Some("3")),
            KeySym::D4 => (Key::D4, Some("4")),
            KeySym::D5 => (Key::D5, Some("5")),
            KeySym::D6 => (Key::D6, Some("6")),
            KeySym::D7 => (Key::D7, Some("7")),
            KeySym::D8 => (Key::D8, Some("8")),
            KeySym::D9 => (Key::D9, Some("9")),
            KeySym::Colon => (Key::OemSemicolon, Some(":")),
            KeySym::Semicolon => (Key::OemSemicolon, Some(";")),
            KeySym::Less => (Key::OemComma, Some("<")),
            KeySym::Equal => (Key::OemPlus, Some("=")),
            KeySym::Greater => (Key::OemPeriod, Some(">")),
            KeySym::Question => (Key::OemQuestion, Some("?")),
            KeySym::At => (Key::Oem3, Some("@")),
            KeySym::A => (Key::A, Some("A")),
            KeySym::B => (Key::B, Some("B")),
            KeySym::C => (Key::C, Some("C")),
            KeySym::D => (Key::D, Some("D")),
            KeySym::E => (Key::E, Some("E")),
            KeySym::F => (Key::F, Some("F")),
            KeySym::G => (Key::G, Some("G")),
            KeySym::H => (Key::H, Some("H")),
            KeySym::I => (Key::I, Some("I")),
            KeySym::J => (Key::J, Some("J")),
            KeySym::K => (Key::K, Some("K")),
            KeySym::L => (Key::L, Some("L")),
            KeySym::M => (Key::M, Some("M")),
            KeySym::N => (Key::N, Some("N")),
            KeySym::O => (Key::O, Some("O")),
            KeySym::P => (Key::P, Some("P")),
            KeySym::Q => (Key::Q, Some("Q")),
            KeySym::R => (Key::R, Some("R")),
            KeySym::S => (Key::S, Some("S")),
            KeySym::T => (Key::T, Some("T")),
            KeySym::U => (Key::U, Some("U")),
            KeySym::V => (Key::V, Some("V")),
            KeySym::W => (Key::W, Some("W")),
            KeySym::X => (Key::X, Some("X")),
            KeySym::Y => (Key::Y, Some("Y")),
            KeySym::Z => (Key::Z, Some("Z")),
            KeySym::BracketLeft => (Key::OemOpenBrackets, Some("[")),
            KeySym::Backslash => (Key::OemPipe, Some("\\")),
            KeySym::Bracketright => (Key::OemCloseBrackets, Some("]")),
            KeySym::Underscore => (Key::OemMinus, Some("_")),
            KeySym::Grave => (Key::Oem8, Some("`")),
            KeySym::a => (Key::A, Some("a")),
            KeySym::b => (Key::B, Some("b")),
            KeySym::c => (Key::C, Some("c")),
            KeySym::d => (Key::D, Some("d")),
            KeySym::e => (Key::E, Some("e")),
            KeySym::f => (Key::F, Some("f")),
            KeySym::g => (Key::G, Some("g")),
            KeySym::h => (Key::H, Some("h")),
            KeySym::i => (Key::I, Some("i")),
            KeySym::j => (Key::J, Some("j")),
            KeySym::k => (Key::K, Some("k")),
            KeySym::l => (Key::L, Some("l")),
            KeySym::m => (Key::M, Some("m")),
            KeySym::n => (Key::M, Some("n")),
            KeySym::o => (Key::O, Some("o")),
            KeySym::p => (Key::P, Some("p")),
            KeySym::q => (Key::Q, Some("q")),
            KeySym::r => (Key::R, Some("r")),
            KeySym::s => (Key::S, Some("s")),
            KeySym::t => (Key::T, Some("t")),
            KeySym::u => (Key::U, Some("u")),
            KeySym::v => (Key::V, Some("v")),
            KeySym::w => (Key::W, Some("w")),
            KeySym::x => (Key::X, Some("x")),
            KeySym::y => (Key::Y, Some("y")),
            KeySym::z => (Key::Z, Some("z")),
            KeySym::BraceLeft => (Key::OemOpenBrackets, Some("{")),
            KeySym::Bar => (Key::OemPipe, Some("|")),
            KeySym::BraceRight => (Key::OemCloseBrackets, Some("}")),
            KeySym::AsciiTilde => (Key::OemTilde, Some("~")),
            _ => (Key::None, None),
        };

        (key, symbol.map(str::to_owned))
    }

    /// `Capture` on the UI thread: copies the last rendered frame of the
    /// window into the framebuffer, which is replaced when the size of the
    /// frame changed.
    fn capture_on_ui_thread(&self) -> Arc<VncFramebuffer> {
        let mut framebuffer = lock(&self.framebuffer);
        let Some(bmp) = self
            .window()
            .and_then(|window| window.get_last_rendered_frame())
        else {
            return framebuffer.clone();
        };

        let pixel_size = bmp.pixel_size();
        if pixel_size.width != framebuffer.width() || pixel_size.height != framebuffer.height() {
            *framebuffer = VncFramebuffer::new(
                "FerroUI",
                pixel_size.width,
                pixel_size.height,
                VncPixelFormat::RGB32,
            );
        }

        {
            let mut buffer = framebuffer.get_buffer();
            bmp.copy_pixels(
                PixelRect::from_size(pixel_size),
                &mut buffer[..],
                framebuffer.stride(),
            );
        }
        bmp.dispose();

        framebuffer.clone()
    }
}

impl IVncFramebufferSource for HeadlessVncFramebufferSource {
    fn supports_resizing(&self) -> bool {
        true
    }

    fn capture(&self) -> Option<Arc<VncFramebuffer>> {
        let this = self.arc();
        match Dispatcher::ui_thread().invoke(move || this.capture_on_ui_thread()) {
            Ok(framebuffer) => Some(framebuffer),
            // The UI thread has shut down: the frame of before stays.
            Err(_) => Some(self.framebuffer()),
        }
    }

    fn set_desktop_size(&self, width: i32, height: i32) -> ExtendedDesktopSizeStatus {
        let this = self.arc();
        Dispatcher::ui_thread().post(
            move || {
                if let Some(window) = this.window() {
                    window.set_width(f64::from(width));
                    window.set_height(f64::from(height));
                }
            },
            DispatcherPriority::NORMAL,
        );
        ExtendedDesktopSizeStatus::Success
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;

    #[test]
    fn letters_digits_and_control_keys_translate_to_keys_and_text() {
        assert_eq!(
            (Key::A, Some("a".to_owned())),
            HeadlessVncFramebufferSource::translate_key(KeySym::a)
        );
        assert_eq!(
            (Key::A, Some("A".to_owned())),
            HeadlessVncFramebufferSource::translate_key(KeySym::A)
        );
        assert_eq!(
            (Key::D7, Some("7".to_owned())),
            HeadlessVncFramebufferSource::translate_key(KeySym::D7)
        );
        assert_eq!(
            (Key::Return, Some("\r".to_owned())),
            HeadlessVncFramebufferSource::translate_key(KeySym::Return)
        );
        assert_eq!(
            (Key::Left, None),
            HeadlessVncFramebufferSource::translate_key(KeySym::Left)
        );
        assert_eq!(
            (Key::Decimal, Some(".".to_owned())),
            HeadlessVncFramebufferSource::translate_key(KeySym::NumPadDecimal)
        );
        assert_eq!(
            (Key::None, None),
            HeadlessVncFramebufferSource::translate_key(KeySym::VoidSymbol)
        );
        // As upstream: the small letter n is the key M.
        assert_eq!(
            (Key::M, Some("n".to_owned())),
            HeadlessVncFramebufferSource::translate_key(KeySym::n)
        );
    }
}
