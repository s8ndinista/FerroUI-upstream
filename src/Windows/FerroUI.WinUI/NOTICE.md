# Third-party notices

## Avalonia

The WinUI integration is a port of `src/Windows/Avalonia.WinUI` of the Avalonia project (MIT, Copyright (c) AvaloniaUI OÜ; the license text is in the `NOTICE.md` at the root of the repository): its structure, its names, its logic and the comments that explain that logic are derived from it, file by file. The mapping of the files is in `docs/porting/winui.md`.

## WinUI 3 and the Windows App SDK

`interop/winui.idl` declares interfaces of the public API of WinUI 3 (`Microsoft.UI.Xaml`), of `Microsoft.UI` and of the Windows Runtime: their names, identifiers and method signatures, as the metadata of the Windows App SDK 1.6 and of the Windows SDK describe them, for interoperability with those libraries. `scripts/winui/winui-idl.py` writes the file from the metadata files of the packages `Microsoft.WindowsAppSDK` and `Microsoft.Windows.SDK.NET.Ref` on nuget.org; no file of those packages is part of the repository. The sources of WinUI 3 and of the Windows App SDK are published by Microsoft under the MIT license (https://github.com/microsoft/microsoft-ui-xaml, https://github.com/microsoft/WindowsAppSDK), Copyright (c) Microsoft Corporation.

The bootstrap of an unpackaged process (`windows_app_sdk.rs`) calls the functions `MddBootstrapInitialize2`, `MddBootstrapShutdown` and `WindowsAppRuntime_EnsureIsLoaded` with the signatures of `MddBootstrap.h` of the Windows App SDK and does what its auto-initializers (`MddBootstrapAutoInitializer.cpp`, `UndockedRegFreeWinRT-AutoInitializer.cpp`, MIT, Copyright (c) Microsoft Corporation and Contributors) do; the window handle of a window identifier is read through the export `Windowing_GetWindowFromWindowId`, as `Microsoft.UI.Interop.h` of the SDK does. `Microsoft.WindowsAppRuntime.Bootstrap.dll` and the Windows App Runtime are not distributed with this crate: an application ships the library and requires the runtime under Microsoft's terms for them.

## Rust for Windows and ink services

`inking.rs` uses the public Windows Runtime ink APIs through Microsoft's
`windows` 0.62.2, `windows-collections` 0.3.2 and `windows-numerics` 0.3.1 crates
(MIT OR Apache-2.0; <https://github.com/microsoft/windows-rs>). Their original
license files remain in the Cargo dependencies. The adapter is original Rust
interoperability code; the installed Windows ink engine, clipboard services,
handwriting recognizers and language data are not copied or distributed.

The matching FerroUI controls are informed by the Microsoft WinUI InkCanvas and
InkToolbar sources (MIT, Copyright (c) Microsoft Corporation, revision
`9427b7216337ea3d7b19ac32d48c327286ee7138`). Their complete attribution and license
are in `src/FerroUI.Controls/inking/NOTICE.md`, and the mapping and platform
boundaries are documented in `docs/porting/winui-inking.md`.
