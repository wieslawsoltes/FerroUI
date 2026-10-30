using System;
using Avalonia.Controls;
using Avalonia.Threading;
using Dock.Avalonia.Controls;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;

namespace DockFactoryMarkupReference;

internal static class CoreHostReference
{
    internal static void Run(Action<string, bool> check, Func<string, object> load)
    {
        var factory = new Factory(); var native = new HostWindow(); IHostWindow host = native;
        var window = factory.CreateDockWindow(); window.Title = "Source window"; host.Window = window;
        var panel = (StackPanel)load("CoreHostBindings"); panel.DataContext = host; Dispatcher.UIThread.RunJobs();
        check("core-host-compiled-title", ((TextBlock)panel.Children[1]).Text == "Source window");
        window.Title = "Updated source"; Dispatcher.UIThread.RunJobs();
        check("core-host-live-model-title", ((TextBlock)panel.Children[1]).Text == "Updated source");
        ((CheckBox)panel.Children[0]).IsChecked = true; Dispatcher.UIThread.RunJobs();
        check("core-host-two-way-tracking", host.IsTracked);
        check("core-host-original-state", ReferenceEquals(((ContentControl)panel.Children[2]).Content, host.HostWindowState) && host.HostWindowState is IDockManagerState);
        var button = (Button)panel.Children[3]; button.Command!.Execute(button.CommandParameter);
        check("core-host-original-command", native.Title == "Core command title");
        panel.DataContext = null; host.Window = null; native.Close();

        native = new HostWindow(); host = native; var layout = factory.CreateToolDock();
        host.SetLayout(layout);
        check("core-host-layout-identity", ReferenceEquals(native.DataContext, layout) && native.IsToolWindow);
        host.SetPosition(-12, 20); host.GetPosition(out double x, out double y);
        check("core-host-position", x == -12 && y == 20);
        host.SetSize(640, 480); host.GetSize(out double width, out double height);
        check("core-host-size", width == 640 && height == 480);
        host.SetSize(double.NaN, 500); host.GetSize(out width, out height);
        check("core-host-nan-size", width == 640 && height == 500);
        foreach (var state in new[] { DockWindowState.Normal, DockWindowState.Minimized, DockWindowState.Maximized, DockWindowState.FullScreen }) {
            host.SetWindowState(state); check("core-host-window-state-" + state, host.GetWindowState() == state);
        }
        host.SetWindowState(DockWindowState.Normal);
        host.SetTitle("Original host"); host.SetTitle(null); host.SetTitle("");
        check("core-host-null-empty-title", native.Title == "Original host");
        host.Present(false); check("core-host-present", native.IsVisible);
        host.SetActive(); host.Exit(); check("core-host-exit", !native.IsVisible);

        var container = new Container {ItemContainerTheme = new TextBlock(), ItemTemplateSelector = "selector key"};
        var view = (StackPanel)load("CoreContainerBindings"); view.DataContext = container; Dispatcher.UIThread.RunJobs();
        check("core-container-control-identity", ReferenceEquals(((ContentControl)view.Children[0]).Content, container.ItemContainerTheme));
        check("core-container-selector", ((TextBox)view.Children[1]).Text == "selector key");
        ((TextBox)view.Children[1]).Text = "edited selector"; Dispatcher.UIThread.RunJobs();
        check("core-container-two-way", Equals(container.ItemTemplateSelector, "edited selector"));
        ((TextBox)view.Children[1]).Text = null; Dispatcher.UIThread.RunJobs();
        check("core-container-two-way-null", container.ItemTemplateSelector is null);
        view.DataContext = null;
        var cases = new (IDockable Model, bool Local, bool Global)[] {
            (factory.CreateDocument(), true, false), (factory.CreateTool(), true, false),
            (factory.CreateDocumentDock(), true, false), (factory.CreateToolDock(), true, false),
            (factory.CreateProportionalDock(), false, true), (factory.CreateRootDock(), false, false),
            (factory.CreateGridDock(), false, false), (factory.CreateProportionalDockSplitter(), false, false)
        };
        int index = 0;
        foreach (var item in cases) check("core-marker-membership-" + index++, (item.Model is ILocalTarget) == item.Local && (item.Model is IGlobalTarget) == item.Global);
    }
    private sealed class Container : IDockItemContainerMetadata {
        public object? ItemContainerTheme {get;set;}
        public object? ItemTemplateSelector {get;set;}
    }
}
