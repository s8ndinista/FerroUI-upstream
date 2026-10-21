//! The WinUI migration output is a common input to the interpreter and Rust emitter.
//!
//! The checked-in generated file is compiled by rustc, then the same lifecycle and live
//! source/target updates are exercised through both backends. Regenerate only this fixture:
//! `cargo test -p ferroui-markup-xaml-loader --features emitter --test winui_compiled
//! regenerate_winui_binding_fixture -- --ignored --exact`

use ferroui_base::controls::NameScope;
use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::Ref;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use ferroui_controls::{Button, ContentControl, Control, StackPanel, TextBlock, TextBox};
use ferroui_markup_xaml::winui::WinUIBindings;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use ferroui_markup_xaml_loader::rust_emitter::generate_file;
use ferroui_markup_xaml_loader::winui::{migrate_winui_xaml, WinUIXBindPolicy, WinUIXamlOptions};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

#[allow(dead_code)]
#[rustfmt::skip]
#[path = "fixtures/winui_bindings.generated.rs"]
mod generated;

const ROOT: &str = include_str!("fixtures/winui_root.xaml");
const TEMPLATE: &str = include_str!("fixtures/winui_template.xaml");

fn options() -> WinUIXamlOptions {
    WinUIXamlOptions {
        x_bind: WinUIXBindPolicy::PropertyPathsWithExplicitLifecycle,
        ..Default::default()
    }
}

fn start() -> UnitTestApplicationScope {
    ferroui_markup_xaml::register_types();
    UnitTestApplication::start(TestServices::mock_threading_interface())
}

fn parts(root: &Control) -> (Ref<TextBlock>, Ref<TextBox>, Ref<TextBlock>) {
    // IDataTemplate::build returns the control; its separate template name scope is
    // consumed by the presenter rather than attached to that control automatically.
    let panel = root.to_ref().cast::<StackPanel>().unwrap();
    let children = panel.children();
    let initial = children.get(0).cast::<TextBlock>().unwrap();
    let editor = children.get(1).cast::<TextBox>().unwrap();
    let mirror = children.get(2).cast::<TextBlock>().unwrap();
    assert_eq!(initial.name().as_deref(), Some("initial"));
    assert_eq!(editor.name().as_deref(), Some("editor"));
    assert_eq!(mirror.name().as_deref(), Some("mirror"));
    (initial, editor, mirror)
}

fn assert_root_behavior(root: Ref<StackPanel>) {
    let (initial, editor, mirror) = parts(&root);
    let scope = NameScope::get_name_scope(&root).expect("document root owns its name scope");
    assert_eq!(scope.find_as::<TextBlock>("initial"), Some(initial.clone()));
    assert_eq!(scope.find_as::<TextBox>("editor"), Some(editor.clone()));
    assert_eq!(scope.find_as::<TextBlock>("mirror"), Some(mirror.clone()));
    let bindings = WinUIBindings::get_for_root(&root).unwrap();
    assert!(!bindings.is_initialized());
    assert_eq!(initial.text(), None);
    assert_eq!(editor.text(), None);
    assert_eq!(mirror.text(), None);
    root.set_tag(Some(Rc::new("assigned after construction".to_string())));
    bindings.initialize().unwrap();
    assert_eq!(
        initial.text().as_deref(),
        Some("assigned after construction")
    );
    assert_eq!(editor.text(), initial.text());
    assert_eq!(mirror.text(), initial.text());
    root.set_tag(Some(Rc::new("live change".to_string())));
    assert_eq!(
        initial.text().as_deref(),
        Some("assigned after construction")
    );
    assert_eq!(editor.text().as_deref(), Some("live change"));
    assert_eq!(mirror.text().as_deref(), Some("live change"));

    editor.set_current_value(TextBox::text_property(), Some("user draft".into()));
    assert_eq!(mirror.text().as_deref(), Some("live change"));
    editor.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
    assert_eq!(
        root.tag().unwrap().downcast_ref::<String>().unwrap(),
        "user draft"
    );
    assert_eq!(mirror.text().as_deref(), Some("user draft"));

    bindings.stop_tracking();
    root.set_tag(Some(Rc::new("while stopped".to_string())));
    assert_eq!(mirror.text().as_deref(), Some("user draft"));
    editor.set_current_value(TextBox::text_property(), Some("blocked write".into()));
    editor.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
    assert_eq!(
        root.tag().unwrap().downcast_ref::<String>().unwrap(),
        "while stopped"
    );
    bindings.initialize().unwrap();
    assert_eq!(initial.text().as_deref(), Some("while stopped"));
    assert_eq!(editor.text(), initial.text());
    assert_eq!(mirror.text(), initial.text());
}

fn item(text: &str) -> Ref<Button> {
    let result = Button::new();
    result.set_content(Some(Rc::new(text.to_string())));
    result
}

