using ScreenKeyboard.Controls;
using ScreenKeyboard.Settings;

namespace ScreenKeyboard.Sample;

public sealed partial class MainPage : Page
{
    public MainPage()
    {
        this.InitializeComponent();

        var keyboard = Host.Keyboard;
        SampleStorage.Load(keyboard.Engine);
        SampleOptions.Apply(keyboard.Engine.Settings);
        SettingsPane.Attach(keyboard);

        keyboard.SettingsRequested += (_, _) => Split.IsPaneOpen = true;
        keyboard.EnterActionRequested += (_, e) =>
        {
            StatusText.Text = $"Enter action '{e.Action}' requested by {(e.Element as FrameworkElement)?.Name ?? e.Element?.GetType().Name}.";
            if (e.Action is Editor.EnterAction.Search or Editor.EnterAction.Send or Editor.EnterAction.Go)
            {
                e.Handled = true;
            }
        };
        keyboard.TargetChanged += (_, _) =>
        {
            var element = keyboard.TargetElement as FrameworkElement;
            StatusText.Text = element is null ? string.Empty : $"Typing into: {(element as TextBox)?.Header ?? (element as PasswordBox)?.Header ?? element.GetType().Name}";
        };

        SizeChanged += (_, e) =>
        {
            // Overlay the settings pane on narrow windows.
            Split.DisplayMode = e.NewSize.Width < 900 ? SplitViewDisplayMode.Overlay : SplitViewDisplayMode.Inline;
            if (e.NewSize.Width < 900 && e.PreviousSize.Width == 0)
            {
                Split.IsPaneOpen = false;
            }
        };

        Loaded += (_, _) =>
        {
            if (SampleOptions.Get("settings") == "false")
            {
                Split.IsPaneOpen = false;
            }
            var focus = SampleOptions.Get("focus") is { } name ? FindName(name) as Control : null;
            (focus ?? MessageBox).Focus(FocusState.Programmatic);
        };
        Unloaded += (_, _) => SampleStorage.Save(keyboard.Engine);
        keyboard.Engine.Settings.PropertyChanged += (_, _) => SampleStorage.SaveSettings(keyboard.Engine.Settings);
        keyboard.Engine.EmojiHistory.Changed += (_, _) => SampleStorage.Save(keyboard.Engine);
    }
}
