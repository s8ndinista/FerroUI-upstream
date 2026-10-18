# Validation and measurements

This file distinguishes commands that have passed from checks that are still in progress. It will be updated with the completed browser run and its artifacts.

## Observed results

| Check | Result |
| --- | --- |
| Core unit tests | 39 passed, including 12 SVG import/affine/atomicity regressions |
| Native full application check | Passed on Linux, Rust 1.99.1; compiled the XAML shell and shared renderer |
| FerroUI browser bridge TypeScript check/build | Passed |
| Browser host JavaScript and shell syntax | Passed |
| Active browser dependency graph | No `ferroui-skia`, `skia-safe` or `skia-bindings` |
| Actual WebAssembly application build | In progress |
| Browser pixels and editing interactions | Pending the WebAssembly build |
| macOS/Windows native runtime | Not run in this environment |
| Exact Figma visual parity | Not measured |

## Document-engine benchmark

The release benchmark creates 10,000 editable nodes, builds the spatial scene, issues point/viewport queries, performs 120 incremental drag updates and measures transaction history. Values below are actual measurements in the development container; they are not browser FPS, GPU measurements or hardware-independent guarantees.

| Operation | Measured result |
| --- | ---: |
| Point hit query, average over 10,000 queries | 2.32 microseconds |
| Viewport query, average | 44.18 microseconds |
| Incremental drag refresh, median | 3.56 milliseconds |
| Incremental drag refresh, p95 | 4.04 milliseconds |
| Transaction commit | 9.52 milliseconds |
| Undo | 3.91 milliseconds |
| Redo | 3.62 milliseconds |

The initial full-index refresh took 8.87 ms median and 10.53 ms p95 for the same drag workload. Updating affected subtrees reduced median refresh time by approximately 2.49 times. The associated correctness tests cover clipped transformed subtrees, stacking invalidation and undo, so the optimization is not based only on a timing result.

Reproduce the workload from the sample directory:

```sh
cargo run --release -p ferrodesign-core --example performance -- 10000
```

## What the browser test verifies

`tests/browser.mjs` launches the actual application at a 1600 by 1000 viewport in two contexts, requesting Software2D and WebGL2. It records the observed canvas context and allows the documented WebGL2-to-Software2D fallback. The CI browser may use SwiftShader; its results establish functional rendering, not real GPU throughput.

The test waits for a completed native frame and dismissal of the loading surface, inspects native canvas pixel diversity, draws a rectangle with mouse input, verifies the document count and changed pixels, performs keyboard undo/redo, then deletes and reimports exported JSON and checks exact document equality. It collects console exceptions, startup failures and FerroUI render-thread panics. Screenshots, Playwright traces and the report are written under `browser-report` and uploaded by GitHub Actions.

An interaction test is evidence for those paths, not for every item in the feature inventory. There is no same-document image comparison to a Figma reference screenshot. Browser rendering timings and memory should be measured on the intended deployment devices before establishing a frame-rate or capacity target.
