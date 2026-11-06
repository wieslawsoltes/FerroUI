# Emoji & clipboard

## Emoji panel

Open it with the emoji key (utility key), the smartbar quick action, a long press on enter or `SwipeAction.SwitchToMediaContext`.

- **Categories**: recently used (with pins), smileys & emotion, people & body, animals & nature, food & drink,
  travel & places, activities, objects, symbols, flags and classic emoticons (`:-)`).
- **Skin tones**: long press an emoji with variants to choose a tone. The choice is remembered per emoji
  (`EmojiHistory.GetPreferredTone`) and `KeyboardSettings.EmojiSkinTone` sets the default.
- **Recents**: long press a recent emoji to pin/unpin or remove it. Disabled by `EmojiHistoryEnabled = false`,
  incognito mode and private fields.
- **Search**: the search button switches the keyboard to a search field in the smartbar; type keywords and tap a
  result to insert it into your text field. Keywords come from Unicode CLDR in English, German, Spanish, French,
  Italian and Portuguese (the language of the active subtype is used).
- **Shortcodes**: typing `:heart` in a text field suggests matching emojis in the smartbar.

## Emoji catalog API

```csharp
var catalog = EmojiCatalog.Load("de");                // cached; unknown languages fall back to English
foreach (var category in catalog.Categories) { ... }
IReadOnlyList<EmojiInfo> smileys = catalog.GetEmojis(EmojiCategory.SmileysEmotion);
EmojiInfo? wave = catalog.Get("👋");
string medium = wave!.WithSkinTone(EmojiSkinTone.Medium);   // 👋🏽
IReadOnlyList<EmojiInfo> results = catalog.Search("rote herz");
IReadOnlyList<Emoticon> emoticons = EmojiCatalog.LoadEmoticons();
```

Custom emoji sets can be parsed with `EmojiCatalog.Parse(text)` using FlorisBoard's format (`[category]` headers,
`emoji;name;keyword|keyword` lines, tab-indented skin tone variants).

## Emoji fonts

Windows, macOS, Linux desktops, Android and iOS have system color emoji fonts that Uno uses automatically. In the
browser (and on bare Linux framebuffer devices) enable the fallback once at startup:

```csharp
EmojiFontFallback.UseNotoColorEmoji();                       // downloads Noto Color Emoji on first use (~10 MB)
EmojiFontFallback.UseNotoColorEmoji("https://my.cdn/NotoColorEmoji.ttf");
EmojiFontFallback.Use(async () => await OpenPackageFileAsync("Assets/Fonts/NotoColorEmoji.ttf"));
```

It plugs into Uno's font fallback chain for emoji code points only; other scripts keep using the previously
configured fallback service. Some recent emoji sequences may render as separate glyphs if the font or text shaper
does not support them.

## Clipboard

The keyboard keeps a clipboard history of text copied with the keyboard (editing panel, cut/copy) and — when the
platform allows it — the system clipboard (`SystemClipboardService`).

- Open the panel with the smartbar quick action, a swipe up on the space bar or `KeyCode.ImeUiModeClipboard`.
- Tap an item to paste it, pin items to keep them, remove single items or clear the history.
- Unpinned items expire after `ClipboardHistory.ExpireAfter` (1 hour) and are limited to `MaxItems` (30).
- `ClipboardHistoryEnabled = false`, incognito mode, password and private fields disable recording.
- Recently copied text is suggested in the smartbar (`ClipboardSuggestion`).

```csharp
var history = keyboard.Engine.ClipboardHistory;
history.MaxItems = 50;
history.ExpireAfter = TimeSpan.FromDays(1);
string json = history.Export(pinnedOnly: true);
history.Import(json);

// Replace the system clipboard integration (e.g. a sandboxed kiosk):
keyboard.Engine.ClipboardService = new InMemoryClipboardService();
```

## Text editing panel

Cursor pad with key repeat, selection mode (arrows extend the selection), line start/end, document start/end,
select all, copy, cut, paste, undo, redo, backspace and enter. Open it with the smartbar quick action or
`KeyCode.ImeUiModeEditing` in a custom layout.
