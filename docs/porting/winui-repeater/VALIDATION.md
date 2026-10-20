# Repeater validation

The inventories identify source contracts and scenario relationships. This page
records execution evidence separately. A mapped function, a generated IDL name
or a test that was merely discovered on disk is not an executed test.

## Reproducible checks

From the repository root:

```sh
python3 scripts/winui-repeater/inventory.py --check
cargo test -p ferroui-controls --lib repeater
cargo test -p ferroui-controls --lib
cargo test -p ferroui-base --lib
cargo test -p xamlx --lib
cargo test -p ferroui-markup-xaml-tests --lib
```

Browser and catalog compilation follow the repository's normal
platform/build workflow; a native-only library test does not exercise a browser
host. Memory-constrained environments can use one build job and reduced debug
information without changing the tested source behavior.

The important scenario groups are ownership/collection changes, virtualized
layout and anchors, partial metadata/frozen rows, template/pool identity,
phased-binding cancellation, frame/timer lifetime, focus and automation order,
native XAML conversions, and lazy hierarchical selection. Test names and source
links for the pinned upstream scenarios are in [TESTS.md](TESTS.md).

## Recorded checkpoints

| Checkpoint | Evidence | Scope |
|---|---|---|
| Recovered branch `5acfd247` | Remote commit records 4,380 Base tests passing, browser TypeScript typecheck/lint passing and DOM preference probes passing | Historical commit message evidence for the motion-preference change; not a fresh full-port test run |
| 2026-10-20 inventory extraction | 302 upstream files and 265 test declarations extracted from the existing NOTICE pin; source hashes and IDL/include identities retained | Reproducible source mapping; no runtime pass claim |
| 2026-10-20 CI, `5db512b2` | Completed Android job: 159 Android-crate host tests and 4,380 Base tests passed; Android catalog target checked successfully | Exact revision, configuration, commands and job link below; Controls and XAML unit results remain pending |
| 2026-10-20 Linux CI, `5db512b2` | Completed framebuffer and Wayland jobs: 70 and 116 backend tests passed, both built the native catalog and completed platform/catalog smoke checks | Actual native catalog-build evidence; captured pages exclude Repeater, and Wayland exercised the dmabuf fallback because a native render node was unavailable |
| 2026-10-20 macOS CI, `5db512b2` | Workspace test build stopped with `E0599` in the acceptance grid-placement helper: `FerroList<i32>` has `count()`, not `len()` | The helper is corrected to the native `count()` API; a fresh compile/run is required. Controls tests did not execute in this job |
| 2026-10-20 continuation | Fresh targeted/full integration results are recorded here as the validation run completes | Work in progress; no placeholder pass counts |

### Completed Android job on `5db512b2`

