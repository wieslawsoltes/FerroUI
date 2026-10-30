// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System;
using System.Globalization;
using System.IO;
using System.Linq;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Headless;
using Avalonia.Input;
using Avalonia.Media;
using Dock.Avalonia.Controls;
using Dock.Avalonia.Diagnostics;
using Dock.Avalonia.Diagnostics.Controls;
using Dock.Avalonia.Diagnostics.Helpers;
using Dock.Settings;

if (args.Length != 1) throw new ArgumentException("Pass the output corpus path.");
CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
AppBuilder.Configure<Application>().UseHeadless(new AvaloniaHeadlessPlatformOptions {UseHeadlessDrawing=true}).SetupWithoutStarting();
Canvas root = new();
Border target = new() {Width=80, Height=35, Background=Brushes.Transparent};
Canvas.SetLeft(target,17); Canvas.SetTop(target,23);root.Children.Add(target);
VisualLayerManager layers = new() {Child=root};
Window window = new() {Width=400,Height=300,Content=layers};window.Show();window.UpdateLayout();
AdornerLayer layer = AdornerLayer.GetAdornerLayer(root) ?? throw new InvalidOperationException("Source layer.");
DebugOverlayHelper helper = new(); helper.AddOverlay(root);
DebugOverlayAdorner adorner = layer.Children.OfType<DebugOverlayAdorner>().Single();
using StreamWriter output = new(args[0]);
output.WriteLine("# Actual unchanged DebugOverlayAdorner.Render; .NET " + Environment.Version);
output.WriteLine("# flags\thover\tstroke-color,thickness,fill-color,x,y,width,height rectangles in source order");
for (int mask=0;mask<8;mask++)
    foreach (bool hover in new[] {false,true})
    {
        DockProperties.SetIsDockTarget(target,(mask&1)!=0);DockProperties.SetIsDragArea(target,(mask&2)!=0);DockProperties.SetIsDropArea(target,(mask&4)!=0);
        adorner.SetPointerOver(hover ? target : null);
        DrawingGroup drawing = new();
        using (DrawingContext context=drawing.Open()) adorner.Render(context);
        string rectangles=string.Join(";",drawing.Children.OfType<GeometryDrawing>().Select(shape=>
            $"{((ISolidColorBrush)shape.Pen!.Brush!).Color.ToUInt32():X8},{shape.Pen.Thickness:R},{(shape.Brush is ISolidColorBrush fill ? fill.Color.ToUInt32().ToString("X8") : "-")},{shape.Geometry!.Bounds.X:R},{shape.Geometry.Bounds.Y:R},{shape.Geometry.Bounds.Width:R},{shape.Geometry.Bounds.Height:R}"));
        output.WriteLine($"{mask}\t{(hover?1:0)}\t{(rectangles.Length==0 ? "-" : rectangles)}");
    }
helper.RemoveOverlay(root);
if (layer.Children.OfType<DebugOverlayAdorner>().Any()) throw new InvalidOperationException("Source removal.");
DockControl first = new(); root.Children.Add(first);window.UpdateLayout();
using IDisposable cleanup=window.AttachDockDebugOverlay();
Press(Key.F8); if (Count()!=0) throw new InvalidOperationException("Wrong source hotkey.");
Press(Key.F9); if (Count()!=1) throw new InvalidOperationException("Source enable.");
DockControl second = new();root.Children.Add(second);window.UpdateLayout(); if (Count()!=2) throw new InvalidOperationException("Source dynamic attach.");
root.Children.Remove(second);window.UpdateLayout(); if (Count()!=2) throw new InvalidOperationException("Source detached helper leaves its old-layer adorner.");
Press(Key.F9); if (Count()!=1) throw new InvalidOperationException("Source disable leaves the orphan.");
Press(Key.F9);cleanup.Dispose(); if (Count()!=1) throw new InvalidOperationException("Source explicit disposal leaves the orphan.");
Press(Key.F9);if (Count()!=1) throw new InvalidOperationException("Source removed handler.");
window.Close();
Console.WriteLine("Captured 16 original render cases; original hotkey/dynamic docking/disposal assertions passed.");
int Count() => layer.Children.OfType<DebugOverlayAdorner>().Count();
void Press(Key key) => window.RaiseEvent(new KeyEventArgs {RoutedEvent=InputElement.KeyDownEvent,Key=key});
