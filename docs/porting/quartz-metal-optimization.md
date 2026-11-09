# Quartz Metal batching and native text

The shared macOS/iOS renderer uses Core Text as its text-quality reference. The
optimizations retain the existing native raster inputs, four-sample path coverage,
clips, blend modes and GPU media imports. They introduce no Skia, HarfBuzz or wgpu
dependency into the native application configuration.

The [measured results and validation](quartz-optimization-results.md) compare this implementation
with Skia and preserve the remaining performance gaps. The latest
[opacity recording results and size](quartz-optimization-round4-results.md) retain
the paired controls, native quality checks and remaining gap.

## Submission and presentation

Common source-over paints use Metal render passes with premultiplied hardware
blending. Compatible draws share an encoder across texture/pipeline changes.
Simple images use compact, precomputed quad inputs. Other blend modes, gradients
and filters retain the compute implementations and their ordering dependencies.

On Apple GPUs, working surfaces use BGRA8. Compatible platform drawables can be
rendered directly, with the same non-retained-frame contract as the Skia Metal
target. A drawable lacking required shader access moves to private storage before
that access. Unsupported formats, framebuffer-only drawables and flipped targets
retain the private-frame presentation path. Intel/AMD keep RGBA working storage.
Apple documents the applicable texture capabilities in its
[Metal feature tables](https://developer.apple.com/metal/Metal-Feature-Set-Tables.pdf).

Rendering uses the platform Metal queue where available. Changing queues orders
the transition with a shared event; presentation retains the event bridge when
queues differ. GPU errors still invalidate the renderer generation. Upload buffers
are reused only after their previous command buffer completes.

## Native glyphs and geometry

Core Text still produces the hinted/color raster inputs. Exact white masks can
be represented losslessly by an R8 atlas; color and subpixel inputs retain their
full channels. Atlas filtering observes source-rectangle boundaries, including
cubic taps. Tint, clipping and composition remain GPU work.

Prepared glyph placements are cached by owned glyph/position bytes, face,
transform, paint, viewport and clip bounds. A full-content discriminator prevents
Foundation data-key hash collisions; equality still checks the complete key.
Non-atlased glyph surfaces are excluded from this persistent placement cache.
Font-size lookup moves out of the per-glyph loop. Text shaping caches immutable
font configuration, not text or shaping results.

Immutable geometries and uniform rounded rectangles retain native paths. GPU
coverage masks use R16Float, which exactly represents the renderer's quarter-step
coverage values. Masks are independent of paint and clip. Their atlas and retained
path lookup avoid repeated flattening, rasterization and texture switches. Filtered
box shadows reuse GPU images keyed by geometry, transform, color and generation.

## Isolated groups and ownership

Allocation hints reserve small groups in padded atlas regions. Actual drawing
outside a hint grows/detaches the group and preserves its earlier pixels. Nested
or large groups use independent storage. Filters and snapshots promote a cropped
group to its complete logical canvas before operations that need those pixels.
Non-source-over composition preserves the effect of transparent pixels outside
the painted region.

Independent group passes precede the drawable pass, whose commands retain their
original order. Snapshot, compute, blit and readback dependencies drain recorded
drawable work. Deferred group views retain their page owner until encoding;
pooled storage cannot be reused while a view or snapshot still owns it.

Budgets are per cache, not a process-memory limit: 16 MiB for glyph atlas pages,
16 MiB for prepared runs, 64 MiB for coverage atlas pages and keys, 32 MiB for
cached shadows, 16 MiB for upload buffers and 64 MiB for pooled render textures.
A drawing context uses at most four 1024² layer-atlas pages. Larger input bitmaps,
active effects, in-flight work and metadata require additional memory. Presentation,
dependencies and bounded command/storage budgets submit work.

## Validation and measurement

The shared [benchmark harness](../../tests/FerroUI.Rendering.Benchmarks/README.md)
keeps the same workloads, resolution, font bytes, hinting and completion fence for
all adapters. It measures warmed full-redraw throughput; it does not establish
application FPS, cold startup, driver-removal recovery or physical iOS performance.

Regression tests cover nested/reused layers, hints that are too small, drawable
batch order, compute fallback without readback, forced glyph-atlas eviction and
native packed-versus-unatlased pixels. The latter compares nearest, linear,
mip-enabled and cubic sampling under scaling, rotation and mirroring, allowing
at most one channel value of resampling error. Existing effects, media, font,
format, clipping and recovery checks also run with separate destination textures.

The [original performance report](quartz-performance.md) records the starting
point and the common measurement method.


## Second optimization pass

The [matched measurements and shipping-size checkpoint](quartz-optimization-round2-results.md)
record this pass against the preserved previous executable and Skia.

Float transform, device-bound and rounded-rectangle keys now include a full-content
hash discriminator ahead of their original bytes. Equality still checks the full key.
Small keys use one owned data allocation. The renderer lock serializes its private
Objective-C state, so property accessors no longer take redundant atomic-property locks;
command-buffer callbacks continue to use only atomic fault/generation counters.

Unclipped, axis-aligned plain solid rectangles avoid constructing Core Graphics paths
and upload 32-byte quad/color inputs to a small solid pipeline. Fractional bounds use
the same four coverage samples; aliased bounds preserve inclusive/exclusive pixel-center
tests. This also avoids inserting newly allocated fractional edges into an identity-keyed
coverage cache on every draw. Clipped and arbitrary-transform geometry retain their
cached native paths and coverage. Fractional axis-aligned,
linearly sampled bitmap quads use a separate 96-byte pipeline with the same four samples
and boundary-texel extension. Its vertex clip bounds preserve the logical canvas inside
shared layer atlases. Nearest, mip/cubic, arbitrary-transform and clipped image paths
retain their existing implementations. Ordinary images, coverage masks and native text
keep their previous vertex payload and shader pipeline.

Uniform Core Text shaping attributes use an immutable attributed string, while OpenType
feature ranges retain the mutable implementation. Both use owned UTF-16 storage, the same
native shaping API, complete surrounding context and the same directional attributes.
There is no shaping-result cache. New tests compare the two paths' glyphs, advances,
offsets and clusters for Latin, Arabic and Hebrew, including combining marks and subranges.
Native packed-mask quality tests remain the text reference.

## Geometry and opacity submission pass

The [matched measurements and shipping-size checkpoint](quartz-optimization-round3-results.md)
record the resulting geometry/opacity costs and native pixel validation.

Fixed-size transform, coverage-variant and rounded-rectangle cache keys use borrowed
stack storage for lookups. The Core Foundation dictionaries copy a complete key only
on insertion, own and release that copy, and compare all original bytes on hash collisions.
Failed optional key allocation skips caching; existing path/coverage limits still apply.
The path cache removes an empty per-path dictionary if insertion fails. Development
fixtures force every key hash to collide and verify geometry before and after cache clears.

Group creation starts with an empty storage owner. Hinted groups acquire their atlas region;
unhinted groups acquire storage when first painted. Empty source-over groups stay metadata
only. An empty Source group still creates and composes the full transparent logical canvas.
Surface owners are refreshed after storage growth or private-surface promotion, keeping
paint recording consistent while avoiding repeated Objective-C property lookups.

Consecutive compatible commands in the deferred drawable stream merge into one draw batch.
Their target owner, texture, clip and compact pipeline must match; clears, dependency drains,
texture/pipeline changes and the existing command/storage budgets remain ordering boundaries.
Independent layer passes may occur between these recorded drawable commands. Their source
owners remain retained until the drawable stream is encoded, preserving atlas and pool leases.
Only compact batches merge; general edge-buffer batches retain their existing offsets.
A regression compares overlapping groups across multiple atlas pages, an intervening draw,
a clip dependency and a nested layer, requiring exact pixels and fewer encoded batches.

Render-pass descriptors are created when opening an encoder, instead of allocating unused
descriptors for compatible batches sharing an encoder. Core Text raster inputs, interpolation,
coverage and sampling retain their existing implementations and quality checks.

## Compact command recording and layer clears

Compact solid/image commands now carry an explicit draw count. They no longer copy
full generic QParams into the recording bucket merely to recover that count later.
The same count follows ordered deferred batches, prepared glyph replay, merge limits,
and recovery. Axis-aligned solid rectangles derive their bounds directly from two
transformed corners, avoiding generic edge-data allocation for fractional rectangles.
Compact paints also omit unused edge copies; general coverage/path paints retain
their complete parameters and edge offsets. Prepared glyph cache keys, raster inputs, shaders, sampling and glyph bounds
are unchanged; replay omits only the redundant generic recording copy.

Atlas region clears use 32-byte solid rectangle vertices instead of generic parameters,
with the existing constant transparent fragment shader and blending disabled. Every
allocated region is still cleared before use. Full-page clearing and a separate atlas
batch stream were tested and rejected because they did not improve completed frame
time reliably; the existing ordering and resource-owner model are retained.

A regression poisons two atlas pages, reuses them, starts with an empty region, leaves
transparent gaps, crosses encoder/page boundaries and nests a layer. Compact atlas
clears must match independent layer textures exactly. Existing native pixel oracles
cover generic/compact transitions, prepared glyph replay, clips, snapshots and recovery.
