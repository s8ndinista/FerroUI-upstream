//! Upstream's user interface automation tests
//! (`tests/*.IntegrationTests.Appium` of the reference): the IntegrationTestApp
//! sample, started as a process of its own, driven through the accessibility
//! API of the platform, as a person or a screen reader would drive it.
//!
//! ```text
//! cargo build -p integration-test-app
//! FERROUI_AUTOMATION_TESTS=1 cargo test -p ferroui-integration-tests-appium --test automation
//! FERROUI_AUTOMATION_TESTS=1 cargo test -p ferroui-integration-tests-appium --test automation -- combo_box
//! FERROUI_AUTOMATION_TESTS=1 cargo test -p ferroui-integration-tests-appium --test automation -- --include-ignored
//! ```
//!
//! Upstream drives the application through an Appium server (WinAppDriver on
//! Windows, the Mac2 driver on macOS); the port drives it with its own layer
//! over UI Automation and over the accessibility API of macOS
//! (`infrastructure`, docs/porting/integration-tests.md). The binary has no
//! test harness: the tests of a collection share one running application and
//! the input of the desktop, so they run one after the other. Every test is
//! printed with its result as the harness prints it; an argument selects the
//! tests whose name contains it. The names are the module of the upstream
//! class and the name of the upstream test in snake case, a theory with its
//! row.
//!
//! The tests need a desktop the run may give input to, and on macOS the
//! Accessibility permission of the process that runs them. They move the
//! pointer and type on the keyboard of the machine, so they run only when
//! `FERROUI_AUTOMATION_TESTS=1` asks for them: a `cargo test` of the whole
//! workspace says so and runs nothing. On a platform without an automation
//! backend the binary compiles and runs nothing.

// Without an automation backend the elements are of a type with no values: the code after one is
// unreachable.
#![cfg_attr(not(any(windows, target_os = "macos")), allow(unreachable_code))]

mod infrastructure;

mod automation_tests;
mod button_tests;
mod check_box_tests;
mod combo_box_tests;
mod context_menu_tests;
mod drag_drop_tests;
mod embedding_tests;
mod gesture_tests;
mod keyboard_tests;
mod list_box_tests;
mod menu_tests;
mod native_menu_tests;
mod pointer_tests;
mod pointer_tests_mac_os;
mod popups_tests;
mod radio_button_tests;
mod screen_tests;
mod scroll_bar_tests;
mod slider_tests;
mod tray_icon_tests;
mod window_decorations_tests;
mod window_tests;
mod window_tests_mac_os;

use infrastructure::backend::Backend;
use infrastructure::platform_fact::os_description;
use infrastructure::registry::TestCase;
use infrastructure::{Collection, DefaultAppFixture, Tests};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::process::ExitCode;

fn register(tests: &mut Tests) {
    automation_tests::tests(tests);
    button_tests::tests(tests);
    check_box_tests::tests(tests);
    combo_box_tests::tests(tests);
    context_menu_tests::tests(tests);
    drag_drop_tests::tests(tests);
    embedding_tests::tests(tests);
    gesture_tests::tests(tests);
    keyboard_tests::tests(tests);
    list_box_tests::tests(tests);
    menu_tests::tests(tests);
    native_menu_tests::tests(tests);
    pointer_tests::tests(tests);
    pointer_tests_mac_os::tests(tests);
    popups_tests::tests(tests);
    radio_button_tests::tests(tests);
    screen_tests::tests(tests);
    scroll_bar_tests::tests(tests);
    slider_tests::tests(tests);
    tray_icon_tests::tests(tests);
    window_decorations_tests::tests(tests);
    window_tests::tests(tests);
    window_tests_mac_os::tests(tests);
}

