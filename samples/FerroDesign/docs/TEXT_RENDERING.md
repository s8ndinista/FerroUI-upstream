# Text rendering and browser observation

FerroDesign submits shaped text through public FerroUI text and drawing APIs.
The browser application uses the Vello Software2D or WebGL2 surface; it does not
select Skia or paint the editor with HTML controls.

## Translation reuse for unhinted outlines

An immutable Vello glyph run now owns a bounded cache for the flattened contours
of unhinted glyphs. The key contains the glyph index and the exact linear part of
the drawing transform. Translation reuses the prepared contour; scale, rotation
and shear use distinct entries. Each run retains at most 256 entries and 128 KiB
of path elements. Over-budget contours retain only a negative cache entry, so
they use the existing curve path without repeating speculative preparation.

The existing 0.05 curve tolerance is applied after the linear transform. Brush
placement, clipping and each glyph's source-over operation remain independent.
In particular, translucent overlapping glyphs are not merged into one fill.
The public scene-sink default still receives canonical curves; CPU, WebGL2,
hybrid and GPU sinks opt into the reusable preparation only for antialiased,
unhinted contours. Recording and custom sinks retain the original precision.

Tests cover translation reuse, transform/glyph misses, eviction, oversized
contours, canonical path precision, and rendered pixels under fractional affine
transforms, clips, translucent gradients and image brushes. Added assertions
are not a claim that their current source has completed CI; consult the exact
commit's native feedback and full validation runs.

## Browser pan benchmark version 2

`tests/text-render-benchmark.mjs` retains the seeded 150-label workload, twelve
warmups and 36 measured real pointer motions per surface. Its timed polling uses
the constant-size, read-only `ferroDesignFrameState()` bridge. It records host
dispatch, native frame observation, native presentation observation and the two
subsequent animation callbacks separately. Bridge duration/payload/poll counts,
bounded trusted input timestamps and browser long tasks are reported alongside
the timing distributions. Compact and full state are compared outside the
timed interval, and every measured motion must advance the expected camera and
native frame/presentation counters.

These observations include automation and browser scheduling. A presentation
counter is not a GPU-completion timestamp, and the benchmark requests
SwiftShader. It makes neither an FPS assertion nor a physical-GPU throughput
claim. Historical builds without the compact bridge are labeled as using the
legacy measurement method, and comparisons across those methods are suppressed.
The previous 1,366.94 ms WebGL2 p95 remains in the historical evidence; a cache
change alone does not establish its cause or demonstrate an improvement.

## Inline text-entry diagnosis

The typography scenario still performs one original two-press gesture with an
80 ms delay and the original 30-second readiness deadline. It checks the actual
native inline control's focus, layout and a published scene/frame. A fixed-size
native pointer ring and bounded browser observations record timestamps, click
counts, buttons and camera state on success or failure. They do not alter click
recognition or retry the gesture. The earlier WebGL2 failure remains a failure
until a new exact-source browser run establishes otherwise.
