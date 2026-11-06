# Layouts & languages

ScreenKeyboard uses FlorisBoard's layout model and JSON format. All FlorisBoard layouts, popup mappings, subtype
presets, composers and currency sets are bundled, and any FlorisBoard layout file can be loaded as is.

## Concepts

| Concept | Description |
|---|---|
| **Layout** | Rows of keys of a given *type*: `characters`, `symbols`, `symbols2`, `numeric`, `numericAdvanced`, `numericRow`, `phone`, `phone2` plus *modifier* layouts (`charactersMod`, `symbolsMod`, `symbols2Mod`). |
| **Modifier layout** | Adds the function keys (shift, delete, mode switch, space, enter). Its placeholder key (`code: 0`) is replaced by the last row of the main layout. |
| **Subtype** | A language + layouts combination: language tag, characters/symbols/numeric layouts, popup mapping, currency set, composer, punctuation rule. |
| **Popup mapping** | Long-press keys per label and field variation (e.g. `a → á à â ä ...` for French). |
| **Composer** | Transforms typed characters (Hangul syllable composition, Japanese dakuten, Vietnamese Telex). |
| **Currency set** | Six currency symbols that replace `currency_slot_1..6` keys. |

Keyboard modes are assembled like in FlorisBoard:

| Mode | Rows |
|---|---|
| Characters | (optional number row) + characters layout merged with `charactersMod` |
| Symbols | number row + symbols layout merged with `symbolsMod` |
| Symbols 2 | symbols2 merged with `symbols2Mod` |
| Numeric / Advanced / Phone | the respective layout |

Keys are sized with FlorisBoard's flexible algorithm: a row is designed for 10 key units, shift/delete/enter and
mode keys are 1.56 units wide, the space bar grows to fill free space and function keys shrink first when a row
overflows.

## Enabling languages

```csharp
settings.Subtypes = ["en-US/qwerty", "de-DE/qwertz", "fr-FR/azerty", "uk-UA/jcuken_ukrainian"];
```

A subtype id is `languageTag/charactersLayoutId`. Presets can be listed at runtime:

```csharp
foreach (var preset in KeyboardResources.Default.SubtypePresets)
    Console.WriteLine($"{preset.Id}: {KeyboardEngine.GetSubtypeDisplayName(preset)}");
```

Any preset can be combined with another characters layout: `"de-DE/qwerty"` uses German popups/currency with
QWERTY. Users switch languages with the globe key, a swipe (configurable), or a long press on the space bar
(language picker).

## Custom layouts

A standalone layout document adds metadata to FlorisBoard's arrangement format:

```json
{
  "type": "characters",
  "id": "alphabetical",
  "label": "Alphabetical (ABC)",
  "direction": "ltr",
  "arrangement": [
    [ "a", "b", "c", "d", "e", "f", "g", "h", "i", "j" ],
    [ "k", "l", "m", "n", "o", "p", "q", "r", "s" ],
    [ "t", "u", "v", "w", "x", "y", "z" ]
  ]
}
```

```csharp
var layout = KeyboardResources.Default.AddLayoutJson(json, extensionId: "myapp");
KeyboardResources.Default.AddSubtypePreset(new Subtype
{
    LanguageTag = "en-US",
    DisplayName = "English (ABC)",
    PopupMapping = new ComponentName(Subtype.CoreLocalization, "en"),
    Layouts = new SubtypeLayoutMap { Characters = layout.Name },
});
settings.Subtypes = ["en-US/alphabetical"]; // the preset id: languageTag/layoutId
```

Layouts can also be built in code (`KeyboardLayout` + `LayoutArrangement`) and registered with `AddLayout`, and
serialized back to JSON with `LayoutJson.WriteLayoutDocument`.

### Key syntax

A key is either a string shorthand (`"q"` → automatic upper-casing text key) or an object:

```json
{ "code": 101, "label": "e", "popup": { "main": { "code": 233, "label": "é" }, "relevant": [ { "code": 232, "label": "è" } ] } }
{ "$": "auto_text_key", "code": 113, "label": "q" }
{ "code": -11, "label": "shift", "type": "modifier" }
{ "label": "view_symbols", "type": "system_gui" }            // function codes can be given by name
{ "$": "multi_text_key", "codePoints": [ 3627, 3633 ], "label": "ห้" }
{ "$": "case_selector", "lower": { "code": 105, "label": "i" }, "upper": { "code": 304, "label": "İ" } }
{ "$": "shift_state_selector", "default": { "code": 49, "label": "1" }, "shiftedManual": { "code": 33, "label": "!" } }
{ "$": "variation_selector", "default": { "code": 44, "label": "," }, "email": { "code": 64, "label": "@" }, "uri": { "code": 47, "label": "/" } }
{ "$": "layout_direction_selector", "ltr": { "code": 40, "label": "(" }, "rtl": { "code": 41, "label": "(" } }
{ "$": "char_width_selector", "full": { ... }, "half": { ... } }
{ "$": "kana_selector", "hira": { ... }, "kata": { ... } }
```

