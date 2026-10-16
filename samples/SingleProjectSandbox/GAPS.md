# SingleProjectSandbox: gaps

What the sample does that the port does differently. Status: 3 documents, converted without an edit, all compiled by the build; `cargo test -p single-project-sandbox` passes (12 tests). The desktop and macOS entry points run on macOS with a smoke run (`FERROUI_SMOKE_SCREENSHOT`, `FERROUI_SMOKE_EXIT_MS`); the Android and iOS entry points are built and packaged by `scripts/android/apk.sh sandbox` and `scripts/ios/bundle.sh --package single-project-sandbox --bin single-project-sandbox-ios`.

The upstream sample is one .NET project with several target frameworks; what is left of it beyond the project format is the application (`App`, `MainView`, `MainWindow`) and one entry point per platform (`Platforms/<platform>/`). The port is one package of the same shape: the application is its library, and each entry point is a target of the package that Cargo builds for its platform (see `lib.rs`).

| Gap | What the sample does | The upstream statement | The port | Status |
|---|---|---|---|---|
| SPS001 | A browser entry point. | `Platforms/Browser/Program.cs` (`StartBrowserAppAsync("out")`), `Platforms/Browser/wwwroot/` (the host page, its script and style) | Not ported: a browser module of the port is an executable built for the Emscripten target with a site of its own (`scripts/build-browser.sh`, as `samples/ControlCatalog.Browser`), which needs the Emscripten SDK of the browser build; the package has no browser target yet. | open |
| SPS002 | The macOS entry point selects the native platform and Skia. | `Platforms/macOS/Program.cs`: the native platform of macOS and `UseSkia()` | Without a text shaping system the setup of the reference fails ("No text shaping system configured"); the port adds HarfBuzz (`use_harfbuzz()`), as the platform detection of the desktop entry point does, and the application starts. | a deviation: the reference is out of date there |
| SPS003 | The macOS and iOS targets make application bundles with property lists and entitlements; the Android target has a manifest. | `Platforms/macOS/Info.plist`, `Entitlements.plist`; `Platforms/iOS/Info.plist`, `Entitlements.plist`, `Resources/LaunchScreen.xib`; `Platforms/Android/AndroidManifest.xml` | The macOS entry point is an executable that runs as it is; the iOS bundle and the Android package are made by the scripts of the repository, which write the property list and the manifest (the name, the identifier and the permission of the reference; an empty launch screen). | not a gap of the framework |
| SPS004 | The project references the colour picker, the Inter font, the view model library and the sample controls beside the Fluent theme, without a statement that uses them. | `SingleProjectSandbox.csproj` | The crate depends on what its statements use. | not a gap of the framework: a difference of the project file |

Not gaps:

- `MainView.xaml` binds the command of its button to the method `ButtonCommand` of the view (`x:DataType="local:MainView"`), but the view has no data context, so the binding has no source, as in the reference.
- The text block `Login` is white, which the light theme variant draws on white, as in the reference.
