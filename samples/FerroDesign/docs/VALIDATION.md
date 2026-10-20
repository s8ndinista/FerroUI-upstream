# Validation and measurements

This file distinguishes implemented paths, completed checks and measurements. Each observation belongs to its stated source and run; older successful runs do not validate later changes. The branch now incorporates FerroUI `ab7a15b8` / `0.4.0-preview.1`, restores Vello for every application target, and adds local style definition editing and public-API flattening. Those changes require their own validation. [PR #254](https://github.com/wieslawsoltes/FerroUI/pull/254) collects the complete source and current review status.

## Preserved validated baseline: `945283ef`

This continuation adds typed native effect authoring, deferred auto-layout drag previews and transactional drops, a retained spatial index for prepared root bounds, and consistent shadow order through rounded and general geometry. Browser regressions additionally wait for a new native frame after selection and verify actual ComboBox popup focus before sending navigation keys.

The following local checks completed on Ubuntu 24.04 using the pinned Rust 1.99.0, Emscripten 6.0.10 and wasm-bindgen 0.2.129 tools:

| Gate | Observed result |
| --- | --- |
| Document core | All 219 tests passed, including atomic typed effect commands and deferred layout-session ownership/history |
| Persistence host | All 17 JavaScript tests passed; Node 24.19.0 used `--test-isolation=none` to expose the individual cases |
| Native application | All 64 tests passed, including native capture loss/cancellation, effect-field parsing, spatial-index/reference-pixel comparisons and eight shadow pixel cases |
| Actual WebAssembly application | The final browser-profile build completed, all six required JavaScript modules resolved, and the active no-Skia dependency check passed |
| Resolved application dependencies | No `ferroui-skia`, `skia-safe` or `skia-bindings` in the normal/build graphs for the browser, Linux x64, Windows x64, macOS ARM64 or macOS x64 targets |

The cross-target dependency audit used locked, offline Cargo graph resolution. It verifies backend selection, not execution of the native desktop application on those operating systems. Production scene painting uses public FerroUI APIs; direct Vello references configure FerroUI's backend or provide backend pixels in native tests. The compiled XAML shell hosts native controls, including inspector dialogs constructed dynamically in Rust.

The corresponding exact-source CI run **completed successfully**, including both artifact uploads and cleanup. Its observed gates are:

| Exact-source CI gate | Observed result |
| --- | --- |
| Document core and persistence host | 219 Rust core tests and 17 JavaScript host tests passed |
| Native text preedit | All 6 focused tests passed |
| Public compositing | All 7 Base recording/replay tests and 7 Vello pixel tests passed |
| Native application | Compilation and all 64 tests passed |
| Actual Vello WebAssembly application | Build, all six required JavaScript imports, 10 web text-input bridge tests and active no-Skia dependency checks passed |
| Actual Chromium IndexedDB | All 5 transaction, migration, rollback, conflict and large-document reload checks passed |
| Chromium Software2D | All 62 interaction checks passed, including the 10,000-layer workload |
| Chromium WebGL2 | All 59 interaction checks passed; actual WebGL2 through ANGLE/Vulkan/SwiftShader |
| Runtime errors | No page errors or console errors in either context |
| Artifact publication | Complete build, screenshots, traces, reports and lockfiles uploaded; both ZIP sizes and GitHub-provided SHA256 digests matched, and both outer ZIPs and inner trace ZIPs passed CRC checks |

Both contexts used Chromium 156.0.8078.4. The first native application frame was observed after 1,498 ms in Software2D and 1,453 ms in WebGL2. All 34 shared static native captures passed on their first attempt, with no retries. Software2D capture attempts took 120–161 ms; WebGL2 attempts took 9,112–23,629 ms. These capture durations include screenshot acquisition and follow-up checks, and do not measure application frame rate or editing latency. The hardware used by this WebGL2 run was SwiftShader, so this is not a hardware-GPU throughput benchmark.

The [source-tagged evidence directory](evidence/2026-10-19-945283ef/README.md) preserves the unchanged report and paired initial/editor, effect-stack and held layout-drag PNGs. Visual inspection found the captured shell, effect controls and insertion marker legible and arranged in both modes. This is a review of this application's pixels; no matched Figma document/reference screenshot was available for a numerical visual-parity comparison.

### Actual 10,000-layer browser workload at `945283ef`

Software2D imported a 12,872,068-byte design containing 10,000 rendered nodes. The native layer list represented all 10,000 rows while realizing only 25 controls, both before and after scrolling.

| Operation | Full preparations | Incremental preparations | Prepared nodes | Rebuilt immutable visuals | Layer-projection builds | Layer-projection reuses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Native keyboard nudge | 0 | 1 | 1 | 0 | 0 | 1 |
| Native keyboard undo | 0 | 1 | 1 | 0 | 0 | 1 |
| Native keyboard redo | 0 | 1 | 1 | 0 | 0 | 1 |

The new zoom fixture then selected **1 root candidate from 10,000 roots with 27 bounds tests**, with one visible node at zoom 17.0833. `rootQueryReused` was false for that observed render, so these are actual query work counts rather than a zero-work cached query. Native tests separately cover unchanged-query reuse, dense full-view traversal, conservative effect bounds, refitting and exact pixels against the original ordered scan.

Import through the first native frame took 829 ms; the measured native keyboard nudge through the browser harness took 151 ms; reload and exact document restoration took 1,226 ms. The edited portable JSON was 20,652,219 bytes and its compact autosave was 13,132,068 bytes; autosave serialization took 33.3 ms. The report's 2 ms `commitMs` is only the remaining wait for the save queue to drain after earlier autosave work, not complete IndexedDB commit latency. The complete restored document SHA256 was `f011b40a7d2bf8d0e21497008f93b613ce7f154e326f2eda41052e83ca776332`.

These counters show avoided preparation, row allocation and root-bound work. Root signatures, drawing visible artwork, wide-parent bounds, structural layout and some history paths can still be linear. The observed browser times include the harness and are single-run functional measurements, not FPS. WebGL2 did not run the 10,000-layer fixture.

### Paired release measurements at `945283ef`

The unchanged [schema-5 benchmark JSON](benchmarks/2026-10-19-945283ef-editor-performance.json) is copied byte-for-byte from the successful CI validation artifact (7,424 bytes, SHA256 `fc7a2776c0f9438204e120096a8746c4b1672bb0081f9313325033a4fb31e497`). It compares scoped commands with an explicit generic full-document transaction reference in one release process on 10,000 flat filled rectangles with one selected node. Exact document/history and serialization equivalence checks passed. The reference is a workload in the same executable, not a previous release or a different CI runner.

| Operation, 48 samples | Full-document reference p50 | Scoped p50 | Scoped p95 |
| --- | ---: | ---: | ---: |
| Nudge | 14.098 ms | 0.026 ms | 0.032 ms |
| Undo | 4.294 ms | 0.017 ms | 0.024 ms |
| Redo | 4.167 ms | 0.015 ms | 0.018 ms |

Across 12 gestures, a complete 24-update gesture measured 108.968 ms p50 on the reference path and 0.053 ms p50 / 0.070 ms p95 on the scoped path. This benchmark measures the existing scoped engine commands, not the new auto-layout drag index or root rendering hierarchy. It excludes browser events, native control layout, drawing and persistence.

Five serialization samples measured portable pretty output at 30.157 ms p50 / 20,152,206 bytes and compact autosave at 16.139 ms p50 / 12,802,055 bytes, a 36.5% smaller payload with exact round-trip equivalence. Parse/validate/index measured 47.701 ms p50 across three samples. A separate dense layout fixture with 9,999 mixed fixed/fill text children measured 8.798 ms p50 across five samples. These native fixtures are separate from the browser workload above.

Component synchronization retains full document history snapshots. Its separate 10,000-node workload includes 5,592,566 bytes of unrelated embedded-image data and measures actual Editor operations including history: variant switching 19.145 ms p50 / 19.369 ms p95; text properties 19.032 / 19.942 ms; visibility properties 25.298 / 26.163 ms, with eight samples each. Complete history restoration and stable materialized IDs passed. The isolated no-history document-operation figures in the raw JSON are not substituted for production Editor costs.

### Framework integration and earlier checkpoints

The separate [standard repository CI run 39306373798](https://github.com/wieslawsoltes/FerroUI/actions/runs/39306373798) completed successfully with all 14 jobs passing. Its checkout log and merge commit identify source `945283ef` tested as merge `70db461c11217d05d774a6736ec44bfa6c982f35` into base `0116724da88df86fa4e4575f7e9a0d78708f2204`. Source conventions, packaging, macOS, Windows x64/ARM64, Linux X11/Wayland/framebuffer, Android, iOS and all three browser configurations passed. These framework integration jobs are separate from the independent FerroDesign workspace's exact-source application tests; they do not establish native FerroDesign GUI execution on every platform.

The preceding combined implementation `12cbecd3`, [run 39304233073](https://github.com/wieslawsoltes/FerroUI/actions/runs/39304233073), passed the core/host, preedit and public compositing gates, then native checking rejected a new test's incorrect `PixelFormat` import. The current source corrects that import; its complete local native suite and the CI native gate above pass. No browser result is attributed to the failed checkpoint.

The resumed baseline `bc2e9984`, [run 39254507451](https://github.com/wieslawsoltes/FerroUI/actions/runs/39254507451), passed its earlier gates and all 53 Software2D checks, then failed after 31 WebGL2 checks at the native blend selector. Its trace showed a changed selection before a new inspector frame; the subsequent arrow input nudged the selected layer. The revised harness retains native pointer/keyboard input, requires arranged controls and popup focus, and checks that blend selection does not move the layer. Both current modes now pass that regression. The older complete result below is retained as historical evidence.

## Complete continuation result: `ecd5078d`

| Gate | Observed result |
| --- | --- |
| Document core and persistence host | 197 Rust core tests and 17 JavaScript host tests passed |
| Native text preedit | All 6 focused tests passed |
| Public compositing | All 7 Base recording/replay tests and 7 Vello pixel tests passed |
| Native application | Compilation and all 43 tests passed, including 8 layer-projection, 9 incremental-renderer and 7 backdrop cases |
| Actual Vello WebAssembly application | Build, required JavaScript imports and active dependency graph checks passed; no active Skia package |
| Actual Chromium IndexedDB | All 5 transaction, migration, rollback, conflict and large-document reload checks passed |
| Chromium Software2D | All 53 interaction checks passed, including the 10,000-layer workload |
| Chromium WebGL2 | All 51 interaction checks passed; actual WebGL2 through ANGLE/Vulkan/SwiftShader |
| Runtime errors | No page errors or console errors in either context |
| Artifact publication | Build, screenshots, traces, reports and lockfiles uploaded successfully; original artifact sizes and GitHub-provided SHA256 digests verified |

Both browser contexts used Chromium 156.0.8078.4. They exercised native vector networks, Unicode/IME and rich-text range editing, variables, component variants and typed overrides, image crop gestures and percentage fields, isolated compositing and transparent background-blur export, the page inspector, grid spans/reorder, compact authored-state autosaves, prototype animation/scroll/reversal, compact-window controls and durable reload. The first native application frame was observed after 1,713 ms in Software2D and 1,634 ms in WebGL2. These are functional checks in one browser engine on a CI runner, not native desktop GUI certification, operating-system IME certification, a hardware-GPU benchmark or a same-document Figma pixel comparison.

The 30 shared static native captures all completed. One WebGL2 variable-dialog capture needed the existing unchanged-state retry: the first attempt timed out after 30.058 seconds, and the second succeeded in 155 ms. Other WebGL2 captures took approximately 8.8–29.6 seconds and passed their first attempt. The report retains each attempt, unchanged document/selection/camera/control checks, context-health checks and actual PNG pixels. These capture-attempt durations include the screenshot call and follow-up checks and are separate from application first-frame and editing measurements; no exact cause for the slower captures is claimed.

### Actual 10,000-layer browser workload

This workload runs only in the Software2D context. It imported a 12,872,068-byte design with 10,000 rendered nodes. The native layer list represented all 10,000 rows while realizing only 25 controls, both before and after scrolling.

| Operation | Full preparations | Incremental preparations | Prepared nodes | Rebuilt immutable visuals | Layer-projection builds | Layer-projection reuses |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Native keyboard nudge | 0 | 1 | 1 | 0 | 0 | 1 |
| Native keyboard undo | 0 | 1 | 1 | 0 | 0 | 1 |
| Native keyboard redo | 0 | 1 | 1 | 0 | 0 | 1 |

Import through the first native frame took 947 ms; the measured keyboard nudge through the harness took 159 ms; reload and exact document restoration took 1,557 ms. The edited portable JSON was 20,652,219 bytes and its compact autosave was 13,132,068 bytes; the recorded autosave serialization took 27 ms. The report's 3 ms `commitMs` is only the remaining wait for the save queue to drain after earlier autosave work, not complete IndexedDB commit latency. The complete restored document SHA256 was `f011b40a7d2bf8d0e21497008f93b613ce7f154e326f2eda41052e83ca776332`.

These preparation/projection counters verify avoided work; root traversal, wide-parent aggregation and some metadata comparisons can still be linear. The observed times are not FPS or a speedup calculated against a different CI runner. WebGL2 did not run this large-document fixture.

### Paired native measurements from the same successful run

The unchanged [schema-5 benchmark JSON](benchmarks/2026-10-19-ecd5078d-editor-performance.json) is copied byte-for-byte from the validation artifact (7,461 bytes, SHA256 `2211dd9f679345f2e0ddc598591874561117d54689fcd6f7bb6ead2835055bfb`). It compares scoped commands with an explicit generic full-document transaction reference in one release process, on 10,000 flat filled rectangles with one selected node. Exact document/history equivalence passed. The reference is a workload in the same executable, not a separately measured previous release.

| Operation, 48 samples | Full-document reference p50 | Scoped p50 | Scoped p95 |
| --- | ---: | ---: | ---: |
| Nudge | 13.742 ms | 0.024 ms | 0.029 ms |
| Undo | 2.572 ms | 0.012 ms | 0.018 ms |
| Redo | 2.714 ms | 0.013 ms | 0.018 ms |

The complete 24-update gesture measured 82.627 ms p50 on the reference path and 0.059 ms on the scoped path, across 12 gestures. These engine measurements exclude browser input, native control layout, drawing and persistence. Five serialization samples measured portable pretty output at 36.188 ms p50 / 20,152,206 bytes and compact autosave at 20.210 ms p50 / 12,802,055 bytes, a 36.5% smaller payload with exact round-trip equivalence. This native benchmark fixture is separate from the browser workload above.

Component synchronization retains full history snapshots. Its separate 10,000-node workload includes a 5,592,566-byte unrelated embedded image and measures the actual Editor operation, including history: variant switching 17.678 ms p50 / 18.761 ms p95; text properties 17.930 / 18.387 ms; visibility properties 24.783 / 25.700 ms, with eight samples each. Its complete history round trip and materialized IDs were exact. The faster isolated no-history document-operation numbers are not substituted for those production Editor costs.

The earlier complete baseline remains `c2d2e71ac8b512bddc4a4e4d4c920d5e03f09457`, [run 39168689287](https://github.com/wieslawsoltes/FerroUI/actions/runs/39168689287). Its separate observations are retained below as history.

## Continuation checkpoints

[Run 39221682967](https://github.com/wieslawsoltes/FerroUI/actions/runs/39221682967), source `0efde37fa53801db1ec1829d7355090203fc52ec`, passed 131 core tests, 17 persistence-host tests, six native preedit tests, ten framework browser-input tests, the paired release benchmark, the actual Vello WASM build and five actual Chromium IndexedDB checks. Software2D completed all 28 browser checks, including Smart Animate midpoint pixels, clamped scroll and reversible hover/press interaction state. WebGL2 completed 15 checks and the native paint model edits, then a page screenshot timed out after fonts had already loaded. Trace frames continued and the diagnostic screenshot succeeded with the complete three-stop dialog. This run is a failure, not a complete validation pass.

[Run 39227652054](https://github.com/wieslawsoltes/FerroUI/actions/runs/39227652054), source `fc212577143968b8fa17886f5389970cd3ec7191`, passed all 183 core tests, 17 host tests and the schema-4 component/editor benchmark. The native preedit compilation gate found a missing `PushCompositingLayerPayload` import in the framework's shared test stream reader. Commit `b48a258c856769e87076e53bb691efb56bcd859b` fixes the import.

[Run 39228488797](https://github.com/wieslawsoltes/FerroUI/actions/runs/39228488797), source `b48a258c`, passed 183 core tests, 17 host tests, six native preedit tests and seven Base compositing tests. Vello's new earlier native pixel gate could not compile because Fontconfig development files were installed only after browser validation. The workflow now installs those files before native tests. This run did not reach a WASM build or browser validation.

[Run 39231069435](https://github.com/wieslawsoltes/FerroUI/actions/runs/39231069435), source `40131410`, passed 197 core tests, 17 host tests, six native preedit tests, seven Base compositing tests and seven Vello compositing tests. Native application compilation then reached the compiler's macro expansion limit in the enlarged diagnostics JSON object. Commit `4e3cfc11` splits that construction into smaller groups while retaining its flat bridge schema. Native application tests, WASM and browser checks were not reached in this run.

[Run 39233328958](https://github.com/wieslawsoltes/FerroUI/actions/runs/39233328958), source `40b77e14`, passed the same core, host, preedit and compositing gates. Native application checking cleared the macro issue and reported a type mismatch introduced by the blend selector: iterator positions were unsigned while FerroUI ComboBox selection indices are signed. The selector now converts its bounded index to `i32`, preserving the native `-1` empty-selection value. This run did not reach native application tests, WASM or browser execution. The next benchmark uses schema 5 labels `portablePretty` and `compactAutosave` to identify the serializers now used by their respective production paths; historical measurements keep their original schemas.

[Run 39234991185](https://github.com/wieslawsoltes/FerroUI/actions/runs/39234991185), source `52b89f0e`, passed 197 core tests, 17 host tests, six native preedit tests, all 14 public compositing tests and native application checking. The complete native application test binary compiled and ran: 32 of 35 tests passed, including all nine incremental-renderer tests. Three backdrop pixel assertions exposed destructive Source group composition clearing pixels beyond the replacement bitmap or its geometry clip. Both bitmap replacements now use public `RenderOptions.bitmap_blending_mode = Source` with the original clips retained. All seven backdrop tests and their pixel assertions remain in place for the next run. CI8 did not reach WASM or browser execution.

[Run 39238255672](https://github.com/wieslawsoltes/FerroUI/actions/runs/39238255672), source `6809faad`, passed the same core, host, preedit, compositing and native compilation gates, followed by all 35 native application tests. All seven backdrop tests and all nine incremental-renderer tests passed. The actual Vello WASM build and five Chromium IndexedDB transaction checks also passed. Software2D completed 20 interaction checks, including vector networks, native rich-text range styling and variable authoring, then could not open the component variant dialog. Its inspector button had mistakenly been placed in the icon table instead of the command-hook table; the native pointer click therefore had no command to invoke. WebGL2 completed 18 checks and the rich-text model assertions before that dialog's raw screenshot timed out after fonts had loaded. There were no recorded page or console errors in either mode. The failure screenshot showed the arranged rich-text dialog, while the earlier WebGL2 paint screenshot had recovered using the unchanged-state retry. This run is a failure, not complete browser validation.

The subsequent shell projection cache has eight native regression cases and adds cumulative projection counters to the existing 10,000-layer browser workload. All eight native cases and the browser nudge/undo/redo projection checks passed at `ecd5078d`, as recorded above. The passing 35-test gate at `6809faad` predates and does not validate them. Cache reuse avoids complete row/name materialization for a scoped edit, while expanded-set comparison and document-history metadata can still have linear costs.

The branch incorporates upstream `3d8b473` (FerroUI 0.3.0-preview.1), with the standalone sample lockfile aligned and all registry versions retained. Checkpoint `26170911` adds a bounded editor change feed and incremental render preparation; `468a02b7` adds native grid placement/spans/reorder, cumulative browser preparation counters and history-depth assertions. All nine native incremental-renderer pixel/count tests passed at `52b89f0e`, including affected leaves, ancestor bounds, camera reuse, undo, identity changes and conservative fallbacks. Their subsequent native and browser checks completed successfully at `ecd5078d`. The workflow runs all native application tests before the browser build. The browser workload requires nudge/undo/redo in a flat 10,000-layer document to each add one prepared leaf, zero full preparations and zero immutable visual rebuilds; those counts are not an FPS benchmark.

The subsequent fixes connect the component inspector button to its command and make the browser check require an open native dialog with the original selection. Static native checkpoints now share the existing verified screenshot path. The expanded harness retains real native pointer/keyboard operations and PNG captures. The paint checkpoint waits for newly arranged native controls and a render after the Add-stop input. A screenshot TimeoutError may receive one bounded retry only if document hash, selection, camera and native control bounds remain unchanged, the page responds, and the renderer has no lost context or panic. Per-attempt diagnostics remain in the report; a second failure, changed state, or invalid/blank capture fails the run. Continuous traces and font readiness remain enabled. Live Smart Animate captures keep their original timing-sensitive path. CI9 demonstrates one successful recovery of an unchanged paint dialog, not a general browser pass. The corrected component commands and all static checkpoints subsequently passed in the complete `ecd5078d` run.

## Framework integration CI

The repository's [standard PR CI run `39245212870`](https://github.com/wieslawsoltes/FerroUI/actions/runs/39245212870) checked head `ecd5078d` as GitHub's merge `86da361083cfd9146dda6bac99571590a61d244b` into base `53cd3e8648d8a27c04794d3fe48b7a88d64a71ff`. It completed with 13 successful jobs out of 14. macOS, Windows, Linux X11/Wayland/framebuffer, Android, iOS, packaging and all three browser build configurations passed, including their actual Chrome interaction suites. The sole failure was the source-convention gate, which identified one direct `thread_local!` declaration in the application's shell storage.

Checkpoint `5744ef7d85426246237aaa50984677ebcf6525da` corrects that declaration to `ferroui_base::ferro_thread_local!`, retaining its initializer while following FerroUI's platform-aware storage convention. The repository's existing thread-local converter check and generated-composition check pass against that correction. These merge-based framework results are separate from the FerroDesign workflow's exact branch-head native/WASM/browser validation; neither `ecd5078d` run validates the later macro correction. Subsequent candidate results are linked in PR #254.

## Earlier paired release measurements

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

The savings come from reducing snapshot, comparison and scene-update work for known mutation sets. They do not include native control layout, scene preparation, rasterization, browser input delivery or JSON serialization. Pretty JSON serialization remained 36.595 ms p50 for 20,152,206 bytes in this run; coalescing before serialization reduces its frequency. A compact-format candidate was measured separately in that run: 19.621 ms p50 for 12,802,055 bytes, preserving an exact compact-to-document-to-pretty round trip. A later implementation adopts this serialization path for autosaves; portable exports retain the pretty format. Its browser gates verify actual committed compact bytes, exact reload and authored-state isolation during active presentation. Those autosave checks subsequently passed in both browser contexts at `ecd5078d`.

The [schema-5 measurements at `52b89f0e`](benchmarks/2026-10-19-52b89f0e-editor-autosave.json) identify the production serializers explicitly. In that run, portable pretty output took 29.076 ms p50 for 20,152,206 bytes; compact autosave output took 15.751 ms p50 for 12,802,055 bytes, with exact document round-trip equivalence. Compact output was 36.5% smaller. These are five-sample native serialization measurements from the same process, independent of IndexedDB commit latency or browser rendering. Timing differences between CI runners are not treated as changes in implementation speed.

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
| Native TextBox preedit regressions | All 6 tests in the focused `preedit` gate passed at `c2d2e71a` in run `39168689287`; it compiles the controls test binary and exercises pending-state ownership, selection deletion, empty updates, detachment and existing preedit regressions |
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

The editor also guards a separate durability boundary: when a native preedit update does remove the selection, its text-changing notification must not replace confirmed document text. `TextBox::is_composing_text()` exposes a stable native-client signal independent of caret/presenter synchronization. The shell keeps those intermediate notifications outside the document draft. A browser regression now starts with confirmed Unicode input, begins unfinished native composition, dispatches the real pagehide listener, checks durable text and rich runs, exercises one-step undo/redo, and reloads the page. These assertions passed in both modes at `c2d2e71a` in run `39168689287`; all six focused native signal regressions also passed in that run.

The [fourth run](https://github.com/wieslawsoltes/FerroUI/actions/runs/39165941373), revision `f1184c3e`, exposed two harness timing assumptions. In Software2D, a second double-click at the identical position followed the first within the native 500 ms click-count window, so the next text editor did not open. The fixture now makes a real blank-canvas click at a different position before each opening double-click, establishing an independent native sequence without arbitrary sleeps. In WebGL2, all native gradient edits succeeded, but `locator.screenshot` timed out waiting for DOM canvas stability; the fallback page screenshot succeeded and showed the correctly arranged three-stop dialog. The paint artifact now uses a page screenshot at the same fixed viewport. All model, keyboard, history, composition and persistence assertions remain in place, and the complete corrected browser suite passed in the fifth run.

## Earlier large-document browser result

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

Lifecycle durability checks cover confirmed on-canvas text and committed document edits. Native inspector fields commit on Enter or focus loss; numeric arrow keys in Effects also commit. Page or window close does not flush a focused, uncommitted inspector draft, so these checks do not establish recovery of such drafts.

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
