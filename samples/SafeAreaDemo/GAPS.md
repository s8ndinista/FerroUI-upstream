# SafeAreaDemo: gaps

What the sample and its hosts use that the framework lacks or does differently. Status: 3 documents (`App.xaml`, `Views/MainView.xaml`, `Views/MainWindow.xaml`), converted without an edit, all compiled by the build; `cargo test -p safe-area-demo --lib` passes (16 tests). The desktop host runs on macOS with a smoke run (`FERROUI_SMOKE_STEPS_MS`, `FERROUI_SMOKE_SCREENSHOTS`); the Android and iOS hosts are built and packaged by `scripts/android/apk.sh safearea` and `scripts/ios/bundle.sh`.

| Gap | What the sample does | The upstream statement | The framework | Status |
|---|---|---|---|---|
| SA001 | A debug build of the desktop host attaches the developer tools to the application. | `SafeAreaDemo.Desktop/Program.cs`: `#if DEBUG .WithDeveloperTools() #endif` | The application builder has no developer tools extension (the diagnostics support package of the upstream project is not ported). The statement is left out of `program.rs`; nothing else depends on it. | open: nothing to reproduce |
| SA002 | The Android host has a splash screen and an icon. | `SafeAreaDemo.Android/Resources/drawable*`, `Resources/values*/styles.xml`, the `Icon` of the activity | The package of `scripts/android/apk.sh` carries no resources but the manifest; its activity has the theme of the system without an action bar. | open: the resources of a package are not built by the script |
| SA003 | The iOS host has a launch screen. | `SafeAreaDemo.iOS/Resources/LaunchScreen.xib` | `scripts/ios/bundle.sh` states an empty launch screen (`UILaunchScreen`) instead of a storyboard. | open: as for `ControlCatalog.iOS` |

Not gaps:

- `ViewLocator.Build` finds the view by the name of the class of the view model through reflection (`Type.GetType`, `Activator.CreateInstance`); the port looks the name up among the classes markup knows (`TypeInfo::find`) and creates the class with its default constructor. `Match` tests `data is ViewModelBase`: a view model of the port embeds `ViewModelBase` instead of deriving from it, so `ViewModels/mod.rs` lists the view models of the sample (`MainViewModel`).
- The transition of the marker button binds its `Duration` and its `Easing` to `InputPaneDuration` and `InputPaneEasing`, which are null until the input pane of the platform changes its state. Both bindings report that null converts to neither type, as the bindings of the reference do (a null does not convert to a value type, and `IEasing?` null is not an `Easing`); `tests/view.rs` accepts exactly these two reports.
- `SafeAreaDemo.Desktop.csproj` references the VNC headless platform, the Linux framebuffer and the X11 platform, which `Program.cs` does not use; the platform detection of the desktop package chooses the X11 platform on Linux.
- On a desktop window there is neither an insets manager nor an input pane: the view shows its bounds, and the options change nothing but the theme variant, as with the reference.
