using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Input;
using ScreenKeyboard.Editor;

namespace ScreenKeyboard.Controls;

/// <summary>Connects the keyboard to a <see cref="TextBox"/>.</summary>
public sealed class TextBoxInputTarget : ITextInputTarget, IDisposable
{
    private readonly TextBox _textBox;
    private readonly Func<EnterAction, bool>? _enterHandler;
    private string? _expectedText;
    private bool _pendingInternalEdit;

    /// <summary>Creates a target for a text box.</summary>
    /// <param name="textBox">The text box.</param>
    /// <param name="enterHandler">Optional handler for enter actions of single-line fields.</param>
    public TextBoxInputTarget(TextBox textBox, Func<EnterAction, bool>? enterHandler = null)
    {
        _textBox = textBox;
        _enterHandler = enterHandler;
        _textBox.TextChanged += OnTextChanged;
        _textBox.SelectionChanged += OnSelectionChanged;
        Attributes = ComputeAttributes(textBox);
    }

    /// <summary>The text box.</summary>
    public TextBox TextBox => _textBox;

    /// <inheritdoc />
    public InputAttributes Attributes { get; }

    /// <inheritdoc />
    public string Text => _textBox.Text ?? string.Empty;

    /// <inheritdoc />
    public int SelectionStart => Math.Clamp(_textBox.SelectionStart, 0, Text.Length);

    /// <inheritdoc />
    public int SelectionLength => Math.Clamp(_textBox.SelectionLength, 0, Text.Length - SelectionStart);

    /// <inheritdoc />
    public event EventHandler? Changed;

    /// <inheritdoc />
    public void Replace(int start, int length, string text)
    {
        if (_textBox.IsReadOnly || !_textBox.IsEnabled)
        {
            return;
        }
        var current = Text;
        start = Math.Clamp(start, 0, current.Length);
        length = Math.Clamp(length, 0, current.Length - start);
        if (_textBox.MaxLength > 0)
        {
            var available = _textBox.MaxLength - (current.Length - length);
            if (available <= 0 && text.Length > 0)
            {
                return;
            }
            if (text.Length > available)
            {
                text = text.Substring(0, available);
            }
        }
        var updated = current.Remove(start, length).Insert(start, text);
        MarkInternalEdit(updated);
        _textBox.Text = updated;
        _textBox.Select(start + text.Length, 0);
    }

    /// <inheritdoc />
    public void Select(int start, int length)
    {
        var textLength = Text.Length;
        start = Math.Clamp(start, 0, textLength);
        length = Math.Clamp(length, 0, textLength - start);
        MarkInternalEdit(Text);
        _textBox.Select(start, length);
    }

    /// <inheritdoc />
    public bool PerformEnterAction(EnterAction action)
    {
        if (_enterHandler?.Invoke(action) == true)
        {
            return true;
        }
        if (action is EnterAction.Next or EnterAction.Previous)
        {
            var options = new FindNextElementOptions { SearchRoot = _textBox.XamlRoot?.Content };
            return FocusManager.TryMoveFocus(action == EnterAction.Next ? FocusNavigationDirection.Next : FocusNavigationDirection.Previous, options);
        }
        return false;
    }

    private void OnTextChanged(object sender, TextChangedEventArgs e) => RaiseIfExternal();

    private void OnSelectionChanged(object sender, RoutedEventArgs e) => RaiseIfExternal();

    private void MarkInternalEdit(string expectedText)
    {
        _expectedText = expectedText;
        if (_pendingInternalEdit)
        {
            return;
        }
        _pendingInternalEdit = true;
        // TextChanged/SelectionChanged may be raised asynchronously: treat events until the next dispatcher pass
        // as caused by the keyboard as long as the text matches what the keyboard wrote.
        var queued = _textBox.DispatcherQueue?.TryEnqueue(Microsoft.UI.Dispatching.DispatcherQueuePriority.Low, () =>
        {
            _pendingInternalEdit = false;
            _expectedText = null;
        }) ?? false;
        if (!queued)
        {
            _pendingInternalEdit = false;
        }
    }

    private void RaiseIfExternal()
    {
        if (_pendingInternalEdit && _expectedText == Text)
        {
            return;
        }
        Changed?.Invoke(this, EventArgs.Empty);
    }

