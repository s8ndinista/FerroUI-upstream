//! Port of the application that upstream's designer support tests preview
//! (`FerroUI.DesignerSupport.TestApp`): an application with the Simple
//! theme whose main window holds a button, `App.xaml` and `MainWindow.xaml`
//! loaded by the run-time XAML loader (`FerroXamlLoader.Load(this)`).
//!
//! An application of the port is the host of its own previewer
//! (DEVIATIONS.md, Designer support): with `--transport` in its arguments the
//! binary registers the run-time XAML loader and runs the previewer with its
//! application builder, as the host application of the original does with the
//! assembly of this application; otherwise it starts with the classic desktop
//! lifetime (`Program.Main`).

// The markup of the documents names the types of this crate by the name of the crate.
extern crate self as designer_support_test_app;

mod app;
mod main_window;

use app::App;
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::TypeInfo;
use ferroui_controls::AppBuilder;
use ferroui_designer_support::remote::RemoteDesignerEntryPoint;
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use main_window::MainWindow;

/// The dotted namespaces of the modules of this crate.
const NAMESPACES: &[(&str, &str)] = &[("designer_support_test_app", "FerroUI.DesignerSupport.TestApp")];

/// What this crate states about itself for markup: the assembly
/// `FerroUI.DesignerSupport.TestApp`.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.DesignerSupport.TestApp",
    crate_name: "designer_support_test_app",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    // What the build of upstream writes into the assembly of the application (the projects
    // of the repository compile bindings by default).
    metadata: &[(MarkupAssembly::USE_COMPILED_BINDINGS_BY_DEFAULT, "True")],
};

/// Registers the namespaces, the classes and the assembly of this crate (and
/// the types of the crates it is built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_markup_xaml::register_types();
        ferroui_themes_simple::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(&[App::TYPE, MainWindow::TYPE]);
        MarkupAssembly::register(&ASSEMBLY);
    });
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The documents of the classes are loaded by the run-time loader.
    register_types();
    FerroRuntimeXamlLoader::register();
    if args.iter().any(|arg| arg == "--transport") {
        RemoteDesignerEntryPoint::main(&args, build_ferro_app);
    }
    std::process::exit(build_ferro_app().start_with_classic_desktop_lifetime(&args));
}

fn build_ferro_app() -> AppBuilder {
    AppBuilder::configure::<App>().use_platform_detect()
}
