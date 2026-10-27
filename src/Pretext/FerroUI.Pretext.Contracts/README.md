# ferroui-pretext-contracts

Portable measurement and shaping contracts for the Pretext text engine.

- `IPretextTextMeasurer` measures a whole string in a fixed font. Contextual
  advances must be included; independent character sums are insufficient for
  proportional typography.
- `IPretextTextMeasurerFactory` creates measurers, reports availability/priority,
  and can expose its corresponding shaping factory.
- `IPretextTextShaper` and `IPretextTextShaperFactory` return immutable glyph-run
  data, including shaped-versus-mapped status, UTF-16 clusters, positions,
  advances, offsets, font run indices and missing-glyph detection.
- `PretextFontParser` parses the source's CSS-like pixel-size/font-family subset
  and generic family aliases.
- `DelegateTextMeasurerFactory` adapts an existing string/font measurement
  function without global state.
- `PretextError` expresses recoverable input/backend failures through `Result`.

Backend ownership uses `Rc` and normal Rust destruction. Assembly reflection,
CLR metadata attributes and manual `IDisposable` emulation are unnecessary.
A shaping option is a value snapshot, so caller mutation cannot alter cached
results. Glyph clusters are UTF-16 code-unit offsets, not UTF-8 byte offsets.

The crate has no dependencies and forbids unsafe code. See the
[port report](https://github.com/wieslawsoltes/FerroUI/blob/main/docs/porting/pretext.md)
for provenance and source-to-Rust mappings. The upstream MIT notice travels with
this crate in `NOTICE.md`.
