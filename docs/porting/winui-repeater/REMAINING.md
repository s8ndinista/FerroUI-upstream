# Repeater acceptance work

The goal is complete portable behavior of the pinned Repeater family using
native FerroUI services. [SOURCES.md](SOURCES.md), [API.md](API.md) and
[TESTS.md](TESTS.md) keep the scope explicit; [VALIDATION.md](VALIDATION.md) keeps
the executed evidence separate.

## Work being closed in this continuation

The continuation review identified lined-flow outer-width constraints, partial
realized rows, partial metadata fallback/range expansion, timer shutdown,
detached-context invalidation and distant-cache trimming. Each correction needs
a regression that fails for the previous behavior and the corresponding source
or framework decision recorded in [DEVIATIONS.md](DEVIATIONS.md).

Runtime/compiled XAML must exercise keyed template dictionaries and native
control/interface unboxing, and the native catalog must demonstrate the
registered repeater/layout/template path. The previously undeclared flow-layout
upstream test module is now declared and must actually compile and run.

## What remains after name and file mapping

1. Review every unresolved IDL row and scenario alias for its actual signature,
   default value, callback ordering, exception boundary and lifetime. A matching
   Rust name cannot establish overload parity or prove that a scenario's setup
   and assertions are equivalent.
2. Execute the complete Repeater suite and affected framework/markup suites on
   the final branch revision. Record failures and their disposition without
   silently excluding inconvenient tests.
3. Review native scrolling, accessibility, reduced-motion and frame-lifecycle
   behavior on the relevant host backends. Preserve explicit distinctions
   between headless portable tests and native platform integration.
4. Compare representative layouts and dynamic mutation sequences with the
   pinned WinUI behavior. Exact visual parity and large-source performance need
   measured workloads and recorded output; neither follows from API counts.
5. Keep exact WinUI sample-page ports separate from control/runtime support.
   The 302-file inventory includes TestUI pages and assets even when a native
   catalog example covers the same control family.

The generator reports missing mappings rather than waiving them. A deliberate
framework replacement must name its concrete implementation and rationale.
Known ignored upstream product concerns remain visible, with an explicit note
when the port preserves the pinned behavior instead of the ignored test's
desired result.
