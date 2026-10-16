//! The type table of this crate: its namespace, its classes, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.

use crate::markup::XamlClass;
use crate::{App, MainView, MainWindow};
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate. A type belongs to the namespace of the
/// longest module path that is a prefix of the path of its declaring module.
const NAMESPACES: &[(&str, &str)] = &[("single_project_sandbox", "SingleProjectSandbox")];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "SingleProjectSandbox",
    crate_name: "single_project_sandbox",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the namespace `SingleProjectSandbox` (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[App::TYPE, MainView::TYPE, MainWindow::TYPE];

/// The classes of the namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[&App::XAML_CLASS, &MainView::XAML_CLASS, &MainWindow::XAML_CLASS];

/// Registers the namespace, the types, the assembly, the embedded assets and the compiled
/// markup of this crate (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup (the markup extensions the compiled
        // documents create), which registers the controls.
        ferroui_markup_xaml::register_types();
        ferroui_themes_fluent::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        MarkupAssembly::register(&ASSEMBLY);
        crate::SAMPLE.register_assets();
        // The loader table of the compiled markup, which the build of the crate generates: a
        // load of a document by its URI creates the class of the document with its
        // constructor.
        crate::compiled_markup::register();
    });
}
