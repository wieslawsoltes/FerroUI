// The tests of the C# bindings over the library (no window, no display):
//
//   make -C samples/CApi test-csharp CARGO_FLAGS="--no-default-features --features type-tables"

using System;
using FerroUI.Bindings;

internal static class Program
{
    private static int checks;

    private static void Check(bool condition, string what)
    {
        checks++;
        if (!condition)
        {
            Console.Error.WriteLine("failed: " + what);
            Environment.Exit(1);
        }
    }

    private static void TestValues()
    {
        // Writes chain on the type of the object, through the members of its bases.
        var text = TextBlock.Create().Text("Hello").FontSize(20).Opacity(0.5).IsVisible(false);
        Check(text.Text() == "Hello", "text");
        Check(text.FontSize() == 20, "font size");
        Check(text.Opacity() == 0.5, "a property of a base");
        Check(!text.IsVisible(), "a boolean");

        var longer = new string('x', 500);
        text.Text(longer);
        Check(text.Text() == longer, "a text longer than the first buffer");
        text.Text("Zażółć");
        Check(text.Text() == "Zażółć", "UTF-8 and UTF-16");
        text.Text("");
        Check(text.Text() == "", "an empty text");
        text.Release();

        var button = Button.Create().HorizontalAlignment(HorizontalAlignment.Center);
        Check(button.HorizontalAlignment() == HorizontalAlignment.Center, "an enumeration");
        button.Release();
    }

    private static void TestContentAndChildren()
    {
        var button = Button.Create().Content("Click me");
        Check(button.Content() == "Click me", "content as text");

        var inner = TextBlock.Create();
        button.Content(inner);
        var held = button.ContentObject();
        Check(held.Handle == inner.Handle, "content as an object");
        held.Release();

        var panel = StackPanel.Create().Spacing(4);
        var child = Border.Create();
        panel.AddChild(child);
        panel.RemoveChild(child);
        button.Release();
        inner.Release();
        panel.Release();
        child.Release();
    }

    private static void TestAttachedProperties()
    {
        var grid = Grid.Create();
        var child = Border.Create();
        grid.AddChild(child);
        Grid.SetRow(child, 2);
        Grid.SetColumn(child, 1);
        Check(Grid.GetRow(child) == 2, "attached row");
        Check(Grid.GetColumn(child) == 1, "attached column");
        child.Release();
        grid.Release();
    }

    private static void TestWrappers()
    {
        var border = Border.Create().Width(10);
        var control = ((IFerroObject)border).As<Control>();
        Check(control.Handle == border.Handle, "the same handle");
        Check(control.Width() == 10, "through another wrapper");
        border.Release();
    }

    private static void TestEvents()
    {
        var button = Button.Create();
        var click = Ferro.EventId("Click");
        var clicks = 0;
        button.OnClick(sender =>
        {
            clicks++;
            sender.Content("Clicked");
        });
        Ferro.Raise(button, click);
        Check(clicks == 1, "the handler ran");
        Check(button.Content() == "Clicked", "with its sender");

        var counted = 0;
        using (button.OnClick(_ => counted++))
        {
            Ferro.Raise(button, click);
            Check(counted == 1, "a second handler");
        }
        Ferro.Raise(button, click);
        Check(counted == 1, "disposed");
        Check(clicks == 3, "the first one still runs");

        // What a handler throws stops at the boundary of the C ABI.
        button.OnClick(_ => throw new InvalidOperationException("from the handler"));
        Ferro.Raise(button, click);
        Check(clicks == 4, "after a throw");
        button.Release();
    }

    private static void TestErrors()
    {
        var border = Border.Create();
        var thrown = false;
        try
        {
            // `Text` is not a property of a border.
            ((IFerroObject)border).As<TextBlock>().Text("no");
        }
        catch (FerroException error)
        {
            thrown = true;
            Check(error.Message.Contains("Text"), "with the message of the library");
        }
        Check(thrown, "a typed error");
        border.Release();
    }

    private static int Main()
    {
        Ferro.Init();
        Ferro.CheckLibrary();
        TestValues();
        TestContentAndChildren();
        TestAttachedProperties();
        TestWrappers();
        TestEvents();
        TestErrors();
        Console.WriteLine($"ok: {checks} checks");
        return 0;
    }
}
