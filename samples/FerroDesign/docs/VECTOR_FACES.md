# Vector-network faces and paint bucket

FerroDesign prepares supported intersections in vector networks, discovers their
checked bounded faces, supports independent region paint stacks, and provides a
native paint bucket with hover preview and captured drag painting. The
authoritative geometry remains the network's vertices, edges and oriented region
contours. Intersection preparation subdivides line and cubic Bézier commands
without converting them into polylines; layer identity and world transform stay
unchanged.
The controls are defined in XAML; face adorners and artwork use public FerroUI
geometry, brush, clipping and `DrawingContext` APIs.

## Native authoring workflow

Select an unlocked vector and enter vector editing. Choose **Paint bucket**, or
use **Shift+B**, then move over a closed face to preview its coverage. Press and
drag to paint the faces crossed by the pointer. The stroke copies the complete
layer fill stack at press time, including paint values, opacity and visibility.
If that stack is empty, the bucket uses a neutral `#D9D9D9` solid for the painted
regions without adding a fill to the layer.

Hold **Shift before pressing** to clear faces instead. Paint versus clear is
fixed for the current stroke; changing Shift after the press does not repaint
previously visited faces with a different operation. Release commits one undo
step for all changed faces. **Escape** cancels the stroke and keeps vector
editing open. An unchanged stroke adds no history entry.

The related native actions have distinct purposes:

| Action | Result |
| --- | --- |
| **Fill all closed regions** | Prepares supported crossings, discovers every bounded elementary face, and fills the inherited topology. Existing matching regions and explicit paint overrides are retained. If inherited regions need a layer fill and the layer has none, this action adds the neutral fill in the same edit. |
| **Paint bucket** | Previews checked faces without changing the graph, then applies any required subdivision and copies the layer's current stack into explicit replacements for visited faces. Shift-drag creates explicit empty replacements. |
| **Region paints** | Opens the existing paint-stack editor for the active region. Edits target that region's stack, with the front paint first. |
| **Use layer fills** | Removes the active region's explicit paint override when its coverage can safely return to inheritance. The coverage rules below determine whether the region itself is retained. |

Open branches remain editable and stroked. They do not become artificial fill
boundaries, and pointing outside all bounded faces does not allocate a region or
apply a subdivision plan. Filling an intersecting open graph with no bounded
face also leaves its topology unchanged.

## Paint ownership and inheritance

`NetworkRegion::fills` is an optional complete paint stack. Existing documents
whose regions omit this field retain their previous compound-fill behavior.

| Stored value | Meaning |
| --- | --- |
| `None` / omitted `fills` | The region participates in the layer's combined inherited fill geometry and uses the current layer fill stack. |
| `Some(fills)` / a nonempty `fills` array | The region replaces earlier fill coverage with its own stack. Solid, linear, radial, angular and image paints use the shared `Fill` representation and validation. |
| `Some(Vec::new())` / `"fills": []` | The region is an explicit transparent replacement. It removes earlier fill coverage inside its footprint; it does not fall back to the layer paints or delete its contours. |

Inherited contours are evaluated together under the network's nonzero or
even-odd fill rule. Rendering them independently would change holes and overlap
behavior. Explicit regions replace that inherited coverage. Where explicit
regions overlap, the later stable region ID owns the footprint, even when its
stack is empty, hidden or fully transparent. This prevents inherited paint from
showing through a clear and prevents applying inherited alpha beneath an
explicit translucent replacement. Paint rows are drawn with the first row
topmost; the layer's opacity still applies to the resulting artwork once.

The region renderer retains original geometry for brush coordinates and applies
replacement coverage through geometry clips. A neighboring override therefore
does not redefine another region's gradient or image coordinate system. The
renderer has its own bounds index for pruning disjoint replacement footprints
and uses balanced geometry unions. This renderer index is separate from the
face-picking map described below.

