# AvaloniaEdit and TextMateSharp port

This document tracks the Rust editor and TextMate port, its upstream inputs, behavioral evidence, and remaining work. It follows [PORTING-GUIDE.md](PORTING-GUIDE.md). A mapped API name or passing smoke test is not proof of complete behavioral parity.

## Pinned references

| Repository | Revision | License |
|---|---|---|
| [AvaloniaUI/AvaloniaEdit](https://github.com/AvaloniaUI/AvaloniaEdit) | `7277e36366d754219d7ede16b6613786b32caf4c` | MIT; Eli Arbel and the original AvalonEdit contributors |
| [danipen/TextMateSharp](https://github.com/danipen/TextMateSharp) | `e0db45be30278c0f3f80302c31f498c8f8f1c4f2` | MIT; Daniel Peñalba and the upstream TextMate implementations |
| FerroUI starting point | `d5405346e680e060c9675572efe02df131428c08` | MIT |

The corresponding `NOTICE.md` files preserve the original notices. Grammar and theme resources retain their individual licenses.

## Architecture and compatibility contracts

| Package | Responsibility | Framework reuse |
|---|---|---|
| `ferroui-edit-core` | UTF-16 document, snapshots, anchors, undo, selections, search, folding, completion, snippets and highlighting | Framework-independent Rust data and services |
| `ferroui-textmate` | Grammar registry, Oniguruma matching, multiline rule stacks, captures, selectors, themes and incremental tokenization | No UI dependency; usable by other FerroUI applications |
| `ferroui-edit` | `TextEditor`, viewport, input, selection, margins, styling and markup integration | FerroUI property system, controls, drawing/text layout, routed input, clipboard and IME |

Offsets and columns at public editor boundaries count UTF-16 code units, matching both upstream APIs and FerroUI text layout. Lines and logical columns are one-based. Rust UTF-8 indices are converted explicitly at regex and string boundaries. A document is a shared handle; snapshots retain immutable storage. User callbacks run outside internal mutable borrows. Edits, anchor movement and undo use one document change contract.

The UI is a FerroUI control rather than a separate platform renderer. Grammar and document services remain independent of renderer/backend selection. The core must not assume native filesystem access, threads or a wall clock.

## Work and evidence

The working pull request is opened before implementation. Incremental commits add the document model, editor services, TextMate engine, framework control and conformance evidence. Source inventory and explicit mapping distinguish implemented, adapted/reused and outstanding behavior; unsupported members are omitted instead of represented by placeholder implementations.

Current stage: implementation in progress. Full upstream parity has not yet been established. Validation results and the exact remaining boundaries will be recorded as each subsystem lands.
