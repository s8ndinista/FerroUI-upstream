# WinUI inking port

## Source baseline

- WinUI: `microsoft/microsoft-ui-xaml` at `9427b7216337ea3d7b19ac32d48c327286ee7138`.
- FerroUI baseline: `86b6b4ab41669bf830fba681504a4ee94752a4e6`.
- Source directories: `controls/dev/InkCanvas` and `controls/dev/InkToolbar`.
- Working branch: `feat/winui-inking-port`.

This document tracks the implementation, dependency replacements, tests, and remaining compatibility work. It follows `PORTING-GUIDE.md`: safe idiomatic Rust, FerroUI properties and events, backend-independent rendering, file-level attribution, and explicit status for every omitted dependency.

## Implementation map

| Upstream area | FerroUI destination | Initial status |
|---|---|---|
| InkCanvas visual, layout, input, lifecycle and automation | `src/FerroUI.Controls/inking/` | Implementation in progress |
| InkPresenter and stable service objects | `src/FerroUI.Controls/inking/` | Implementation in progress |
| Windows stroke, point, drawing attributes and container services | `src/FerroUI.Controls/inking/` | Portable replacements in progress |
| InkToolbar, pen/tool/toggle controls and configuration | `src/FerroUI.Controls/inking/` | Implementation in progress |
| Toolbar templates, palettes, accessibility and localization | FerroUI controls/themes | Mapping in progress |
| WinUI API tests | Tests beside corresponding Rust modules | Port in progress |

## External dependency boundary

WinUI delegates the actual digital-ink engine to `Windows.UI.Input.Inking` through `InkDesktopHost`, COM and DirectComposition. Those implementations are not part of the WinUI source tree. FerroUI will reuse its existing pointer routing, rendering, transforms, brushes, property system, layout, subscriptions and automation services. Any portable replacement for an external service is documented as such. Native Windows ISF, clipboard, recognizer and latency interoperability require their own fixtures and platform verification; a portable stroke format is not evidence of ISF compatibility.

## Work log

1. Audited the current WinUI source, IDLs, API tests and service ownership boundaries.
2. Created the working branch and opened this tracking document before implementation.

The PR remains a draft while implementation and validation are in progress. Subsequent commits update this mapping with verified behavior and test results.
