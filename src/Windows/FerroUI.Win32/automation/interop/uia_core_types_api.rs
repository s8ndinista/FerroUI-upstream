//! The type functions of the automation core of the system (the port of
//! `Interop/UiaCoreTypesApi.cs`).

#[cfg(windows)]
use ferroui_microcom::Guid;

/// The kind of identifier a lookup asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub(crate) enum AutomationIdType {
    Property,
    Pattern,
    Event,
    ControlType,
    TextAttribute,
}

/// The type functions and the result codes of the automation core.
pub(crate) struct UiaCoreTypesApi;

#[allow(dead_code)] // The codes are the ones the reference declares; the nodes answer two of them.
impl UiaCoreTypesApi {
    pub const UIA_E_ELEMENTNOTENABLED: i32 = 0x8004_0200u32 as i32;
    pub const UIA_E_ELEMENTNOTAVAILABLE: i32 = 0x8004_0201u32 as i32;
    pub const UIA_E_NOCLICKABLEPOINT: i32 = 0x8004_0202u32 as i32;
    pub const UIA_E_PROXYASSEMBLYNOTLOADED: i32 = 0x8004_0203u32 as i32;

    /// `UiaLookupId`: the number of the identifier of `guid`; zero when
    /// the core does not know it.
    #[cfg(windows)]
    pub fn uia_lookup_id(type_: AutomationIdType, guid: &Guid) -> i32 {
        #[cfg_attr(not(target_arch = "x86"), link(name = "uiautomationcore", kind = "raw-dylib"))]
        #[cfg_attr(target_arch = "x86", link(name = "uiautomationcore", kind = "raw-dylib", import_name_type = "undecorated"))]
        unsafe extern "system" {
            fn UiaLookupId(type_: i32, guid: *const Guid) -> i32;
        }
        // SAFETY: a kind of identifier and an identifier that lives
        // through the call.
        unsafe { UiaLookupId(type_ as i32, guid) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_kinds_of_identifier_and_the_result_codes_are_those_of_the_system() {
        assert_eq!(0, AutomationIdType::Property as i32);
        assert_eq!(4, AutomationIdType::TextAttribute as i32);
        assert_eq!(0x8004_0200, UiaCoreTypesApi::UIA_E_ELEMENTNOTENABLED as u32);
        assert_eq!(0x8004_0201, UiaCoreTypesApi::UIA_E_ELEMENTNOTAVAILABLE as u32);
        assert_eq!(0x8004_0202, UiaCoreTypesApi::UIA_E_NOCLICKABLEPOINT as u32);
        assert_eq!(0x8004_0203, UiaCoreTypesApi::UIA_E_PROXYASSEMBLYNOTLOADED as u32);
    }
}
