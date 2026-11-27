# Web fonts

This document is the research record and the plan for web font support in FerroUI: the fonts
an application declares the way a web page does (`@font-face`), loads with the API of the CSS
Font Loading module (`FontFace`, `FontFaceSet`), and ships or fetches as WOFF or WOFF2, on every
platform and rendering backend.

Tracking issue: #1205. Milestone: 0.6.0 (Incremental Font Transfer: 0.7.0). Status: being implemented. Nothing in this document is implemented yet unless a later
revision says so, and no number in it is a measurement. Section 11 lists what must be measured.

## 1. Why, and what exists today

A browser build has no installed fonts it can read, so an application has to ship its fonts.
Today that means TrueType or OpenType files embedded as assets, which are large, loaded in full
at start-up, and read synchronously.

What FerroUI has:

- `FontManager`, with collections by `fonts:` key, `add_font_collection`, and
  `invalidate_glyph_typefaces`, which makes every lookup run again after a collection changes.
- `FontCollectionBase` and `EmbeddedFontCollection`: assets (`ferres:`, `resm:`) and `file:`
  sources, read synchronously through
  `IFontManagerImpl::try_create_glyph_typeface_from_stream`.
- Backends that build typefaces from bytes: Skia (`FontMgr::new_from_data`, which copies them
  into an `SkData`), Vello (a shared `peniko::Blob`), Quartz (`CGFontCreateWithDataProvider`
  over a `CFData` copy) and Direct2D.
- `read-fonts` and `skrifa` in the Vello backend, `harfbuzz-sys` in the shaper, and `flate2`
  with its pure Rust backend in the workspace.

What it does not have: any parser of `@font-face`, `unicode-range`, `font-display` or the Font
Loading API, any WOFF or WOFF2 decoder, any font sanitizer, and any font fetching in
`src/Browser`.

## 2. Sources

Standards, read from the published documents:

- CSS Fonts Module Level 4 and Level 5 (W3C).
- CSS Font Loading Module Level 3 (W3C).
- WOFF File Format 1.0 and WOFF File Format 2.0 (W3C).
- Incremental Font Transfer (W3C Candidate Recommendation Draft, 13 October 2026).
- Fetch (WHATWG), for destinations and credentials.
- Local Font Access API (MDN; experimental, not Baseline).

Frameworks and platform interfaces:

- **The upstream framework** (read-only checkout, `main`). `EmbeddedFontCollection` and
  `FontCollectionBase` load assets and files. There is no WOFF, `@font-face` or font fetching.
  The API here extends that model and does not replace it.
- **WinUI** (`microsoft-ui-xaml`, `winui3/main`). Text uses the DirectWrite system collection,
  which includes downloadable fonts. The core services loop drains
  `IDWriteFontDownloadQueue` and invalidates layout from the `DownloadCompleted` callback
  (`dxaml/xcp/pal/win/DWriteFontAndScriptServices.cpp`, `dxaml/xcp/core/dll/xcpcore.cpp`).
  The lesson taken here: a font arriving late is a layout invalidation, never a wait.
- **DirectWrite.** `IDWriteFactory5::UnpackFontFile` unpacks WOFF and WOFF2 (Windows 10 build
  15063) and `IDWriteInMemoryFontFileLoader` loads fonts from memory (build 14393).
- **Flutter.** `FontLoader` collects `Future<ByteData>` per family and loads the family into
  the engine.
- **Compose Multiplatform.** Font resources per family; TTF, OTF, TTC and variable fonts on all
  platforms, WOFF and WOFF2 only on web and macOS, a preload API on web.
- **Android downloadable fonts.** A provider application fetches and shares fonts between
  applications; requests run on a non-UI handler; fonts can be pre-declared to avoid a slow
  first layout.
- **Qt.** `QFontDatabase::addApplicationFont` and `addApplicationFontFromData` return an id
  that later removes the font. The documented formats are TrueType, TrueType collections and
  OpenType.
