// Hello, World! in C# through the generated bindings of FerroUI (bindings/README.md): a window
// with a text and a button whose clicks change its text, the C# version of hello_world.c.
//
//   make -C samples/CApi run-csharp
//
// FERROUI_SMOKE_EXIT_MS=<n> closes the window after n milliseconds.

using System;
using FerroUI.Bindings;

internal static class Program
{
    private static int Main()
    {
        try
        {
            Ferro.RunApp(Start);
            return 0;
        }
        catch (FerroException error)
        {
            Console.Error.WriteLine(error.Message);
            return 1;
        }
    }

    private static void Start()
    {
        var text = TextBlock.Create().Text("Hello, World!").FontSize(28);

        var clicks = 0;
        var button = Button.Create().HorizontalAlignment(HorizontalAlignment.Center).Content("Click me");
        button.OnClick(sender =>
        {
            clicks++;
            sender.Content(clicks == 1 ? "Clicked 1 time" : $"Clicked {clicks} times");
        });

        var panel = StackPanel.Create()
            .HorizontalAlignment(HorizontalAlignment.Center)
            .VerticalAlignment(VerticalAlignment.Center)
            .Spacing(12)
            .AddChild(text)
            .AddChild(button);

        var window = Window.Create().Title("Hello, World!").Width(400).Height(240).AddChild(panel).Show();

        // The window holds its tree and the platform holds the open window.
        text.Release();
        button.Release();
        panel.Release();
        window.Release();
    }
}
