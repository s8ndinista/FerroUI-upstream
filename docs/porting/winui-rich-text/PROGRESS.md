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
| complete | 0 | 0 |
| partial | 128 | 348 |
| pending | 159 | 910 |
| reference | 201 | 0 |
| reuse_candidate | 138 | 0 |

Public API inventory: **1258 exact documented member identifiers**. Type and enum entries are retained alongside properties, methods and events; this denominator is not a count of methods.

`reference` entries are build/generated/test/metadata inputs, not completed runtime functionality. `reuse_candidate` is not verified parity. A `partial` file may contain working behavior while still having unmapped members. All claims of completion require target and test evidence. Internal lexical callable candidates do not establish complete member enumeration.

## Implemented capability evidence

| Capability | State | Evidence / remaining scope |
|---|---|---|
| Structured document and live positions | partial | Shared blocks/inlines, nested snapshots, checked UTF-16 edits, weak live gravity pointers and embedded control identity. Native synthetic element markers and unmapped document APIs remain pending. |
| Rich view and overflow | partial | Existing TextFormatter/TextLine integration, linked continuation, selection geometry, embedded controls and hyperlink interaction. Full native formatting, appearance and control surface remain pending. |
| Editing, history and text object services | partial | Shared editor, coalesced/grouped undo, rich range replacement, typing format, live range facade, five text units and Unicode lowercase search. Full external TOM ABI/options/objects remain pending. |
| Clipboard, RTF and input method | partial | Exact in-process snapshot transfer, explicit bounded RTF subset, asynchronous stale-paste guards and composition preview/commit/cancel. Unsupported RTF features and real backend IME/service parity remain pending. |
| Text accessibility and Windows bridge | partial | Portable six Text/eighteen TextRange operations share live document selection/geometry with sixteen attributes; native COM/dispatcher marshalling and tests are present. Text2/TextEdit/TextChild, annotations, extra attributes, hyperlink peers, other native backends and actual Windows clients remain pending. |
| Text highlighters | partial | Signed copied range values, shared collections, solid-brush validation, weak invalidation subscriptions and last-layer region precedence. All ten source overlap fixtures are adapted; native high-contrast/raster and complete view conformance remain pending. |

## Verification records

- `cargo test --locked --offline -p ferroui-controls --lib rich_ --no-fail-fast` — Passed: 97 passed; 0 failed; 0 ignored; 0 measured; 4426 filtered out. This checkpoint contains 21 document, 45 editor, 16 view and 15 portable automation tests. Later view/highlighter additions, four portable follow-up tests, native bridge tests and markup/catalog tests are not covered by this historical run.

Full WinUI rich text parity remains open until the pending member mappings and [DEVIATIONS.md](DEVIATIONS.md) boundaries are closed with evidence.
