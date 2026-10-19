# Rich text port progress

Generated from the pinned source inventory and reviewed `mapping.json`.

## Inventory coverage

| Scope | Files |
|---|---:|
| `dxaml/xcp/core/text/` | 230 |
| `dxaml/xcp/core/native/text/` | 62 |
| `dxaml/xcp/components/text/` | 23 |
| Additional explicit dependencies, projections, metadata and test source | 311 |
| Total selected source/reference files | 626 |
| Additional framework headers reached through unambiguous includes | 913 |
| Include boundary edges requiring SDK/generated-header/include-order review | 806 |

## Reviewed implementation status

| State | Files | Public API entries |
|---|---:|---:|
| complete | 0 | 9 |
| partial | 130 | 397 |
| pending | 157 | 852 |
| reference | 201 | 0 |
| reuse_candidate | 138 | 0 |

Public API inventory: **1258 exact documented member identifiers**. Type and enum entries are retained alongside properties, methods and events; this denominator is not a count of methods.

`reference` entries are build/generated/test/metadata inputs, not completed runtime functionality. `reuse_candidate` is not verified parity. A `partial` file may contain working behavior while still having unmapped members. All claims of completion require target and test evidence. Internal lexical callable candidates do not establish complete member enumeration.

## Implemented capability evidence

| Capability | State | Evidence / remaining scope |
|---|---|---|
| Structured document and live positions | partial | Shared blocks/inlines, nested snapshots, checked UTF-16 edits, weak live gravity pointers and embedded control identity. Native synthetic element markers and unmapped document APIs remain pending. |
| Rich view and overflow | partial | Existing TextFormatter/TextLine integration, linked continuation, selection geometry, embedded controls and hyperlinks, plus native paragraph spacing rules and signed indents with point conversion and RTF sl/slmult. Full native formatting, templates, appearance and remaining control members stay pending. |
| Editing, history and text object services | partial | Shared default formats/history, cancellable and reentrant selection, typed get/set/find flags with explicit unsupported modes, native newline fixture, end-relative UTF-32, formatted copies and live scalar-safe range operations. Remaining native units, streams, advanced formats and external TOM ABI remain pending. |
| Clipboard, RTF and input method | partial | Exact in-process snapshot transfer, explicit bounded RTF subset, asynchronous stale-paste guards and composition preview/commit/cancel. Unsupported RTF features and real backend IME/service parity remain pending. |
| Text accessibility and Windows bridge | partial | Portable Text/TextRange/Text2/TextChild, stable weak hyperlink peers, shared document/overflow geometry, 25 typed attributes and native COM/dispatcher marshalling have implementation and focused regressions. TextEdit, annotations, remaining attributes, other native backends and actual Windows client verification remain pending. |
| Text highlighters | partial | Signed copied range values, shared collections, solid-brush validation, weak invalidation subscriptions and last-layer region precedence. All ten source overlap fixtures are adapted; native high-contrast/raster and complete view conformance remain pending. |

## Verification records

- `cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast` — Passed: 97 passed; 0 failed; 0 ignored; 0 measured; 4426 filtered out. This checkpoint contains 21 document, 45 editor, 16 view and 15 portable automation tests. Later view/highlighter additions, four portable follow-up tests, native bridge tests and markup/catalog tests are not covered by this historical run.
- `cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast` — Passed on Linux: 181 passed; 0 failed; 0 ignored; 0 measured; 4434 filtered out. Includes 27 focused TOM, eleven new paragraph/RTF and 26 portable rich automation cases. Does not cover subsequent presenter/stream changes or text_highlight filter.
- `cargo test --locked --offline -p ferroui-win32 --lib text_ --no-fail-fast` — Passed on Linux-hosted Win32 test harness: 34 passed; 0 failed; 0 ignored; 0 measured; 234 filtered out. Includes ten automation text pattern/range bridge cases; not a Windows OS accessibility-client or screen-reader run.

Full WinUI rich text parity remains open until the pending member mappings and [DEVIATIONS.md](DEVIATIONS.md) boundaries are closed with evidence.