**Use layer fills** uses integer winding propagation across the elementary-face
dual graph to examine earlier coverage. If earlier inherited or explicit
geometry covers the complete region, reset removes the redundant override. If
none of its footprint is covered, reset retains the region and changes its
stack to `None`. A composite region with both covered and uncovered portions is
rejected with `AmbiguousRegionReset`; the implementation does not change an
even-odd hole or duplicate coverage to guess at the intended reset. Clearing a
region, in contrast, retains its ID and contours.

Region paint edits leave the layer's fill-style and variable links intact.
Editing a component instance's authoritative network records the existing
`path` override so reconciliation preserves the region edit. The copied
explicit stack is a value snapshot; it does not keep inheriting later layer
paint changes. Regional image paints use the paint editor's controls; the
direct canvas image-crop tool remains a layer-fill operation.

## Automatic intersection preparation

The native fill and bucket paths use `NetworkFaceMap::prepare`. Already planar
networks retain their existing geometry. For an unsplit network, preparation
builds a separate candidate, finds supported contacts, subdivides its edges, and
requires the complete candidate to pass ordinary checked face discovery before
it can be used. Hover retains this candidate in memory; it does not reserve IDs
in the document or create history.

The preparation algorithm handles transverse line/line and line/cubic crossings,
bounded cubic/cubic intersections, and supported cubic self-intersections. A
control-hull sweep limits candidate pairs. Line/cubic intersections use the
curve polynomial's bracketed roots and polish them against Bézier evaluation.
Cubic pairs are recursively isolated; a common monotone projection and separated
derivative-slope intervals establish a unique transverse crossing in an accepted
pair. A bounded Newton solve then refines that root. Self-intersection discovery
compares monotone subpieces of the same curve. Exact end-to-end contacts can also
be welded when their control hulls are separable except at the endpoint,
including collinear continuations. Interior tangencies and overlapping runs are
not inferred into junctions.

Subdivision uses de Casteljau on the authored controls. The first subedge keeps
the original edge ID; subsequent subedges and shared intersection vertices use
the serialized monotonic allocator. Exact coincident endpoint vertices retain
the lowest existing ID and retire the duplicates. Distinct existing endpoint
coordinates are never snapped together. Each stored region reference expands in
its original traversal direction, including reversed references, while its
region ID, contour order, fill stack and replacement priority survive. Stroke
flags survive on every subedge. New junctions use Corner constraints; subdivision
relaxes an existing Symmetric handle constraint to Smooth only when valid, or to
Corner when the new junction requires it.

Root finding and shared-vertex coordinates use `f64`, not arbitrary precision.
Independent curve evaluations must agree within 256 machine epsilons in
normalized coordinate space. Root parameters within 128 machine epsilons are
candidates for duplicate-event checking, not automatic welding. Repeated solves
of the same curve pair merge only when the common parameter rectangle has a
unique-crossing derivative-cone certificate. Events involving three different
lines require an exact concurrence predicate on the original authored
coordinates. That fixed-degree predicate uses floating-point expansions with
power-of-two scaling and checked exponent bounds; it does not replace the other
curve calculations with arbitrary-precision geometry. Unproved nearby roots
reject atomically instead of collapsing a narrow face into one junction.

These checks allow bounded evaluation round-off without treating proximity as
permission to merge topology. The limits are unrelated to screen pixels or
zoom. Near contacts that cannot satisfy these bounds and the final embedding
certificate return an error. Higher-order curve junctions without a supported
duplicate-event certificate can also be declined. This is a checked subset of
curve arrangements, not a claim that every cubic degeneracy can be prepared.

`VectorNetwork::planarize_intersections` exposes the topology operation directly.
`fill_prepared_faces` and `paint_prepared_face_at` combine it atomically with the
requested authoring action. The strict `detect_faces`, `fill_all_faces`, and
`paint_face_at` APIs remain available for callers that require an already planar
network and want an error instead of automatic topology changes.

## Checked topology and original curves

`VectorNetwork::detect_faces` validates the authoritative network and its planar
embedding before returning read-only faces. Supported geometry includes
straight and cubic edges, parallel curves, simple self-loop curves, shared
junctions, implicit unstroked closing edges, disconnected components and nested
components.

