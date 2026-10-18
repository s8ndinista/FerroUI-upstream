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
| partial | 0 | 0 |
| pending | 277 | 1258 |
| reference | 199 | 0 |
| reuse_candidate | 150 | 0 |

Public API inventory: **1258 exact documented member identifiers**. Type and enum entries are retained alongside properties, methods and events; this denominator is not a count of methods.

`reference` entries are build/generated/test/metadata inputs, not completed runtime functionality. `reuse_candidate` is not verified parity. A `partial` file may contain working behavior while still having unmapped members. All claims of completion require target and test evidence. Internal lexical callable candidates do not establish complete member enumeration.

## Implemented capability evidence

No runtime capability has been marked complete in the initial source-map batch.

## Verification records

Runtime tests have not yet been recorded in this mapping. See [TESTS.md](TESTS.md) for the verification plan.

No successful Rust runtime-test command has been recorded yet; source-map checks and linked test definitions are traceability evidence only.

Full WinUI rich text parity remains open until the pending member mappings and [DEVIATIONS.md](DEVIATIONS.md) boundaries are closed with evidence.
