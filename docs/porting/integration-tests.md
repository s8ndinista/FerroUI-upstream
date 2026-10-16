# The user interface automation tests

The port of upstream's `tests/Avalonia.IntegrationTests.Appium` (the tracked commit, `17350180`): the tests that start `samples/IntegrationTestApp` as a process of its own and drive it from outside, through the accessibility API of the platform and the input of the desktop. The crate is `ferroui-integration-tests-appium` (`tests/FerroUI.IntegrationTests.Appium`: the directory mirrors upstream's project, as every directory of the workspace does, although the port does not use Appium, section 1); the sample is `samples/IntegrationTestApp`, ported before (its `GAPS.md`).

Marks used below: **[V]** verified on the Mac (compiled, read in the sources named, or run there), **[CI]** to be established by the step of the Windows job of CI (section 6), **[VM]** to be established in the owner's Windows 11 virtual machine (section 6), **[P]** waits for the Accessibility permission on the Mac (section 7).

## 1. The decision: the port's own automation layer, not Appium

Upstream drives the sample through an Appium server: WinAppDriver (UI Automation) on Windows, the Mac2 driver (XCTest over the accessibility API) on macOS, with the .NET client of Selenium and Appium in the tests. The port does not use Appium, the .NET SDK, XCTest or a WebDriver server. It ports the test classes onto a layer of its own that does what upstream's `AppiumDriver`, its elements and the session helpers do, implemented directly over the platform:

- **Windows**: UI Automation through the client of the Windows backend (`ferroui_win32::automation::testing`, the client the smoke run of `win32_window --a11y` uses, `win32-platform.md`, section 13.5), extended for this harness, and the input of the desktop through `SendInput`.
- **macOS**: the accessibility API of the system (`AXUIElement` of the application services framework, from another process) and the input of the window server (`CGEventPost`), through declarations of the system functions in the crate (no binding crate is needed for the twenty functions used).

Why: the tests are a port of the steps and the assertions, and those depend on a small, well-defined part of the drivers (find, read, click, type, a few pointer sequences, the windows of the application). A layer of the port's own is a few thousand lines of Rust that run with `cargo test`, where Appium needs Node, a server, drivers, Xcode and the .NET SDK on every machine that runs the tests, and the Mac2 driver needs an application bundle registered with Launch Services. The cost is that the driver semantics the tests depend on are re-created here; section 3 lists them, and where the drivers disagree the layer follows the one upstream runs on that platform.

The existing in-process harness of `tests/FerroUI.IntegrationTests.Win32` (the port of the other, in-process project of upstream) stays as it is: those tests create windows of the window class in the test process. The automation tests drive the sample from another process, as a person or a screen reader would.

## 2. The design

| Upstream | Port | What it is |
|---|---|---|
| `AppiumDriver`, `AppiumWebElement`, `AppiumDriverEx.cs` | `infrastructure/session.rs` | `Session` and `Element`: find by accessibility id, name, XPath, class name and tag name (one, with an implicit wait, or all), text, enabled, selected, size, rect, attributes, click, clear, send keys, a picture of the screen under an element, the window handles of the session, switching and closing windows. |
| the XPath of the drivers | `infrastructure/xpath.rs` | The subset of XPath the tests use, evaluated over the tree of the backend (section 3). |
| `Actions`, `PointerInputDevice`, `ActionBuilder` of Selenium | `infrastructure/actions.rs` | Chains of mouse and keyboard steps and pointer sequences. |
| `Keys` of Selenium | `infrastructure/keys.rs` | The keys of the WebDriver protocol (characters of the private use area) and their meaning. |
| `DefaultAppFixture.cs`, `OverlayPopupsAppFixture.cs`, `CollectionDefinitions.cs` | `infrastructure/default_app_fixture.rs` | The collections (`Default`, `WindowDecorations`, `OverlayPopups`): each starts the sample once (the last with `--overlayPopups`) and ends it after its tests. |
| `TestBase.cs` | `infrastructure/test_base.rs` | Selects the page of the class in the pager before a test, with upstream's retries. |
| `ElementExtensions.cs` | `infrastructure/element_extensions.rs` | `WindowChrome`, the system and client chrome buttons, `GetComboBoxValue`, `GetName`, `GetIsChecked`, `GetIsFocused`, `GetCurrentSingleWindow`, `GetWindowById`, `OpenWindowWithClick`, `SendClick`, `SendDoubleClick`, `MovePointerOver`. |
| `PlatformFactAttribute.cs`, `PlatformTheoryAttribute.cs` | `infrastructure/platform_fact.rs`, `infrastructure/registry.rs` | The platforms of a test, upstream's `Skip` reasons, and the registration of each test (one per row of a theory) with the constructor and the `Dispose` of its class. |
| the test runner of xUnit | `main.rs` | One binary without the test harness (`--test automation`): it prints every test with its result and the reason of every test it does not run, as `cargo test` prints them. |
| (the Appium server and its drivers) | `infrastructure/backend/{windows,macos,unsupported}.rs` | One type `Backend` with the same methods on every platform: start the application, the tree (children, tag names, attributes), the properties the session reads, press, focus, clear, searches, the windows, pictures of the screen, and the input of the mouse and the keyboard. |

