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
| 2026-10-20 continuation | Fresh targeted/full integration results are recorded here as the validation run completes | Work in progress; no placeholder pass counts |

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
the executed source acceptance target.

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