The geometric check normalizes coordinates and uses Bézier convex-hull
separation. A monotone coordinate certifies an individual curve piece as simple;
other curves are subdivided for certification. Pairs of control hulls must be
separable or meet only at an explicitly shared vertex. Subdivision is temporary
analysis data: returned contours retain the original edge references and control
point values. A sweep over control-hull bounds reduces ordinary pair comparisons.

Iterative bridge discovery excludes open branches from face traversal without
removing them from the document. Outgoing directed edges are ordered by endpoint
tangents. Walking the predecessor of the reverse edge at each junction keeps a
face on the same side of the walk. Signed area is integrated from the original
Bézier polynomial. Positive walks describe bounded faces; a negative walk from
a nested component becomes a hole in the smallest containing face. The resulting
oriented contours support both fill rules.

Hole ownership and point containment use curve-polynomial winding. Derivative
roots partition each curve into monotone intervals; bracketed ray intersections
use half-open endpoint rules to count shared vertices once and cancel tangencies.
Query-relative normalization supports the tested micro-scale curves and narrow
curved annuli without consulting the document's flattened hit-test path or an
absolute flattening tolerance. Calculations use floating-point arithmetic;
unresolved separation or orientation is an error, not a claim of arbitrary
precision.

For example, nested squares produce an annular face and a separate inner face.
Filling both covers both areas, while painting only the annular face keeps its
hole. A square divided by a diagonal produces two faces that reference the shared
edge with opposite orientations.

## Cached hover and pointer-segment queries

`NetworkFaceMap::new` runs strict checked discovery and retains elementary faces,
their exact paths, and a balanced median-partitioned bounding-volume hierarchy
(BVH). Its bounds come from the original curves' control hulls, so a narrow curved
face cannot be discarded because an approximate flattened bound missed it. The
map keeps a geometry snapshot of vertices, edges and the fill rule; it does not
copy existing regions' potentially large paint or embedded-image payloads.
`NetworkFaceMap::prepare` adds an optional certified subdivision plan. Preparing
or querying either map allocates no IDs in the authored network.

`NetworkFaceMap::face_at` prunes that BVH and evaluates original-curve winding for
candidate faces. `path` exposes the retained contour path for the hover adorner.
The native canvas caches a prepared map or its discovery error with the selected
node and editor render token. On a changed token it can retain a successful map
when `matches_source` confirms either the original graph and allocator or the
applied prepared geometry. A region paint change is retained when a plan is
applied; preparation never copies cached paints back into the document. A new
allocation before applying a plan invalidates it so provisional IDs cannot
collide with an intervening edit. `matches_geometry` continues to identify the
already prepared geometry alone.
Continuing bucket hover/painting after a paint-only edit therefore does not
repeat planar-embedding certification. Geometry changes require a new map;
stale mapped paint calls are rejected with `StaleFaceMap`.

Hover converts the pointer through the current inverse world transform, selects
one face and retains its FerroUI `Geometry`. Public `DrawingContext` transforms,
brushes and a screen-scaled outline draw the preview, including holes, in the
editor overlay. Moving within the same face and paint/clear mode reuses that
geometry. Hover changes neither the document nor its allocator, revision or
history, and the adorner is excluded from artwork export. Invalid topology
produces no repeated hover notices; a paint attempt reports its diagnostic once.

`NetworkFaceMap::faces_along_segment` traces the straight segment between
delivered pointer positions. It gathers candidate boundary edges through the
BVH, solves intersections between the pointer line and each original curve
polynomial, sorts the segment parameters, and queries interior samples between
consecutive crossings. Endpoint queries are included. This finds narrow faces
between pointer events without fixed pixel-step sampling. The method returns
one sample per distinct face in traversal order; the gesture also keeps a
visited-face set so revisiting a face does not repaint it.

## Identity, transactions and persistence

After a network is planar, paint-only edits leave its vertex and edge IDs,
controls and stroke visibility unchanged. A required preparation uses the
subdivision and welding rules above in the same authoring edit; layer bounds and
world transforms do not change. Matching regions
keep their IDs and serialized contour order, including matches recognized after
a global reversal of the contours. New explicit faces use the existing monotonic
network allocator. **Fill all closed regions** also retains explicit composite
overrides and their priority rather than discarding them when filling the
underlying elementary topology.

