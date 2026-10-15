use crate::interop::{CorePhysicalKeyStatus, VirtualKeyModifiers};
#[cfg(windows)]
use crate::interop::VirtualKey;
use ferroui_base::input::RawInputModifiers;
#[cfg(windows)]
use ferroui_base::input::{Key, PhysicalKey};
#[cfg(windows)]
use ferroui_win32::input::KeyInterop;

pub(crate) struct WinUIKeyInterop;

impl WinUIKeyInterop {
    /// Resolves WinUI key-event metadata to the framework's [`Key`],
    /// [`PhysicalKey`] and key symbol, using the shared Win32 mapping
    /// tables in [`KeyInterop`]. WinUI's `KeyRoutedEventArgs` exposes
    /// virtual-key + scan-code + extended flag — packing those into the
    /// same lParam layout the Win32 backend uses lets us reuse all of its
    /// mapping logic (Numpad disambiguation, left/right modifier split, etc.).
    #[cfg(windows)]
    pub(crate) fn resolve(virtual_key: VirtualKey, status: CorePhysicalKeyStatus) -> (Key, PhysicalKey, Option<String>) {
        let key_data = Self::encode_key_data(status);
        let vk = virtual_key.0;
        let key = KeyInterop::key_from_virtual_key(vk, key_data);
        let physical = KeyInterop::physical_key_from_virtual_key(vk, key_data);
        let symbol = KeyInterop::get_key_symbol_from_virtual_key(vk);
        (key, physical, symbol)
    }

    #[cfg(windows)]
    #[allow(dead_code)] // As in the reference, which has no caller of it either.
    pub(crate) fn key_from_virtual_key(virtual_key: VirtualKey) -> Key {
        // Fallback used by callers that don't have KeyStatus (rare).
        KeyInterop::key_from_virtual_key(virtual_key.0, 0)
    }

    pub(crate) fn modifiers_from_virtual_key_modifiers(modifiers: VirtualKeyModifiers) -> RawInputModifiers {
        let mut result = RawInputModifiers::NONE;
        if modifiers.contains(VirtualKeyModifiers::Control) {
            result |= RawInputModifiers::CONTROL;
        }
        if modifiers.contains(VirtualKeyModifiers::Shift) {
            result |= RawInputModifiers::SHIFT;
        }
        if modifiers.contains(VirtualKeyModifiers::Menu) {
            result |= RawInputModifiers::ALT;
        }
        if modifiers.contains(VirtualKeyModifiers::Windows) {
            result |= RawInputModifiers::META;
        }
        result
    }

    fn encode_key_data(status: CorePhysicalKeyStatus) -> i32 {
        // Mirror the WM_KEYDOWN lParam layout that Win32's KeyInterop expects:
        //   bits 16-23 : scan code (low byte)
        //   bit 24     : extended-key flag
        // Repeat count, context, previous/transition bits are unused by the
        // key/physical-key/symbol resolvers, so leave them zero.
        let mut data = ((status.scan_code & 0xFF) << 16) as i32;
        if status.is_extended_key != 0 {
            data |= 1 << 24;
        }
        data
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the reference has no tests of the integration.
    use super::*;

    fn status(scan_code: u32, extended: bool) -> CorePhysicalKeyStatus {
        CorePhysicalKeyStatus {
            repeat_count: 1,
            scan_code,
            is_extended_key: u8::from(extended),
            is_menu_key_down: 0,
            was_key_down: 0,
            is_key_released: 0,
        }
    }

    #[test]
    fn the_key_data_is_the_l_param_of_a_key_message() {
        assert_eq!(0x001E_0000, WinUIKeyInterop::encode_key_data(status(0x1E, false)));
        assert_eq!(0x011C_0000, WinUIKeyInterop::encode_key_data(status(0x1C, true)));
        // Only the low byte of the scan code.
        assert_eq!(0x0034_0000, WinUIKeyInterop::encode_key_data(status(0xE034, false)));
    }

    #[test]
    fn the_modifiers_of_a_pointer_event() {
        assert_eq!(RawInputModifiers::NONE, WinUIKeyInterop::modifiers_from_virtual_key_modifiers(VirtualKeyModifiers::None));
        assert_eq!(
            RawInputModifiers::CONTROL | RawInputModifiers::SHIFT | RawInputModifiers::ALT | RawInputModifiers::META,
            WinUIKeyInterop::modifiers_from_virtual_key_modifiers(
                VirtualKeyModifiers::Control | VirtualKeyModifiers::Shift | VirtualKeyModifiers::Menu | VirtualKeyModifiers::Windows
            )
        );
        assert_eq!(RawInputModifiers::ALT, WinUIKeyInterop::modifiers_from_virtual_key_modifiers(VirtualKeyModifiers::Menu));
    }
}