- **OpenType Sanitizer (OTS).** The parse, validate and serialize library integrated into
  Chromium and Firefox. It is the model for section 7.

What the comparison shows: no native UI framework among these implements the CSS model. The
closest are per-family loaders. FerroUI takes the CSS model as its contract, because it is the
only complete, specified one, and exposes it with the names of the Font Loading API.

## 3. Standards matrix

"Core" is the part that ships first (section 10, phase A). "Later" is implemented after the
core (phases B and C). Section 12 lists what is deliberately left out.

### 3.1 `@font-face` descriptors

| Descriptor | Support | Phase |
| --- | --- | --- |
| `font-family` | Full | Core |
| `src`: `url()`, `format()`, `tech()`, ordered fallback, skip unsupported | Full | Core |
| `src`: `local()` | Installed fonts through the backend; see section 5 for the browser | B |
| `font-style` with `oblique` angle ranges | Full | Core |
| `font-weight` range | Full | Core |
| `font-width` and legacy `font-stretch` range | Full | Core |
| `unicode-range` (points, intervals, wildcards) | Full | Core |
| `font-display` | Full (section 6.3) | Core |
| `font-feature-settings` | Applied at the descriptor step of CSS Fonts 4, 7.2 | B |
| `font-variation-settings` | Applied at the descriptor step | B |
| `font-named-instance` | Resolved from `fvar` and `name` | B |
| `font-language-override` | Parsed and passed to the shaper; at risk in the standard | B |
| `ascent-override`, `descent-override`, `line-gap-override` | Full | B |
| `size-adjust` | Full | B |
| Level 5 `font-size` range | Parsed; used for matching optical faces | C |
| Level 5 superscript and subscript overrides | Parsed; used where synthesis exists | C |

### 3.2 `format()` and `tech()`

| Value | Answer |
| --- | --- |
| `truetype`, `opentype`, `collection` | Supported, sanitized |
| `woff` | Supported |
| `woff2` | Supported, with `brotli-decompressor` 5.0.3 behind the feature `woff2` |
| `embedded-opentype`, `svg` (the SVG font format) | Not supported, skipped as the standard allows |
| `variations` | Supported on every backend |
| `features-opentype` | Supported through HarfBuzz |
| `features-aat`, `features-graphite` | Reported only where the shaper build supports them |
| `color-COLRv0`, `color-COLRv1`, `color-CBDT`, `color-sbix`, `color-SVG` | Per backend, section 5.3 |
| `palettes` | Phase C |
| `incremental` | Phase C, glyph-keyed patches only (section 9) |

A `tech()` the active backend cannot draw makes the entry unsupported, so the next `src` entry
is used. That is what the standard intends, and it keeps a color font that cannot be drawn from
replacing a plain one that can.

### 3.3 Font Loading API

`FontFace` (constructor with a source or binary data, every descriptor attribute, `status`,
`load`, `loaded`), `FontFaceSet` (`add`, `delete`, `clear`, `load`, `check`, `ready`, `status`
and the events `loading`, `loadingdone`, `loadingerror`) and the per-application set that plays
the role of `document.fonts`. All core.

Differences that follow from not being a browser:

- Promises become a small completion handle usable from callbacks and `async` code.
- "Pending on the environment" means the first layout pass has not run yet.
- A worker's font source maps to a `FontFaceSet` created for that thread. Faces are bound to
  the UI thread's font manager, because glyph typefaces are `Rc`; only the decoded bytes cross
  threads.

### 3.4 Matching

The font matching algorithm of CSS Fonts 4, section 5.2, in the standard's order: width, then
style (including the 11 degree oblique threshold and the `ital` and `slnt` axes), then weight
(including the special case for 400 to 500). Composite faces are searched in reverse order of
definition. A face is loaded only when its `unicode-range` includes the character, and is used
only if its effective character map has the character.

### 3.5 Containers and color

