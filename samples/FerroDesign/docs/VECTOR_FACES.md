# Automatic vector-network faces

In vector editing mode, **Fill all closed regions** discovers the bounded faces
of the layer's planar network and fills each face with the existing layer paint
stack. If the layer has no fills, the native action adds one neutral solid fill.
The complete operation is one undoable edit. Open branches remain editable and
stroked; they do not become artificial fill boundaries.

This builds on explicit network regions. It does not introduce a second vector
representation, change layer identity, or replace curves with polylines.
`VectorNetwork::detect_faces` returns regions containing stable, oriented edge
references; `VectorNetwork::face_at` selects a bounded face by a node-local
point. Clients can inspect those read-only results before choosing regions.
`VectorNetwork::fill_all_faces` replaces the current region selection with every
discovered bounded face, and `Editor::fill_network_faces` supplies scoped history.

## Topology and geometry

Discovery first validates the authoritative network and checks its embedding.
The supported geometry is straight edges and cubic Bézier edges, including
parallel curves, simple self-loop curves, shared junctions, implicit unstroked
closing edges, disconnected components and nested components.

The geometric check uses normalized coordinates and Bézier convex-hull
separation. A monotone coordinate certifies an individual curve piece as simple;
other curves are subdivided for certification. Pairs of control hulls must be
separable or meet only at an explicitly shared vertex. Subdivision is temporary
analysis data. The returned fill contours retain the original edge and control
point values exactly. A sweep over control-hull bounds reduces ordinary pair
comparisons.

After geometric validation, iterative bridge discovery removes open branches
from face traversal without removing them from the document. Outgoing directed
edges are ordered by their endpoint tangents. Walking the predecessor of the
reverse edge at each junction keeps one face consistently on the same side of
the walk. Signed area is integrated from the original Bézier polynomial rather
than from sampled points. Positive walks describe bounded faces; a negative
walk belonging to a nested component becomes a hole in the smallest containing
face. Contour orientation supports both nonzero and even-odd fill rules.

Hole ownership and `face_at` use curve-polynomial winding directly. Derivative
roots partition each curve into monotone intervals; bracketed ray intersections
use half-open endpoint rules to count shared vertices once and cancel tangencies.
Query-relative normalization preserves micro-scale curves and narrow curved
annuli. These operations do not use the document's approximate flattened hit-test
path or an absolute flattening tolerance.

For example, two nested squares produce an annular face and an inner square
face. Filling all faces fills both. A client that selects only the annular face
through `face_at` retains the actual hole. A square divided by a diagonal
produces two faces whose shared edge has opposite orientations.

## Identity, history and failure behavior

Vertex and edge IDs, control points, stroke visibility, layer bounds and world
transforms never change during face authoring. Existing matching regions keep
their IDs and serialized contour order; matching also recognizes a global
reversal of every contour in one region. Newly discovered faces allocate IDs
from the existing monotonic network allocator. A second invocation over the
same filled topology is a no-op.

Geometry discovery and all ID reservations happen before mutation. Invalid
network data, exhausted IDs, unresolved geometry and no-op calls preserve the
document, selection, revision and redo state. The native command displays the
diagnostic before opening an edit scope, so it cannot leave a pending
transaction attached to subsequent input. Creating a default fill also uses
the normal property-edit boundary to detach a conflicting empty fill style.

## Explicit limits

Every crossing must already be represented by shared vertices. Automatic
curve-intersection splitting and welding are not part of this operation.
Crossing edges, coincident or overlapping segments, unwelded contacts and
coincident outgoing tangents are declined. Very close contacts, singular curves
or nearly zero-area boundaries can also be declined when geometric separation
or orientation cannot be established reliably. Their authored geometry is
preserved for further editing.

Discovery accepts at most 32,768 edges, spends at most one million geometric
work units, and subdivides a certification pair at most 40 times. Reaching a
limit returns a diagnostic before any document change. The broad phase does
not make adversarial overlapping control hulls linear-time. Face discovery is
an explicit authoring action, not work repeated on each render or pointer move.

All regions continue to use their layer's fill stack. Separate region paints,
automatic intersection topology, and a dedicated point-and-click paint-bucket
tool remain additional work. These limits are separate from the implemented
read-only face-at-point query.

## Regression coverage

The core tests in `vector_faces_tests.rs` cover adjacent cells, exact shared
edge orientation, disconnected and nested holes, opposite source winding,
bridges and dangling trees, articulation vertices, cubic lenses and self-loops,
unstroked closing edges, crossings and contacts, invalid/degenerate/exhausted
networks, stable existing region IDs, affine reflection/shear/translation,
idempotent 16-by-16 cells, locks and exact undo/redo.

`tests/network-browser.mjs` exercises the actual compiled XAML button and native
canvas, verifies pixels in two discovered faces, checks that an open branch
stays outside all fill boundaries, and checks that an unsplit crossing leaves
the preceding redo transaction available. Added tests are test coverage; their
execution status belongs in the source-specific validation report.
