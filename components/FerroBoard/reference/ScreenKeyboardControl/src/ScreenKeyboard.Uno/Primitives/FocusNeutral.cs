using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Controls;

namespace ScreenKeyboard.Controls.Primitives;

/// <summary>
/// Makes keyboard UI "focus neutral": interacting with it must never move keyboard focus away from the text field
/// being edited. Uno Platform moves focus when a pointer presses an element that allows focus on interaction.
/// </summary>
internal static class FocusNeutral
{
    /// <summary>Disables focus-on-interaction for <paramref name="element"/> and all of its logical descendants.</summary>
    public static T Apply<T>(T element) where T : DependencyObject
    {
        Walk(element, 0);
        return element;
    }

    private static void Walk(DependencyObject? element, int depth)
    {
        if (element is null || depth > 64)
        {
            return;
        }
        if (element is FrameworkElement fe)
        {
            fe.AllowFocusOnInteraction = false;
        }
        if (element is Control control)
        {
            control.IsTabStop = false;
        }
        switch (element)
        {
            case Panel panel:
                foreach (var child in panel.Children)
                {
                    Walk(child, depth + 1);
                }
                break;
            case Border border:
                Walk(border.Child, depth + 1);
                break;
            case ScrollViewer scrollViewer:
                Walk(scrollViewer.Content as DependencyObject, depth + 1);
                break;
            case UserControl userControl:
                Walk(userControl.Content as DependencyObject, depth + 1);
                break;
            case ContentControl contentControl:
                Walk(contentControl.Content as DependencyObject, depth + 1);
                break;
        }
    }
}
