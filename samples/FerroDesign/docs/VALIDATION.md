# Validation and measurements

This file distinguishes commands that have passed from checks that are still in progress. It will be updated with the completed browser run and its artifacts.

## Observed results

| Check | Result |
| --- | --- |
| Core unit tests | 79 passed: existing document/SVG/layout coverage plus vector-edit, Unicode text-range, prototype timing/key/overlay, isolated-root hit testing and interaction-authoring regressions |
| Native full application check | Passed on Linux, Rust 1.99.0, with `--lib --tests`; compiled the XAML shell, native text/paint editors, prototype runtime and test targets |
| FerroUI browser bridge TypeScript check/build | Passed |
| Browser text-input event regressions | 10 passed with Node, bundling the actual TypeScript bridge; includes Unicode insertion, composition deduplication, canceled-client recovery and listener cleanup |
| Browser host JavaScript and shell syntax | Passed |
| Browser persistence host tests | 13 passed with Node |
| Actual IndexedDB transactions in Chromium | All 5 checks passed, including atomic migration, rollback, stale-tab conflicts and reload of a 12,000,042-character document |
| Native atomic-save tests | 2 passed in the standalone host harness |
| Active browser dependency graph | No `ferroui-skia`, `skia-safe` or `skia-bindings` |
| Actual WebAssembly application build | Passed with the pinned Emscripten toolchain, including linking and optimization; all six required JavaScript modules verified in the distribution |
| Browser pixels and editing interactions | [CI run 39161125725](https://github.com/wieslawsoltes/FerroUI/actions/runs/39161125725), revision `b17b4ffd`: Software2D passed all 23 checks; real WebGL2 passed 13, including complete native Unicode editing, then sampled a rebuilt gradient field before native layout had arranged it. The harness now waits for arranged native bounds; the complete rerun is pending. |
| macOS/Windows native runtime | Not run in this environment |
| Exact Figma visual parity | Not measured |

The `b17b4ffd` CI browser distribution contains 16 files totaling 48,634,628 bytes. Its WebAssembly module is 47,940,555 bytes before HTTP compression, SHA256 `6fc7e13aefc539c9f6f7eed2344de8c5c3156b2d1c0deded77f1ec2ec4e49820`. These packaging checks establish that the application is buildable and its imports are complete; rendered behavior is checked separately in Chromium.

The [first CI browser run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39152600835) found that text inserted without a keydown updated only the hidden DOM input. FerroUI's browser bridge now forwards non-composing `insertText` payloads through the native text event, preserving native selection and resynchronizing the DOM after the native control finishes its edit. Composition input remains owned by the composition handlers, with a bounded duplicate-commit guard. This is a targeted text-insertion fix; browser autocorrect replacement ranges and input-only deletion are not newly implemented by this patch. The original multiline Unicode assertions passed in both rendering modes in the second run.

The second run's WebGL2 failure state contained the expected linear gradient and visible native paint controls with zero-size bounds. The harness sampled those controls about 7 ms after the document transaction, before the next native arrangement. A bounded readiness predicate now requires visible, finite, positive native bounds before deriving pointer coordinates. It does not bypass the field interaction or alter the paint assertions.

The same review found a native composition lifecycle bug: losing a text client before `compositionend` left the framework composing flag set, blocking subsequent input-only text. Native reset/client replacement now cancels preedit, and the TypeScript bridge synchronizes its composition state when the shared hidden input is cleared. Late events from the canceled composition cannot edit the replacement client. The browser suite now exercises normal composition, cancellation, resumed Unicode input before a delayed end, and late-event isolation against the actual WASM TextBox.

## Observed large-document browser result

The Software2D part of run `39161125725` completed the full 10,000-layer workload in Chromium 156.0.8078.4. It rendered all 10,000 nodes, realized only 25 native layer rows before and after scrolling, exercised real pointer selection and keyboard nudge/undo/redo, and restored the exact 20,292,219-byte edited document from IndexedDB after a browser reload.

| Operation | Observed CI time |
| --- | ---: |
| Import through first native frame | 1,052 ms |
| Keyboard nudge through the browser harness | 321 ms |
| Reload and exact restored document | 1,800 ms |

These are end-to-end harness observations on one CI runner using CPU rendering, not browser frame-rate or hardware-GPU measurements. The report's `commitMs` field is the remaining wait for the save queue to drain after earlier autosave work; it is not complete save latency. The saved document SHA256 was `13c63f58a702cfd42eaf05cd113e0d8e63ba0603b1613a535e9fccadcf55782e`.

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

The test waits for a completed native frame and dismissal of the loading surface, inspects native pixel diversity, draws a rectangle, edits vector anchors and pen tangents, edits multiline Unicode in the native on-canvas TextBox, changes paints through the native dialog, and dispatches prototype hover/press/drag/key/delay actions. It checks one-transaction history, canceled text edits, UTF-8 rich-run ranges, JSON import/export equality, durable reload, and bounded virtualized rows with a 10,000-layer document larger than 5 MiB. Deterministic browser composition events exercise preedit, exactly-once commits and focus loss through the real WASM text client; they do not certify operating-system IME behavior on every platform. Prototype rules are imported as fixtures for runtime validation; interaction-dialog authoring controls have not been browser-operated by this suite. The separate IndexedDB browser test uses actual transactions for migration, rollback, stale-tab conflicts and a document larger than 10 MiB. The application browser test also collects console exceptions, startup failures and FerroUI render-thread panics. Its screenshots, Playwright traces and report are written under `browser-report` and uploaded by GitHub Actions.

An interaction test is evidence for those paths, not for every item in the feature inventory. There is no same-document image comparison to a Figma reference screenshot. Browser rendering timings and memory should be measured on the intended deployment devices before establishing a frame-rate or capacity target.

## Browser QA environment

The connected local browser cannot reach the build container loopback server. The supported managed preview supervisor also fails during its process-mount setup (`bwrap: Cannot mount proc`), including the permitted escalated attempt. No replacement browser-control mechanism is used. Browser interaction evidence for this revision must come from the repository CI workflow and its rendered screenshots/reports; a successful native compile does not establish browser interaction success.
