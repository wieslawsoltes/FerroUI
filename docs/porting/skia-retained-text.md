# Retained text on Skia

Can Skia keep what it prepared to draw text from one frame to the next, the way frameworks with
their own GPU text renderers keep glyph quads in GPU buffers, and if not, can it be patched to?
Part of #644. The Skia version is the one FerroUI's bindings build (m153, skia-safe 0.153.3,
`external/rust-skia-bindings`); file names below are Skia's.

## 1. The cost

On the gauntlet (about 5,000 text runs on screen, the screen repainted every frame) a profile
of the render thread on Graphite (Metal) puts about a third of its time in text:
`SkCanvas::drawTextBlob` → `skgpu::graphite::Device::onDrawGlyphRunList` →
`sktext::gpu::TextBlobRedrawCoordinator::drawGlyphRunList`, split into `findOrCreateBlob`
(about 12 %, of it `sktext::gpu::TextBlob::Make` about 7 %) and `drawAtlasSubRun` /
`drawGeometry` (about 15 %). FerroUI keeps the same `SkTextBlob` (and so the same unique id) for
text whose layout did not change (desktop-performance.md sections 9.12 and 9.14).

## 2. What Graphite keeps and what it does again

- **The glyph atlas.** Without ordered recordings (`ContextOptions::fRequireOrderedRecordings`,
  off by default), `Recorder::snap` evicts every atlas at the end of every recording
  (`src/gpu/graphite/Recorder.cpp`, `TextAtlasManager::evictAtlases`): a recording may be inserted
  in any order, so the glyphs one recording uploaded cannot be assumed present for the next. The
  next frame rasterizes and uploads every glyph it draws again (`GlyphData::regenerateAtlas`). That
  work is inside the `drawAtlasSubRun` share of the profile. With ordered recordings the atlas is
  kept. Skia's own Metal window sets the option
  (`tools/window/GraphiteNativeMetalWindowContext.mm`), and FerroUI now does (section 9.17 of
  desktop-performance.md). The cost of the option: once one recording fails to insert, the
  context refuses the recorder's later recordings, so a failed insert has to make the context
  again (FerroUI treats it as a lost device).
- **The blob cache.** `TextBlobRedrawCoordinator` keys its entries by the blob's id, with the
  paint, color and matrix checked when an entry is found. It makes the blob again
  (`TextBlob::Make`) when the entry was evicted (a budget of 4 MiB, a constant without a setter;
  5,000 runs are of that order, so once a frame's blobs exceed it nearly every lookup misses, as
  they are drawn in the same order every frame), when the blob moved by a fraction of a device
  pixel (a translation reuses an entry only by whole device pixels; each new fraction adds an
  entry under the same id), or when the scale, the rotation or, for LCD text, the color changed.
  The lookup itself is paid by every draw, hit or miss.
- **Every draw.** Each glyph run is a draw of its own: its clip, its paint key, its place in the
  draw list, its glyph instances written into the buffers of the frame. Nothing in Graphite keeps
  the instance data of a draw from one frame to the next; this part cannot be removed by a small
  change.

## 3. The other mechanisms

- **Slugs** (`include/private/chromium/Slug.h`, `sktext::gpu::Slug::ConvertBlob`,
  `SkCanvas::drawSlug`): a blob converted ahead of time into the sub-runs a device would make for
  it. Graphite and Ganesh draw them. A slug skips the cache lookup and cannot be evicted (its
  owner keeps it), but it still pays the per-draw cost and keeps no GPU data across frames; its
  owner decides when it is stale, and a slug is drawn by one recorder. rust-skia does not expose
  slugs (about 120 lines of bindings).
- **Replaying a recording:** a recording can be inserted onto a target surface at an integer
  translation (`InsertRecordingInfo::fTargetSurface`, `fTargetTranslation`), but inserting one
  more than once is not documented, fails with ordered recordings, and without them brings the
  atlas eviction back. Not a way to keep drawing across frames.
- **Pictures and drawables:** an `SkPicture` replays its calls, nothing is kept on the GPU; on
  Graphite `SkCanvas::drawDrawable` draws nothing (`src/gpu/graphite/Device.h`), a drawable is
  drawn by calling its `draw`.
- **Layer caching:** drawing a subtree that does not change into a texture and composing the
  texture, as Chromium and Flutter's raster cache do. It works with stock Skia, and FerroUI has
  the mechanism (`CompositionBitmapCache`). What it lacks is a policy that applies it by itself,
  within a budget (Flutter caches a layer after it was drawn three times unchanged, at most three
  new entries a frame). Its costs are the memory of the textures and drawing them again on a
  change of scale or a move by a fraction of a pixel; LCD text needs an opaque layer.

## 4. Patching Skia

- **Raising the blob cache budget:** one line (about 25 to make it a setting), carried as the
  existing patch of the bindings is (an exact hunk that fails loudly when Skia changes the code;
  the constant has not changed in years). Worth it only when counting `TextBlob::Make` in a steady
  frame shows the cache thrashing (its count close to the number of runs drawn).
- **A retained text path inside Skia:** keeping the instance data of text draws across frames
  would touch about ten files of Graphite that change a lot upstream (600 to 1,500 lines) and would
  still leave the per-draw cost. Not a minor patch, not easy to maintain: not recommended.

## 5. The plan

1. Ordered recordings (done, section 9.17 of desktop-performance.md).
2. Count `TextBlob::Make` in a steady frame; raise the cache budget with the one-line patch only
   if it thrashes.
3. Text origins on whole device pixels where the layout allows it, so moved text reuses its
   blobs.
4. An automatic, budgeted policy for caching unchanged subtrees as layers.
5. Retained glyph quads belong to a renderer that owns its buffers: FerroUI's own GPU text in the
   ProGPU backend (#369), not a patch of Skia. (egui does not retain vertices either: it uploads
   every mesh every frame; its advantage is batching.)
