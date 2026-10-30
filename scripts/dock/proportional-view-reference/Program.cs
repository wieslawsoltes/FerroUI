// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System;
using Avalonia;
using Avalonia.Headless;
using Avalonia.Themes.Fluent;
using DockReactiveUISample.Views;

sealed class ProbeApp : Application
{
    public override void Initialize() => Styles.Add(new FluentTheme());
}

static class Program
{
    [STAThread]
    static void Main()
    {
        AppBuilder.Configure<ProbeApp>()
            .UseHeadless(new AvaloniaHeadlessPlatformOptions { UseHeadlessDrawing = true })
            .SetupWithoutStarting();
        try
        {
            _ = new ProportionalStackPanelView();
        }
        catch (NullReferenceException)
        {
            Console.WriteLine("construction\tNullReferenceException");
            return;
        }
        throw new InvalidOperationException("The original view no longer rejects its null ItemsSource; review the porting contract.");
    }
}
