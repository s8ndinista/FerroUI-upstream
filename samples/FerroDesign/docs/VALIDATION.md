# Validation and measurements

This file distinguishes commands that have passed from checks that are still in progress. It will be updated with the completed browser run and its artifacts.

## Observed results

| Check | Result |
| --- | --- |
| Core unit tests | 79 passed: existing document/SVG/layout coverage plus vector-edit, Unicode text-range, prototype timing/key/overlay, isolated-root hit testing and interaction-authoring regressions |
| Native full application check | Passed on Linux, Rust 1.99.0, with `--lib --tests`; compiled the XAML shell, native text/paint editors, prototype runtime and test targets |
| FerroUI browser bridge TypeScript check/build | Passed |
| Browser host JavaScript and shell syntax | Passed |
| Browser persistence host tests | 13 passed with Node |
| Native atomic-save tests | 2 passed in the standalone host harness |
| Active browser dependency graph | No `ferroui-skia`, `skia-safe` or `skia-bindings` |
| Actual WebAssembly application build | In progress |
| Browser pixels and editing interactions | Pending the WebAssembly build |
| macOS/Windows native runtime | Not run in this environment |
| Exact Figma visual parity | Not measured |

## Document-engine benchmark

The release benchmark creates 10,000 editable nodes, builds the spatial scene, issues point/viewport queries, performs 120 incremental drag updates and measures transaction history. Values below are actual measurements in the development container; they are not browser FPS, GPU measurements or hardware-independent guarantees.

| Operation | Measured result |
| --- | ---: |
| Point hit query, average over 10,000 queries | 3.192 microseconds |
| Viewport query, average | 76.997 microseconds |
| Incremental drag refresh, median | 5.658 milliseconds |
| Incremental drag refresh, p95 | 7.623 milliseconds |
| Transaction commit | 11.944 milliseconds |
| Undo | 4.707 milliseconds |
| Redo | 5.251 milliseconds |

This run created 10,000 nodes in 12.354 ms and built the initial index in 21.735 ms. It ran concurrently with the WebAssembly compiler. These are current container CPU/model timings; there is no same-run baseline for calculating a speedup and no browser-FPS claim. Correctness tests compare incremental state against full rebuilds, including clipped transformed subtrees, stacking changes and history.

Reproduce the workload from the sample directory:

```sh
cargo run --release -p ferrodesign-core --example performance -- 10000
```

## What the browser test verifies

`tests/browser.mjs` launches the actual application at a 1600 by 1000 viewport in two contexts, requesting Software2D and WebGL2. It records the observed canvas context and allows the documented WebGL2-to-Software2D fallback. The CI browser may use SwiftShader; its results establish functional rendering, not real GPU throughput.

The test waits for a completed native frame and dismissal of the loading surface, inspects native pixel diversity, draws a rectangle, edits vector anchors and pen tangents, edits multiline Unicode in the native on-canvas TextBox, changes paints through the native dialog, and dispatches prototype hover/press/drag/key/delay actions. It checks one-transaction history, canceled text edits, UTF-8 rich-run ranges, JSON import/export equality, durable reload, and bounded virtualized rows with a 10,000-layer document larger than 5 MiB. The separate IndexedDB browser test uses actual transactions for migration, rollback, stale-tab conflicts and a document larger than 10 MiB. The application browser test also collects console exceptions, startup failures and FerroUI render-thread panics. Its screenshots, Playwright traces and report are written under `browser-report` and uploaded by GitHub Actions.

An interaction test is evidence for those paths, not for every item in the feature inventory. There is no same-document image comparison to a Figma reference screenshot. Browser rendering timings and memory should be measured on the intended deployment devices before establishing a frame-rate or capacity target.

## Browser QA environment

The connected local browser cannot reach the build container loopback server. The supported managed preview supervisor also fails during its process-mount setup (`bwrap: Cannot mount proc`), including the permitted escalated attempt. No replacement browser-control mechanism is used. Browser interaction evidence for this revision must come from the repository CI workflow and its rendered screenshots/reports; a successful native compile does not establish browser interaction success.
