// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System;
using System.Globalization;
using System.IO;
using System.Linq;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Dock.Avalonia.Controls;

if (args.Length != 1) throw new ArgumentException("Pass the output corpus path.");
CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
AppBuilder.Configure<Application>().UseHeadless(new AvaloniaHeadlessPlatformOptions { UseHeadlessDrawing = true }).SetupWithoutStarting();
using StreamWriter output = new(args[0]);
output.WriteLine("# Actual original DelayedUniformTabPanel; Avalonia 12.1.3; .NET " + Environment.Version);
output.WriteLine("# count\twidth\tmax\tmin\tspacing\thidden\tchild-min\tdesired\tx,width pairs");
foreach (int count in new[] { 0, 1, 3, 7 })
    foreach (double width in new[] { 79d, 500d, 1000d, double.PositiveInfinity })
        Capture(count, width, 220, 80, 4, -1, 0);
Capture(3, 500.5, 220, 80, 4, -1, 0);
Capture(3, 500, 50, 80, 4, -1, 0);
Capture(3, 100, 220, 80, 4, -1, 0);
Capture(3, 500, 220, 300, 4, -1, 0);
Capture(3, 500, 220, 80, -6, -1, 0);
Capture(3, 500, 220, 80, 4, 1, 0);
Capture(3, 500, 220, 80, 4, -1, 200);
Capture(1, 0, 220, 80, 0, -1, 0);
void Capture(int count, double width, double max, double min, double spacing, int hidden, double childMin)
{
    DelayedUniformTabPanel panel = new() { MaxTabWidth = max, MinTabWidth = min, ItemSpacing = spacing, ExpansionDelay = TimeSpan.Zero };
    for (int i = 0; i < count; i++) panel.Children.Add(new Border { Height = 32, IsVisible = i != hidden, MinWidth = i == 0 ? childMin : 0 });
    panel.Measure(new Size(width, 32));
    panel.Arrange(new Rect(0, 0, double.IsFinite(width) ? width : panel.DesiredSize.Width, 32));
    string bounds = string.Join(";", panel.Children.Select(child => $"{child.Bounds.X:R},{child.Bounds.Width:R}"));
    if (bounds.Length == 0) bounds = "-";
    output.WriteLine($"{count}\t{width:R}\t{max:R}\t{min:R}\t{spacing:R}\t{hidden}\t{childMin:R}\t{panel.DesiredSize.Width:R}\t{bounds}");
}
