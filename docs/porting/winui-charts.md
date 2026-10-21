# WinUI Charts port

## Scope and provenance

This work tracks the Charts implementation referenced by
[microsoft/microsoft-ui-xaml discussion 12343](https://github.com/microsoft/microsoft-ui-xaml/discussions/12343)
and its adaptation to FerroUI. The exact upstream implementation, revision, license,
and dependency inventory are being audited before implementation. A discussion
link alone is not evidence that a control ships in Microsoft WinUI.

The port follows [PORTING-GUIDE.md](PORTING-GUIDE.md): retain observable upstream
behavior, use safe idiomatic Rust, reuse FerroUI services, preserve source
attribution, and track every deliberate adaptation or unimplemented member.

## Implementation sequence

1. Pin the upstream source and enumerate source files, public types, members,
   resource dictionaries, services, tests, and dependencies.
2. Map dependency objects, collections, drawing, text, input, animation, themes,
   accessibility, and markup to the existing FerroUI APIs.
3. Port data models and chart layout/rendering algorithms with behavior tests.
4. Implement native FerroUI controls, properties, input, accessibility, and XAML
   integration; avoid dependencies on a specific rendering backend.
5. Add executable samples and integration tests; verify the target platforms
   supported by the available build environment.
6. Record the final source-to-Rust mapping, verification evidence, deviations,
   and any remaining boundaries in this document.

## Progress

| Area | State | Evidence |
| --- | --- | --- |
| Upstream source audit | In progress | Discussion linked above |
| FerroUI API mapping | In progress | Existing control libraries and porting guide |
| Chart implementation | Pending | No implementation claim yet |
| Integration and verification | Pending | Commands and results will be recorded here |

## Work log

- 2026-10-21: Created the port branch from FerroUI `main` at
  `86b6b4ab41669bf830fba681504a4ee94752a4e6`. Opened this tracking document before
  implementation so the draft pull request can track incremental commits.
