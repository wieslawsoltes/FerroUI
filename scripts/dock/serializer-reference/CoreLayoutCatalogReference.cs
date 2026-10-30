// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Text.Json;
using Dock.Model.Controls;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
using Dock.Model.ReactiveUI.Core;
using Dock.Serializer.SystemTextJson;

internal static class CoreLayoutCatalogReference
{
    internal static void Run(string path)
    {
        var serializer = new DockSerializer();
        using var checks = new StreamWriter(path + ".tsv");
        void Check(string name, bool passed)
        {
            if (!passed) throw new InvalidOperationException(name);
            checks.WriteLine(name + "\t1");
        }
        void Record<T>(T value, string name)
        {
            var wire = serializer.Serialize(value);
            File.WriteAllText(path + "." + name + ".json", wire);
            var restored = serializer.Deserialize<T>(wire)!;
            Check("catalog-default-" + name, serializer.Serialize(restored) == wire);
        }
        Record(new DockDock(), "dock-dock");
        Record(new Document(), "document");
        Record(new DocumentDock(), "document-dock");
        Record(new GridDock(), "grid-dock");
        Record(new GridDockSplitter(), "grid-dock-splitter");
        Record(new ProportionalDock(), "proportional-dock");
        Record(new ProportionalDockSplitter(), "proportional-dock-splitter");
        Record(new RootDock(), "root-dock");
        Record(new SplitViewDock(), "split-view-dock");
        Record(new StackDock(), "stack-dock");
        Record(new Tool(), "tool");
        Record(new ToolDock(), "tool-dock");
        Record(new UniformGridDock(), "uniform-grid-dock");
        Record(new WrapDock(), "wrap-dock");
        Record(new DockWindow(), "window");

        Record(new RootDock {
            Id = "containers",
            VisibleDockables = new System.Collections.ObjectModel.ObservableCollection<IDockable> {
                new DockDock { Id = "dock", LastChildFill = false },
                new GridDock { Id = "grid", ColumnDefinitions = "Auto,*,2*", RowDefinitions = "100,*" },
                new GridDockSplitter { Id = "grid-splitter", ResizeDirection = GridResizeDirection.Rows },
                new ProportionalDock { Id = "proportional", Orientation = Orientation.Vertical, Proportion = 0.3 },
                new ProportionalDockSplitter { Id = "proportional-splitter", CanResize = false, ResizePreview = true },
                new SplitViewDock { Id = "split-view", CompactPaneLength = 32, DisplayMode = SplitViewDisplayMode.CompactInline, IsPaneOpen = true, OpenPaneLength = 256, PanePlacement = SplitViewPanePlacement.Right, UseLightDismissOverlayMode = true, PaneDockable = new Tool { Id = "pane" }, ContentDockable = new Document { Id = "content" } },
                new StackDock { Id = "stack", Orientation = Orientation.Vertical, Spacing = 12.5 },
                new UniformGridDock { Id = "uniform", Rows = 2, Columns = 3 },
                new WrapDock { Id = "wrap", Orientation = Orientation.Vertical }
            }
        }, "containers");

        var factory = new Factory();
        var document = new Document { Id = "document", Title = "Editor", MdiBounds = new DockRect(10, 20, 400, 300), MdiState = MdiWindowState.Maximized, MdiZIndex = 7, IsModified = true, DockGroup = "workspace", Column = 2, RowSpan = 3, ShowInSelector = false, SelectorTitle = "Editor selector" };
        var documents = new DocumentDock { Id = "documents", Title = "Documents", CanCreateDocument = true, EnableWindowDrag = true, TabsLayout = DocumentTabLayout.Left, VisibleDockables = factory.CreateList<IDockable>(document), ActiveDockable = document, DefaultDockable = document };
        var tool = new Tool { Id = "tool", Title = "Explorer", MdiZIndex = 4, KeepPinnedDockableVisible = true, PinnedBounds = new DockRect(1, 2, 240, 360) };
        var tools = new ToolDock { Id = "tools", Alignment = Alignment.Left, AutoHide = false, GripMode = GripMode.AutoHide, IsExpanded = true, VisibleDockables = factory.CreateList<IDockable>(tool), ActiveDockable = tool };
        var split = new ProportionalDock { Id = "split", Orientation = Orientation.Horizontal, VisibleDockables = factory.CreateList<IDockable>(tools, new ProportionalDockSplitter { Id = "splitter", ResizePreview = true }, documents) };
        var pinned = new Tool { Id = "pinned", Title = "Pinned tool" };
        var hidden = new Tool { Id = "hidden", Title = "Hidden tool" };
        var root = new RootDock { Id = "root", Title = "Layout", VisibleDockables = factory.CreateList<IDockable>(split), DefaultDockable = split, LeftPinnedDockables = factory.CreateList<IDockable>(pinned), HiddenDockables = factory.CreateList<IDockable>(hidden), PinnedDockDisplayMode = PinnedDockDisplayMode.Inline, EnableAdaptiveGlobalDockTargets = true };
        var floatingTools = new ToolDock { Id = "floating-tools", VisibleDockables = factory.CreateList<IDockable>(new Tool { Id = "floating-tool" }) };
        var floating = new RootDock { Id = "floating-root", VisibleDockables = factory.CreateList<IDockable>(floatingTools) };
        var window = new DockWindow { Id = "floating-window", Layout = floating, X = 100, Y = 200, Width = 640, Height = 480, WindowState = DockWindowState.Maximized, ShowInTaskbar = false };
        root.Windows = factory.CreateList<IDockWindow>(window, window);
        root.Window = new DockWindow { Id = "main-window", Layout = root };
        window.ParentWindow = root.Window;
        factory.InitLayout(root);
        var wire = serializer.Serialize<IRootDock>(root);
        File.WriteAllText(path + ".graph.json", wire);
        var loaded = serializer.Deserialize<IRootDock>(wire)!;
        var loadedSplit = (ProportionalDock)loaded.VisibleDockables![0];
        var loadedTools = (ToolDock)loadedSplit.VisibleDockables![0];
        var loadedDocuments = (DocumentDock)loadedSplit.VisibleDockables[2];
        var loadedDocument = (Document)loadedDocuments.VisibleDockables![0];
        Check("catalog-graph-wire-stable", serializer.Serialize(loaded) == wire);
        Check("catalog-graph-owner-cycle", ReferenceEquals(loadedSplit.Owner, loaded));
        Check("catalog-graph-active-alias", ReferenceEquals(loadedDocuments.ActiveDockable, loadedDocument));
        Check("catalog-graph-window-alias", ReferenceEquals(loaded.Windows![0], loaded.Windows[1]));
        Check("catalog-graph-parent-window", ReferenceEquals(loaded.Windows[0].ParentWindow, loaded.Window));
        Check("catalog-graph-main-root-cycle", ReferenceEquals(loaded.Window!.Layout, loaded));
        Check("catalog-graph-pinned-hidden", loaded.LeftPinnedDockables![0].Id == "pinned" && loaded.HiddenDockables![0].Id == "hidden");
        Check("catalog-graph-mdi-grid-selector", loadedDocument.MdiZIndex == 7 && loadedDocument.MdiState == MdiWindowState.Maximized && loadedDocument.Column == 2 && loadedDocument.RowSpan == 3 && !loadedDocument.ShowInSelector && loadedDocument.SelectorTitle == "Editor selector");
        Check("catalog-graph-tools", loadedTools.Alignment == Alignment.Left && loadedTools.GripMode == GripMode.AutoHide && !loadedTools.AutoHide && ((Tool)loadedTools.VisibleDockables![0]).PinnedBounds!.Value.Width == 240);
        var restoredFactory = new Factory();
        restoredFactory.InitLayout(loaded);
        Check("catalog-graph-original-factory", ReferenceEquals(loadedDocument.Factory, restoredFactory) && ReferenceEquals(loadedDocuments.Owner, loadedSplit));
        var added = new Document { Id = "added" };
        loadedDocuments.AddDocument(added);
        Check("catalog-graph-original-add-document", ReferenceEquals(added.Owner, loadedDocuments) && ReferenceEquals(loadedDocuments.ActiveDockable, added));
    }
}
