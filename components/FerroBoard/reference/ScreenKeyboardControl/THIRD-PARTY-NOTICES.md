# Third-party notices

ScreenKeyboard is licensed under the [Apache License 2.0](LICENSE). It builds on the following third-party work.

| Component | Used for | License |
|---|---|---|
| [FlorisBoard](https://github.com/florisboard/florisboard) | Layouts (90+), popup mappings (55+), subtype presets (70+), composers, currency sets, English dictionary, emoji data, glide typing classifier (ported), layout algorithms (ported), theme palettes | Apache-2.0 |
| [Unicode CLDR](https://cldr.unicode.org/) | Emoji names, keywords and categories (via FlorisBoard) | Unicode License v3 |
| [Material Symbols](https://github.com/google/material-design-icons) | Key and panel icons (converted to path data) | Apache-2.0 |
| [Noto Color Emoji](https://github.com/googlefonts/noto-emoji) | Optional emoji font downloaded at runtime by `EmojiFontFallback.UseNotoColorEmoji()` (not redistributed) | SIL Open Font License 1.1 |
| [Uno Platform](https://platform.uno) | UI framework | Apache-2.0 |

The FlorisBoard license text is included in the NuGet package under `licenses/florisboard/LICENSE`.
