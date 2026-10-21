# Repeater validation

The inventories identify source contracts and scenario relationships. This page
records execution evidence separately. A mapped function, a generated IDL name
or a test that was merely discovered on disk is not an executed test.

## Reproducible checks

From the repository root:

```sh
python3 scripts/winui-repeater/inventory.py --check
cargo test --locked -p ferroui-build-scan --lib
python3 scripts/generate_view_builders.py --check
cargo test --locked -p ferroui-controls --lib repeater
cargo test --locked -p ferroui-controls --lib
cargo test --locked -p ferroui-base --lib
cargo test --locked -p xamlx --lib
cargo test --locked -p ferroui-markup-xaml-tests --lib
cargo test --locked -p ferroui-view --lib
cargo test --locked -p control-catalog --lib tests::items_repeater_page::
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

### 2026-10-21 continuation

The recovered remote head was `3facaa20`. The continuation merged main
`86b6b4ab` and published metadata, scheduler/template and selection corrections
as `91675d5b`, `46718059` and `aaab8857`. These checkpoints are separate from
later scrolling/collection and rendered-catalog validation.

The scrolling and pinned-collection correction is published in `e2a16bb3`,
native diagnostics and strengthened ClearValue assertions in `8fd6b892`, and
the compiled sample/browser regression in `9039d2a5`.

The full local Controls run of these runtime sources passed **4,847 tests,
0 failed, 0 ignored**, including **415 Repeater tests**. It used Rust 1.99.0
on `x86_64-unknown-linux-gnu`, with one build job, incremental compilation
disabled and reduced debug information:

```sh
CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 \
CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 MALLOC_ARENA_MAX=2 \
cargo test --offline --locked -p ferroui-controls --lib \
  --config profile.test.package.ferroui-controls.codegen-units=512 \
  -- --test-threads=2
```

The codegen-unit override only reduced peak compiler memory; it is not a source
or feature change. The run required permission to bind local loopback sockets
for the six pre-existing remote-transport tests. An earlier sandboxed run
reported those six `PermissionDenied` failures and one diagnostic regression
whose setup reused cached Stack controls. The test now replaces its source to
require a genuine realization; its exact event assertions remain intact. The
successful complete rerun did not exclude any test. An earlier compiler attempt
was killed by the shared environment's memory limit before tests executed.

| Revision / run | Executed check | Result and limit |
|---|---|---|
| [`9039d2a5`, focused CI job 119200628970](https://github.com/wieslawsoltes/FerroUI/actions/runs/39593601866/job/119200628970) | Complete Controls library with `ferroui-base/testing,ferroui-base/compiler-metadata,ferroui-base/markup-functions` | **4,847 passed, 0 failed, 0 ignored**, including **415 Repeater tests**; the animated aligned-bring identity assertion and all new lifecycle regressions execute |
| Same focused CI job | Regenerate both compiler snapshots, rebuild and execute runtime/compiled XAML, verify checked-in snapshot diff | **627 passed, 0 failed, 17 ignored** in the main suite; both explicitly invoked snapshot generators separately passed; generated Rust, source map and Rust-path check were current |
| [`9039d2a5`, Android CI job 119200802343](https://github.com/wieslawsoltes/FerroUI/actions/runs/39593601938/job/119200802343) | Android host suite, Base with `--cfg ferro_tls_table`, Android catalog compilation and smoke-APK packaging | **159 Android and 4,380 Base tests passed, 0 failed, 0 ignored**; arm64 catalog compilation and packaging succeeded; not an on-device run |
| [`9039d2a5`, Linux framebuffer CI job 119200802262](https://github.com/wieslawsoltes/FerroUI/actions/runs/39593601938/job/119200802262) | Framebuffer unit suite, native catalog build and virtual framebuffer/card smoke checks | **70 passed, 0 failed**; one separate documentation test ignored; catalog build and smoke checks succeeded; these are platform integration checks, not Repeater-specific captures |
| [`9039d2a5`, iOS simulator CI job 119200802432](https://github.com/wieslawsoltes/FerroUI/actions/runs/39593601938/job/119200802432) | iOS unit and dispatcher integration suites, simulator build/install, device-target and catalog checks | **104 unit and 10 dispatcher tests passed, 0 failed**; simulator installation and target checks succeeded; not a Repeater-specific rendered or physical-device run |
| Local `91675d5b` | `ferroui-build-scan` library suite and View-builder generator consistency | **44 passed**; direct bitflag expressions and generated native Repeater/layout builders are covered |
| Local `9039d2a5` | `cargo test --offline --locked -p ferroui-view --lib -- --test-threads=2` | **49 passed, 0 failed, 0 ignored**; the generated Repeater/layout builders compile with the complete native View library and its XAML/reconciliation tests |
| Local 2026-10-21, Base/XamlX sources from `aaab8857` | `cargo test --offline --locked -p ferroui-base -p xamlx --lib -- --test-threads=2` | **4,380 Base and 103 XamlX tests passed**, with no failures or ignored tests; native Linux host, Rust 1.99.0 |
| Local `aaab8857` | Complete `repeater` filter in the native Controls library | **404 passed, 1 failed**; the only failure retained the target-identity assertion in animated aligned bring |
| [`aaab8857`, focused CI job 119191031867](https://github.com/wieslawsoltes/FerroUI/actions/runs/39590722843/job/119191031867) | Regenerate both compiler snapshots, compile their output, execute runtime/compiled XAML suite, verify snapshot diff | **627 passed, 0 failed, 17 ignored**; both explicitly invoked snapshot generators passed and checked-in output was current |
| Same focused CI job | Full Controls library with `ferroui-base/testing,ferroui-base/compiler-metadata,ferroui-base/markup-functions` | **4,836 passed, 1 failed**; the same animated aligned-bring target-identity assertion, with no failing test filtered out |
| [`aaab8857`, Android CI job 119190990284](https://github.com/wieslawsoltes/FerroUI/actions/runs/39590722859/job/119190990284) | Android host suite, Base with `--cfg ferro_tls_table`, Android catalog cross-check/package | **159 Android and 4,380 Base tests passed**; catalog target checks succeeded; not an on-device run |

The final-source focused run completed successfully. Its regenerated compiler
output is retained in [snapshot artifact 12342684493](https://github.com/wieslawsoltes/FerroUI/actions/runs/39593601866/artifacts/12342684493).
The workflow checks the pull-request merge commit `8286c00b` for branch head
`9039d2a5`; that head includes every runtime and catalog change listed above.
The earlier failed scrolling checkpoints below remain as the history of the
regression, not an outstanding failure of this revision.

The local compiled-catalog attempt stopped before test execution when the shared
filesystem filled: `wgsl-parse` could not write an intermediate `.rmeta` file
(`No space left on device`). This task's build intermediates were removed after
the command exited; completed test logs were retained. No catalog pass is
inferred from that attempt. CI catalog execution is recorded independently.

The original `3facaa20` focused CI run had **4,827 passing Controls tests and
the same single failure**, plus **627 passing XAML tests and 17 ignored**.
The continuation read the completed logs instead of retaining the older
handoff's list of three outstanding Controls failures.

The new browser regression was first executed against the exact `3facaa20`
nonthreaded `browser-sites` artifact from
[CI run 39496708534](https://github.com/wieslawsoltes/FerroUI/actions/runs/39496708534).
Both **WebGL2 and Software2D passed** real wheel input, all four layout switches,
insert/move-to-end/remove/reset, realization bounds, glyph pixels and changed
viewport pixels. The initial 10,000-item source realized 24 Stack, 162
UniformGrid, 162 Flow and 259 LinedFlow items on the 1280 × 1000 test surface.
Those counts describe that artifact and workload; they do not certify a later
revision or a universal memory bound. Screenshot inspection exposed the
sample's hard minimum-width interaction with LinedFlow's documented constrained
row overflow, prompting a sample metadata correction and a later rebuild.

The browser test and screenshot/report command is:

```sh
scripts/build-browser.sh control-catalog-browser
node scripts/browser/tests/repeater.test.mjs target/browser/control-catalog-browser \
  --screenshot-dir target/browser/screenshots/repeater
