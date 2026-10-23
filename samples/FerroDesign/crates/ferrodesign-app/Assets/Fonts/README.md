# Embedded fallback fonts

These unmodified upstream fonts supply Hebrew and color emoji through
FerroUI's public `FontManagerOptions`, `FontFallback` and
`EmbeddedFontCollection` APIs. Inter remains the application's primary family.
The browser and native application register the same assets; the browser does
not need an installed host font for these characters.

| Asset | Family / face | Bytes | Pinned upstream |
| --- | --- | ---: | --- |
| `NotoSansHebrew-Regular.ttf` | Noto Sans Hebrew, weight 400 | 26,900 | [notofonts/noto-fonts, ffebf8c1](https://github.com/notofonts/noto-fonts/blob/ffebf8c1ee449e544955a7e813c54f9b73848eac/hinted/ttf/NotoSansHebrew/NotoSansHebrew-Regular.ttf) |
| `NotoSansHebrew-Bold.ttf` | Noto Sans Hebrew, weight 700 | 27,164 | [notofonts/noto-fonts, ffebf8c1](https://github.com/notofonts/noto-fonts/blob/ffebf8c1ee449e544955a7e813c54f9b73848eac/hinted/ttf/NotoSansHebrew/NotoSansHebrew-Bold.ttf) |
| `NotoColorEmoji-COLRv1.ttf` | Noto Color Emoji, COLR version 1 | 5,038,236 | [googlefonts/noto-emoji, e20cbc2b](https://github.com/googlefonts/noto-emoji/blob/e20cbc2bbec1926686be9f9bee7d1d2cfa1fea0e/2D/fonts/Noto-COLRv1.ttf) |

All three files are licensed under the SIL Open Font License 1.1. The complete,
unchanged upstream license texts are retained in
[NotoSansHebrew-OFL.txt](NotoSansHebrew-OFL.txt) and
[NotoColorEmoji-OFL.txt](NotoColorEmoji-OFL.txt). Their individual copyright
notices remain in those files and in the font metadata. No glyphs, names,
tables or font data have been modified or subsetted.

[provenance.json](provenance.json) records the exact source commits, download
URLs, Git blob identities, byte counts and SHA-256 hashes of both fonts and
licenses. Each downloaded file was checked against its upstream Git blob
before use. Independent font-table inspection records nonzero glyphs for
the Hebrew test text and crab emoji; it is separate from actual FerroUI
shaping and rendering validation.

The static Hebrew faces are deliberate. The inspected Google Fonts variable
candidate defaults to weight 100; the current public embedded-collection
matcher selects available faces and does not instantiate its weight axis.
The static files provide actual Regular and Bold faces without application
calls into backend-specific font internals.

Color emoji use the existing Vello color-font path through ordinary FerroUI
text layout and drawing. No emoji images, browser text overlay or separate
graphics implementation is introduced. A color glyph remains unsuitable for
the application's monochrome-only text-outline conversion; that operation
must continue to decline instead of discarding its colors.

These assets address the observed Hebrew/emoji coverage gap. They do not
supply every writing system or all fonts a document may name. The three font
files add 5,092,300 uncompressed bytes before build compression; browser build
size and timing are measured separately at their exact source checkpoints.