Each upstream test file is a Rust file of the same name (`ButtonTests.cs`, `button_tests.rs`; `WindowTests_MacOS.cs`, `window_tests_mac_os.rs`): a struct for the class (its constructor is the constructor of the class with the one of `TestBase`, its `TestClass::dispose` the `Dispose`), one method per test with the steps and assertions of upstream in the same order, and a function `tests` that registers them with their platforms. A `try`/`finally` of upstream is a `catch_unwind` with the finally block after it; a `using` is a value whose drop disposes. The abstract classes with two collections (`ComboBoxTests`, `MenuTests`, `PopupsTests`) are registered twice, as `...::default` and `...::overlay_popups`. A theory is one method with upstream's parameters, registered once per row of its data (`window_tests::startup_location(400, 300, Owned, CenterOwner, true)`).

The runner runs the tests only with `FERROUI_AUTOMATION_TESTS=1`: they move the pointer and type on the keyboard of the machine, and a `cargo test` of the whole workspace (the macOS job of CI) must not do that. Without the variable the binary says so and runs nothing; `--list` lists the tests of the platform of the run with the reason of each one that does not run.

The sample is found by `FERROUI_INTEGRATION_TEST_APP`, else as `integration-test-app` in the build directory of the test binary (`cargo build -p integration-test-app` builds it there). Upstream's fixture asks the Appium server to start the executable on Windows and the registered bundle (by its identifier) on macOS; the port starts the executable on both, so the sample needs no bundle (`GAPS.md` of the sample, "Not ported").

## 3. The semantics of the drivers that the tests depend on

| What | Windows (as WinAppDriver) | macOS (as the Mac2 driver) |
|---|---|---|
| The root of a search from the session | the current window (`CurrentWindowHandle`) and the elements below it | the application and everything below it |
| A search from an element | the elements below it | the elements below it |
| How a search is made | one search of UI Automation (`FindAll` in the control view); when it finds nothing, a walk of the control view (section 9) | a walk of the children of the accessibility API |
| Accessibility id | `AutomationId` | an element whose identifier, title or label is the value: the Mac2 driver finds the accessibility id and the name alike with `matchingIdentifier:` of XCTest, which matches any of the three (a native menu item, whose identifier is its action `didSelectItem:`, is found by its title); the buttons of a title bar have XCTest's identifiers `_XCUI:CloseWindow`, `_XCUI:MinimizeWindow`, `_XCUI:ZoomWindow`, `_XCUI:FullScreenWindow` (from their subrole) |
| Name | `Name` | as the accessibility id; the attribute `name` is the first non-empty of the identifier, the title and the label (the description) |
| Class name, tag name | `ClassName`; the control type (`Window`, `Button`, `TitleBar`) | the `XCUIElementType` of the role (`XCUIElementTypeWindow`, `XCUIElementTypeStaticText`; a menu bar item of the menu extras is `XCUIElementTypeStatusItem`) |
| The tree of XPath | the control view of UI Automation | the children of the accessibility API; the application also has its menu extras |
| Text | the value of the value pattern, else the value of the range value pattern, else the name | the first non-empty of the value, the title and the label; a number is written as `NSNumber` writes it (`20`, `52.5`), a boolean as `0` or `1` |
| Attributes | the properties by their names (`Name`, `AutomationId`, `HasKeyboardFocus`, `AcceleratorKey`, `Toggle.ToggleState`, ...): booleans `True`/`False`, enumerations as their number | `identifier`, `title`, `label`, `value`, `enabled`, `selected`, `hittable` (booleans `true`/`false`) |
| `Click()` | a click of the mouse at the centre, after the element is brought into view (its scroll item pattern) where its centre is outside one of its ancestors up to the window or it is off the screen | a click of the mouse at the centre (`click` of XCTest), after `AXScrollToVisible` where a click at the centre would not reach the element; a menu item and an item of the menu bar are pressed (`AXPress`), as XCTest selects a menu item through its menus whether or not its menu is open |
| `SendKeys(text)` | the keyboard focus to the element (`SetFocus`), then the keys; no click, whether or not the element reports the focus afterwards | the same; where setting the focus does not take, a click at the centre first |
| `Clear()` | the value pattern set to the empty text, else select all and delete | the value set to the empty text, else select all and delete |
| `Selected`, `Enabled` | the selection item pattern; `IsEnabled` | `AXSelected`; `AXEnabled` |
| Size and rect | pixels of the screen (the process is aware of the scale of every monitor) | points of the screen |
| Window handles | the top-level windows of the process in UI Automation, as hexadecimal handles; `SwitchTo().Window` makes one the root and brings it to the front; `Close()` closes it through its window pattern | not used by the tests |
| A picture of the screen under an element | the pixels of the screen copied with `BitBlt` | not provided (section 5) |

