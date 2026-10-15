# The WinUI integration

The design of the port of upstream's `src/Windows/Avalonia.WinUI` (the tracked commit, `17350180`): the framework's content inside a WinUI 3 application of the Windows App SDK. The crate is `ferroui-winui` (`src/Windows/FerroUI.WinUI`), which held the reserved name until this port (`docs/release.md`, section 2.2). The differences from upstream are in `DEVIATIONS.md`, "WinUI integration"; the state is in `CRITICAL-PATH.md`, row 32.

Marks used below: **[V]** verified here (read in the sources named, or run on this machine), **[CI]** to be established by the step of the Windows job of CI (section 7.3), **[VM]** established in the owner's Windows 11 ARM64 virtual machine (section 7.4; the runs are section 7.5).

## 1. What is ported

| Upstream project | Files | Types | Members | State |
|---|---:|---:|---:|---|
| `Avalonia.WinUI` | 15 | 16 | 88 | ported file by file (this document); tracking 15/15, 16/16, 88/88 |
| `Avalonia.Win32.Interoperability` | 2 | - | - | not applicable: hosting in Windows Forms (section 8) |

The sample of the reference that uses the integration, `samples/ControlGallery.WinUI`, is ported as a Rust program (section 7.2). `samples/WinUIEmbedSample` (WinUI controls beside the embedded content) is not ported yet.

## 2. What upstream does

`Avalonia.WinUI` is a library for a C# application of the Windows App SDK. Its one public class is `AvaloniaSwapChainPanel`, a `Microsoft.UI.Xaml.Controls.SwapChainPanel` that an application puts into the XAML of a window and gives a `Control` as `Content`. Everything else is internal:

| Upstream file | What it does |
|---|---|
| `AvaloniaSwapChainPanel.cs` | The panel. On `Loaded` it creates a `SwapChainGlSurface` and a `SwapChainTopLevelImpl` of the Windows backend and an `EmbeddableControlRoot` over them, starts rendering, and hands the swap chain the surface creates on the render thread to the panel (`ISwapChainPanelNative::SetSwapChain`, on the dispatcher queue of the panel). It forwards pointer, wheel, key and character events of WinUI as raw events of the framework, escapes the tab cycle of the content to the next WinUI element, maps the cursor the content asks for to an `InputCursor` of a transparent overlay grid (`ProtectedCursor` is protected), takes drags in (`DragEnter` builds a data transfer from the `DataPackageView`) and starts drags out (`StartDragAsync` of the overlay with the last pointer point), and creates its automation peer. A static constructor binds the drag source. |
| `WinUITextInputMethod.cs` | The text input method of the top-level over a `CoreTextEditContext` of the system: the text and the selection the input method sees, the composition (preedit, commit), the layout bounds in screen pixels. |
| `WinUICursorFactory.cs` | The standard cursors as `InputSystemCursor` shapes; a bitmap cursor falls back to the arrow (WinUI 3 has no cursor from an image). It is not registered: the cursors of the content are those of the Windows backend, which the panel maps back to their shapes. |
| `WinUIDragSource.cs` | The drag source of the platform: finds the panel of the top-level that started the drag and lets it start a drag of WinUI. |
| `WinUIKeyInterop.cs` | The key, physical key and symbol of a key event of WinUI through the key tables of the Windows backend (the scan code and the extended flag packed as the `lParam` of `WM_KEYDOWN`); the modifiers of a pointer event. |
| `Automation/*.cs` (10 files) | `AvaloniaSwapChainPanelAutomationPeer`, a `FrameworkElementAutomationPeer` whose children are the automation peers of the content, and `AvaloniaToXamlPeerProxy` (a partial class in eight files), an `AutomationPeer` of XAML for each peer of the framework, with nine patterns. `ControlTypeMap` maps the control types. |

