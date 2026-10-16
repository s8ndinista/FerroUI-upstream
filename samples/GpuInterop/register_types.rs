//! The type table of this crate: its namespaces, its classes, what it states about itself for
//! markup, its embedded assets and the loader table of its compiled markup.

use crate::markup::XamlClass;
use crate::{d3d_demo, vulkan_demo, App, DrawingSurfaceDemoBase, GpuDemo, MainWindow};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, MarkupType};
use ferroui_base::TypeInfo;

/// The dotted namespaces of the modules of this crate. A type belongs to the namespace of the
/// longest module path that is a prefix of the path of its declaring module.
const NAMESPACES: &[(&str, &str)] = &[
    ("gpu_interop", "GpuInterop"),
    ("gpu_interop::vulkan_demo", "GpuInterop.VulkanDemo"),
    ("gpu_interop::d3d_demo", "GpuInterop.D3DDemo"),
];

/// What this crate states about itself for markup.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "GpuInterop",
    crate_name: "gpu_interop",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

/// The classes of the namespace `GpuInterop` (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[App::TYPE, MainWindow::TYPE, GpuDemo::TYPE, DrawingSurfaceDemoBase::TYPE];

/// The classes of the namespace that have a document (`&X::XAML_CLASS`).
pub(crate) const CLASSES: &[&XamlClass] = &[&App::XAML_CLASS, &MainWindow::XAML_CLASS, &GpuDemo::XAML_CLASS];

/// The types of the namespace declared with `ferro_markup_type!` / `ferro_markup_enum!`
/// (`<X as MarkupTyped>::MARKUP`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];

/// Registers the namespaces, the types, the assembly, the embedded assets and the compiled
/// markup of this crate (and of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // The runtime library of compiled markup (the markup extensions the compiled
        // documents create), which registers the controls.
        ferroui_markup_xaml::register_types();
        ferroui_themes_fluent::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        for types in [TYPES, vulkan_demo::TYPES, d3d_demo::TYPES] {
            TypeInfo::register_all(types);
        }
        MarkupType::register_all(MARKUP_TYPES);
        ValueTypes::register_global(register_value_types);
        register_lists();
        MarkupAssembly::register(&ASSEMBLY);
        crate::SAMPLE.register_assets();
        // The loader table of the compiled markup, which the build of the crate generates: a
        // load of a document by its URI creates the class of the document with its
        // constructor.
        crate::compiled_markup::register();
    });
}

/// What the untyped value conversions must know about the types of this namespace
/// (`ValueTypes::register_reference::<X>()`, nullable forms, casts to contracts).
fn register_value_types() {}

/// Makes the typed lists of this namespace known to markup (`ferro_markup_list!`).
fn register_lists() {}