| Field | Meaning |
|---|---|
| `code` | Unicode code point, or a negative function code (see `KeyCode`). |
| `label` | Displayed text; for function keys the function name (`delete`, `shift`, `enter`, `space`, `view_symbols`, `language_switch`, `ime_ui_mode_media`, `ime_ui_mode_clipboard`, `ime_ui_mode_editing`, `undo`, `redo`, `arrow_left`, `clipboard_paste`, `currency_slot_1`, ...). |
| `type` | `character` (default), `numeric`, `modifier`, `enter_editing`, `function`, `system_gui`, `navigation`, `lock`, `placeholder`. |
| `groupId` | `1` = key left of space (`~left` popups), `2` = key right of space (`~right`), `3` = enter (`~enter`). |
| `popup` | Explicit long-press keys (`main` is pre-selected, `relevant` are the others). |

Useful function codes: `-7` delete, `-11` shift, `-13` caps lock, `-21..-24` arrows, `-31..-35` copy/cut/paste/select/select all,
`-131/-132` undo/redo, `-201..-207` view modes, `-211..-214` panels (text, emoji, clipboard, editing), `-227` language switch,
`-232` hide keyboard, `-301` settings, `-801..-806` currency slots.

## Popup mappings

```json
{
  "all": {
    "a": { "main": { "code": 228, "label": "ä" }, "relevant": [ { "code": 224, "label": "à" } ] },
    "~right": { "relevant": [ { "code": 33, "label": "!" }, { "code": 63, "label": "?" } ] }
  },
  "uri": { "~right": { "relevant": [ { "code": 46, "label": ".com" } ] } }
}
```

```csharp
KeyboardResources.Default.AddPopupMapping(LayoutJson.ReadPopupMapping(json, id: "myapp-en", extensionId: "myapp"));
```

Long-press popups combine, in order: the key's explicit `popup`, the subtype's mapping and FlorisBoard's default
mapping. Number and symbol hints (from the number row and symbols layout) are added according to
`KeyboardSettings.HintMode`.

## FlorisBoard extensions

A whole FlorisBoard keyboard extension (`extension.json` with layouts, popup mappings, subtype presets, composers and
currency sets) can be registered:

```csharp
var manifest = LayoutJson.ReadExtensionManifest(File.ReadAllText("ext/extension.json"));
KeyboardResources.Default.AddExtension(manifest, relativePath => File.ReadAllText(Path.Combine("ext", relativePath)));
```

## Composers

Built-in: `appender` (default), `hangul-unicode`, `kana-unicode` and rule based composers such as `telex`. Custom
composers implement `IComposer`:

```csharp
/// Turns two hyphens into an em dash.
public sealed class EmDashComposer : IComposer
{
    public string Id => "em-dash";
    public string Label => "Em dash";
    public int ToRead => 1;
    public (int Delete, string Insert) GetActions(string preceding, string toInsert) =>
        toInsert == "-" && preceding.EndsWith('-') ? (1, "—") : (0, toInsert);
}

KeyboardResources.Default.AddComposer(new EmDashComposer(), "myapp");
// Reference it from a subtype: Composer = new ComponentName("myapp", "em-dash")
```

## Right-to-left

Layouts declare `"direction": "rtl"`; `layout_direction_selector` keys (brackets) mirror automatically.

## Bundled layouts

Characters layouts include: arabic, armenian (eastern/western/phonetic), azerbaijani, azerty, bengali unijoy, bépo,
bone, bulgarian (BDS/phonetic), canadian french, catalan, colemak (+DH, DHm), danish, diktor, dvorak (+de, es, se),
esperanto, estonian, faroese, georgian, german, greek, halmak, hebrew, hindi, hungarian, icelandic, igbo, IPA,
JCUKEN (russian, ukrainian, interslavic), JIS, korean (+phonetic), kurdish, nalmy, neo2, norwegian, persian (3
variants), qwerty, qwertz, rusyn, sangaline, serbian (cyrillic/latin), slovenian, spanish, swedish/finnish, swiss
(french, german, italian), tamil, thai (kedmanee, manoonchai), turkish (F, Q), udmurt, urdu phonetic, warang citi
and workman. Use `KeyboardResources.Default.GetLayoutMetadata(LayoutType.Characters)` for the complete list.
