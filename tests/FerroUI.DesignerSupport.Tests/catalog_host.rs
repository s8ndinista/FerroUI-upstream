//! The previewer of the ControlCatalog sample, for the rows of upstream's
//! designer support test that preview the catalog
//! (`ControlCatalog.Desktop`): the host application of the original loads the
//! assembly of the catalog and builds its application builder
//! (`BuildFerroApp`, `AppBuilder.Configure<App>().UsePlatformDetect()`);
//! here this binary links the catalog library and hands the previewer that
//! builder (DEVIATIONS.md, Tests and test support).

use ferroui_controls::AppBuilder;
use ferroui_designer_support::remote::RemoteDesignerEntryPoint;
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    control_catalog::register_types();
    FerroRuntimeXamlLoader::register();
    RemoteDesignerEntryPoint::main(&args, || AppBuilder::configure::<control_catalog::App>().use_platform_detect());
}