The core API exposes `fill_network_faces`, `paint_network_face`,
`set_network_region_fills`, and the cached-map variants
`paint_mapped_network_face` / `paint_mapped_network_faces`. Editor paint updates
operate on a cloned node and publish only after successful preflight. The batch
method validates once and makes one retained node update for all newly visited
faces in a pointer segment. An allocator failure partway through that batch
publishes none of it. Failed core preflight and direct no-op edits preserve the
document, allocator, history and redo state.

The native bucket opens a scoped node transaction before painting the pressed
face. Its first successful paint applies any required prepared geometry inside
that transaction. Each subsequent segment reuses the same prepared map and
updates that same transaction. Release commits all changed faces and subdivision
as one undo step; an unchanged gesture preserves redo and adds
no history entry. Normal transaction finalization can still refresh internal
render revisions even when it creates no authored history entry.

Escape and native pointer-capture loss roll back the owned stroke, including
subdivision, welded vertex identities, paint stacks and allocator. Undo or Redo
during a stroke cancels that preview before touching earlier history. The canvas
checks the editor transaction identity and render token, selected node, page,
tool, visibility, lock state and affine transform before continuing. Reconciliation
cancels a no-longer-current owned gesture while preserving a newly chosen
selection/page. A stale gesture cannot resume writing or commit/cancel a newer
unrelated transaction. Ordinary shell commands finish the stroke before applying
their separate operation; direct canvas mode changes also reconcile or cancel
the pending gesture.

Pointer hover is read-only; a held paint stroke is a cancellable document preview.
`CanvasEvent::Draft` marks the active stroke in the browser host's existing draft
state, combined with inline-text and inspector drafts. Background autosave
serialization defers while the stroke is active. Release or cancellation resumes
any previous pending save, including when the bucket itself was a no-op. This
matters when the earlier edit's one-shot 220 ms save timer expired during capture.

At an explicit lifecycle boundary, the host's draft committer invokes the native
`commit-edit` command. That command finishes the stroke before inspector commits
and document serialization. The pagehide path can therefore commit a stroke
started from an already saved document, and a later mouse release cannot add a
second history entry. IndexedDB durability is completed by the host's existing
save transaction; no document JSON is serialized for each pointer movement.

## Limits and failure behavior

Strict discovery requires crossings to be represented by shared vertices;
native fill and bucket actions first prepare supported crossings as described
above. Coincident or overlapping segments, unresolved contacts, interior
tangencies and coincident outgoing tangents are declined. Very close contacts,
ill-conditioned roots, singular curves and nearly
zero-area boundaries can also be declined when certification cannot establish
reliable separation or orientation. Authored geometry remains available for
editing after the error.

| Guard | Current limit or behavior |
| --- | --- |
| Discovery size | At most 32,768 edges. |
| Prepared size | At most 32,768 collected intersection events and 32,768 resulting edges; repeated isolation hits count against the event limit. |
| Intersection preparation work | A separate 1,000,000-unit budget covers pair enumeration, subdivision, collected hits and assembly. Region-contour expansion has its own 1,000,000-unit guard. |
| Geometric discovery work | At most 1,000,000 counted work units per discovery. |
| Subdivision | At most 40 levels for curve/pair isolation and for final embedding certification. |
| Pointer-segment tracing | A separate 1,000,000-unit budget includes candidate traversal and checked face containment. |
| Cached batch painting | At most 32,768 supplied sample points, with a separate 1,000,000-unit checked-query budget. |
| Ambiguous inheritance reset | Returns `AmbiguousRegionReset` and preserves the region instead of partially changing its coverage. |
| Gesture error | Rolls back the current owned stroke and reports the failure once. |

