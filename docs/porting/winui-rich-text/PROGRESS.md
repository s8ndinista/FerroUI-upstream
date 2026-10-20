# Rich text port progress

Generated from the pinned source inventory and reviewed `mapping.json`.

## Inventory coverage

| Scope | Files |
|---|---:|
| `dxaml/xcp/core/text/` | 230 |
| `dxaml/xcp/core/native/text/` | 62 |
| `dxaml/xcp/components/text/` | 23 |
| Additional explicit dependencies, projections, metadata and test source | 317 |
| Total selected source/reference files | 632 |
| Additional framework headers reached through unambiguous includes | 942 |
| Include boundary edges requiring SDK/generated-header/include-order review | 826 |

## Reviewed implementation status

| State | Files | Public API entries |
|---|---:|---:|
| complete | 0 | 36 |
| partial | 142 | 428 |
| pending | 153 | 794 |
| reference | 200 | 0 |
| reuse_candidate | 137 | 0 |

Public API inventory: **1258 exact documented member identifiers**. Type and enum entries are retained alongside properties, methods and events; this denominator is not a count of methods.

`reference` entries are build/generated/test/metadata inputs, not completed runtime functionality. `reuse_candidate` is not verified parity. A `partial` file may contain working behavior while still having unmapped members. All claims of completion require target and test evidence. Internal lexical callable candidates do not establish complete member enumeration.

## Implemented capability evidence

| Capability | State | Evidence / remaining scope |
|---|---|---|
| Structured document and live positions | partial | Shared blocks/inlines, nested snapshots, checked UTF-16 edits and weak live gravity pointers updated immediately per replacement, including nested batches and rollback. Native structural markers and remaining document APIs stay pending. |
| Rich view and overflow | partial | Existing TextFormatter/TextLine integration, linked continuation, selection, embedded controls and hyperlinks, paragraph spacing/signed indents, inherited language, shaped explicit/default tabs with leaders and RTL decimal fields, and a persistent editor presenter with logical scrolling. Full native formatting/appearance and remaining members stay pending. |
| Editing, history and text object services | partial | Shared formats/history, cancellable reentrant selection, typed flags, UTF-16 live ranges, eight native-valued TOM units including shaped Cluster, validated movement/index services, structured Unicode case conversion, accepted composition causes and transactional Rust streams. Further units, arbitrary stories, advanced formats, locale-specific casing and external ABI stay pending. |
| Clipboard, RTF and input method | partial | Exact in-process snapshots, bounded RTF with language/spacing/indentation/tab context, 17 portable byte encodings and font-scoped charset/codepage decoding, explicit Read/Write policies, stale-paste guards, semantic IME ownership and exact UTF-16 replacement/selection acceptance. CoreText acknowledgement state is retained for failed-notification retries. Unsupported RTF content/pages, clause/proofing services, remaining legacy/backend transport and native conformance stay pending. |
| Text accessibility and Windows bridge | partial | Portable Text/TextRange/Text2/TextChild/TextEdit, stable weak peers, raw presenter owner separation, aligned geometry/scrolling, 25 typed attributes, accepted composition causes, public peer TextEdit/weak EventsSource routing, runtime metadata and native COM/BSTR/SAFEARRAY marshalling. Conversion-target data, annotations, remaining attributes, other backends and Windows client verification stay pending. |
| Text highlighters | partial | Signed copied range values, shared collections, solid-brush validation, weak invalidation subscriptions and last-layer region precedence. All ten source overlap fixtures are adapted; native high-contrast/raster and complete view conformance remain pending. |
| Rich editor themes and markup | partial | Compiled WinUI, Fluent and Simple ControlThemes host the persistent presenter and framework command flyout. Independent mixed bold/italic/underline queries and one-transaction toggles retain formatting/history. Runtime/compiled rich-document fixtures and a registered ControlCatalog page exercise markup ownership/binding paths. Native command-bar/touch/lazy-part behavior, complete localization, native appearance and unrecorded execution gates remain pending. |

## Verification records

- `cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast` — Passed: 97 passed; 0 failed; 0 ignored; 0 measured; 4426 filtered out. This checkpoint contains 21 document, 45 editor, 16 view and 15 portable automation tests. Later view/highlighter additions, four portable follow-up tests, native bridge tests and markup/catalog tests are not covered by this historical run.
- `cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast` — Passed on Linux: 181 passed; 0 failed; 0 ignored; 0 measured; 4434 filtered out. Includes 27 focused TOM, eleven new paragraph/RTF and 26 portable rich automation cases. Does not cover subsequent presenter/stream changes or text_highlight filter.
- `cargo test --locked --offline -p ferroui-win32 --lib text_ --no-fail-fast` — Passed on Linux-hosted Win32 test harness: 34 passed; 0 failed; 0 ignored; 0 measured; 234 filtered out. Includes ten automation text pattern/range bridge cases; not a Windows OS accessibility-client or screen-reader run.
- `cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast` — Passed on Linux: 246 passed; 0 failed; 0 ignored; 0 measured; 4434 filtered out. Adds persistent editor presenter/scrolling, native-valued TOM unit/index operations, transactional streams, immediate live-pointer rebasing, inherited language and automation owner/aligned-scroll regressions. Subsequent TextEdit/composition/flyout changes and theme work are outside this checkpoint.
- `cargo test --locked --offline -p ferroui-controls --lib --no-fail-fast` — Passed on Linux with loopback access: 4706 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out. Includes all control/highlighter tests plus 17 accepted-composition, four portable TextEdit and five generic-host/flyout-focus additions. The only subsequent source change before publication made an equivalent explicit type registration for the metadata scanner; its separate scan passed.
- `cargo test --locked --offline -p ferroui-win32 --lib text_ --no-fail-fast` — Passed on the Linux-hosted Win32 test harness: 41 passed; 0 failed; 0 ignored; 0 measured; 234 filtered out. Adds four native TextEdit interface/range/payload regressions and three SAFEARRAY conversion/shape/unwind cleanup cases. This is not an actual Windows OS accessibility-client or screen-reader run.
- `cargo test --locked --offline -p ferroui-build-scan --lib scanner::tests::real_crates_are_scanned_without_skipping_a_declaration -- --exact` — Passed: 1 passed; 0 failed; 43 filtered out. TextHighlightRange markup registration names its actual type rather than unresolved Self, closing the CI scanner failure without changing runtime type identity.
- `python scripts/porting/winui-rich-text-inventory.py --reference /workspace/scratch/a7a0d1878f25/winui-reference --check` — Passed for the 2026-10-20 continuation working tree: 632 selected source/reference files, 1258 exact public API identifiers, 942 additional framework dependencies and 826 include boundary edges. This checks generated-record reproducibility, pinned Git content and existing target/test/deviation anchors; it executes no Rust, native, theme or markup tests.

Full WinUI rich text parity remains open until the pending member mappings and [DEVIATIONS.md](DEVIATIONS.md) boundaries are closed with evidence.