What it uses of the Windows backend: `SwapChainGlSurface` (ANGLE, a composition swap chain), `SwapChainTopLevelImpl`, `KeyInterop`, `CursorImpl` and `CursorFactory`. It uses none of the composition modes of the backend (the panel presents through WinUI's own compositor), none of its input method (text input is `CoreTextEditContext`) and none of its UI Automation providers (accessibility goes through XAML's automation peers). A C# application of the sample sets the framework up with `UseWin32().UseSkia().UseHarfBuzz().SetupWithLifetime(...)` in its `OnLaunched` and runs XAML's message loop, which also pumps the message window of the backend on the same thread.

## 3. The bindings

### 3.1 Why the MicroCom generator

The reference calls WinUI through C#/WinRT, the projection of its runtime. The port has three choices for Rust:

| | MicroCom IDL (the port's generator) | the `windows` crate | a projection generated with `windows-bindgen` |
|---|---|---|---|
| WinUI 3 (`Microsoft.UI.*`) | declared in the crate's IDL | not covered (the crate projects the namespaces of the Windows SDK, not the Windows App SDK) | a build step over the metadata of the Windows App SDK, which is not in the repository |
| What the Windows backend uses | the same generator (`win_rt/winrt.idl` and three more files) | `windows-sys` only; `windows` is in the lock file for `fontique` of the Vello backend | - |
| Implementing interfaces (delegates, overrides) | `Impl` traits of the generator | the crate's `implement` macro | the same |

The integration uses the port's generator, as `win32-platform.md`, section 2.2, decided for the Windows backend: one COM model in the backends of Windows, and no second reference-counting discipline. The `windows` crate would not have removed the work anyway: it has none of the WinUI interfaces.

### 3.2 The interface definition file

`src/Windows/FerroUI.WinUI/interop/winui.idl` (2,470 lines) declares 132 interfaces and delegates, 22 enumerations and 15 structures; the build script of the crate runs the generator over it (`$OUT_DIR/winui.rs`, 40,000 lines of bindings, which compile on every host). The reference has no such file.

- **Where it comes from.** `scripts/winui/winui-idl.py` writes it from the metadata: `Microsoft.UI.Xaml.winmd` and `Microsoft.UI.winmd` of the package `Microsoft.WindowsAppSDK` 1.6.241114003, and `Windows.Foundation.UniversalApiContract.winmd` and `Windows.Foundation.FoundationContract.winmd` of `Microsoft.Windows.SDK.NET.Ref` 10.0.19041.57, both on nuget.org (both were in the package cache of this machine, restored by upstream's build). The script reads them through the IL disassembler of Mono (`ikdasm`) and emits every interface of its list with its identifier and its methods in the order of the vtable. Running it again reproduces the committed file byte for byte **[V]**.
- **Release 1.6.** Upstream builds against 1.8 (`Microsoft.WindowsAppSDK` 1.8.260209005). An interface of the Windows Runtime never changes once published, and every interface the reference calls exists in 1.6; the bindings work with any runtime from 1.6 on. The bootstrap asks for the release by its version (section 3.3), 1.6 by default.
- **Conventions.** A property is a `[propget]` and a `[propput]` method (`actual_width()`, `set_handled(..)`), an event the two methods `add_X` and `remove_X`, an overloaded method has the name of its overload attribute (`TryMoveFocusWithOptions`, `SetStorageItemsReadOnly`). A runtime class is passed as its default interface. A parameter of a type the file does not declare is `IInspectable*` or `void*`; an enumeration it does not declare is its underlying type. The ABI's `boolean` is one byte (`u8`): `win_rt.idl` of the backend maps it to four, which the backend never passes.
- **Parameterized types.** Each instance the integration uses (`IVector<AutomationPeer>`, `TypedEventHandler<CoreTextEditContext, CoreTextTextUpdatingEventArgs>`, `IAsyncOperation<StorageFile>`, 34 in all) is declared under a name of its own with the identifier the Windows Runtime derives from its signature (a version 5 UUID of SHA-1 over a namespace and the signature). The signature is in the comment above each; a test of the crate derives every identifier again with its own SHA-1 **[V]**, and checks the derivation against a value of the Windows SDK headers (`IVectorView<String>`).
- **Beside the IDL**, `ISwapChainPanelNative` (from `microsoft.ui.xaml.media.dxinterop.h`, the identifier the reference names) and `IAgileObject`.

### 3.3 The bootstrap of the Windows App SDK

A C# application of the SDK gets two initializers from the build tasks of the SDK's package: `MddBootstrapAutoInitializer` (calls `MddBootstrapInitialize2` of `Microsoft.WindowsAppRuntime.Bootstrap.dll`, which adds the framework package of the runtime to the package graph of an unpackaged process) and `UndockedRegFreeWinRT-AutoInitializer` (loads `Microsoft.WindowsAppRuntime.dll` and calls `WindowsAppRuntime_EnsureIsLoaded`, which makes the classes of the framework package activatable). A Rust program has no such build tasks. `windows_app_sdk.rs` does both: `WindowsAppSdk::initialize(&WindowsAppSdkOptions)` loads the bootstrap library at run time (`LoadLibraryW`, so nothing is linked against it and the crate builds without the SDK), calls `MddBootstrapInitialize2` with the release (`0x00010006`, no tag, any runtime version, no dialog by default where the SDK's initializer shows one), loads the runtime library and calls its export; the value calls `MddBootstrapShutdown` when dropped. A failure is a message that names what is missing.

What the machine needs: the Windows App Runtime of the release, installed (`windowsappruntimeinstall-<arch>.exe` of Microsoft), and `Microsoft.WindowsAppRuntime.Bootstrap.dll` of that release beside the executable (in the package `Microsoft.WindowsAppSDK`, `runtimes/win-<arch>/native`; the library is redistributable with an application). `scripts/winui/fetch-windows-app-sdk.ps1` puts the library beside an executable and, with `-InstallRuntime`, installs the runtime.

### 3.4 The application and its windows

The sample of the reference is C# and XAML that the XAML compiler turns into code: `App : Application` (`OnLaunched`), `MainWindow : Window` with a grid that holds the panel, a `Main` that calls `Application.Start`. `xaml_application.rs` is that code for Rust: `XamlApplication::start(on_launched)` initialises the thread as a single-threaded apartment (`RoInitialize`, the `[STAThread]` of the C# program), and calls `Application.Start` with a callback that creates the application object, composed (section 3.5) so that it overrides `OnLaunched`, which calls the closure. `XamlWindow` and `XamlGrid` are a window and a grid of WinUI. Not done: `Application.Resources` with `XamlControlsResources` (the merged dictionary of the reference's `App.xaml`): the panel and the grid use no control template, and the resources need the XAML type information (`IXamlMetadataProvider`) of a compiled application.

### 3.5 What the projection of the reference's runtime does, here

| Projection feature | Here |
|---|---|
| Deriving from a composable class (`class AvaloniaSwapChainPanel : SwapChainPanel`, a peer derived from `FrameworkElementAutomationPeer`, `App : Application`) | `interop/composable.rs`: the derived class is the outer object of a composition. It is a COM object of the MicroCom runtime made with the overridable interfaces it implements (`IUIElementOverrides`, `IAutomationPeerOverrides`, `IApplicationOverrides`); the composable factory of the base class creates the inner object for it; `ferroui_microcom::set_inner` (added to the runtime for this) makes the outer object ask the inner object for every other interface and keeps it alive. A method that is not overridden calls the inner object's implementation; a protected member (`ProtectedCursor`, `ProviderFromPeer`) is an interface of the inner object. |
| Delegates (`+=` of a lambda) | `interop/event_handlers.rs`: a COM object of the delegate's interface over a closure, run under the guard of the backend (section 6). A delegate passed as an `object` (`UIElement.AddHandler`) is boxed as an `IReference<T>` of it, as C#/WinRT and C++/WinRT box it. |
| `await` of an asynchronous operation | `interop/async_operation.rs`: a future whose completion handler (agile) wakes the task; the tasks run on the dispatcher of the UI thread (`start_local`), where the await of the reference resumes too. |
| `List<T>` passed as `IList<T>` or `IEnumerable<T>` | `interop/collections.rs`: `IVector<AutomationPeer>` (with its view and iterator) and `IIterable<IStorageItem>`. |
| `string` | string handles of the backend (`HStringInterop`), created and deleted around a call. |
| `ConditionalWeakTable` of the proxies | a table of the UI thread from peer to proxy that does not count the proxy (section 5). |

## 4. The panel

`FerroSwapChainPanel` (`ferro_swap_chain_panel.rs`) has the members of the reference. Ownership differs from the reference's, where the panel is the XAML element and lives as long as anything references it:

- The application owns the panel (`Rc<FerroSwapChainPanel>`, from `FerroSwapChainPanel::new()`) and adds its element (`as_ui_element()`) to the tree of a window. The panel holds its element; the element (the outer object) holds the panel weakly, and every handler it registers holds the panel weakly. Counted references would make a cycle (element, handler, panel, element) that only a garbage collector breaks.
- An element whose panel was dropped is a plain swap chain panel: its overrides call the base implementation and its handlers do nothing. Dropping the panel does what `Unloaded` does (stops rendering, disposes the root and the swap chain).

The life cycle is the reference's: `Loaded` creates the surface, the top-level and the root and starts rendering; `Unloaded` stops and disposes them; the size and the composition scale are read on the UI thread and handed to the render thread through atomics (the reference reads two fields from that thread).

### 4.1 File table

| Upstream file | Lines | FerroUI file | State and notes |
|---|---:|---|---|
| `AvaloniaSwapChainPanel.cs` | 915 | `ferro_swap_chain_panel.rs` | ported; the decisions of the pointer events and the text of a character are functions of values, tested on every host |
| `WinUICursorFactory.cs` | 91 | `win_ui_cursor_factory.rs` | ported; the shape of each standard cursor tested on every host |
| `WinUIDragSource.cs` | 42 | `win_ui_drag_source.rs` | ported; the drag runs as a future on the dispatcher |
| `WinUIKeyInterop.cs` | 62 | `win_ui_key_interop.rs` | ported; the key data and the modifiers tested on every host |
| `WinUITextInputMethod.cs` | 384 | `win_ui_text_input_method.rs` | ported; the text is kept as UTF-16, as the ranges of the edit context count; the operations on it tested on every host |
| `Automation/AvaloniaSwapChainPanelAutomationPeer.cs` | 143 | `automation/ferro_swap_chain_panel_automation_peer.rs` | ported (composed `FrameworkElementAutomationPeer`) |
| `Automation/AvaloniaToXamlPeerProxy.cs` | 141 | `automation/ferro_to_xaml_peer_proxy.rs` | ported (composed `AutomationPeer`) |
| `Automation/AvaloniaToXamlPeerProxy.*.cs` (7) | 178 | `automation/ferro_to_xaml_peer_proxy_*.rs` | ported, one file per pattern as the partial class has; the state mappings tested on every host |
| `Automation/ControlTypeMap.cs` | 52 | `automation/control_type_map.rs` | ported, tested on every host |
| `Avalonia.WinUI.csproj` | 30 | `Cargo.toml` | not applicable: the project file |

### 4.2 Where the port differs, and why

- **A surrogate pair of `CharacterReceived`.** The reference sends each UTF-16 unit as a string, a lone surrogate among them; a Rust string holds no lone surrogate. The first half of a pair is kept and the text is sent with the second.
- **Intermediate points.** The reference fills one static list for every move event; the lazy value of the base library owns its points, so each event has a list of its own.
- **The swap chain on its way to the UI thread** is counted (the reference hands the pointer over uncounted and relies on the surface keeping it).
- **Errors.** A call of WinUI that fails is an `HResult`, not an exception: what the reference does with an exception (logs it, or lets it reach the system) is done with the result; where the reference has no handler, the result is ignored as the default value of the call would be, and a failure the reference lets escape (`Marshal.ThrowExceptionForHR` of `SetSwapChain`) is a panic, which the guard keeps (section 6).

## 5. Automation

The panel's element creates its peer through the override of `OnCreateAutomationPeer` on the outer object. The peer of the panel and the proxies are composed objects whose `IAutomationPeerOverrides` is the reference's: the class name, the control type, the children (the base's children and the proxy of the embedded root), the focused element and the element at a point, and for a proxy every member of its peer with the patterns `GetPatternCore` answers with the proxy itself. A pattern a provider of the framework refuses with `ElementNotEnabledException` answers `UIA_E_ELEMENTNOTENABLED`, as the exception of the reference reaches the client.

The reference keeps a proxy per peer in a `ConditionalWeakTable`, which lets the proxy live while its peer does. A counted proxy that holds its peer cannot be held by a table that lets the peer go; here the table does not count the proxy, a proxy leaves the table when it is destroyed, and while XAML holds a proxy, the same peer gets the same proxy. The proxies hold the peer of the panel weakly (it holds the panel weakly too).

## 6. Threads

Everything of the integration runs on the thread of the XAML application, which is the UI thread of the framework: `Application.Start` runs XAML's message loop, which dispatches the messages of the message window of the Windows backend too, so the dispatcher, its timers and its tasks work as they do in a window of the backend.

| Thread | What of the integration runs there |
|---|---|
| UI thread | every handler of WinUI and of the edit context, the overrides of the composed objects, the tasks of drag and drop |
| render thread (the render timer of the backend's composition mode) | the size and scaling functions of the surface, and the callback that receives the swap chain, which posts the work to the dispatcher queue of the panel (an agile `DispatcherQueueHandler`) |
| any thread | the completion handlers of asynchronous operations (agile; they only record the status and wake the task) |

**Panics.** The system calls a handler through its own frames, which a panic must not cross. Every handler runs under the guard of the Windows backend (`ferroui_win32::wnd_proc_guard`, public now): a panic is kept, the handler returns, and a hook the application sets ends XAML's message loop (`Application.Exit`); `XamlApplication::start` raises the panic again once `Application.Start` has returned. The reference's runtime carries an exception through the frames of the system instead.

## 7. How it is tested

### 7.1 On any host

- `cargo test -p ferroui-winui`: 21 tests **[V]** of what is logic: the identifiers of the parameterized instances against their signatures, a few identifiers against the metadata and the headers, the layout of the structures of the ABI, the key data and the modifiers, the cursor shapes, the control types and the states of the patterns, the raw events of a press and a release, the text of a character and a surrogate pair, the operations on the UTF-16 text of the input method, the boolean of the ABI, the release of the bootstrap.
- `cargo test -p ferroui-microcom`: the composed object (a query answered by the inner object, the inner object released with the outer one) **[V]**.
- `cargo check -p ferroui-winui --target x86_64-pc-windows-msvc` and `--target aarch64-pc-windows-msvc` **[V]**, with `--tests` for the first: every call matches its binding. The workspace dependency on `ferroui-win32` is without ANGLE now, which makes this check possible (ANGLE compiles C++ for the target).
- The sample cannot be checked for a Windows target on a Mac (the catalog depends on Skia, and HarfBuzz and ANGLE compile C++); its own sources were checked against stand-ins of those three crates **[V]**.

### 7.2 The smoke run of the sample

`samples/ControlGallery.WinUI` (`control-gallery-winui`) is the sample of the reference: the bootstrap, `Application.Start`, `App::on_launched` (the framework set up with the catalog's application, the Windows backend, Skia and HarfBuzz, and a single view lifetime), `MainWindow` (a window of WinUI with a grid and the panel, whose content is the main view). With `--smoke` it prints each check and exits with 0 only if all passed:

1. the Windows App SDK is bootstrapped (the release, the runtime found);
2. the framework is set up; the catalog set the main view of the lifetime;
3. the main window with the panel is created and activated;
4. the panel loads (`Loaded` created the root), and the content has a client size;
5. the swap chain is set on the panel (`ISwapChainPanelNative::SetSwapChain` succeeded on the UI thread after the first render target created it);
6. 30 frames of the content are rendered (animation frames of its top-level);
7. the automation tree XAML builds of the panel (`FrameworkElementAutomationPeer.CreatePeerForElement` of its element): the class name `FerroSwapChainPanel` and the control type `Pane` (overrides of the composed peer), at least one child (the proxy of the embedded root), which has the content's peers as children;
8. the window closes, and the application ends.

A check that has not passed after 60 seconds fails the run. Each call of the system in step 7 and the close of step 8 is announced by a `[step]` line, and the automation objects of the integration print a `[trace]` line on the error stream for what XAML asks of them (`FERROUI_WINUI_TRACE`, which the smoke run sets): a run that ends in the frames of the system names the call it was in. When the window is closed, the application lets go of it (and of its panel) inside the handler of `Closed`, while XAML still runs, and then ends the loop. What it does not check: pixels of the swap chain (a capture of a window does not reliably hold a composition swap chain), input (it needs a person or injected input), the input method, drag and drop and the cursor.

### 7.3 CI

The `windows` job has an informative step (`continue-on-error`), after the pictures of the catalog: the tests of the crate on the target, the build of the sample, `fetch-windows-app-sdk.ps1 -InstallRuntime` (the runtime 1.6 installed on the runner, the bootstrap library beside the executable) and the smoke run. Not measured yet: whether the image of `windows-latest` installs the runtime (the installer adds packages; the runner is an administrator) and runs a WinUI 3 application in its session. The step stays informative until a first run has shown that; if the image cannot install or run it, the reason goes here and the run moves to the virtual machine alone.

### 7.4 The virtual machine

`scripts/windows/vm-smoke.ps1 -WinUI` runs the tests of the crate, fetches the bootstrap library beside the sample, checks that the Windows App Runtime 1.6 is installed (it installs nothing; `-InstallWindowsAppRuntime` installs it, with the owner's authorisation), builds the sample and runs its smoke mode in the session of the interactive user. Steps, from the host:

```
scripts/windows/vm-sync.sh <commit>
# in the machine, from the drive the share is mapped to:
powershell -NoProfile -ExecutionPolicy Bypass -File <S>:\ferroui-vm-windows\src-<commit>\scripts\windows\vm-smoke.ps1 -TargetDir C:\ferroui-target -Jobs 3 -InteractiveUser <user> -WinUI -InstallWindowsAppRuntime -Report <S>:\ferroui-vm-windows\vm-smoke-report.txt
```

The runtime the machine needs: the Windows App Runtime 1.6 for ARM64, from Microsoft's installer `windowsappruntimeinstall-arm64.exe` of release 1.6.241114003 (`https://aka.ms/windowsappsdk/1.6/1.6.241114003/windowsappruntimeinstall-arm64.exe`), which `-InstallWindowsAppRuntime` downloads and runs with `--quiet`. Once installed, later runs omit the switch.

### 7.5 What the runs in the virtual machine showed (2026-10-15)

The first run in the virtual machine (commit `629520ed`; the Windows App Runtime 1.6 installed by `-InstallWindowsAppRuntime`, installer exit code 0) **[VM]**: the 21 tests passed on the target; the sample built (508 s); the smoke run passed steps 1 to 6 and the first two checks of step 7: the bootstrap, `Application.Start` with the composed application, the window with the composed panel, `Loaded` (client size 1334 by 666), the swap chain set on the panel 3.2 s after activation, 46 frames, and the peer of the panel created through the override of `OnCreateAutomationPeer` with the class name and the control type of the overrides of the composed peer. It then ended with an access violation (`0xC0000005`) in the next call, `GetChildren` of the peer of the panel.

The cause: the creation of a proxy (`FerroToXamlPeerProxy::new`) read the value of the proxy from the pointer of its `IAutomationPeer`, which is an interface of the inner object (an object of XAML), not of the outer object the runtime of the port made; it wrote the pointer of the proxy into the memory of XAML's object, and left the proxy without its own pointer (`SetParent` of each child then dereferenced null). The value is now read from the outer object's own first interface before the query. The other three places that read the value of a composed object read it from that interface already.

The second run (commit `c22d5163`, the same machine, the runtime installed by the first) **[VM]**: `RESULT: passed`. The 21 tests passed on the target (255 s), the bootstrap library was fetched, the sample built (578 s), and the smoke run exited with 0 and passed every check of section 7.2:

- the bootstrap of the Windows App SDK 1.6, the framework set up, the main view set, the window with the panel created and activated;
- `Loaded` with the client size 1334 by 666; the swap chain set on the panel 2044 ms after activation; 40 frames rendered;
- the peer of the panel: the class name `FerroSwapChainPanel` and the control type `Pane` of the overrides of the composed peer, one child (the proxy of the embedded root, created, given its parent with `SetParent` and appended to the list of the base, as the `[trace]` lines show); the peer of the content (`EmbeddableControlRoot`) with one child, whose class name is `Panel`; the peer of the content is not a control element, as the reference collapses it;
- the window closes, the application lets go of it inside `Closed` (the `[trace]` lines show the two proxies destroyed), and the process ends with 0.

The same run passed the steps of the Windows backend that `vm-smoke.ps1` runs before it (the builds with and without ANGLE, the tests, the smoke runs in software, through ANGLE, WGL and ANGLE with DirectComposition).

## 8. `Avalonia.Win32.Interoperability`: not applicable

The project hosts the content in Windows Forms: `WinFormsAvaloniaControlHost`, a `System.Windows.Forms.Control` that creates an `EmbeddableControlRoot` (whose top-level is the embedded window of the Windows backend) as its child window and forwards focus, size and dialog characters, and `WinFormsAvaloniaMessageFilter`, an `IMessageFilter` of the Windows Forms message loop that dispatches the key messages of the framework's own windows before Windows Forms takes them. Windows Forms is a toolkit of .NET; a Rust program has no Windows Forms to host in. What stands for it in Rust:

- a window of another toolkit hosts the content through the embedded window of the Windows backend (`EmbeddedWindowImpl`, the same class the reference's control host creates), as the native control host of the framework does the other way round;
- a WinUI 3 application hosts it through this crate.

The tracking records both files as not applicable with this reason (`docs/porting/data/path-overrides.toml`), the project is in scope with no crate, and the sample that uses it (`samples/interop/WindowsInteropTest`) is not applicable for the same reason (`docs/porting/data/samples.toml`).

## 9. What is not verified

The runs in the virtual machine (section 7.5) established the bootstrap, `Application.Start` with the composed application, the composed panel with its override of `OnCreateAutomationPeer`, `Loaded`, the swap chain set on the panel and frames rendered, the composed peer of the panel with its overrides, the proxies of the content's peers with their parents and children, and the close of the window with the release of the panel and of the proxies. Not verified yet, in the order a run would show them:

- that the protected interfaces are answered by the inner objects (the cursor of the overlay); the picture the swap chain presents, with the inverse scale transform;
- input: pointer, wheel, keys, characters, the tab boundary, the cursor of the overlay;
- the input method over `CoreTextEditContext` (`GetForCurrentView` in a desktop application of WinUI, as the reference calls it);
- drag and drop in both directions, and the agile completion handlers of the operations;
- the automation tree with a client of UI Automation, and a screen reader;
- the release of the composed objects (no leak, no use after release) when a window closes.