fn assert_template_behavior(template: Rc<dyn IDataTemplate>) {
    let first = template.build(&None).unwrap();
    let second = template.build(&None).unwrap();
    let (initial, editor, mirror) = parts(&first);
    let (second_initial, second_editor, second_mirror) = parts(&second);
    let first_item = item("first");
    let second_item = item("second");
    first.set_data_context(Some(Rc::new(first_item.clone())));
    second.set_data_context(Some(Rc::new(second_item.clone())));
    let first_bindings = WinUIBindings::get_for_root(&first).unwrap();
    let second_bindings = WinUIBindings::get_for_root(&second).unwrap();
    assert!(!Rc::ptr_eq(&first_bindings, &second_bindings));
    assert_eq!(initial.text(), None);
    first_bindings.initialize().unwrap();
    assert_eq!(initial.text().as_deref(), Some("first"));
    assert_eq!(editor.text(), initial.text());
    assert_eq!(mirror.text(), initial.text());
    assert_eq!(second_initial.text(), None);
    second_bindings.initialize().unwrap();
    assert_eq!(second_initial.text().as_deref(), Some("second"));

    first_item.set_content(Some(Rc::new("changed source".to_string())));
    assert_eq!(initial.text().as_deref(), Some("first"));
    assert_eq!(editor.text().as_deref(), Some("changed source"));
    assert_eq!(mirror.text().as_deref(), Some("changed source"));
    editor.set_current_value(TextBox::text_property(), Some("changed target".into()));
    assert_eq!(
        first_item
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "changed target"
    );
    assert_eq!(mirror.text().as_deref(), Some("changed target"));
    assert_eq!(second_editor.text().as_deref(), Some("second"));

    let recycled_item = item("recycled item");
    first.set_data_context(Some(Rc::new(recycled_item.clone())));
    assert_eq!(initial.text().as_deref(), Some("recycled item"));
    assert_eq!(editor.text(), initial.text());
    assert_eq!(mirror.text(), initial.text());
    first_item.set_content(Some(Rc::new("old item is detached".to_string())));
    assert_eq!(mirror.text().as_deref(), Some("recycled item"));
    first_bindings.stop_tracking();
    editor.set_current_value(TextBox::text_property(), Some("blocked write".into()));
    assert_eq!(
        recycled_item
            .content()
            .unwrap()
            .downcast_ref::<String>()
            .unwrap(),
        "recycled item"
    );
    second_item.set_content(Some(Rc::new("second remains live".to_string())));
    assert_eq!(second_mirror.text().as_deref(), Some("second remains live"));
}

#[test]
fn interpreted_root_defers_initialization_and_tracks_live_source_and_target_changes() {
    let _app = start();
    assert_root_behavior(FerroRuntimeXamlLoader::parse_winui_as(ROOT, None, &options()).unwrap());
}

#[test]
fn interpreted_typed_templates_keep_live_bindings_and_recycling_independent() {
    let _app = start();
    let host: Ref<ContentControl> =
        FerroRuntimeXamlLoader::parse_winui_as(TEMPLATE, None, &options()).unwrap();
    assert_template_behavior(host.content_template().unwrap());
}

#[test]
fn compiled_root_defers_initialization_and_tracks_live_source_and_target_changes() {
    let _app = start();
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    assert_root_behavior(generated::build_root_xaml(Some(provider)).unwrap());
}

#[test]
fn compiled_typed_templates_keep_live_bindings_and_recycling_independent() {
    let _app = start();
    let provider = XamlIlRuntimeHelpers::create_root_service_provider_v3(None);
    let host = generated::build_template_xaml(Some(provider)).unwrap();
    assert_template_behavior(host.content_template().unwrap());
}

#[test]
fn generated_winui_binding_fixture_matches_the_current_compiler() {
    assert_eq!(
        include_str!("fixtures/winui_bindings.generated.rs"),
        emitted_fixture(),
        "regenerate the focused fixture with the command in this module's header"
    );
}

fn emitted_fixture() -> String {
    ferroui_markup_xaml::register_types();
    let documents: Vec<_> = [("root.xaml", ROOT), ("template.xaml", TEMPLATE)]
        .into_iter()
        .map(|(name, source)| {
            let native = migrate_winui_xaml(source, &options())
                .unwrap()
                .into_xaml()
                .unwrap();
            (name, native)
        })
        .collect();
    let borrowed: Vec<_> = documents
        .iter()
        .map(|(name, native)| (*name, native.as_str()))
        .collect();
    let file = generate_file(
        "FerroUI.WinUI.BindingFixture",
        "ferres://FerroUI.WinUI.BindingFixture/",
        &borrowed,
        &RuntimeXamlLoaderConfiguration::new(),
        &[],
    );
    for (document, reason) in &file.documents {
        assert!(reason.is_none(), "{document}: {reason:?}");
    }
    file.source
}

#[test]
#[ignore = "updates the checked-in fixture; run explicitly after reviewing emitter changes"]
fn regenerate_winui_binding_fixture() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/winui_bindings.generated.rs");
    std::fs::write(path, emitted_fixture()).unwrap();
}
