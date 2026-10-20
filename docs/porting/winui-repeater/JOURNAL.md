# Repeater port journal

## Published starting points

| Commit | Change |
|---|---|
| `f67b92f3` | Stable-key mapping added to the existing native item-source views |
| `0dd3a661` | Hierarchical selection port with lazy source traversal |
| `fa1d1ae1` | Repeater control, services, ownership, layout algorithms and source-derived tests |
| `5acfd247` | Native animation preferences connected to Repeater transitions |
| `50265bb5` | Continuation branch incorporates the current FerroUI main branch |
| `c3d46d1c` | Source/API/dependency/scenario inventories and the native acceptance checkpoint published to PR #269 |

These are recoverable commits on `feat/winui-repeater-port`, not inferred local
state. The source notice pins WinUI
`1af995b98a02d86b1951c4babe35caa269a6d569`.

## 2026-10-20 — source-based continuation

The remote branch was recovered at `5acfd247`. Earlier conversational reports
mentioned source/API maps, XAML integration work and validation failures, but the
remote branch did not contain a Repeater documentation directory. The maps in
this directory were therefore reconstructed from the pinned official source
and the actual Rust tree; unverifiable earlier claims are not reused as current
validation evidence.

The extraction records all 302 upstream files, all defined IDL declarations,
direct include boundaries, and 265 test declarations. Three test declarations
are commented out in the upstream source; ignored source tests are retained as
such. API/test identifiers remain navigation and mapping evidence only.

The GitHub Contents API comparison on 2026-10-20 found Repeater tree object
`1de63a85615582e33d24e19795bd73d374195ac1` at both pinned revision and upstream
main. This establishes source-subtree currency for that checkpoint while
keeping the existing derivation pin stable.

Review found that `flow_layout_upstream_tests.rs` existed without a parent-module
declaration, so those source-derived tests were not part of the recovered test
binary. The declaration is restored for the integration build; execution is
tracked separately in the validation record.

Lined-flow review separated true source behavior from native framework
adaptation: FerroUI's outer `DesiredSize` requires margin-aware constraint
handling; partial metadata/realization and timer lifetime need explicit
regressions; the pinned aspect-ratio cache midpoint expression uses a relative
count where an absolute source index is required. That correction is documented
as intentional rather than silently attributed to upstream.

The native `ItemsSourceView` path now preserves an explicitly supplied view's
identity, notifications and stable-key mapping. New source-identity and release
scenarios are present for integration validation. The lined-flow release test
exercises its existing `Drop` timer shutdown; the continuation also adds a
defensive stop when a queued timer callback cannot upgrade its weak owner.

The acceptance module reproduces two nested-template/cache scenarios using
native data-context bindings, templates, notifying groups and scrolling hosts.
It adds exact stable-reset read-count/identity checks and the ignored upstream
grid-insertion placement scenario. Three interaction scenarios retain their
layout/offset/count assertions with native scroll/collection actions in place
of Windows UI Automation. These tests are recorded as present and pending the
integration run, not as passing because their names match.

Runtime metadata tests cover native class names and constructor requirements in
place of COM activation. Compiled XAML tests exercise native layout/template
construction, keyed factory dictionaries, item binding and control-item
identity. The native catalog example supplies all four layouts and a large
notifying source; source TestUI assets remain reference entries.

The final two partial scenario relationships now point to dedicated source-derived
tests. Reentrancy coverage exercises all four rejected API changes, restoration
of the rejected layout property, original template contents and layout lifetime.
The element-factory scenario checks the empty/add/remove data and factory traces
with the same element and parent identities. It retains FerroUI's live source
count contract instead of reproducing WinRT's cached `GetSizeCore` call counts;
that intentional boundary is recorded in WR-22. All 265 upstream declarations
now have a named scenario relationship or an explicit known difference. This
closes the mapping ledger, not the still-separate runtime validation gate.

Future entries should state the concrete behavior changed, source comparison,
regression test, executed command/result and published commit. Do not record an
intended implementation or a source-name match as a completed parity gate.
