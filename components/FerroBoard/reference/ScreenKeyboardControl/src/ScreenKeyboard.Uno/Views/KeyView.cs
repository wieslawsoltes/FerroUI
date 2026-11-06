using Microsoft.UI.Text;
using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Automation;
using Microsoft.UI.Xaml.Controls;
using Microsoft.UI.Xaml.Media;
using ScreenKeyboard.Controls.Icons;
using ScreenKeyboard.Controls.Primitives;
using ScreenKeyboard.Controls.Themes;
using ScreenKeyboard.Keys;
using ScreenKeyboard.Layouts;
using Windows.UI;

namespace ScreenKeyboard.Controls.Views;

/// <summary>Resolved visual style of a key.</summary>
internal readonly record struct KeyStyle(Color Background, Color Foreground, double FontSize, double CornerRadius);

/// <summary>Helpers resolving theme values for keys.</summary>
internal static class KeyStyles
{
    public static bool IsFunctionKey(ComputedKey key) =>
        key.Data.Type is not (KeyType.Character or KeyType.Numeric) && !key.Data.IsSpace ||
        key.Data.Code == KeyCode.Enter;

    public static KeyStyle Resolve(KeyboardTheme theme, ComputedKey key, KeyboardMode mode, ShiftState shift, bool pressed, double fontScale)
    {
        var function = IsFunctionKey(key);
        var background = function
            ? pressed ? theme.FunctionKeyPressedBackground : theme.FunctionKeyBackground
            : pressed ? theme.KeyPressedBackground : theme.KeyBackground;
        var foreground = function ? theme.FunctionKeyForeground : theme.KeyForeground;
        var label = key.Label ?? string.Empty;
        var fontSize = key.Data.IsSpace ? theme.SpaceFontSize
            : label.Length > 1 && TextUtils.GetCodePoints(label).Length > 1 ? theme.FunctionKeyFontSize
            : theme.KeyFontSize;
        if (key.Data.IsSpace)
        {
            foreground = theme.SpaceKeyForeground;
        }
        if (key.Code == KeyCode.Enter)
        {
            background = pressed ? theme.AccentKeyPressedBackground : theme.AccentKeyBackground;
            foreground = theme.AccentKeyForeground;
        }
        if (key.Code == KeyCode.Shift && shift == ShiftState.CapsLock)
        {
            foreground = theme.CapsLockForeground;
        }
        var corner = theme.KeyCornerRadius;
        foreach (var rule in theme.KeyRules)
        {
            if (!rule.Matches(key.Code, key.Type, mode, shift))
            {
                continue;
            }
            if (pressed && rule.PressedBackground is { } pb)
            {
                background = pb;
            }
            else if (rule.Background is { } bg && (!pressed || rule.PressedBackground is null))
            {
                background = pressed ? Darken(bg) : bg;
            }
            if (rule.Foreground is { } fg) foreground = fg;
            if (rule.FontSize is { } fs) fontSize = fs;
            if (rule.CornerRadius is { } cr) corner = cr;
        }
        return new KeyStyle(background, foreground, fontSize * fontScale, corner);
    }

    public static Color Darken(Color c, double factor = 0.85) =>
        Color.FromArgb(c.A, (byte)(c.R * factor), (byte)(c.G * factor), (byte)(c.B * factor));
}

/// <summary>Visual representation of one key.</summary>
internal sealed partial class KeyView : Grid
{
    private readonly Border _shadow = new() { IsHitTestVisible = false };
    private readonly Border _face = new() { IsHitTestVisible = false };
    private readonly Grid _faceContent = new() { IsHitTestVisible = false };
    private readonly TextBlock _label = new()
    {
        HorizontalAlignment = HorizontalAlignment.Center,
        VerticalAlignment = VerticalAlignment.Center,
        TextAlignment = TextAlignment.Center,
        TextTrimming = TextTrimming.CharacterEllipsis,
        TextWrapping = TextWrapping.NoWrap,
        IsHitTestVisible = false,
    };
    private readonly TextBlock _hint = new()
    {
        HorizontalAlignment = HorizontalAlignment.Right,
        VerticalAlignment = VerticalAlignment.Top,
        Margin = new Thickness(0, 2, 4, 0),
        IsHitTestVisible = false,
    };
    private readonly TextBlock _subLabel = new()
    {
        HorizontalAlignment = HorizontalAlignment.Center,
        VerticalAlignment = VerticalAlignment.Bottom,
        IsHitTestVisible = false,
    };
    private KeyIcon _iconKind = KeyIcon.None;
    private double _iconSize;
    private Microsoft.UI.Xaml.Shapes.Path? _icon;

    public KeyView(ComputedKey key)
    {
        Key = key;
        IsHitTestVisible = false;
        IsTabStop = false;
        _faceContent.Children.Add(_label);
        _faceContent.Children.Add(_hint);
        _faceContent.Children.Add(_subLabel);
        _face.Child = _faceContent;
        Children.Add(_shadow);
        Children.Add(_face);
    }

    public ComputedKey Key { get; }

    public bool IsPressed { get; set; }