/// Why a test does not run, if it does not.
fn ignored_because(case: &TestCase, include_ignored: bool) -> Option<String> {
    if !case.platforms.is_supported() {
        let reason = case.platform_reason.as_deref().map(|r| format!(" reason: '{r}'")).unwrap_or_default();
        return Some(format!("Ignored on {}{reason}", os_description()));
    }
    if include_ignored {
        return None;
    }
    case.skip.clone()
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let include_ignored = arguments.iter().any(|a| a == "--include-ignored" || a == "--ignored");
    // A diagnostic of the runs: a search for one element that does not find it prints the tree
    // it searched.
    infrastructure::session::set_dump_tree(arguments.iter().any(|a| a == "--dump-tree"));
    let filters: Vec<&String> = arguments.iter().filter(|a| !a.starts_with('-')).collect();

    let mut tests = Tests::default();
    register(&mut tests);
    let selected: Vec<&TestCase> = tests
        .cases
        .iter()
        .filter(|case| filters.is_empty() || filters.iter().any(|filter| case.name.contains(filter.as_str())))
        .collect();

    if arguments.iter().any(|a| a == "--list") {
        // The tests of the platform of the run, and the reason of every test that does not run.
        for case in &selected {
            match ignored_because(case, include_ignored) {
                None => println!("{}: test", case.name),
                Some(reason) => println!("{}: test, ignored, {reason}", case.name),
            }
        }
        return ExitCode::SUCCESS;
    }

    let runnable: Vec<&TestCase> =
        selected.iter().copied().filter(|case| ignored_because(case, include_ignored).is_none()).collect();

    if std::env::var("FERROUI_AUTOMATION_TESTS").as_deref() != Ok("1") {
        println!(
            "the automation tests drive the desktop of the machine (they move the pointer and type on the keyboard): \
             set FERROUI_AUTOMATION_TESTS=1 to run them; nothing run ({} tests selected)",
            selected.len()
        );
        return ExitCode::SUCCESS;
    }

    if !cfg!(any(windows, target_os = "macos")) {
        println!(
            "the automation tests drive the application through UI Automation on Windows and the accessibility API \
             on macOS: nothing to run on {}",
            os_description()
        );
        return ExitCode::SUCCESS;
    }

    if !runnable.is_empty() {
        if let Err(reason) = Backend::check() {
            println!("\nthe run cannot start: {reason}");
            return ExitCode::FAILURE;
        }
    }

    println!("\nrunning {} tests", selected.len());
    let mut failed: Vec<&str> = Vec::new();
    let mut ignored = 0usize;
    for case in &selected {
        if let Some(reason) = ignored_because(case, include_ignored) {
            println!("test {} ... ignored, {reason}", case.name);
            ignored += 1;
        }
    }

    for collection in [Collection::Default, Collection::WindowDecorations, Collection::OverlayPopups] {
        let cases: Vec<&TestCase> = runnable.iter().copied().filter(|case| case.collection == collection).collect();
        if cases.is_empty() {
            continue;
        }
        println!("\n-- the application of the collection {collection:?}");
        let fixture = match DefaultAppFixture::new(collection) {
            Ok(fixture) => fixture,
            Err(error) => {
                println!("the application of the collection {collection:?} did not start: {error}");
                for case in cases {
                    println!("test {} ... FAILED", case.name);
                    failed.push(case.name.as_str());
                }
                continue;
            }
        };
        // Where the main window is: whether it shows the whole list of pages depends on the size
        // of the desktop.
        if let Ok(main) = fixture.session().try_find_element_by_accessibility_id("MainWindow") {
            let rect = main.rect();
            println!("the main window: at {}, {}, {} by {}", rect.x, rect.y, rect.width, rect.height);
        }
        for case in cases {
            let result = catch_unwind(AssertUnwindSafe(|| case.run(&fixture)));
            // A test that failed with a button or a key down leaves it down otherwise.
            let _ = catch_unwind(AssertUnwindSafe(|| fixture.session().release_input()));
            match result {
                Ok(()) => println!("test {} ... ok", case.name),
                Err(_) => {
                    println!("test {} ... FAILED", case.name);
                    failed.push(case.name.as_str());
                }
            }
        }
        drop(fixture);
    }

    if !failed.is_empty() {
        println!("\nfailures:");
        for name in &failed {
            println!("    {name}");
        }
    }
    println!(
        "\ntest result: {}. {} passed; {} failed; {} ignored; {} filtered out",
        if failed.is_empty() { "ok" } else { "FAILED" },
        runnable.len() - failed.len(),
        failed.len(),
        ignored,
        tests.cases.len() - selected.len()
    );

    if failed.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
