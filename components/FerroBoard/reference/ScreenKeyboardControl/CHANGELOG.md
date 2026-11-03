# Changelog

All notable changes to this project are documented in this file. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project follows [Semantic Versioning](https://semver.org/).

## [1.0.0] - 2026-09-30

### Added

- `ScreenKeyboard.Core`: platform independent keyboard engine
  - FlorisBoard compatible layouts (90+), popup mappings, 70+ subtype presets, composers (Hangul, Kana, Telex and
    rule based), currency sets and punctuation rules; custom layouts from JSON or code
  - key evaluation with selectors, hints, popups and the flexible key sizing algorithm, split layout
  - shift/caps lock state machine, auto-capitalization, double-space period, smart punctuation spacing
  - suggestions: completion, keyboard-aware autocorrect with undo, learning user dictionary, next-word prediction,
    emoji shortcodes, clipboard suggestion; bundled English dictionary
  - glide typing (port of FlorisBoard's statistical classifier)
  - touch processor: rollover, long press popups, key repeat, glide, space bar cursor control, precise delete, swipes
  - emoji catalog (CLDR v48) with skin tones, search in 6 languages, history; emoticons
  - clipboard history with pins and expiry
  - 60+ observable, JSON serializable settings
- `ScreenKeyboard.Uno`: `OnScreenKeyboard` and `ScreenKeyboardHost` controls with smartbar, emoji, clipboard,
  text editing and language panels, key previews and popups, one-handed, split and floating modes, interactive
  resize mode, 12 built-in themes,
  JSON/XAML themes, key style rules, TextBox/PasswordBox integration with `InputScope` mapping and attached
  properties, haptics, system clipboard integration, emoji font fallback for WebAssembly, localizable UI strings
  (`KeyboardStrings`)
- Sample application, documentation, CI, GitHub Pages and NuGet release workflows with NuGet Trusted Publishing (OIDC)

[1.0.0]: https://github.com/wieslawsoltes/ScreenKeyboardControl/releases/tag/v1.0.0