[CI run 39422650581, job 118627822735](https://github.com/wieslawsoltes/FerroUI/actions/runs/39422650581/job/118627822735)
completed successfully on 2026-10-20 for commit
[`5db512b289b6b7b34af30bac0b1f898ae8569739`](https://github.com/wieslawsoltes/FerroUI/commit/5db512b289b6b7b34af30bac0b1f898ae8569739).
The job used the Ubuntu 24.04 runner, host target
`x86_64-unknown-linux-gnu`, and Rust 1.99.1
(`58cf8f9c3`, 2026-10-13). `CARGO_INCREMENTAL=0`,
`CARGO_PROFILE_DEV_DEBUG=0` and `CARGO_PROFILE_TEST_DEBUG=0` were set by CI.

| Check | Command / configuration | Target | Result |
|---|---|---|---|
| Android crate logic | `cargo test --locked -p ferroui-android --no-fail-fast` | Linux host, `x86_64-unknown-linux-gnu` | **159 passed, 0 failed, 0 ignored**; the subsequent documentation-test phase contained zero tests |
| Base services with table-based thread-local storage | `RUSTFLAGS="--cfg ferro_tls_table" cargo test --locked -p ferroui-base --lib --no-fail-fast` | Linux host, `x86_64-unknown-linux-gnu` | **4,380 passed, 0 failed, 0 ignored** |
| Android catalog compilation | Source `scripts/android/env.sh`, then `cargo check --locked -p control-catalog-android --target aarch64-linux-android` | Cross-check for `aarch64-linux-android` | **Succeeded**; compilation/check evidence, not an on-device run |

These commands validate the named host suites and the Android catalog build.
They do not execute the `ferroui-controls`, `xamlx` or
`ferroui-markup-xaml-tests` unit suites. Those continuation results remain
pending and must be recorded separately when their runs complete.

### Completed Linux catalog/platform jobs on `5db512b2`

The following jobs in the same [CI run 39422650581](https://github.com/wieslawsoltes/FerroUI/actions/runs/39422650581)
completed successfully for exact commit
`5db512b289b6b7b34af30bac0b1f898ae8569739` on the native Linux host.
Both ran `cargo build --locked -p control-catalog-desktop` successfully. These
are executed native catalog-build gates for this revision, with the Repeater
page included in the catalog compilation.

| Job | Unit-test command and result | Completed platform/catalog checks |
|---|---|---|
| [Linux framebuffer, job 118627822444](https://github.com/wieslawsoltes/FerroUI/actions/runs/39422650581/job/118627822444) | `cargo test --locked -p ferroui-linuxframebuffer --no-fail-fast`: **70 passed, 0 failed, 0 ignored**; the separate documentation-test phase contained **1 ignored** test | Backend smoke checks; catalog picture smoke on the `vkms` framebuffer `/dev/fb1` and virtual DRM card `/dev/dri/card0`, both at 1024 × 768; both catalog processes exited successfully |
| [Wayland under headless sway, job 118627823391](https://github.com/wieslawsoltes/FerroUI/actions/runs/39422650581/job/118627823391) | `cargo test --locked -p ferroui-wayland --no-fail-fast`: **116 passed, 0 failed, 0 ignored** | Window rendering/input smoke checks; catalog popup, input-method and backend-detection checks; the catalog exited successfully |

The framebuffer catalog captures covered `Home`, `Buttons`, `TextBlock`,
`ListBox`, `Image`, `Calendar` and `Slider`. The Wayland captures exercised the
existing popup and text-input pages. **Neither job captured the Repeater page**;
these results establish platform/catalog integration, not Repeater-specific
visual equivalence.

The Wayland runner had no DRM render node. Its requested dmabuf path fell back
to `wl_egl_window`, and that fallback was exercised successfully. Native dmabuf
swapchain execution was **not available** on this runner. The completed job
must not be interpreted as evidence for that unavailable path.

### macOS workspace compile blocker on `5db512b2`

[macOS job 118627822985](https://github.com/wieslawsoltes/FerroUI/actions/runs/39422650581/job/118627822985)
completed with a build failure for the same exact head. Its command was:

```sh
cargo test --workspace --locked --no-fail-fast --lib --tests --exclude ferroui-render-backends-comparison --exclude ferroui-vello
```

The final log reported one compiler error, `E0599`, at
`src/FerroUI.Controls/repeater/acceptance_tests.rs:540`: the grid-placement
verification helper called `len()` on `&FerroList<i32>`. FerroUI's collection API
uses `count()`. The helper has been corrected to call `source.count()`; its
placement and realization assertions are unchanged. This is a source correction,
not a successful validation result. **Controls tests did not execute**, and a
fresh workspace compile/run must establish the corrected result. No Controls or
XAML pass counts are inferred from this failed build.

The continuation review found an existing `flow_layout_upstream_tests.rs` file
that was not declared from its parent module. Its declaration has been restored
for the current integration build. Source discovery alone would otherwise have
overstated coverage; it still does not substitute for a completed test run.

The native acceptance module reproduces the nested-template scroll destinations,
the offscreen cached group/inner-item assertion, stable-reset zero source reads,
and the ignored scrolled-grid insertion placement assertions. It also preserves
the three interaction scenarios' layout/offset/count assertions through direct
native scrolling and collection operations. The Windows UI Automation driver
and wheel transport are replaced; their native input behavior is not covered by
these headless equivalents. The source animated nested-scroll variant already
returns early under microsoft-ui-xaml#779; the nine nonanimated destinations are
the source acceptance target.

## Acceptance boundaries

Runtime and compiled XAML need actual native-template/interface/nullability
round trips, not only type registration. The native class declaration scanner
and actual catalog build are useful additional gates for those paths.

Headless automation and keyboard-order tests establish portable peer/tree
semantics. They do not establish Windows UI Automation client interoperability,
actual screen-reader announcements or every native input backend. Browser
compilation alone does not establish scroll-frame behavior or rendered output.

Keep platform runs, visual checks and benchmark measurements separate from unit
test counts. Record the revision, target/backend, workload and actual result
when adding such evidence. No visual or throughput parity measurement is
currently supplied by the inventory generator.
