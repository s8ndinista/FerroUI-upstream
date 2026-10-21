# WinUI / Gallery API and port-state scanner

This scanner keeps upstream source accounting, namespace/member candidates, and
behavioral port evidence separate. A same-name type or method **never** counts as
functional parity. The checked-in source snapshots work without network access,
.NET, Rust, Windows or the upstream checkouts.

```sh
# Regenerate candidate maps and port-state reports from current Rust sources.
python3 scripts/winui-gallery/scan.py

# Offline CI: validate pinned snapshot hashes, evidence anchors and exact reports.
python3 -m unittest discover -s scripts/winui-gallery -p 'test_*.py'
python3 scripts/winui-gallery/scan.py --check
python3 scripts/winui-gallery/event_targets.py --check
python3 scripts/winui-gallery/symbol_map.py --check

# Deliberately fails while API contracts or Gallery pages lack execution evidence.
python3 scripts/winui-gallery/scan.py --check --require-complete
```

## Source refresh

`docs/porting/winui-gallery/data/sources.lock.json` names exact commits of
`microsoft/microsoft-ui-xaml` and `microsoft/WinUI-Gallery`. Prepare clean checkouts
at those commits, then regenerate all immutable reference snapshots:

```sh
python3 scripts/winui-gallery/scan.py --refresh-reference \
    --winui-root ../upstream-winui --gallery-root ../upstream-gallery
```

A mismatched HEAD or modified tracked upstream file fails the refresh. Updating
the pins is a reviewed change: update both commit fields, refresh, inspect the
source-file and API deltas, then commit the new lock and snapshots together. The
regular offline scan verifies every snapshot against its SHA-256 in the lock.

## Outputs

All generated outputs live in `docs/porting/winui-gallery/data/`.

| File | Meaning |
|---|---|
| `sources.lock.json` | Upstream commit pins and immutable snapshot hashes |
| `source-files.json` | SHA-256, byte count and repository for every selected API/Gallery input |
| `upstream-api.json` | Namespaced MIDL 3 and C# API-model types, bases, members, signatures and source locations |
| `gallery-pages.json` | Every upstream catalog page, code-behind, example and page-directory snippet |
| `gallery-usage.json` | Every Gallery XAML file's expanded type/member names and markup features; C# imports and explicitly qualified references |
| `api-mappings.json` | Reviewed adaptation candidates, member renames and semantic cautions; hand-maintained input |
| `symbol-glyphs.tsv` / `symbol-sources.lock.json` | Every pinned Symbol API value and modern rendered glyph, with API-model and native-renderer hashes |
| `api-evidence.json` | Contract-specific implementation/test anchors and exact-source execution records; hand-maintained input |
| `api-map.json` | Namespace-scoped Rust candidates and inherited-member presence; behavior remains unverified |
| `gallery-xaml-map.json` | XAML member requirements joined through both inheritance trees, with declaring owner and every use site |
| `xaml-feature-map.json` | Audited syntax adaptations, source/test anchors, explicit rejected features and every use site |
| `gallery-page-map.json` | Curated sample implementation evidence joined with upstream requirements |
| `summary.json` / `REPORT.md` | Machine/human summaries with limits and unresolved platform namespaces |

The WinUI input covers `controls/dev/**/*.idl` and the API model under
`dxaml/xcp/tools/XCPTypesAutoGen/{XamlOM/Model,Modules}/**/*.cs`. The model contains
preview and private APIs as well as released APIs; private contracts are retained
and counted separately. No SDK-version or ABI completeness claim is inferred.
The Gallery input includes **all** tracked XAML, C#, snippet text, project and
JSON files under `WinUIGallery`, plus its public machine-readable catalog.

Type mappings can include an `overloads` object keyed by the exact source
signature. This distinguishes a constructor such as `SymbolIcon(Symbol)` from
the parameterless constructor when their Rust entry points differ. The mapping
still establishes only a declaration candidate; behavioral evidence is separate.

The icon-specific `symbol_map.py` gate checks all 197 public values, their
modern glyph conversions from WinUI's `icon.cpp`, the Rust iteration table and
the XAML enum names. It runs offline. Refreshing its immutable reference uses
`--refresh-source --winui-root <clean pinned checkout>`. See
`docs/porting/winui-gallery/CONTROL-CONTRACTS.md` for native rendering test scope
and installed-font limitations.

The `event_targets.py` gate generates the XAML loader's exact event-owner table
from the pinned public API snapshot. It follows WinUI inheritance and requires an
unambiguous namespace/type identity. A custom property with the same spelling as
a WinUI event does not become an event by name alone. The table prepares strict
target diagnostics; it does not establish that an event-binding contract is implemented.

The exported catalog is an index, not the original snippet text. Its exporter
may remove substitution tokens from the displayed XAML. Every example retains
its `catalog_transformations`, original `.txt` source path and raw-file hash;
sample execution and source display must use those original files. The current
pin has 355 actual `ControlExample` nodes and 350 exported examples, so both
inventories remain visible and both participate in complete-page verification.

## Curated Gallery evidence

The sample owns `samples/WinUIGallery/port-status.json`:

```json
{
  "schema_version": 1,
  "pages": {
    "Button": {
      "status": "partial",
      "implementation": ["samples/WinUIGallery/pages.rs"],
      "tests": [{"path": "samples/WinUIGallery/tests.rs", "name": "button_click_updates_output"}],
      "notes": "The click example is executable; all remaining upstream examples need evidence."
    }
  }
}
```

Omitted pages are `pending`. `partial` means some live implementation exists;
`ported` means the author claims the page is implemented; `verified` requires
implementation files, existing test anchors, every catalog example and actual
`ControlExample` instance accounted for as verified, and a distinct execution
record. Test names alone do not prove that
the tests ran. Generated reports never promote a page automatically. A separate
runtime integration test must establish that the sample actually creates controls
and executes the contracts; browsing original source does not establish that.

An execution reference has the form `{"record": "path/to/execution.json"}`. Its
record contains `schema_version: 1`, `result: "passed"`, the actual `command`,
`passed_tests`, and `source_sha256` for every implementation/test input. The
scanner rejects failed, empty, unrelated or stale records. API evidence uses the
exact `id` from `api-map.json`, including member kind and signature, so overloads
cannot accidentally share one completion flag. A type requires separate evidence
for construction/inheritance and every public member before it can be verified.

## Deliberate scanner limits

- Source-level lexical parsers do not run Microsoft's MIDL/C# code generators or
  evaluate SDK/preview feature conditions. Source attributes (including default
  values and projected overload names) remain in the reference for review.
- C# imports and explicitly qualified references are inventoried. Unqualified
  receiver expressions, aliases, extensions and overload resolution require a
  Roslyn/WinMD semantic pass; the scanner does not invent bindings for them.
- Rust comparison uses the crate's namespace table, public member declarations
  and unambiguous base-class lookup. It does not prove Rust signatures, XAML
  registration behavior, property defaults, event ordering or visual equality.
- Alternate control candidates such as `ListView` to `ListBox` retain explicit
  semantic cautions and cannot be treated as interchangeable aliases. New opt-in
  implementations such as `NumberBox` map to their own WinUI namespace types.
- Platform imports remain unresolved adapter requirements until native service
  implementations and platform-specific execution evidence exist.

The strict gate is intentionally stronger than report freshness. It must not be
used as a required green check while this port is partial, or weakened by treating
source-name matches as implementation evidence.
