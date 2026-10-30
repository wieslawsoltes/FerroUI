using System;
using System.ComponentModel;
using Avalonia.Controls;
using Avalonia.Controls.Recycling;
using Avalonia.Controls.Recycling.Model;
using Avalonia.Threading;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;

namespace DockFactoryMarkupReference;

internal static class CoreContractReference
{
    internal static void Run(Action<string, bool> check, Func<string, object> load)
    {
        var factory = new Factory(); IDock dock = factory.CreateDocumentDock();
        dock.Title = "Core dock";
        IDockable first = factory.CreateDocument(); first.Title = "First";
        IDockable next = factory.CreateDocument(); next.Title = "Next";
        var list = factory.CreateList<IDockable>(first);
        dock.VisibleDockables = list; dock.ActiveDockable = first; dock.DefaultDockable = next;
        var panel = (StackPanel)load("CoreDockBindings"); panel.DataContext = dock;
        Dispatcher.UIThread.RunJobs();
        check("core-contract-compiled-title", ((TextBlock)panel.Children[0]).Text == "Core dock");
        check("core-contract-compiled-active", ((TextBlock)panel.Children[1]).Text == "First");
        check("core-contract-compiled-count", ((TextBlock)panel.Children[2]).Text == "1");
        first.Title = "Updated first"; list.Add(next); Dispatcher.UIThread.RunJobs();
        check("core-contract-live-active", ((TextBlock)panel.Children[1]).Text == "Updated first");
        check("core-contract-live-count", ((TextBlock)panel.Children[2]).Text == "2" && ReferenceEquals(dock.VisibleDockables, list));
        ((CheckBox)panel.Children[3]).IsChecked = true; Dispatcher.UIThread.RunJobs();
        check("core-contract-two-way", dock.IsActive);
        var button = (Button)panel.Children[4];
        check("core-contract-command-parameter", ReferenceEquals(button.CommandParameter, next));
        check("core-contract-command-identity", ReferenceEquals(button.Command, dock.Navigate));
        button.Command!.Execute(button.CommandParameter); Dispatcher.UIThread.RunJobs();
        check("core-contract-navigation", ReferenceEquals(dock.ActiveDockable, next) && dock.CanGoBack && ((TextBlock)panel.Children[1]).Text == "Next");
        dock.GoBack.Execute(null);
        check("core-contract-back", ReferenceEquals(dock.ActiveDockable, first) && dock.CanGoForward);
        dock.GoForward.Execute(null);
        check("core-contract-forward", ReferenceEquals(dock.ActiveDockable, next));
        panel.DataContext = null;

        IDockable document = factory.CreateDocument(); document.Title = "Before";
        var control = new TextBlock(); document.Context = control;
        var view = (StackPanel)load("CoreDockableBindings"); view.DataContext = document;
        Dispatcher.UIThread.RunJobs();
        check("core-contract-context", ReferenceEquals(((ContentControl)view.Children[1]).Content, control));
        ((TextBox)view.Children[0]).Text = "Edited"; Dispatcher.UIThread.RunJobs();
        check("core-contract-document-two-way", document.Title == "Edited");
        view.DataContext = null;
        document.Id = "stable";
        check("core-contract-recycling-id", ((IControlRecyclingIdProvider)document).GetControlRecyclingId() == "stable");

        var cache = new ControlRecycling(); IControlRecycling recycling = cache;
        var data = new object(); recycling.Add(data, control);
        check("core-contract-cache-add", recycling.TryGetValue(data, out var cached) && ReferenceEquals(cached, control));
        check("core-contract-cache-build", ReferenceEquals(recycling.Build(data, null, null), control));
        recycling.Clear();
        check("core-contract-cache-clear", !recycling.TryGetValue(data, out _));

        var observed = factory.CreateList<IDockable>(); dock.VisibleDockables = observed;
        int counts = 0;
        ((INotifyPropertyChanged)observed).PropertyChanged += (_, args) => { if (args.PropertyName == "Count") counts++; };
        observed.Add(first); observed.Add(next); observed.Remove(first); observed.Clear();
        check("core-contract-list-notifications", counts == 4 && observed.Count == 0 && ReferenceEquals(dock.VisibleDockables, observed));
    }
}
