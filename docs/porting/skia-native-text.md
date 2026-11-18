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
- Still to do: measure how far the advances move against `CTRunGetAdvances` on macOS and against
  DirectWrite on Windows.
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