The bounds trees reduce work for spatially separated candidates. They do not
make heavily overlapping bounds or adversarial geometry constant-time, and
paint updates still clone the edited node for atomic publication. The budgets
are operation limits, not frame-time guarantees. Segment tracing follows the
straight path between input events; it cannot reconstruct motion the platform
did not deliver. The current bucket paints the selected editable network rather
than searching across arbitrary layers.

## Regression coverage and source map

| Source | Covered behavior |
| --- | --- |
| [`vector_faces_tests.rs`](../crates/ferrodesign-core/src/vector_faces_tests.rs) | Adjacent cells, nested/disconnected holes, original cubic geometry, micro annuli, bridge exclusion, rejected crossings, stable IDs, affine transforms, idempotent 16-by-16 cells, immutable cached maps, subpixel intermediate faces, stale maps and atomic batch allocator exhaustion. |
| [`vector_planarize_tests.rs`](../crates/ferrodesign-core/src/vector_planarize_tests.rs) | Read-only preparation; transverse line/cubic combinations; cubic self-intersections and three roots; shared junctions and exact endpoint welding; preserved Bézier controls, stroke flags and reversed painted contours; atomic overlap/tangency and allocator failure; stale plans; affine geometry; native core fill/paint history and outside no-ops. |
| [`vector_region_paints_tests.rs`](../crates/ferrodesign-core/src/vector_region_paints_tests.rs) | Legacy JSON, explicit empty stacks, replacement priority, inheritance reset, composite-reset rejection, paint validation, locks, exact history, styles, component overrides and preserved explicit paints during fill-all. |
| [`network_bucket_tests.rs`](../crates/ferrodesign-app/src/canvas/network_bucket_tests.rs) | Reused hover preparation and actual native bitmap pixels, affine fast drags, one-step history/no-ops, clear, Escape, capture loss, selection changes, transaction ownership, tool changes, cached overlap failures, and cubic preparation canceled or committed with its stroke. |
| [`region_tests.rs`](../crates/ferrodesign-app/src/renderer/region_tests.rs), [`region_scaling_tests.rs`](../crates/ferrodesign-app/src/renderer/region_scaling_tests.rs), [`svg_regions_tests.rs`](../crates/ferrodesign-app/src/renderer/svg_regions_tests.rs) | Native alpha replacement, transparent cutouts, paint order, layer opacity, masks, incremental/fresh pixel equivalence, disjoint-region pruning and balanced geometry depth, and regional SVG round-trip pixels. |
| [`network-browser.mjs`](../tests/network-browser.mjs) | Compiled XAML actions and native canvas input, visible discovered faces, open branches, automatic crossing subdivision with original first-edge IDs, exact undo/redo, and fresh exact-error rejection of ambiguous overlapping edges. |
| [`region-paints-browser.mjs`](../tests/region-paints-browser.mjs) | Native region paint controls, stack editing, reset/clear behavior, exported artwork, exact history and regional image-target isolation. |
| [`network-bucket-browser.mjs`](../tests/network-bucket-browser.mjs) | Native hover and a three-face drag with an intermediate face between events; exact undo/redo and no-op history; Shift clear and Escape; clean-baseline pagehide commit and late release; unchanged-stroke autosave resumption after the debounce deadline; cubic crossing preparation with cancel/undo allocator restoration; exact PNG dimensions, painted colors and transparent unpainted faces without hover adorners. |

Implementation entry points are
[`vector_faces.rs`](../crates/ferrodesign-core/src/vector_faces.rs),
[`vector_planarize.rs`](../crates/ferrodesign-core/src/vector_planarize.rs),
[`canvas/network_bucket.rs`](../crates/ferrodesign-app/src/canvas/network_bucket.rs),
[`renderer/regions.rs`](../crates/ferrodesign-app/src/renderer/regions.rs), and the
native shell's draft/save handling. Execution status, renderer modes and
source-specific evidence belong in [VALIDATION.md](VALIDATION.md); the coverage
listed here is not a hardware throughput benchmark.

The nearby-root regression checks three lines at one exact shared junction and
three distinct intersections separated by approximately 1e-14, at two scales
and both signs. The former succeeds; the ambiguous tiny faces must reject with
unchanged topology, fill state, allocator and history.