    public void Update(KeyboardTheme theme, KeyboardMode mode, ShiftState shift, double fontScale, FontFamily? fontFamily)
    {
        var key = Key;
        var bounds = key.VisibleBounds;
        Width = Math.Max(0, bounds.Width);
        Height = Math.Max(0, bounds.Height);
        Canvas.SetLeft(this, bounds.X);
        Canvas.SetTop(this, bounds.Y);
        Visibility = key.IsVisible ? Visibility.Visible : Visibility.Collapsed;

        var style = KeyStyles.Resolve(theme, key, mode, shift, IsPressed, fontScale);
        var corner = new CornerRadius(Math.Min(style.CornerRadius, Math.Min(bounds.Width, bounds.Height) / 2));
        _face.Background = Brushes.Get(style.Background);
        _face.CornerRadius = corner;
        _face.BorderBrush = Brushes.Get(theme.KeyBorderColor);
        _face.BorderThickness = new Thickness(theme.KeyBorderThickness);

        var shadowVisible = theme.KeyShadowDepth > 0 && theme.KeyShadowColor.A > 0 && style.Background.A > 0;
        _shadow.Visibility = shadowVisible ? Visibility.Visible : Visibility.Collapsed;
        if (shadowVisible)
        {
            _shadow.Background = Brushes.Get(theme.KeyShadowColor);
            _shadow.CornerRadius = corner;
            _shadow.Margin = new Thickness(0, theme.KeyShadowDepth, 0, -theme.KeyShadowDepth);
        }

        var foreground = Brushes.Get(style.Foreground);
        var maxFont = Math.Max(8, bounds.Height * 0.52);
        if (key.Icon != KeyIcon.None && IconFactory.HasIcon(key.Icon))
        {
            _label.Visibility = Visibility.Collapsed;
            var size = Math.Min(theme.IconSize * fontScale, Math.Max(10, Math.Min(bounds.Height, bounds.Width) * 0.6));
            if (_icon is null || _iconKind != key.Icon || Math.Abs(_iconSize - size) > 0.1)
            {
                if (_icon is not null)
                {
                    _faceContent.Children.Remove(_icon);
                }
                _icon = IconFactory.Create(key.Icon, size, foreground);
                _icon.HorizontalAlignment = HorizontalAlignment.Center;
                _icon.VerticalAlignment = VerticalAlignment.Center;
                _faceContent.Children.Insert(0, _icon);
                _iconKind = key.Icon;
                _iconSize = size;
            }
            _icon.Fill = foreground;
            _icon.Visibility = Visibility.Visible;
        }
        else
        {
            if (_icon is not null)
            {
                _icon.Visibility = Visibility.Collapsed;
            }
            _label.Visibility = Visibility.Visible;
            _label.Text = key.Label ?? string.Empty;
            _label.FontSize = Math.Min(style.FontSize, key.Data.IsSpace ? maxFont * 0.7 : maxFont);
            _label.Foreground = foreground;
            _label.MaxWidth = Math.Max(0, bounds.Width - 4);
            if (fontFamily is not null)
            {
                _label.FontFamily = fontFamily;
            }
        }

        if (!string.IsNullOrEmpty(key.HintLabel) && bounds.Height > 30)
        {
            _hint.Visibility = Visibility.Visible;
            _hint.Text = key.HintLabel;
            _hint.FontSize = Math.Min(theme.HintFontSize * fontScale, bounds.Height * 0.3);
            _hint.Foreground = Brushes.Get(theme.KeyHintForeground);
            if (fontFamily is not null)
            {
                _hint.FontFamily = fontFamily;
            }
        }
        else
        {
            _hint.Visibility = Visibility.Collapsed;
        }

        if (!string.IsNullOrEmpty(key.SubLabel) && bounds.Height > 30)
        {
            _subLabel.Visibility = Visibility.Visible;
            _subLabel.Text = key.SubLabel;
            _subLabel.FontSize = Math.Min(theme.HintFontSize * fontScale, bounds.Height * 0.22);
            _subLabel.Foreground = Brushes.Get(theme.KeyHintForeground);
            _subLabel.Margin = new Thickness(0, 0, 0, Math.Max(1, bounds.Height * 0.06));
            _label.Margin = new Thickness(0, 0, 0, bounds.Height * 0.18);
        }
        else
        {
            _subLabel.Visibility = Visibility.Collapsed;
            _label.Margin = default;
        }

        AutomationProperties.SetName(this, GetAutomationName(key));
    }

    internal static string GetAutomationName(ComputedKey key) => key.Code switch
    {
        KeyCode.Shift => KeyboardStrings.Shift,
        KeyCode.Delete => KeyboardStrings.Backspace,
        KeyCode.Enter => KeyboardStrings.Enter,
        KeyCode.Space => KeyboardStrings.Space,
        KeyCode.LanguageSwitch => KeyboardStrings.SwitchLanguage,
        KeyCode.ImeUiModeMedia => KeyboardStrings.Emoji,
        _ => string.IsNullOrEmpty(key.Label) ? key.Icon.ToString() : key.Label!,
    };
}

/// <summary>Font weight helper.</summary>
internal static class FontWeightsEx
{
    public static readonly Windows.UI.Text.FontWeight SemiBold = new() { Weight = 600 };
}