| Standard | Support | Phase |
| --- | --- | --- |
| WOFF 1.0 | Every rejection rule of the standard | Core |
| WOFF2: tables, collection directory, `glyf`, `loca` and `hmtx` transforms | Full | Core |
| Variable fonts | Full, through the existing axis code of each backend | Core |
| COLR v0/v1, CBDT, sbix, SVG in OpenType | As the backend draws them (section 5.3) | Core |
| `@font-palette-values`, `font-palette` | Implemented where a backend takes a palette | C |
| Incremental Font Transfer | Glyph-keyed patches, implemented last (section 9) | C |
| Local Font Access API | Browser only, on request, behind a permission | C |

## 4. Architecture

```text
 XAML / builders / code
        |  declare faces
        v
 ferroui-fonts-web            FontFace, FontFaceSet, WebFontCollection,
   (opt-in crate)             fetchers, scheduler, limits, swap
        |            \
        |             v
        |      ferroui-fonts-web-core   CSS parser, sfnt, sanitizer, WOFF, WOFF2,
        |        (no framework, no I/O)  matching, display timeline
        v
 ferroui-base                 IFontCollection, FontManager (unchanged),
        |                     IFontManagerImpl::try_create_glyph_typeface_from_data
        v
 Skia | Vello | Quartz | Direct2D | (ProGPU, seam only)
        |
        v
 the existing shaping and GPU glyph path of each backend
```

Four rules follow from the requirements.

1. **The feature is a crate.** Nothing in the framework depends on `ferroui-fonts-web`. An
   application that does not name it links none of it, which is how hello world stays the size
   it is. `ferroui-base` does not change at all (section 4.2).
2. **The core has no framework dependency.** `ferroui-fonts-web-core` is plain data in, plain
   data out, so it runs on any thread and its tests build without the framework.
3. **Decoded fonts are plain OpenType.** After the decoder, every backend gets the same sfnt
   bytes. No backend learns about WOFF, so every glyph stays on the path it uses today.
4. **`font_manager.rs` does not change.** Web fonts register through `add_font_collection`, and
   a face that arrives calls `invalidate_glyph_typefaces`.

### 4.1 The flow of one face

1. A face is declared. Its descriptors are parsed and it enters the set as `unloaded`. Nothing
   is fetched.
2. Text asks the collection for a family. The collection runs the matching algorithm on the
   declared descriptors, without any font bytes.
3. If the chosen face is not loaded and its `unicode-range` covers the text, a load is queued
   and the display timeline starts. The lookup returns at once with what the timeline allows
   (section 6.3).
4. A fetcher delivers the bytes without blocking the UI thread.
5. The decode task sniffs the container, decodes, sanitizes and produces one `Arc<[u8]>`.
6. On the UI thread, at a frame boundary, all faces that finished are installed together, and
   the glyph typeface generation advances once. Layout runs once for the whole batch.

### 4.2 The seam to the backends

`IFontManagerImpl` does not change. A new method would put a vtable entry, and with it code, in
every application, whether or not it uses web fonts.

The crate reaches the active backend through what every backend already has:
`FontManager::current().platform_impl()` and `try_create_glyph_typeface_from_stream`. It passes a
small `Read` type that owns the decoded `Vec<u8>` and overrides `read_to_end`: into an empty
vector, it hands its buffer over instead of copying it. Every backend reads a font with
`read_to_end` into an empty vector, so the buffer moves in. A backend that reads another way
still works, through plain `read`, with a copy. An application never names its backend, and a
new backend (ProGPU) works with nothing added.

## 5. Platforms and backends

### 5.1 Platforms

| Platform | Fetch | Decode | Notes |
| --- | --- | --- | --- |
| macOS | `file:`, `data:`, assets, `https:` (feature `http`) | Worker thread | |
| Windows | Same | Worker thread | |
| Linux | Same | Worker thread | |
| iOS | Same | Worker thread | App Transport Security does not apply to `ureq`; HTTPS only by default |
| Android | Same | Worker thread | Needs the `INTERNET` permission for `https:` |
| Browser, threaded build | `fetch()` | Worker thread | Browser HTTP cache |
| Browser, single-threaded | `fetch()` | Time-sliced on the UI thread | Section 6.2 |

