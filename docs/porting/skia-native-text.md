# Skia with Core Text and DirectWrite

Can the Skia backends use the platform text stacks, Core Text on macOS and iOS and DirectWrite on
Windows, for fonts, glyph rasterization and shaping? Issue #660. The versions are the ones the
bindings build (Skia m153, skia-safe 0.153.3, `external/rust-skia-bindings`).

## 1. Fonts, rasterization and fallback are native already

- `FontMgr::new()` and `FontMgr::default()` call `SkFontMgr::New...System` through the bindings
  (`external/rust-skia-bindings/src/bindings.cpp`): Core Text (`SkFontMgr_mac_ct`) on macOS and iOS,
  DirectWrite (`SkFontMgr_win_dw`, GDI only as a fallback) on Windows, fontconfig with FreeType on
  Linux, the Android manager with FreeType on Android, an empty FreeType manager in the browser.
  The cargo features do not change this; the target does.
- FerroUI's `font_manager_impl.rs` uses that system manager. On Apple platforms a typeface made
  from font data also goes through Core Text (`CTFontManagerCreateFontDescriptorFromData`) and its
  glyphs are rasterized by Core Graphics (`SkScalerContext_mac_ct`); on Windows by DirectWrite
  (`SkScalerContext_DW`). One limit: on Apple, a collection index other than 0 of a `.ttc` loaded
  from bytes is not supported.
- Character fallback (`matchFamilyStyleCharacter`) is the platform's.
- The glyph indices HarfBuzz returns are the font's, so they go into `SkTextBlob` runs as they are.

## 2. Shaping is HarfBuzz, and should stay so by default

- Skia's Core Text shaper (`modules/skshaper/src/SkShaper_coretext.cpp`) breaks lines itself and
  does not give what FerroUI's text formatter needs from `ITextShaperImpl` (bidi levels, clusters,
  one font a run); in rust-skia it needs the `textlayout` feature, which adds ICU and a second
  HarfBuzz. Skia has no DirectWrite shaper.
- The native renderer's Core Text shaper (`src/Quartz/FerroUI.Quartz/fonts.rs`, `native.m`) shows
  a Core Text shaper can meet the contract (it has to drop the glyphs of fonts Core Text
  substitutes silently).
- Chromium and Firefox shape with HarfBuzz over native rasterization; Firefox moved Apple's AAT
  fonts to HarfBuzz in 2019.

## 3. Quality: what to fix

- On Apple platforms Skia follows the system's font smoothing, so text has the native weight;
  `TextHintingMode::None` turns the smoothing off and gives lighter text.
- Done (issue #664): the shaper sets the point size of the HarfBuzz font (`hb_font_set_ptem`) to
  the em size of each run, so the tracking table (`trak`) of `.AppleSystemUIFont` (the macOS
  theme's font) is applied. The font of a typeface is shared by every size, so the size is set
  before each shaping; HarfBuzz only stores it, and no cache of the font depends on it.
- Done (issue #664): the variation coordinates of a typeface reach HarfBuzz, so a variable font
  (SF, Segoe UI Variable) is shaped at the instance it is drawn at, not at its default instance.
  `IPlatformTypeface::variation_settings` gives the position, empty by default; the Skia
  typeface reports `SkTypeface::getVariationDesignPosition`, the Vello typeface the axis values it
  was made with. They are set once, when the HarfBuzz font is made (`hb_font_set_variations`).
- Measured on macOS against Core Text (the width of one line of 68 characters in
  `.AppleSystemUIFont`, in pixels, by `CTLineGetTypographicBounds`; FerroUI with the Skia
  backend):

  | Size | Core Text | Core Text, optical size 17 | FerroUI before | FerroUI now |
  | ---: | ---: | ---: | ---: | ---: |
  | 9 | 314.591 | 314.591 | 267.847 | 314.591 |
  | 12 | 404.314 | 404.314 | 357.129 | 404.314 |
  | 14 | 461.474 | 461.474 | 416.650 | 461.474 |
  | 24 | 736.198 | 813.410 | 714.258 | 813.410 |
  | 48 | 1452.422 | 1641.164 | 1428.516 | 1641.164 |

  Before, the line was the default instance of the font without tracking (optical size 28, the
  default of the axis: 357.129 is what Core Text gives at 12 points for that optical size), 10 to
  15 % narrower than the glyphs drawn up to 14 points. Now it is what Core Text gives for the
  instance of the typeface at every size.
- The optical size (issue #859). The system font has an `opsz` axis from 17 to 96 and Core Text
  moves along it with the size of the text (17 up to 17 points, the size above); a typeface of
  Skia stays at one optical size for every size, the one it was made at (17 here), and so did the
  glyphs drawn and their advances: from 20 points up a line was the text design at a large size,
  10 to 13 % wider than the same line of a native application. A text is now shaped and drawn
  with the instance of its typeface at its optical size (`GlyphTypeface::at_optical_size`): the
  size of the text rounded to a whole point and clamped to the axis, one instance a whole point,
  kept by the typeface it is made from, so the glyph caches and the HarfBuzz font are shared
  inside a class. The run properties choose it once (`cached_glyph_typeface`, also for a font a
  fallback found). The backends make the instance: Skia clones the typeface at the variation
  position (`SkTypeface::makeClone`), Vello makes a typeface of the same data at the axis values.
  A font without the axis, and a family whose name names an optical size (`Segoe UI Variable
  Display`), keep their typeface. The table above at 9 to 96 points is to be measured again.
- Still to do: the same measurement against DirectWrite on Windows.
- Windows is tuned already (docs/porting/text-rendering.md). That document's section 2.3 should be
  checked: this Skia version's build appears to define `SK_GAMMA_APPLY_TO_A8` for every library
  source (`BUILD.gn`), FreeType included.

## 4. Size and speed

HarfBuzz is about 0.8 MB of a Windows executable (harfbuzz-sys 0.8.0, HarfBuzz 8.4.0, the newest
on crates.io); a native shaper would save about that much on Apple platforms and Windows only if
HarfBuzz left those builds (Linux, Android and the browser need it). Rasterization is native and
cached already: there is no saving a frame.

## 5. Recommendation

1. Fonts, rasterization and fallback: no change.
2. Shaping: HarfBuzz everywhere by default; `ferroui-harfbuzz` sets the point size and passes the
   variation coordinates (done, issue #664).
3. Opt-in native shapers only where they let a build drop HarfBuzz: on Apple, the native renderer's
   Core Text shaper factored out of `ferroui-quartz` (Skia's `SkTypeface_GetCTFontRef` gives the
   `CTFontRef` through a small shim; skia-safe does not bind it); on Windows, a DirectWrite shaper
   with the Direct2D backend (Skia exposes no getter for the DirectWrite face, so the face is made
   again).
4. Do not enable `textlayout`, or harfbuzz-sys's `coretext` / `directwrite` features, for a native
   shaper: neither drops HarfBuzz, and `textlayout` adds ICU.
5. The risk of a native shaper is layout that differs between platforms: the render tests share
   one set of expected images (allowed error 0.022), so a native shaper needs expectations of its
   own for each platform.
