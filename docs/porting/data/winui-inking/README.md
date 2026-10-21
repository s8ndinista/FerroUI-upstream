# Inking source and compatibility ledger

Baseline: Microsoft WinUI
`9427b7216337ea3d7b19ac32d48c327286ee7138` (2026-10-21).

| File | What it records |
|---|---|
| `source-map.json` | Every file under `controls/dev/InkCanvas` and `controls/dev/InkToolbar`: path, immutable Git blob SHA, byte size, category, disposition and Rust mapping. The pinned inventory contains 164 files. |
| `upstream/*.idl` | Byte-for-byte copies of all six IDLs in those directories. Their Git blob hashes are checked against the source inventory. |
| `api-map.json` | Every public type and declared member in those IDLs, including enum values and dependency-property registrations: 45 types and 181 members. Each member has its own disposition. |
| `tests-map.json` | The 65 methods in the two upstream API-test suites, with explicit local test mappings where they exist. Interaction suites and test UIs remain separate entries in the source map. |
| `dependencies.json` | OS services used by the source, portable replacements, compatibility boundaries and adjacent Windows Ink services. These are not presented as source files in WinUI. |

## Status meanings

- `implemented`: the mapped contract has an implementation with no known gap at
  that record's scope. This is not a claim that the entire control or OS engine
  is equivalent.
- `adapted`: the behavior is expressed through a documented FerroUI/Rust
  equivalent. Ownership, framework types or invocation style can differ.
- `partial`: mapped code exists, with a specific remaining semantic or UI gap.
- `not_ported`: no equivalent is claimed; the reason or remaining work is explicit.
- `not_applicable`: build/language scaffolding replaced by the existing toolchain.
- `external`: the implementation belongs to an OS dependency outside the pinned
  WinUI source. Its replacement and validation boundary are explained.

A source can be `partial` while several of its public members are `adapted`.
For example, mapping a stroke container's in-memory selection does not establish
its ISF persistence or native clipboard interoperability. A constructor mapping
does not establish its template, accessibility patterns or localization.

Run from the repository root:

```sh
python scripts/inking/check_port.py
python scripts/inking/check_port.py --json
python scripts/inking/check_port.py --require-complete
```

The normal check validates ledger structure, immutable IDL hashes, exhaustive
type/member accounting and local evidence paths/symbols. It deliberately does
not infer behavior from symbol names. The last command additionally fails while
any partial, missing or external record remains. Test execution, platform
interoperability and visual comparisons are separate evidence described in
[`../../winui-inking.md`](../../winui-inking.md).

## Updating a record

1. Read the pinned upstream implementation and matching tests.
2. Add the implementation and meaningful regression tests using FerroUI APIs.
3. Record the target file, exact API symbol, behavior change and remaining gap.
4. Link a test only if it checks the upstream scenario; construction alone is
   insufficient for a test about layout, clipping or accessibility.
5. Run the ledger checker and the relevant Rust tests, then update the human
   report with the actual command and result.

Do not change the pinned inventory or snapshots to make an omitted member
disappear. A new upstream baseline needs a deliberate inventory/API comparison.

## License

The IDL snapshots and derived declarations are Copyright (c) Microsoft
Corporation. All rights reserved, under the MIT License. The full notice is in
[`../../../../src/FerroUI.Controls/inking/NOTICE.md`](../../../../src/FerroUI.Controls/inking/NOTICE.md).
