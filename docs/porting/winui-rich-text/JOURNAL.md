# Rich text implementation journal

This journal records how the port is developed. [PROGRESS.md](PROGRESS.md) and
the reviewed mapping remain the authority for implementation status; a checkpoint
preserves work and does not establish complete parity.

## Reference and first published batches — 2026-10-17

The source reference is fixed at
[`fe244ca6ac4d482712b234a9de3cf771cc6481de`](https://github.com/microsoft/microsoft-ui-xaml/tree/fe244ca6ac4d482712b234a9de3cf771cc6481de).
The inventory retains all 230 files in `core/text`, 62 in `core/native/text`, and
23 in `components/text`. Repository inspection established that the public source
contains the RichEdit host and LineServices adapters, while the implementations
of `WinUIEdit.dll`, the external text object model, and the line engine behind
`Ptls-min.h` are external. Those boundaries are documented in [DEPENDENCIES.md](DEPENDENCIES.md).

The initial tracking batch was published as
[`656509591a045f50abb86807da7ecf11ad6375ce`](https://github.com/wieslawsoltes/FerroUI/commit/656509591a045f50abb86807da7ecf11ad6375ce).
It retained 609 source/reference files and 1,079 exact public XML identifiers.
Its reviewed map was empty; no runtime or parity success was asserted.

The core document, display/overflow, editor, RTF/clipboard, input-method and Rust
text object model batch was published as
[`432d10fb0335be3fb586290bf7467126a790646f`](https://github.com/wieslawsoltes/FerroUI/commit/432d10fb0335be3fb586290bf7467126a790646f).
The follow-up
[`fb15fa7bf618e21233edaf567a9ed0c0a5096e74`](https://github.com/wieslawsoltes/FerroUI/commit/fb15fa7bf618e21233edaf567a9ed0c0a5096e74)
corrected the hyperlink test to follow the source's Enter-on-key-release behavior.
Those published test files contain 21 document, 37 editor and 16 view test
definitions. Their presence is separate from the command/result records in
`mapping.json`.

## Workspace reconstruction and expanded audit — 2026-10-18

The execution environment disconnected before later work was checkpointed.
The restored environment did not contain that working tree. Work resumed by
cloning the published branch and the pinned source again. Later implementation,
mapping and documentation are reconstructed and reviewed against the actual
restored files; they are not represented as byte-for-byte recovery of unpublished
files. Previously observed local totals are not reused as current completion
claims.

The next inventory revision makes the block declaration, caret browsing,
hyperlink peer and text-options dependencies explicit. It also adds the native
UI Automation wrapper implementation/header, the generated native enum reference,
and the three automation model inputs. The public inventory now includes the
Text/Text2/TextRange/TextRange2/TextEdit/TextChild interfaces, their range units and
selection policy, text-attribute and annotation types, and text-specific pattern
and event dispatch members. Unsupported patterns and attributes stay pending;
expanding the denominator does not implement them.

The map is updated in implementation batches. Each reviewed entry names the
actual Rust symbol, its test evidence, and any observable translation. Runtime
test results are recorded only after their commands finish. Native Windows
execution, real input-method sessions, screen-reader interaction, exact rendering,
and external-engine contracts remain separate acceptance gates in [PLAN.md](PLAN.md).

## Continued source review and service contracts — 2026-10-19

The published branch was recovered at `2489efdcc60229a78eadbb52bcf4bc07bc5a3f86`
in an isolated checkout. Its GitHub CI failures were traced to a missing unsafe
block around an owned-BSTR COM test call; the repair was published as
`1b4704b2f96ac844dbfbe48b96fa5621e12e3ed3`.

Paragraph spacing and signed indentation were published as
`32b19d14facbf06a94c9715f1120d8e2a5b75a90`. This batch adds native-valued spacing
rules, point conversion, actual line/overflow behavior, signed indent storage,
RTF multiple spacing and eleven independent regressions. The enum values have
complete member mappings; the broader paragraph/native formatter contracts do
not inherit that completion status.

The document-service, edit-event and accessibility batch was published as
`871d566d25aab3e89021d7fc5db6edf32c07267e`. It adds shared default formats,
explicit get/set/find flags, native newline fixture assertions, UTF-32 and range
operations, cancellable/reentrant selection, synchronous TextChanging/deferred
TextChanged, framework content presenters, Text2 caret and TextChild hyperlink
providers. The integrated rich-text filter passed all 181 tests. Initial fixture
failures were corrected by using the existing per-thread test scope, installing
the framework keyboard device for focus tests and performing layout before
inspecting the automation visual tree. Production behavior was not weakened to
make those assertions pass.

The Win32 text-related filter subsequently passed 34 tests, including ten text
pattern/range bridge cases. Its first compilation caught a pattern-array arity
mismatch after TextChild was added; `f1a051f8eaddd2d9b651a731206eebd9fa3a252a`
repairs the complete twelve-interface registration. These runs occurred on a
Linux host using the portable Win32 test harness; they are not Windows
screen-reader or real native accessibility-client execution.

The mapping audit also found three historical test symbols absent from the
published sources. The ellipsis entry now points to an existing executed test,
and two absent scroll-test references were removed pending real regression
coverage. The generator checks every retained target and test symbol. No
unpublished test definition is treated as recovered or executed evidence.

Further source review established that real templates require RichEditBox to use
FerroUI TemplatedControl with a persistent rich presenter, not inherit the display
control's finite-height pagination behavior. It also distinguished display
batching from model notification batching, and native tab stops from the current
formatter's fixed-width tabs. These are implementation tasks, not stored-property
substitutes. The next batch preserves one document/editor while completing the
template/presenter, scrolling and stream-service boundaries.
