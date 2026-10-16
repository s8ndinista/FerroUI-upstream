//! The type table of this crate: its namespace, its class, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.

use crate::App;
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[("platform_sanity_checks", "PlatformSanityChecks")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "PlatformSanityChecks",
    crate_name: "platform_sanity_checks",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the namespace `PlatformSanityChecks` (`X::TYPE`).
const TYPES: &[&TypeInfo] = &[App::TYPE];

/// Registers the namespace, the class, the assembly, the embedded assets and the compiled
/// markup of this crate (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_themes_simple::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupAssembly::register(&ASSEMBLY);
        crate::SAMPLE.register_assets();
        crate::compiled_markup::register();
    });
}
