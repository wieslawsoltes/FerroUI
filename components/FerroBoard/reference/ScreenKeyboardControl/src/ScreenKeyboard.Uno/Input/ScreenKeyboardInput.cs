using Microsoft.UI.Xaml;
using ScreenKeyboard.Editor;

namespace ScreenKeyboard.Controls;

/// <summary>
/// Attached properties that configure how the on-screen keyboard treats an input element.
/// </summary>
/// <example>
/// <code language="xml"><![CDATA[
/// <TextBox sk:ScreenKeyboardInput.InputKind="Email" sk:ScreenKeyboardInput.EnterAction="Next" />
/// <TextBox sk:ScreenKeyboardInput.IsEnabled="False" />
/// ]]></code>
/// </example>
public static class ScreenKeyboardInput
{
    /// <summary>Overrides the input kind (<see cref="Editor.InputKind"/> value or its name).</summary>
    public static readonly DependencyProperty InputKindProperty = DependencyProperty.RegisterAttached(
        "InputKind", typeof(object), typeof(ScreenKeyboardInput), new PropertyMetadata(null));

    /// <summary>Overrides the enter key action (<see cref="Editor.EnterAction"/> value or its name).</summary>
    public static readonly DependencyProperty EnterActionProperty = DependencyProperty.RegisterAttached(
        "EnterAction", typeof(object), typeof(ScreenKeyboardInput), new PropertyMetadata(null));

    /// <summary>Overrides capitalization (<see cref="CapitalizationMode"/> value or its name).</summary>
    public static readonly DependencyProperty CapitalizationProperty = DependencyProperty.RegisterAttached(
        "Capitalization", typeof(object), typeof(ScreenKeyboardInput), new PropertyMetadata(null));

    /// <summary>Disables suggestions and auto-correction when <c>false</c>.</summary>
    public static readonly DependencyProperty SuggestionsEnabledProperty = DependencyProperty.RegisterAttached(
        "SuggestionsEnabled", typeof(bool), typeof(ScreenKeyboardInput), new PropertyMetadata(true));

    /// <summary>Marks a field as private: nothing typed is learned or stored.</summary>
    public static readonly DependencyProperty IsPrivateProperty = DependencyProperty.RegisterAttached(
        "IsPrivate", typeof(bool), typeof(ScreenKeyboardInput), new PropertyMetadata(false));

    /// <summary>When <c>false</c> the keyboard does not attach to (or open for) the element.</summary>
    public static readonly DependencyProperty IsEnabledProperty = DependencyProperty.RegisterAttached(
        "IsEnabled", typeof(bool), typeof(ScreenKeyboardInput), new PropertyMetadata(true));

    public static object? GetInputKind(DependencyObject element) => element.GetValue(InputKindProperty);
    public static void SetInputKind(DependencyObject element, object? value) => element.SetValue(InputKindProperty, value);
    public static object? GetEnterAction(DependencyObject element) => element.GetValue(EnterActionProperty);
    public static void SetEnterAction(DependencyObject element, object? value) => element.SetValue(EnterActionProperty, value);
    public static object? GetCapitalization(DependencyObject element) => element.GetValue(CapitalizationProperty);
    public static void SetCapitalization(DependencyObject element, object? value) => element.SetValue(CapitalizationProperty, value);
    public static bool GetSuggestionsEnabled(DependencyObject element) => (bool)element.GetValue(SuggestionsEnabledProperty);
    public static void SetSuggestionsEnabled(DependencyObject element, bool value) => element.SetValue(SuggestionsEnabledProperty, value);
    public static bool GetIsPrivate(DependencyObject element) => (bool)element.GetValue(IsPrivateProperty);
    public static void SetIsPrivate(DependencyObject element, bool value) => element.SetValue(IsPrivateProperty, value);
    public static bool GetIsEnabled(DependencyObject element) => (bool)element.GetValue(IsEnabledProperty);
    public static void SetIsEnabled(DependencyObject element, bool value) => element.SetValue(IsEnabledProperty, value);

    internal static T? GetEnum<T>(object? value) where T : struct, Enum => value switch
    {
        T typed => typed,
        string s when Enum.TryParse<T>(s, true, out var parsed) => parsed,
        _ => null,
    };

    /// <summary>
    /// Applies the attached property overrides of <paramref name="element"/> (or its templated parent chain) to attributes.
    /// </summary>
    internal static InputAttributes Apply(DependencyObject element, InputAttributes attributes)
    {
        // Controls like NumberBox/AutoSuggestBox host an inner TextBox: honor properties set on the outer control.
        var current = element;
        for (var depth = 0; current is not null && depth < 6; depth++)
        {
            if (GetEnum<InputKind>(GetInputKind(current)) is { } kind)
            {
                attributes = attributes with { Kind = kind };
                break;
            }
            current = Microsoft.UI.Xaml.Media.VisualTreeHelper.GetParent(current);
        }
        if (GetEnum<EnterAction>(GetEnterAction(element)) is { } action)
        {
            attributes = attributes with { EnterAction = action };
        }
        if (GetEnum<CapitalizationMode>(GetCapitalization(element)) is { } cap)
        {
            attributes = attributes with { Capitalization = cap };
        }
        if (!GetSuggestionsEnabled(element))
        {
            attributes = attributes with { AllowSuggestions = false, AllowAutoCorrect = false };
        }
        if (GetIsPrivate(element))
        {
            attributes = attributes with { IsPrivate = true };
        }
        return attributes;
    }

    /// <summary>Returns <c>false</c> if the keyboard is disabled for the element or one of its ancestors.</summary>
    internal static bool IsEnabledFor(DependencyObject element)
    {
        var current = element;
        for (var depth = 0; current is not null && depth < 8; depth++)
        {
            if (!GetIsEnabled(current))
            {
                return false;
            }
            current = Microsoft.UI.Xaml.Media.VisualTreeHelper.GetParent(current);
        }
        return true;
    }
}