    /// <summary>Derives <see cref="InputAttributes"/> from a text box (InputScope, AcceptsReturn, attached properties).</summary>
    public static InputAttributes ComputeAttributes(TextBox textBox)
    {
        var kind = InputKind.Text;
        var cap = CapitalizationMode.Sentences;
        var scope = textBox.InputScope?.Names?.FirstOrDefault()?.NameValue;
        switch (scope)
        {
            case InputScopeNameValue.Number:
            case InputScopeNameValue.Digits:
                kind = InputKind.Number;
                break;
            case InputScopeNameValue.CurrencyAmount:
            case InputScopeNameValue.CurrencyAmountAndSymbol:
            case InputScopeNameValue.NumberFullWidth:
            case InputScopeNameValue.Formula:
            case InputScopeNameValue.FormulaNumber:
                kind = InputKind.Decimal;
                break;
            case InputScopeNameValue.TelephoneNumber:
            case InputScopeNameValue.TelephoneAreaCode:
            case InputScopeNameValue.TelephoneCountryCode:
            case InputScopeNameValue.TelephoneLocalNumber:
                kind = InputKind.Phone;
                break;
            case InputScopeNameValue.EmailSmtpAddress:
            case InputScopeNameValue.EmailNameOrAddress:
                kind = InputKind.Email;
                break;
            case InputScopeNameValue.Url:
                kind = InputKind.Uri;
                break;
            case InputScopeNameValue.Search:
            case InputScopeNameValue.SearchIncremental:
                kind = InputKind.Search;
                break;
            case InputScopeNameValue.Chat:
            case InputScopeNameValue.ChatWithoutEmoji:
                kind = InputKind.Chat;
                break;
            case InputScopeNameValue.Password:
                kind = InputKind.Password;
                break;
            case InputScopeNameValue.NumericPin:
            case InputScopeNameValue.NumericPassword:
                kind = InputKind.NumericPassword;
                break;
            case InputScopeNameValue.DateMonthNumber:
            case InputScopeNameValue.DateDayNumber:
            case InputScopeNameValue.DateYear:
            case InputScopeNameValue.TimeHour:
            case InputScopeNameValue.TimeMinutesOrSeconds:
                kind = InputKind.DateTime;
                break;
            case InputScopeNameValue.PersonalFullName:
            case InputScopeNameValue.NameOrPhoneNumber:
                cap = CapitalizationMode.Words;
                break;
            case InputScopeNameValue.AlphanumericHalfWidth:
            case InputScopeNameValue.AlphanumericFullWidth:
                cap = CapitalizationMode.None;
                break;
        }
        var attributes = new InputAttributes
        {
            Kind = kind,
            Capitalization = cap,
            IsMultiline = textBox.AcceptsReturn,
            AllowAutoCorrect = textBox.IsSpellCheckEnabled || textBox.IsTextPredictionEnabled,
            AllowSuggestions = textBox.IsTextPredictionEnabled,
        };
        return ScreenKeyboardInput.Apply(textBox, attributes);
    }

    /// <inheritdoc />
    public void Dispose()
    {
        _textBox.TextChanged -= OnTextChanged;
        _textBox.SelectionChanged -= OnSelectionChanged;
    }
}

/// <summary>Connects the keyboard to a <see cref="PasswordBox"/> (caret is always at the end).</summary>
public sealed class PasswordBoxInputTarget : ITextInputTarget, IDisposable
{
    private readonly PasswordBox _passwordBox;
    private readonly Func<EnterAction, bool>? _enterHandler;
    private string? _expected;
    private int _caret = -1;

    /// <summary>Creates a target for a password box.</summary>
    public PasswordBoxInputTarget(PasswordBox passwordBox, Func<EnterAction, bool>? enterHandler = null)
    {
        _passwordBox = passwordBox;
        _enterHandler = enterHandler;
        _passwordBox.PasswordChanged += OnPasswordChanged;
        var numeric = passwordBox.InputScope?.Names?.FirstOrDefault()?.NameValue is InputScopeNameValue.NumericPin or InputScopeNameValue.NumericPassword or InputScopeNameValue.Number;
        Attributes = ScreenKeyboardInput.Apply(passwordBox, new InputAttributes
        {
            Kind = numeric ? InputKind.NumericPassword : InputKind.Password,
            Capitalization = CapitalizationMode.None,
            AllowSuggestions = false,
            AllowAutoCorrect = false,
            IsPrivate = true,
        });
    }

    /// <inheritdoc />
    public InputAttributes Attributes { get; }

    /// <inheritdoc />
    public string Text => _passwordBox.Password ?? string.Empty;

    /// <inheritdoc />
    public int SelectionStart => _caret < 0 || _caret > Text.Length ? Text.Length : _caret;

    /// <inheritdoc />
    public int SelectionLength => 0;

    /// <inheritdoc />
    public event EventHandler? Changed;

    /// <inheritdoc />
    public void Replace(int start, int length, string text)
    {
        var current = Text;
        start = Math.Clamp(start, 0, current.Length);
        length = Math.Clamp(length, 0, current.Length - start);
        if (_passwordBox.MaxLength > 0)
        {
            var available = _passwordBox.MaxLength - (current.Length - length);
            text = text.Length > available ? text.Substring(0, Math.Max(0, available)) : text;
        }
        var updated = current.Remove(start, length).Insert(start, text);
        _expected = updated;
        _caret = start + text.Length;
        _passwordBox.Password = updated;
    }

    /// <inheritdoc />
    public void Select(int start, int length) => _caret = Math.Clamp(start, 0, Text.Length);

    /// <inheritdoc />
    public bool PerformEnterAction(EnterAction action)
    {
        if (_enterHandler?.Invoke(action) == true)
        {
            return true;
        }
        if (action is EnterAction.Next or EnterAction.Previous)
        {
            var options = new FindNextElementOptions { SearchRoot = _passwordBox.XamlRoot?.Content };
            return FocusManager.TryMoveFocus(action == EnterAction.Next ? FocusNavigationDirection.Next : FocusNavigationDirection.Previous, options);
        }
        return false;
    }

    private void OnPasswordChanged(object sender, RoutedEventArgs e)
    {
        if (_expected == Text)
        {
            return;
        }
        _expected = null;
        _caret = -1;
        Changed?.Invoke(this, EventArgs.Empty);
    }

    /// <inheritdoc />
    public void Dispose() => _passwordBox.PasswordChanged -= OnPasswordChanged;
}
