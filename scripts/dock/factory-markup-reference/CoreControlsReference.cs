using System;
using Avalonia.Controls;
using Avalonia.Threading;
using Dock.Model.Controls;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;

namespace DockFactoryMarkupReference;

internal static class CoreControlsReference
{
    internal static void Run(Action<string, bool> check, Func<string, object> load)
    {
        var f = new Factory();
        var root = f.CreateRootDock(); root.Title = "Original root";
        var windows = f.CreateList<IDockWindow>(); var pins = f.CreateList<IDockable>();
        root.Windows = windows; root.LeftPinnedDockables = pins;
        var p = (StackPanel)load("CoreControlsRootBindings"); p.DataContext = root;
        Dispatcher.UIThread.RunJobs();
        check("controls-root-title", ((TextBlock)p.Children[0]).Text == "Original root");
        check("controls-root-initial-windows", ((TextBlock)p.Children[1]).Text == "0");
        check("controls-root-initial-pins", ((TextBlock)p.Children[2]).Text == "0");
        windows.Add(f.CreateDockWindow()); pins.Add(f.CreateTool()); root.Title = "Updated root";
        Dispatcher.UIThread.RunJobs();
        check("controls-root-live-title", ((TextBlock)p.Children[0]).Text == "Updated root");
        check("controls-root-live-windows", ((TextBlock)p.Children[1]).Text == "1" && ReferenceEquals(root.Windows, windows));
        check("controls-root-live-pins", ((TextBlock)p.Children[2]).Text == "1" && ReferenceEquals(root.LeftPinnedDockables, pins));
        ((CheckBox)p.Children[3]).IsChecked = false; Dispatcher.UIThread.RunJobs();
        check("controls-root-two-way", !root.IsFocusableRoot);
        windows.Clear(); pins.Clear(); Dispatcher.UIThread.RunJobs();
        check("controls-root-clear", ((TextBlock)p.Children[1]).Text == "0" && ((TextBlock)p.Children[2]).Text == "0");
        p.DataContext = null;

        var dock = f.CreateDocumentDock(); dock.Factory = f; dock.VisibleDockables = f.CreateList<IDockable>(); dock.Title = "Document dock";
        var doc = f.CreateDocument(); dock.DefaultDockable = doc;
        var content = new TextBlock(); dock.EmptyContent = content;
        p = (StackPanel)load("CoreControlsDocumentBindings"); p.DataContext = dock; Dispatcher.UIThread.RunJobs();
        check("controls-document-title", ((TextBox)p.Children[0]).Text == "Document dock");
        check("controls-document-content", ReferenceEquals(((ContentControl)p.Children[1]).Content, content));
        ((TextBox)p.Children[0]).Text = "Edited dock"; ((CheckBox)p.Children[2]).IsChecked = true; Dispatcher.UIThread.RunJobs();
        check("controls-document-two-way-title", dock.Title == "Edited dock");
        check("controls-document-two-way-create", dock.CanCreateDocument);
        var button = (Button)p.Children[3]; button.Command!.Execute(button.CommandParameter); Dispatcher.UIThread.RunJobs();
        check("controls-document-add", ReferenceEquals(dock.VisibleDockables![0], doc));
        check("controls-document-active", ReferenceEquals(dock.ActiveDockable, doc));
        check("controls-document-owner", ReferenceEquals(doc.Owner, dock));
        dock.EmptyContent = null; Dispatcher.UIThread.RunJobs();
        check("controls-document-null-content", ((ContentControl)p.Children[1]).Content is null); p.DataContext = null;

        var tools = f.CreateToolDock(); tools.Factory = f; tools.VisibleDockables = f.CreateList<IDockable>();
        var tool = f.CreateTool(); tool.Title = "Tool"; tools.DefaultDockable = tool;
        p = (StackPanel)load("CoreControlsToolBindings"); p.DataContext = tools; Dispatcher.UIThread.RunJobs();
        ((CheckBox)p.Children[1]).IsChecked = false; Dispatcher.UIThread.RunJobs();
        check("controls-tool-two-way", !tools.AutoHide);
        button = (Button)p.Children[2]; button.Command!.Execute(button.CommandParameter); Dispatcher.UIThread.RunJobs();
        check("controls-tool-active", ReferenceEquals(tools.ActiveDockable, tool));
        check("controls-tool-title", ((TextBlock)p.Children[0]).Text == "Tool");
        tool.Title = "Updated tool"; Dispatcher.UIThread.RunJobs();
        check("controls-tool-live-title", ((TextBlock)p.Children[0]).Text == "Updated tool");
        check("controls-tool-owner", ReferenceEquals(tool.Owner, tools)); p.DataContext = null;

        var mdi = (IMdiDocument)f.CreateDocument(); mdi.Title = "MDI"; mdi.MdiZIndex = 3;
        p = (StackPanel)load("CoreControlsMdiBindings"); p.DataContext = mdi; Dispatcher.UIThread.RunJobs();
        check("controls-mdi-z", ((TextBlock)p.Children[1]).Text == "3");
        ((TextBox)p.Children[0]).Text = "Edited MDI"; Dispatcher.UIThread.RunJobs();
        check("controls-mdi-inherited-title", mdi.Title == "Edited MDI");
        mdi.MdiZIndex = 9; mdi.MdiState = MdiWindowState.Maximized; Dispatcher.UIThread.RunJobs();
        check("controls-mdi-live-z", ((TextBlock)p.Children[1]).Text == "9");
        check("controls-mdi-live-state", ((TextBlock)p.Children[2]).Text == "Maximized"); p.DataContext = null;

        var grid = f.CreateGridDock(); grid.ColumnDefinitions = "*,2*"; grid.RowDefinitions = "Auto,*";
        p = (StackPanel)load("CoreControlsGridBindings"); p.DataContext = grid; Dispatcher.UIThread.RunJobs();
        check("controls-grid-columns", ((TextBox)p.Children[0]).Text == "*,2*");
        ((TextBox)p.Children[1]).Text = "1*,3*"; Dispatcher.UIThread.RunJobs();
        check("controls-grid-two-way", grid.RowDefinitions == "1*,3*");
        grid.ColumnDefinitions = null; Dispatcher.UIThread.RunJobs();
        check("controls-grid-null", ((TextBox)p.Children[0]).Text is null); p.DataContext = null;

        var stack = f.CreateStackDock(); stack.Spacing = 12; stack.Orientation = Orientation.Horizontal;
        p = (StackPanel)load("CoreControlsStackBindings"); p.DataContext = stack; Dispatcher.UIThread.RunJobs();
        check("controls-stack-spacing", ((TextBlock)p.Children[0]).Text == "12");
        check("controls-stack-orientation", ((TextBlock)p.Children[1]).Text == "Horizontal");
        stack.Spacing = 24; stack.Orientation = Orientation.Vertical; Dispatcher.UIThread.RunJobs();
        check("controls-stack-live-spacing", ((TextBlock)p.Children[0]).Text == "24");
        check("controls-stack-live-orientation", ((TextBlock)p.Children[1]).Text == "Vertical"); p.DataContext = null;
    }
}
