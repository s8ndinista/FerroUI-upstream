# Rich text deviations and remaining contracts

This file distinguishes deliberate FerroUI translations from unfinished behavior.
A deliberate architectural translation does not waive missing observable
behavior. File/member status and test evidence live in `mapping.json` and its
generated reports.

## Architectural translations

| ID | WinUI source/contract | Rust/FerroUI translation | Consequence to verify |
|---|---|---|---|
| RT-A01 | C++ dependency objects, COM wrappers, WinRT collections | Existing `Ref<T>` class model, styled/direct properties, shared collection handles, Rust errors and events | Matching property inheritance, observable mutation, object identity, markup access, lifetime and event order. |
| RT-A02 | LineServices plus DirectWrite text analysis and Direct2D rendering | Existing FerroUI formatter, Unicode/shaping/font platform contracts, and `DrawingContext` | Line-break decisions, fallback, glyph clusters, metrics, justification and typography need semantic checks; platform rasterizers can produce different pixels. |
| RT-A03 | `WinUIEdit.dll` and `Microsoft.UI.Text.winmd` | Rust document/editor implementation using the externally documented contract | The native engine source is absent from the requested repository; the public host is not a complete implementation. Every still-unimplemented API stays pending. |
| RT-A04 | Win32 windowless text host, TSF and UI-manager callbacks | FerroUI input-method client, dispatcher, focus, routed input and platform clipboard | Equivalent IME/preedit/caret/surrounding-text behavior must be verified per backend. |
| RT-A05 | Separate C++ headers/implementations and generated WinRT files | Paired sources may share a Rust module; markup/property registration replaces generated ABI plumbing | Every input stays in the map. ABI boilerplate is reference-only; observable members still need individual coverage. |
| RT-A06 | Windows Unicode classification tables | Reuse existing FerroUI Unicode tables and algorithms after version/behavior review | Unicode-version drift, locale-dependent word breaking and dictionary boundaries remain visible rather than assuming table equality. |

## Remaining parity boundaries

| ID | Boundary | Current disposition | Required closure |
|---|---|---|---|
| RT-P01 | Exact LineServices internals and `Ptls-min.h` | External implementation is unavailable in the reference repository. Existing formatting is a reuse candidate, not exact engine parity. | Port visible adapters by semantic role and validate line, hit-test, bidi, justification, trimming and incremental-break behavior. |
| RT-P02 | Full RichEdit text object model | Public XML documentation supplies an exact member inventory; implementation is external. | Implement and test every document/range/selection/format contract or record a reviewed explicit deviation. |
| RT-P03 | RTF features, OLE content, embedded objects, tables, math, advanced formatting | Must be tracked at the content feature and API member level as implementations arrive. | Lossless or explicitly specified behavior with imported/exported fixtures; never report discarded content as a complete rich-text import. |
| RT-P04 | Proofing, prediction, handwriting, candidate windows, touch grippers | Windows adapters are visible; their underlying service implementations are external. | FerroUI service providers and platform tests, including capability absence and cancellation/lifetime behavior. |
| RT-P05 | Accessibility Text/TextRange/annotation patterns | Existing general automation infrastructure is reusable; rich text pattern coverage requires implementation. | Document/selection ranges, geometry, attributes, movement, embedded elements, eventing and screen-reader validation. |
| RT-P06 | Exact native text appearance | Requires native reference captures with identical fonts, DPI and configuration. | Record comparisons separately from model/unit-test success; do not claim pixel parity from a headless build. |
| RT-P07 | Source/member extraction completeness | Public API identifiers are exact XML entries; internal C++ callable extraction is lexical and cannot resolve macros/conditional compilation. | Review all internal declarations before setting a source file complete. No count of lexical candidates is a parity percentage. |

## Implementation-specific differences

Add one row per behavior difference in the same commit that introduces or resolves
it. Cite the pinned source member, current Rust member, reason, and test. The
generated `PROGRESS.md` reports only reviewed mappings and does not infer closure
from file names.
