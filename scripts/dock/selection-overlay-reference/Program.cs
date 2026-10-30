// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System;
using System.Collections.Generic;
using System.Collections.Specialized;
using System.Globalization;
using System.IO;
using System.Linq;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Primitives;
using Avalonia.Headless;
using Dock.Avalonia.Controls;
using Dock.Avalonia.Diagnostics.Helpers;
using Dock.Avalonia.Diagnostics.Controls;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;

if (args.Length != 1) throw new ArgumentException("Pass the output corpus path.");
CultureInfo.CurrentCulture = CultureInfo.InvariantCulture;
AppBuilder.Configure<Application>().UseHeadless(new AvaloniaHeadlessPlatformOptions { UseHeadlessDrawing = true }).SetupWithoutStarting();
using StreamWriter output = new(args[0]);
output.WriteLine("# Actual unchanged SelectionOverlayHelper.TryGetControl; .NET " + Environment.Version);
output.WriteLine("# visible-state\tpinned-state\ttab-state\troot\tcontrol");
for (int visible = 0; visible < 6; visible++)
    for (int pinned = 0; pinned < 6; pinned++)
        for (int tab = 0; tab < 6; tab++)
        {
            Factory factory = new();
            IDockable model = new Tool();
            int[] states = [visible, pinned, tab];
            for (int mode = 0; mode < 3; mode++)
            {
                (IDictionary<IDockable, IDockableControl> controls, IDictionary<IDockable, object> roots) = mode switch
                {
                    0 => (factory.VisibleDockableControls, factory.VisibleRootControls),
                    1 => (factory.PinnedDockableControls, factory.PinnedRootControls),
                    _ => (factory.TabDockableControls, factory.TabRootControls),
                };
                string name = mode.ToString(CultureInfo.InvariantCulture);
                switch (states[mode])
                {
                    case 0: break;
                    case 1: controls[model] = new DockableControl { Name = name }; break;
                    case 2: roots[model] = new Border { Name = name }; break;
                    case 3:
                        controls[model] = new DockableControl { Name = name };
                        roots[model] = "not a control";
                        break;
                    case 4:
                        controls[model] = new DockableControl { Name = name };
                        roots[model] = new Border { Name = name };
                        break;
                    case 5:
                        controls[model] = new NonControl();
                        roots[model] = new Border { Name = name };
                        break;
                }
            }
            bool found = SelectionOverlayHelper.TryGetControl(factory, model, out Control? root, out Control? control);
            if (!found && (root is not null || control is not null)) throw new InvalidOperationException("Source failure must clear both outputs.");
            output.WriteLine($"{visible}\t{pinned}\t{tab}\t{root?.Name ?? "-"}\t{control?.Name ?? "-"}");
        }

Canvas canvas = new();
Border target = new() { Width = 80, Height = 35 };
Canvas.SetLeft(target, 17);
Canvas.SetTop(target, 23);
canvas.Children.Add(target);
VisualLayerManager layers = new() { Child = canvas };
Window window = new() { Width = 400, Height = 300, Content = layers };
window.Show();
window.UpdateLayout();
AdornerLayer layer = AdornerLayer.GetAdornerLayer(canvas) ?? throw new InvalidOperationException("Missing source adorner layer.");
SelectionOverlayHelper helper = new();
helper.AttachOverlay(canvas);
SelectionOverlayAdorner adorner = layer.Children.OfType<SelectionOverlayAdorner>().Single();
helper.Highlight(target);
if (adorner.HighlightRect != new Rect(17, 23, 80, 35)) throw new InvalidOperationException("Initial source highlight.");
Canvas.SetLeft(target, 42);
target.Width = 105;
window.UpdateLayout();
if (adorner.HighlightRect != new Rect(42, 23, 105, 35)) throw new InvalidOperationException("Live source highlight.");
helper.Highlight(new Border());
if (adorner.HighlightRect is not null) throw new InvalidOperationException("Detached target must clear the source highlight.");
helper.RemoveOverlay();
if (adorner.Parent is not null || layer.Children.Contains(adorner)) throw new InvalidOperationException("Source overlay removal.");
if (AdornerLayer.GetAdornedElement(adorner) != canvas) throw new InvalidOperationException("Source removal must retain the attached adorned value.");
helper.AttachOverlay(canvas);
adorner = layer.Children.OfType<SelectionOverlayAdorner>().Single();
layers.Child = null;
if (AdornerLayer.GetAdornerLayer(canvas) is not null) throw new InvalidOperationException("Detached source root must have no layer.");
helper.RemoveOverlay();
if (adorner.Parent is not null || !layer.Children.Contains(adorner)) throw new InvalidOperationException("Source detached removal must leave the old layer child.");
layer.Children.Remove(adorner);
layers.Child = canvas;
window.UpdateLayout();
helper.AttachOverlay(canvas);
NotifyCollectionChangedEventHandler nestedRemoval = (_, e) =>
{
    if (e.Action == NotifyCollectionChangedAction.Remove) helper.RemoveOverlay();
};
layer.Children.CollectionChanged += nestedRemoval;
bool reentryFailed = false;
try { helper.RemoveOverlay(); }
catch (NullReferenceException) { reentryFailed = true; }
finally { layer.Children.CollectionChanged -= nestedRemoval; }
if (!reentryFailed) throw new InvalidOperationException("Source nested removal must fail on cleared fields.");
window.Close();
Console.WriteLine("Captured 216 actual source lookup rows; attached layout/update/detached/removal/reentry source checks passed.");

sealed class NonControl : IDockableControl { }