```

The test runs both rendering modes by default. The same script accepts the
threaded site directory and supplies cross-origin isolation automatically.
Generated reports identify the supplied site and backend, layout counters and
visible item geometry. The browser CI screenshot artifact upload retains them.

### Earlier recorded evidence

| Checkpoint | Evidence | Scope |
|---|---|---|
| Recovered branch `5acfd247` | Remote commit records 4,380 Base tests passing, browser TypeScript typecheck/lint passing and DOM preference probes passing | Historical commit message evidence for the motion-preference change; not a fresh full-port test run |
| 2026-10-20 inventory extraction | 302 upstream files and 265 test declarations extracted from the existing NOTICE pin; source hashes and IDL/include identities retained | Reproducible source mapping; no runtime pass claim |
| 2026-10-20 CI, `5db512b2` | Completed Android job: 159 Android-crate host tests and 4,380 Base tests passed; Android catalog target checked successfully | Exact revision, configuration, commands and job link below; Controls and XAML were not executed by this job |
| 2026-10-20 Linux CI, `5db512b2` | Completed framebuffer and Wayland jobs: 70 and 116 backend tests passed, both built the native catalog and completed platform/catalog smoke checks | Actual native catalog-build evidence; captured pages exclude Repeater, and Wayland exercised the dmabuf fallback because a native render node was unavailable |
| 2026-10-20 macOS CI, `5db512b2` | Workspace test build stopped with `E0599` in the acceptance grid-placement helper: `FerroList<i32>` has `count()`, not `len()` | The helper was corrected to the native `count()` API; subsequent focused CI compiled and executed Controls and XAML. This historical job executed neither suite |

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
`ferroui-markup-xaml-tests` unit suites. Subsequent continuation results are
recorded separately above.

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
fresh workspace compile/run was required at that checkpoint. The later focused
CI above compiled and executed both libraries. No Controls or XAML pass counts
are inferred from this failed historical build.

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

Runtime and compiled XAML scenarios exercise native-template/interface/nullability
round trips as well as type registration. The native class declaration scanner
and actual catalog build are separate integration gates for those paths.

Headless automation and keyboard-order tests establish portable peer/tree
semantics. They do not establish Windows UI Automation client interoperability,
actual screen-reader announcements or every native input backend. Browser
compilation alone does not establish scroll-frame behavior or rendered output.

Keep platform runs, visual checks and benchmark measurements separate from unit
test counts. Record the revision, target/backend, workload and actual result
when adding such evidence. No visual or throughput parity measurement is
currently supplied by the inventory generator.