Native HTTP is opt-in behind the `http` feature (`ureq`). It is HTTPS only unless plain HTTP is
explicitly enabled, and it limits redirects, response size and total time. Without the feature,
`file:`, `data:` and asset sources work, and an application can supply its own
`IFontFaceFetcher`.

In the browser, `fetch()` is called with `mode: "cors"` and `credentials: "same-origin"`, the
behaviour the standard requires for fonts. Script `fetch()` is governed by the `connect-src`
directive of a Content Security Policy rather than `font-src`, which the documentation for
applications has to state. Reuse of `<link rel="preload" as="font">` responses by such a
request is unverified and is listed in section 11.

`local()` in the browser cannot read installed fonts without the Local Font Access permission
prompt, so there it is treated as not found and the next `src` entry is used, unless the
application has enabled that API.

### 5.2 How a face reaches each backend

| Backend | Typeface from data | Shaping | GPU glyph path |
| --- | --- | --- | --- |
| Skia | The stream path: one `SkData`, the moved buffer dropped | HarfBuzz over the table access | Unchanged |
| Vello, CPU | The stream path: a `Blob` over the moved buffer, no copy | Same | CPU mode stays what it is |
| Vello, hybrid and GPU | Same | Same | Unchanged: `glifo` outlines |
| Vello, WebGL2 | Same | Same | Unchanged |
| Quartz | The stream path: a data provider over one shared buffer | Core Text | Unchanged |
| Direct2D | The stream path: an in-memory file over one shared buffer | DirectWrite | Unchanged |
| ProGPU | Seam only: the stream path, with nothing added | | Implemented with #369 |

No backend gets a new rasterizer. A face that a backend cannot draw on the GPU today is treated
as it is today.

There is exactly one long-lived copy of a decoded font. Skia's safe API has no release callback
for external memory, so the moved buffer is copied once into `SkData` and dropped when the call
returns. Vello keeps the moved buffer itself. Quartz and Direct2D turn it into one `Arc<[u8]>`,
a short overlap, and hand the native font a reference to that buffer with a release callback.

### 5.3 Color formats

`tech()` answers come from a capability query on the backend. The table of what each backend
draws is to be filled from validation, not assumed: the code shows Vello has handling for
`sbix`, `CBDT`, `EBDT`, `COLR` and `SVG ` glyph tables, and Skia, Core Text and DirectWrite each
have their own support that varies by OS version.

## 6. Performance

### 6.1 Start-up

- Declaring a face parses a few strings. There is no I/O and no decode.
- Nothing in the framework runs for an application without the crate.
- With the crate, nothing is loaded until text needs it, except faces marked for preloading.

### 6.2 No blocking

- Fetching is asynchronous on every platform.
- Where threads exist, decoding runs on a worker.
- The single-threaded browser build runs the decoder as a resumable task in slices, between
  frames. The decoder is written as a state machine with a step budget for this reason.
- In the browser, decoding never makes a long task on the main thread, in either build. A
  slice stays well below the 50 ms that defines a long task, and it yields between slices.

### 6.3 `font-display`

| Value | Block period | Swap period |
| --- | --- | --- |
| `auto` | As `swap` (the standard leaves it to the implementation) | Infinite |
| `block` | 3 s | Infinite |
| `swap` | 100 ms | Infinite |
| `fallback` | 100 ms | 3 s |
| `optional` | 100 ms, or until the first paint | None: never swaps after a fallback was painted |

Timers start at the first use of the face. During a block period text is laid out with the
fallback font's metrics and drawn invisibly. The invisible face is a generated OpenType font
that keeps the fallback's `cmap` and metrics and has empty outlines. It goes through the normal
path of each backend, so no backend needs a special case and nothing moves off the GPU.