XPath: the tree below the root of the search is a document whose root element is that root, and a relative path starts at it (`XCUIElementTypeWindow` from the session is the windows of the application; `/*/XCUIElementTypeStaticText` from a window is its static texts). The subset is absolute and relative paths, `/` and `//`, the axes `child`, `parent`, `self`, `descendant`, `descendant-or-self` (and `.`, `..`), the node tests `*`, a name and `node()`, and the predicates `[n]`, `[@attribute]` and `[@attribute='value']`, with the positions of XPath (`//*[@AutomationId='NativeTextBox']//*[1]` is the first child of each node below the host). The evaluation was checked against a mock tree with every expression the tests use **[V]**.

The keys of the protocol: a modifier in a string of keys is pressed by one occurrence and released by the next, `Null` releases every modifier, any other key is pressed and released; `SendKeys` of an element releases the modifiers still down at its end, the actions keep them down until a key up (`KeyDown(Keys.Control).SendKeys("g").KeyUp(Keys.Control)`). A character is typed as the key of the layout that produces it (with the shift key where it needs it), so that the application sees the key and the physical key of the character (`KeyboardTests.Unhandled_Key_Still_Produces_Text` reads both); on macOS the event also carries the character.

The actions: `MoveToElement(element)` moves to the centre, `MoveToElement(element, x, y)` to an offset from the top left corner (upstream's default configuration is Appium 1, whose client measures that offset from the corner; `WindowTests_MacOS` grabs the resize grip of a window at `(100, 1)` that way), `MoveToElementCenter(element, x, y)` and a pointer move of a sequence to an offset from the centre. A button or a key pressed by an action stays down until an action releases it, also across `Perform` (`GestureTests.Tapped_Is_Raised_Slow`). A move while a button is down takes 250 ms in steps (the default duration of a move of the actions of Selenium), so that a drag sees the moves between; a move with no button down is immediate. After every test the runner releases the buttons and keys still down (no command of the drivers does that: it keeps a test that fails with a button or a modifier down from leaving it down for the next tests and for the desktop after the run).

Bringing an element into view before a click is the "scroll into view" step of the element click of WebDriver, which XCTest does before a tap. Upstream's runs do not depend on it on Windows: their agent has a desktop of 4K, where the main window of the sample shows every item of its list of pages. On a smaller desktop the default size of the window (`CW_USEDEFAULT`) is lower than the list (20 items of 35 pixels and the bars of the window need about 754 units of height), and the last pages are below the end of the list (section 9).

Timing: a search for one element waits up to one second for it to appear (the drivers wait for the application to be idle, XCTest more than WinAppDriver); every command that gives input waits a moment afterwards for the application to handle it (150 ms on Windows, 500 ms on macOS: 250 ms was too short for the action of a native menu item). A test that asserts that an element is gone (`Assert.Throws<WebDriverException>`) therefore takes the second of the wait.

## 4. The tests per class and platform

117 upstream tests in 22 files (`TESTS.md`), 216 rows registered on macOS and 224 on Windows (the theories of `WindowTests.WindowState` leave out full screen for owned and modal windows on macOS, as upstream's data does).

| Upstream file | Tests | Rows | Windows | macOS |
|---|---:|---:|---|---|
| `AutomationTests.cs` | 2 | 2 | 2 | 2 |
| `ButtonTests.cs` | 5 | 5 | 5 | 4 (`ButtonWithAcceleratorKey` is Windows only) |
| `CheckBoxTests.cs` | 3 | 3 | 3 | 3 |
| `ComboBoxTests.cs` | 7 | 14 | 14 | 4 (the keyboard tests are Windows only) |
| `ContextMenuTests.cs` | 1 | 1 | 1 | 0 |
| `DragDropTests.cs` | 3 | 3 | 1 | 3 |
| `EmbeddingTests.cs` | 4 | 4 | 4 | 0 (upstream: "Not yet working on macOS") |
| `GestureTests.cs` | 10 | 10 | 8 | 9 |
| `KeyboardTests.cs` | 6 | 6 | 6 | 6 (below) |
| `ListBoxTests.cs` | 4 | 4 | 3 | 2 (`Can_Select_Items_By_Ctrl_Clicking` skipped upstream on both) |
| `MenuTests.cs` | 10 | 20 | 20 | 4 |
| `NativeMenuTests.cs` (`NativeMenuTests`, `DockMenuTests`) | 6 | 6 | 2 | 3 (`MacOS_Native_Menu_Has_ToolTip_If_Defined` skipped upstream: flaky) |
| `PointerTests.cs` | 1 | 1 | 1 | 1 |
| `PointerTests_MacOS.cs` | 0 | 0 | - | - (a class with helpers and no tests at the tracked commit; ported, not registered) |
| `PopupsTests.cs` | 5 | 10 | 10 | 0 |
| `RadioButtonTests.cs` | 2 | 2 | 2 | 2 |
| `ScreenTests.cs` | 2 | 2 | 2 | 2 |
| `ScrollBarTests.cs` | 1 | 1 | 1 | 1 |
| `SliderTests.cs` | 4 | 4 | 2 | 2 (the two drags skipped upstream: flaky) |
| `TrayIconTests.cs` | 3 | 3 | 0 | 0 (all three skipped upstream: flaky) |
| `WindowDecorationsTests.cs` | 6 | 14 | 8 | 8 |
| `WindowTests.cs` | 15 | 90 on Windows, 82 on macOS | 90 | 77 (the two pictures of the screen are not provided, section 5; three are Windows only) |
| `WindowTests_MacOS.cs` | 17 | 19 | 0 | 18 (`Does_Not_Switch_Space_...` skipped upstream: flaky) |
| **Total** | **117** | **224 / 216** | **185 run, 39 not** | **151 run, 65 not** |

Every test that does not run on a platform is printed with its reason: `Ignored on <system>` (with the reason of a `[PlatformFact]` that has one), upstream's `Skip`, or "not provided by the harness" with what is missing (section 5). `--include-ignored` runs the skipped ones as well.

Where the configuration of upstream decides:

- `KeyboardTests`: upstream builds the six tests as `[Fact]` with Appium 2 and as Windows only with Appium 1, because the Mac2 driver of Appium 1 does not implement the key actions of the protocol. The harness posts the keys itself on both platforms, so the tests are the `[Fact]` of the Appium 2 configuration.
- `WindowTests_MacOS.WindowOrder_Owned_Dialog_Stays_InFront_Of_Parent` and `..._Of_FullScreen_Parent` carry their `[PlatformFact(MacOS)]` only in the Appium 2 configuration (`#if APPIUM2`); they are registered, for the same reason.
- `TrayIconTests` selects the page `Desktop`, which the sample does not have (its page is `DesktopPage`): the three tests are skipped upstream, and would fail in their constructor if run, upstream as here.
- `GestureTests.RightTapped_Is_Not_Raised_For_Drag` builds its pointer sequence and does not perform it (upstream does not call `PerformActions`); the port does the same.

## 5. What the harness does not provide

- **A picture of the screen on macOS** (`WindowTests.TransparentWindow`, `TransparentPopup`). Copying the screen needs the Screen Recording permission and ScreenCaptureKit (the function upstream's driver used, `CGWindowListCreateImage`, is obsolete since macOS 15), and the runs of this port do not capture the screen of the host. The two tests are reported as not provided on macOS; on Windows the harness copies the screen under the element as WinAppDriver does.
- **Nothing else** that the registered tests use: the windows of the session, the chrome of the windows, the menu bar and the menu extras, pointer sequences, modifiers and key presses are provided on both platforms. Whether the sample and the framework answer them as upstream's do is what the runs show (sections 6 and 7).

## 6. Windows

- `cargo check --target x86_64-pc-windows-msvc` and `aarch64-pc-windows-msvc` of the crate with its tests, and of the extended client of `ferroui-win32` **[V]**. The slots and identifiers the client uses beyond the ones of `win32-platform.md`, section 13.5 (`CompareElements`, `GetRootElement`, `ElementFromPoint`, `get_ControlViewWalker`, `get_ControlViewCondition`, `CreateTrueCondition`, `CreatePropertyCondition`, `CreateAndCondition`, `FindAll`, `GetCurrentPropertyValue`, the process identifier, accelerator key, automation identifier, window handle and off-screen properties, `IUIAutomationElementArray`, the expand collapse, selection item and window patterns, `GetParentElement`) were compared with the declarations of the `windows` crate 0.62.2 (generated from the metadata of the Windows SDK) **[V]**.
- The sample on Windows now has what the upstream sample has there: the native text box of the Windows platform (`Embedding/win32_text_box_factory.rs`, `win32_window_control_handle.rs`, `win_api.rs`) for the embedding tests, and the option `OverlayPopups` of the Windows backend for the collection `OverlayPopups` (`GAPS.md` of the sample). The desktop entry point of the sample cannot be checked from the Mac (Skia and HarfBuzz need the C++ toolchain of the target); its Windows files were checked for both targets with the real crates they use **[V]**.
- CI: the step "Automation tests of the IntegrationTestApp sample" of the job `windows` (x86-64; the hosted runner has an interactive desktop, as for the integration tests of windows): `cargo build -p integration-test-app`, then the tests with `FERROUI_AUTOMATION_TESTS=1`. It does not fail the job until it has a green run **[CI]**.
- The virtual machine: `scripts\windows\vm-smoke.ps1 -Automation` (with `-InteractiveUser`; `-AutomationArguments` passes a filter or `--include-ignored`) builds the sample and the tests and runs them in the session of the user **[VM]**.

## 7. macOS

- The crate, its tests and the sample build on the Mac **[V]**; the sample starts and ends on its own (`FERROUI_SMOKE_EXIT_MS`) **[V]**.
- The run needs the Accessibility permission of the process that runs the tests: the accessibility API of another application and the posted input are refused without it. On the Mac of the port the agent's process does not have it (`AXIsProcessTrusted()` is false, a read of another application answers `kAXErrorAPIDisabled`, -25211), so the tests have not run against the sample yet **[P]**. The runner checks the permission first and stops with what is missing. The permission is given in System Settings, Privacy & Security, Accessibility, to the application that starts the run (the terminal, or the application of the agent).
- What was found by reading for the run: the native layer exposes what the tests read (identifier, title, value, selected, enabled, frame, press, the role of every control type, the window's identifier from its automation peer); the nested window element upstream's comment mentions does not exist in the port (the view of a window lists the children of the window's peer), which the XPath of `GetCurrentSingleWindow` handles either way.

## 8. How to run

```text
cargo build -p integration-test-app
FERROUI_AUTOMATION_TESTS=1 cargo test -p ferroui-integration-tests-appium --test automation
FERROUI_AUTOMATION_TESTS=1 cargo test -p ferroui-integration-tests-appium --test automation -- combo_box_tests
cargo test -p ferroui-integration-tests-appium --test automation -- --list
FERROUI_AUTOMATION_TESTS=1 cargo test -p ferroui-integration-tests-appium --test automation -- embedding_tests --dump-tree
```

`--dump-tree` (or `FERROUI_AUTOMATION_DUMP_TREE=1`) is a diagnostic of the runs, not a command of the drivers: a search for one element that does not find it prints the tree it searched (the walk of the children with the tag, the identifier, the name and the bounds of every element) and, on Windows, what one search of UI Automation returns for the whole subtree. The runner also prints where the main window is when a collection starts.

Nobody should use the mouse or the keyboard of the machine while the tests run.
