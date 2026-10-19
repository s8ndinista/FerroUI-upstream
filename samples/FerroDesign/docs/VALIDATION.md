# Validation and measurements

This file distinguishes implemented paths, completed checks and measurements. The earlier complete baseline is `c2d2e71ac8b512bddc4a4e4d4c920d5e03f09457`. [CI run 39168689287](https://github.com/wieslawsoltes/FerroUI/actions/runs/39168689287) completed successfully: source/core, all six focused native preedit tests, the actual WebAssembly build, the complete Chromium interaction suite and the native application compilation check all passed. That baseline does not validate every feature subsequently added to the branch.

## Continuation checkpoints

[Run 39221682967](https://github.com/wieslawsoltes/FerroUI/actions/runs/39221682967), source `0efde37fa53801db1ec1829d7355090203fc52ec`, passed 131 core tests, 17 persistence-host tests, six native preedit tests, ten framework browser-input tests, the paired release benchmark, the actual Vello WASM build and five actual Chromium IndexedDB checks. Software2D completed all 28 browser checks, including Smart Animate midpoint pixels, clamped scroll and reversible hover/press interaction state. WebGL2 completed 15 checks and the native paint model edits, then a page screenshot timed out after fonts had already loaded. Trace frames continued and the diagnostic screenshot succeeded with the complete three-stop dialog. This run is a failure, not a complete validation pass.

[Run 39227652054](https://github.com/wieslawsoltes/FerroUI/actions/runs/39227652054), source `fc212577143968b8fa17886f5389970cd3ec7191`, passed all 183 core tests, 17 host tests and the schema-4 component/editor benchmark. The native preedit compilation gate found a missing `PushCompositingLayerPayload` import in the framework's shared test stream reader. Commit `b48a258c856769e87076e53bb691efb56bcd859b` fixes the import.

[Run 39228488797](https://github.com/wieslawsoltes/FerroUI/actions/runs/39228488797), source `b48a258c`, passed 183 core tests, 17 host tests, six native preedit tests and seven Base compositing tests. Vello's new earlier native pixel gate could not compile because Fontconfig development files were installed only after browser validation. The workflow now installs those files before native tests. This run did not reach a WASM build or browser validation.

[Run 39231069435](https://github.com/wieslawsoltes/FerroUI/actions/runs/39231069435), source `40131410`, passed 197 core tests, 17 host tests, six native preedit tests, seven Base compositing tests and seven Vello compositing tests. Native application compilation then reached the compiler's macro expansion limit in the enlarged diagnostics JSON object. Commit `4e3cfc11` splits that construction into smaller groups while retaining its flat bridge schema. Native application tests, WASM and browser checks were not reached in this run.

The branch now incorporates upstream `3d8b473` (FerroUI 0.3.0-preview.1), with the standalone sample lockfile aligned and all registry versions retained. Checkpoint `26170911` adds a bounded editor change feed and incremental render preparation; `468a02b7` adds native grid placement/spans/reorder, cumulative browser preparation counters and history-depth assertions. Their new runtime tests remain pending until the next complete CI result. The workflow runs all native application tests before the browser build, including nine incremental-renderer pixel/count tests, seven background-blur tests and native authoring parsers. The browser workload requires nudge/undo/redo in a flat 10,000-layer document to each add one prepared leaf, zero full preparations and zero immutable visual rebuilds; those counts are not an FPS benchmark.

The expanded browser harness retains real native pointer/keyboard operations and PNG captures. The paint checkpoint now waits for newly arranged native controls and a render after the Add-stop input. A screenshot TimeoutError may receive one bounded retry only if document hash, selection, camera and native control bounds remain unchanged, the page responds, and the renderer has no lost context or panic. Per-attempt diagnostics remain in the report; a second failure, changed state, or invalid/blank capture fails the run. Continuous traces and font readiness remain enabled.

## Paired release measurements

These are **native editor-engine measurements on one GitHub runner**, not browser frame rate or hardware-GPU throughput. The same process compares the scoped command/gesture path with an explicitly defined generic full-document transaction reference, and asserts exact document/history equivalence. The flat fixture has 10,000 filled rectangles, one selected node, 48 nudge operations and 12 gestures of 24 updates. Raw results are preserved in [the source-tagged JSON](benchmarks/2026-10-19-0efde37f-editor-10000-nodes.json).

| Operation | Full-document reference p50 | Scoped path p50 |
| --- | ---: | ---: |
| Nudge | 9.253 ms | 0.020 ms |
| Undo | 1.509 ms | 0.008 ms |
| Redo | 1.491 ms | 0.008 ms |
| Gesture begin | 2.346 ms | 0.010 ms |
| Gesture update | 1.454 ms | 0.000631 ms |
| Gesture commit | 7.248 ms | 0.009 ms |
| Complete 24-update gesture | 46.008 ms | 0.044 ms |

The savings come from reducing snapshot, comparison and scene-update work for known mutation sets. They do not include native control layout, scene preparation, rasterization, browser input delivery or JSON serialization. Pretty JSON serialization remained 36.595 ms p50 for 20,152,206 bytes in this run; coalescing before serialization reduces its frequency. A compact-format candidate was measured separately in that run: 19.621 ms p50 for 12,802,055 bytes, preserving an exact compact-to-document-to-pretty round trip. A later implementation adopts this serialization path for autosaves; portable exports retain the pretty format. Its browser gates verify actual committed compact bytes, exact reload and authored-state isolation during active presentation. Those new autosave gates still require a completed CI result.

The [component workload at `fc212577`](benchmarks/2026-10-19-fc212577-editor-components.json) includes two matched 32-child variants, typed text/visibility changes, embedded images and vector paths, and a 5,592,566-byte unrelated image data URI among 10,000 nodes. The paired no-history document operation is compared with the same operation plus an explicit whole-document candidate clone; this is a reference workload, not a measured previous-version speedup. Both variants of the benchmark preserve exact document state and matching materialized IDs. Actual Editor operations are measured separately with their current full history snapshots:

| Component operation | Editor with history p50 | Editor with history p95 |
| --- | ---: | ---: |
| Variant switch | 16.656 ms | 17.138 ms |
| Text property | 16.986 ms | 17.677 ms |
| Visibility property | 20.875 ms | 21.322 ms |

The complete component undo/redo round-trip was exact. The remaining full-snapshot cost is visible rather than being hidden behind the faster isolated document-operation numbers.

## Earlier complete baseline: observed results

| Check | Result |
| --- | --- |
| Core unit tests | 79 passed: existing document/SVG/layout coverage plus vector-edit, Unicode text-range, prototype timing/key/overlay, isolated-root hit testing and interaction-authoring regressions |
| Native TextBox preedit regressions | All 6 tests in the focused `preedit` gate passed for the current implementation; it compiles the controls test binary and exercises pending-state ownership, selection deletion, empty updates, detachment and existing preedit regressions |
| Native full application check | Passed on Ubuntu 24.04 with Rust 1.99.0: `cargo check --locked -p ferrodesign-app --lib --tests --jobs 1`, checking the XAML shell, native text/paint editors, prototype runtime and test targets. This is a compilation check, not a native GUI runtime test. |
| FerroUI browser bridge TypeScript check/build | Passed |
| Browser text-input event regressions | 10 passed with Node, bundling the actual TypeScript bridge; includes Unicode insertion, composition deduplication, canceled-client recovery and listener cleanup |
| Browser host JavaScript and shell syntax | Passed |
| Browser persistence host tests | 13 passed with Node |
| Actual IndexedDB transactions in Chromium | All 5 checks passed, including atomic migration, rollback, stale-tab conflicts and reload of a 12,000,042-character document |
| Native atomic-save tests | 2 passed in the earlier standalone development host harness; this is separate from the current CI gates |
| Active browser dependency graph | No `ferroui-skia`, `skia-safe` or `skia-bindings` |
| Actual WebAssembly application build | Passed with the pinned Emscripten toolchain, including linking and optimization; all six required JavaScript modules verified in the distribution |
| Browser pixels and editing interactions | Software2D passed all 25 checks; WebGL2 passed all 24 checks. Both completed native vector/text/paint input, composition, pagehide/reload, prototype interactions, serialization and history. Software2D additionally completed the 10,000-layer workload. |
| Browser runtime errors | Zero page errors and zero console errors in both modes |
| Native GUI runtime | Not run on Linux, macOS or Windows in this environment |
| Exact Figma visual parity | Not measured |

The `c2d2e71a` CI browser distribution contains 16 files totaling 48,637,191 bytes. Its WebAssembly module is 47,941,251 bytes before HTTP compression, SHA256 `fbf2cb2818bbeff0b24f8d25ac4ae4b3f35cbb20691607436a504477a3b24f75`. The 438-line resolved dependency manifest contains Vello and no Skia package. Both build and validation archives passed CRC verification. These packaging checks establish build and import completeness; the Chromium interaction suite separately verifies rendered behavior.

The browser was Chromium 156.0.8078.4. The two runs are contexts in that browser engine, not tests of two different browser engines.

| Requested mode | Observed surface | First application frame |
| --- | --- | ---: |
| Software2D | Software2D / Vello CPU | 1,794 ms |
| WebGL2 | WebGL2 via ANGLE / Vulkan 1.3.0 / SwiftShader Device (Subzero) | 3,120 ms |

The WebGL2 context exercised the actual API path through a software GPU implementation. These first-frame observations include the local CI application startup; they do not establish hardware GPU throughput or production network latency.

## Browser regressions corrected

The [first CI browser run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39152600835) found that text inserted without a keydown updated only the hidden DOM input. FerroUI's browser bridge now forwards non-composing `insertText` payloads through the native text event, preserving native selection and resynchronizing the DOM after the native control finishes its edit. Composition input remains owned by the composition handlers, with a bounded duplicate-commit guard. This is a targeted text-insertion fix; browser autocorrect replacement ranges and input-only deletion are not newly implemented by this patch. The original multiline Unicode assertions passed in both rendering modes in the second run.

The second run's WebGL2 failure state contained the expected linear gradient and visible native paint controls with zero-size bounds. The harness sampled those controls about 7 ms after the document transaction, before the next native arrangement. A bounded readiness predicate now requires visible, finite, positive native bounds before deriving pointer coordinates. It does not bypass the field interaction or alter the paint assertions.

The same review found a native composition lifecycle bug: losing a text client before `compositionend` left the framework composing flag set, blocking subsequent input-only text. Native reset/client replacement now cancels preedit, and the TypeScript bridge synchronizes its composition state when the shared hidden input is cleared. Late events from the canceled composition cannot edit the replacement client. The browser suite now exercises normal composition, cancellation, resumed Unicode input before a delayed end, and late-event isolation against the actual WASM TextBox.

The [third run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39163539589), revision `fd800712`, passed the 13 existing editor checks in each mode and then exposed an incorrect assumption in the new preedit assertion. The real compiled-XAML TextBox retained its confirmed document value, while the test expected an intermediate selection deletion. That internal value depends on whether the native presenter exists when the TextBox attaches. The corrected test checks the native input client's explicit pending-preedit state and the complete confirmed document node, then requires exactly-once composition commit and cancellation behavior.

The editor also guards a separate durability boundary: when a native preedit update does remove the selection, its text-changing notification must not replace confirmed document text. `TextBox::is_composing_text()` exposes a stable native-client signal independent of caret/presenter synchronization. The shell keeps those intermediate notifications outside the document draft. A browser regression now starts with confirmed Unicode input, begins unfinished native composition, dispatches the real pagehide listener, checks durable text and rich runs, exercises one-step undo/redo, and reloads the page. These assertions passed in both modes in the current run; all six focused native signal regressions also passed.

The [fourth run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39165941373), revision `f1184c3e`, exposed two harness timing assumptions. In Software2D, a second double-click at the identical position followed the first within the native 500 ms click-count window, so the next text editor did not open. The fixture now makes a real blank-canvas click at a different position before each opening double-click, establishing an independent native sequence without arbitrary sleeps. In WebGL2, all native gradient edits succeeded, but `locator.screenshot` timed out waiting for DOM canvas stability; the fallback page screenshot succeeded and showed the correctly arranged three-stop dialog. The paint artifact now uses a page screenshot at the same fixed viewport. All model, keyboard, history, composition and persistence assertions remain in place, and the complete corrected browser suite passed in the fifth run.

## Observed large-document browser result

The Software2D part of run `39168689287` completed the full 10,000-layer workload in Chromium 156.0.8078.4. It imported a 12,592,068-byte document, rendered all 10,000 nodes, realized only 25 native layer rows before and after scrolling, exercised real pointer selection and keyboard nudge/undo/redo, and restored the exact 20,292,219-byte edited document from IndexedDB after a browser reload.

| Operation | Observed CI time |
| --- | ---: |
| Import through first native frame | 1,024 ms |
| Keyboard nudge through the browser harness | 322 ms |
| Reload and exact restored document | 1,756 ms |

These are end-to-end harness observations on one CI runner using CPU rendering, not browser frame-rate or hardware-GPU measurements. The report's `commitMs` field is the remaining wait for the save queue to drain after earlier autosave work; it is not complete save latency. The saved document SHA256 was `13c63f58a702cfd42eaf05cd113e0d8e63ba0603b1613a535e9fccadcf55782e`.

## Document-engine benchmark

The release benchmark creates 10,000 editable nodes, builds the spatial scene, issues point/viewport queries, performs 120 incremental drag updates and measures transaction history. Values below are actual measurements from the earlier development container; they are not browser FPS, GPU measurements or hardware-independent guarantees.

| Operation | Measured result |
| --- | ---: |
| Point hit query, average over 10,000 queries | 3.192 microseconds |
| Viewport query, average | 76.997 microseconds |
| Incremental drag refresh, median | 5.658 milliseconds |
| Incremental drag refresh, p95 | 7.623 milliseconds |
| Transaction commit | 11.944 milliseconds |
| Undo | 4.707 milliseconds |
| Redo | 5.251 milliseconds |

This run created 10,000 nodes in 12.354 ms and built the initial index in 21.735 ms. It ran concurrently with the WebAssembly compiler. These are container CPU/model timings from that development run; there is no same-run baseline for calculating a speedup and no browser-FPS claim. Correctness tests compare incremental state against full rebuilds, including clipped transformed subtrees, stacking changes and history.

Reproduce the workload from the sample directory:

```sh
cargo run --release -p ferrodesign-core --example performance -- 10000
```

## What the browser test verifies

`tests/browser.mjs` launches the actual application at a 1600 by 1000 viewport in two contexts, requesting Software2D and WebGL2. It records the observed canvas context and allows the documented WebGL2-to-Software2D fallback. The CI browser may use SwiftShader; its results establish functional rendering, not real GPU throughput.

The test waits for a completed native frame and dismissal of the loading surface, inspects native pixel diversity, draws a rectangle, edits vector anchors and pen tangents, edits multiline Unicode in the native on-canvas TextBox, changes paints through the native dialog, and dispatches prototype hover/press/drag/key/delay actions. It checks one-transaction history, canceled text edits, UTF-8 rich-run ranges, JSON import/export equality, durable reload, and bounded virtualized rows with a 10,000-layer document larger than 5 MiB. Deterministic browser composition events exercise explicit native preedit state, exactly-once commits, focus loss and confirmed-text durability through pagehide/reload; they do not certify operating-system IME behavior on every platform. Prototype rules are imported as fixtures for runtime validation; interaction-dialog authoring controls have not been browser-operated by this suite. The separate IndexedDB browser test uses actual transactions for migration, rollback, stale-tab conflicts and a document larger than 10 MiB. The application browser test also collects console exceptions, startup failures and FerroUI render-thread panics. Its screenshots, Playwright traces and report are written under `browser-report` and uploaded by GitHub Actions.

The focused native preedit gate can be reproduced from the sample directory:

```sh
CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_TEST_CODEGEN_UNITS=16 \
  cargo test --locked --manifest-path ../../Cargo.toml \
  -p ferroui-controls --lib preedit --jobs 1 -- --test-threads=1
```

The filter narrows execution, while Cargo still compiles and links the controls test binary. This gate uses the pure Rust controls/base test graph; it does not introduce a separate graphics backend.

An interaction test is evidence for those paths, not for every item in the feature inventory. There is no same-document image comparison to a Figma reference screenshot. Browser rendering timings and memory should be measured on the intended deployment devices before establishing a frame-rate or capacity target.

## Browser QA environment

The connected local browser cannot reach the build container loopback server. The supported managed preview supervisor also fails during its process-mount setup (`bwrap: Cannot mount proc`), including the permitted escalated attempt. No replacement browser-control mechanism is used. Browser interaction evidence for this revision must come from the repository CI workflow and its rendered screenshots/reports; a successful native compile does not establish browser interaction success.