Faces that finish in the same frame swap together: one generation change, one layout pass.

### 6.4 `unicode-range`

Each segment is its own face and loads on demand. The range check is a binary search over
sorted intervals, and the per-text check runs only when the set still has unloaded faces.

### 6.5 Caching

- Browser: the HTTP cache through `fetch()`.
- Native: optional on-disk cache keyed by URL, honoring validators, in a later step.
- In memory: one entry per URL, so faces that share a file share its bytes.

## 7. Security

A font from the network is untrusted input to complex native parsers. The sanitizer follows the
approach of OTS: parse, validate, and write out a new font containing only what passed.

- The table directory is rebuilt: sorted, no overlaps, every range inside the file, checksums
  recomputed.
- Required tables are validated, including `head`, `maxp`, `hhea`, `hmtx`, `cmap`, `loca` and
  `glyf` bounds, composite depth and cycles, and point counts. A font that fails is refused.
- An optional table that fails is dropped and the font lives. A variation table goes with an
  invalid `fvar`.
- Tables outside a known list are dropped, `DSIG` included.
- Hinting is kept by default, as in browsers, because removing it changes how text looks and
  would break the rule that a font renders the same from TTF, WOFF and WOFF2.
  `SanitizeOptions::strip_hinting` removes the programs and the instructions of each glyph.
- **Every table that is kept has its content validated, or it is not kept.** The offset graph is
  walked with the traversal of `read-fonts` under a work budget, so that subtables shared many
  times cannot multiply the work. On top of the walk come the checks it does not make: glyph
  ids, lookup and feature indices, the order of coverage and class ranges, layer and palette
  indices, cycles of color glyphs, the value records of `GPOS`, the tuple stores of `gvar` and
  `cvar`, the names of `post`, `kern` pairs, and every charstring of `CFF ` and `CFF2`, run with
  a limit on the work of subroutines.
- **Fail closed.** A table that fails, or that has no validator (`JSTF`, `MATH`, `VDMX`, `LTSH`,
  `PCLT`, `EBSC`, the AAT tables), is dropped when the font works without it. A failure in a
  table the font needs refuses the font, and the next `src` entry is used.
- **This is not the OpenType Sanitizer.** It takes the same approach and is not equal to it: the
  checks are its own, and its verdicts have not been compared with those of OTS.
- **The traversal can panic** on a table too short for the fields of its version. Those fields
  are a closed list in `read-fonts` 0.41, and each is guarded before the walk. The list has to be
  checked again whenever the pin moves, which follows `skrifa`.
- Three tolerances exist because shipped fonts need them and no reader is harmed: an offset of
  zero is null even where the format does not call it optional, coverage ranges may share their
  last glyph with the next range, and a coverage array may repeat a glyph.
- Decompression output is bounded by the declared size and by the limits below, before any
  allocation. No code path panics on input; a fuzz target covers the decoders.
- Failure marks the face as `error` and the next `src` entry is tried.

Default limits, adjustable through `WebFontLimits`: 32 MiB encoded per file, 64 MiB decoded per
font, 128 MiB decoded in total, 8 concurrent fetches, 5 redirects, 30 s per fetch.

CORS applies only in the browser, where the browser enforces it. A native `https:` request has
no origin, so the limits above and the HTTPS default are its protection.

## 8. Memory

- One immutable decoded buffer per font, shared by reference with whatever backend can share it.
- Encoded bytes are dropped once the font is decoded, unless the budget policy keeps them to
  allow a cheap re-decode.
- Tables are parsed lazily by the existing readers.
- Over the budget, the least recently used faces with no live glyph typeface are evicted. Their
  status stays `loaded` to script, as the standard has no other state for them, and they are
  fetched or decoded again when next used.

## 9. Incremental Font Transfer

