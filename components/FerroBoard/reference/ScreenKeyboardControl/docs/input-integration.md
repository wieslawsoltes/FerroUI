# Input integration

The keyboard types into an `ITextInputTarget`. Adapters exist for `TextBox`, `PasswordBox` and an in-memory buffer;
you can implement the interface for any custom editor.

## TextBox and PasswordBox

`OnScreenKeyboard.AttachTo(element)` (and the automatic focus tracking) create a `TextBoxInputTarget` or
`PasswordBoxInputTarget`. Controls built on a `TextBox` (`AutoSuggestBox`, `NumberBox`, ...) work automatically
because their inner `TextBox` receives focus.

The field type is derived from the standard WinUI properties:

| WinUI | Keyboard behavior |
|---|---|
| `InputScope="Number"`, `Digits` | Numeric pad |
| `CurrencyAmount`, `Formula`, `NumberFullWidth` | Advanced numeric pad |
| `TelephoneNumber` (and area/country/local variants) | Phone pad |
| `EmailSmtpAddress`, `EmailNameOrAddress` | Email variation (`@` key, no auto-capitalization, URI popups) |
| `Url` | URI variation (`/` key), enter = Go |
| `Search`, `SearchIncremental` | Enter = Search |
| `Chat` | Enter = Send, emoji friendly |
| `Password`, `NumericPin`/`NumericPassword` | No suggestions, nothing is learned or stored |
| `PersonalFullName` | Words are capitalized |
| `DateMonthNumber`, `TimeHour`, ... | Advanced numeric pad |
| `AcceptsReturn="True"` | Multi-line: enter inserts a new line |
| `IsTextPredictionEnabled="False"` | No suggestions |
| `IsSpellCheckEnabled="False"` and `IsTextPredictionEnabled="False"` | No auto-correction |
| `MaxLength`, `IsReadOnly` | Respected |

## Attached properties

`ScreenKeyboardInput` overrides the derived behavior per element:

```xml
<TextBox sk:ScreenKeyboardInput.InputKind="Email"
         sk:ScreenKeyboardInput.EnterAction="Next"
         sk:ScreenKeyboardInput.Capitalization="None"
         sk:ScreenKeyboardInput.SuggestionsEnabled="False"
         sk:ScreenKeyboardInput.IsPrivate="True" />

<!-- Never show the on-screen keyboard for this field (or any field inside this panel) -->
<StackPanel sk:ScreenKeyboardInput.IsEnabled="False"> ... </StackPanel>
```

| Property | Values |
|---|---|
| `InputKind` | `Text`, `Number`, `Decimal`, `Phone`, `DateTime`, `Email`, `Uri`, `Password`, `NumericPassword`, `Search`, `Chat` |
| `EnterAction` | `Default`, `NewLine`, `Go`, `Search`, `Send`, `Next`, `Previous`, `Done` |
| `Capitalization` | `None`, `Characters`, `Words`, `Sentences` |
| `SuggestionsEnabled` | `true`/`false` |
| `IsPrivate` | `true` disables learning, clipboard history and emoji history for the field |
| `IsEnabled` | `false` excludes the element (and its children) |

`InputKind` set on an outer control (for example a `NumberBox`) applies to its inner `TextBox`.

## Enter actions

For single-line fields the keyboard raises `OnScreenKeyboard.EnterActionRequested`:

```csharp
keyboard.EnterActionRequested += (sender, e) =>
{
    if (e.Action == EnterAction.Search && e.Element == SearchBox)
    {
        RunSearch(SearchBox.Text);
        e.Handled = true;
    }
};
```

Default behavior when not handled: `Next`/`Previous` move focus to the next/previous focusable element, `Done`
raises `HideRequested`. Multi-line fields always insert a line break.

## Custom targets

Implement `ITextInputTarget` to connect any editor (a code editor, a canvas based control, a terminal, a remote
session...):

```csharp
public sealed class TerminalTarget : ITextInputTarget
{
    public InputAttributes Attributes { get; } = new() { Kind = InputKind.Text, AllowSuggestions = false, Capitalization = CapitalizationMode.None };
    public string Text => _buffer.ToString();
    public int SelectionStart => _buffer.Length;
    public int SelectionLength => 0;
    public event EventHandler? Changed;

    public void Replace(int start, int length, string text)
    {
        _buffer.Remove(start, length).Insert(start, text);
        _terminal.Send(text);
    }

    public void Select(int start, int length) { }
    public bool PerformEnterAction(EnterAction action) { _terminal.Send("\r"); return true; }

    private readonly StringBuilder _buffer = new();
    private readonly Terminal _terminal;
}

keyboard.Attach(new TerminalTarget(...));
```

Rules for implementers:

- `Replace` must place the caret right after the inserted text.
- Raise `Changed` only for changes that did **not** come from `Replace`/`Select` (user edits, caret moves). The
  engine then refreshes auto-capitalization and suggestions.
- Return `true` from `PerformEnterAction` when the action was handled; otherwise multi-line targets get a new line.

`TextBufferTarget` is a ready-made in-memory implementation, useful for tests, previews and search boxes.

## Focus

Every element of the keyboard is non-focusable (`IsTabStop=false`, `AllowFocusOnInteraction=false`), so tapping keys
never takes focus away from the edited field. Custom UI you add to the keyboard should follow the same rule — use
`ScreenKeyboard.Controls.Primitives.TouchButton` instead of `Button`.