IFT is implemented last, in milestone 0.7.0 (#1204). It does not block 0.6.0, by the owner's
decision, for these reasons:

- It is a Candidate Recommendation Draft whose test suite is still in development.
- Table-keyed patches use the LZ77 dictionary of the Shared Brotli format (draft 15, section
  3.2). The custom dictionary of `brotli-decompressor` 5.0.3 is a prefix of its ring buffer: the
  addressing is the same only while the dictionary and the output fit in the window, so those
  patches need a decoder with real Shared Brotli support. Glyph-keyed patches are plain RFC 7932.
- `unicode-range` segments already give on-demand loading for most of the same cases.

Scope of the client: **glyph-keyed patches only**. FerroUI does not write its own Shared Brotli
addressing and does not take a second decoder for it. A font that needs a table-keyed patch is
never decoded approximately: the face fails over to the next `src` entry, as for any font that
cannot be used.

Until the client lands, an `src` entry with `tech(incremental)` is skipped, as the standard
requires of an unsupported technology, so a page-style declaration with a WOFF2 fallback works.

## 10. Phases

Each step is one draft pull request, stacked on the one below.

**Phase A, the shippable core**

1. This plan.
2. `ferroui-fonts-web-core`: the CSS parser for `@font-face` and every descriptor.
3. sfnt reader and sanitizer.
4. WOFF 1 decoder.
5. WOFF2 decoder (the Brotli step waits for approval, behind an internal seam).
6. Font matching and composite faces.
7. `FontFace`, `FontFaceSet` and the display timeline, as state machines.
8. One long-lived copy of a font in the stream path of every backend.
9. `ferroui-fonts-web`: `WebFontCollection`, the scheduler and the swap.
10. Fetchers: `data:`, `file:`, assets, the worker thread, and `https:` behind `http`.
11. The browser: `fetch()`, CORS, and time-sliced decoding.

**Phase B, the complete descriptor set**

12. Metric overrides and `size-adjust`.
13. Feature and variation descriptors, and named instances.
14. `local()`.
15. Declarative and XAML declaration of faces.

**Phase C, the rest of the standards**

16. Color capability answers, `@font-palette-values` and `font-palette`.
17. Level 5 descriptors and Local Font Access.
18. Incremental Font Transfer, glyph-keyed patches only (milestone 0.7.0).

## 11. What must be measured

Nothing below has been measured yet.

- **Size:** hello world, gallery and catalog, release stripped and `__text`, before and after
  each part that touches `ferroui-base` or a backend. Any growth without the crate drops the
  change. Separately, the cost of the crate and of the `woff2` feature to an application that
  enables them.
- **Start-up:** the start-up check, with and without declared faces. A slower start for an
  application without the crate does not ship.
- **Throughput:** WOFF and WOFF2 decode time and peak memory per font, on the fonts in this
  repository, on desktop and in the browser.
- **Frame time:** the longest frame during a swap, and that a batch causes a single layout.
- **Memory:** peak and settled bytes per loaded face on each backend, confirming one long-lived
  copy.
- **Robustness:** fuzz runs of the decoders and sanitizer.
- **Rendering:** the same text on each backend from TTF, WOFF and WOFF2 is pixel-identical on
  that backend. Colour fonts and variable fonts on every backend and platform.
- **Browser:** the `fetch()` credentials and CORS behaviour, Content Security Policy behaviour,
  reuse of preloaded responses, and cache hits on a second load.
- **Platforms:** macOS, Windows VM, Linux VM, iOS simulator, Android emulator, browser.

## 12. Deliberately not supported

- **EOT (`embedded-opentype`) and SVG fonts (`svg`).** Legacy formats that browsers have
  dropped or never shipped widely. They are skipped so that the next `src` entry is used.
- **A CPU glyph path for any format.** A font that cannot be drawn on a backend's GPU path gets
  no new CPU path.
- **Table-keyed IFT patches.** They need the Shared Brotli LZ77 dictionary, which the planned
  decoder does not implement. Such a font falls through to the next `src` entry.
- **Sharing fetched fonts between applications**, as Android's provider model does. It needs a
  trusted system service.
