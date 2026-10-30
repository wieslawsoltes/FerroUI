// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System;
using System.Globalization;
using System.IO;
using System.Linq;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Threading;
using Avalonia.Interactivity;
using Avalonia.VisualTree;
using Dock.Avalonia.Diagnostics.Controls;
using Dock.Model.ReactiveUI.Controls;
CultureInfo.CurrentCulture=CultureInfo.InvariantCulture;
if(args.Length!=1) throw new ArgumentException("Pass the output corpus path.");
AppBuilder.Configure<Application>().UseHeadless(new AvaloniaHeadlessPlatformOptions()).SetupWithoutStarting();
using StreamWriter output=new(args[0]);
output.WriteLine("# Original unchanged compiled inspector views; .NET "+Environment.Version);
(UserControl View,string Name)[] views=[
(new GridDockDebugView(),"GridDockDebugView"),
(new UniformGridDockDebugView(),"UniformGridDockDebugView"),
(new DocumentContentDebugView(),"DocumentContentDebugView"),
(new DocumentDockContentDebugView(),"DocumentDockContentDebugView"),
(new ToolDockDebugView(),"ToolDockDebugView"),
(new DockableDebugView(),"DockableDebugView"),
(new ToolContentDebugView(),"ToolContentDebugView"),
(new DocumentDockDebugView(),"DocumentDockDebugView"),
(new DockDockDebugView(),"DockDockDebugView"),
(new GridDockSplitterDebugView(),"GridDockSplitterDebugView"),
(new DocumentTemplateDebugView(),"DocumentTemplateDebugView"),
(new DockWindowDebugView(),"DockWindowDebugView"),
(new WrapDockDebugView(),"WrapDockDebugView"),
(new RootDockInfoDebugView(),"RootDockInfoDebugView"),
(new ProportionalDockDebugView(),"ProportionalDockDebugView"),
(new StackDockDebugView(),"StackDockDebugView"),
(new DockDebugView(),"DockDebugView"),
(new ProportionalDockSplitterDebugView(),"ProportionalDockSplitterDebugView"),
];
string Kind(Control child)=>child switch { TextBox=>"TextBox", CheckBox=>"CheckBox", TextBlock=>"TextBlock", _=>throw new InvalidOperationException("Unexpected source field") };
foreach((UserControl view,string name) in views){
 Grid grid=(Grid)view.Content!;
 string fields=string.Join(";",grid.Children.Select(child=>$"{Kind(child)},{Grid.GetRow(child)},{Grid.GetColumn(child)},{(child.IsEnabled?1:0)},{(child is TextBox text && text.IsReadOnly?1:0)}"));
 output.WriteLine($"{name}\t{grid.RowDefinitions.Count}\t{grid.ColumnDefinitions.Count}\t{fields}");
}
Tool tool=new(){Id="id",Title="title"};
DockableDebugView dockableView=new(){DataContext=tool};
TextBox Field(UserControl view,int row)=>(TextBox)((Grid)view.Content!).Children.Single(child=>Grid.GetRow(child)==row&&Grid.GetColumn(child)==1);
CheckBox Toggle(UserControl view,int row)=>(CheckBox)((Grid)view.Content!).Children.Single(child=>Grid.GetRow(child)==row&&Grid.GetColumn(child)==1);
Window window=new(){Content=dockableView};window.Show();window.UpdateLayout();
Field(dockableView,1).Text="edited title";Toggle(dockableView,20).IsChecked=false;Dispatcher.UIThread.RunJobs();
if(tool.Title!="edited title"||tool.CanClose)throw new InvalidOperationException("Source edits.");
ToolDock owner=new(){Title="first owner"};tool.Owner=owner;Dispatcher.UIThread.RunJobs();
if(Field(dockableView,3).Text!="first owner")throw new InvalidOperationException("Source owner binding.");
owner.Title="updated owner";Dispatcher.UIThread.RunJobs();
if(Field(dockableView,3).Text!="updated owner")throw new InvalidOperationException("Source nested notification.");
GridDockSplitter splitter=new();GridDockSplitterDebugView splitterView=new(){DataContext=splitter};window.Content=splitterView;
foreach(string input in new[]{"Rows","rows"," Rows ","1","Columns, Rows","invalid","","2147483648"}){
 splitter.ResizeDirection=Dock.Model.Core.GridResizeDirection.Columns;Dispatcher.UIThread.RunJobs();Field(splitterView,0).Text=input;Dispatcher.UIThread.RunJobs();
 output.WriteLine($"# edit\tGridSplitter\t{input}\t{(int)splitter.ResizeDirection}");
}
DocumentDock document=new();DocumentDockDebugView documentView=new(){DataContext=document};window.Content=documentView;
foreach(string input in new[]{"Right","right"," Right ","2","Top, Right","invalid","","2147483648"}){
 document.TabsLayout=Dock.Model.Core.DocumentTabLayout.Top;Dispatcher.UIThread.RunJobs();Field(documentView,3).Text=input;Dispatcher.UIThread.RunJobs();
 output.WriteLine($"# edit\tDocumentDock\t{input}\t{(int)document.TabsLayout}");
}
StackDock stack=new();StackDockDebugView stackView=new(){DataContext=stack};window.Content=stackView;
foreach(string input in new[]{"Vertical","vertical"," Vertical ","1","Horizontal, Vertical","invalid","","2147483648"}){
 stack.Orientation=Dock.Model.Core.Orientation.Horizontal;Dispatcher.UIThread.RunJobs();Field(stackView,0).Text=input;Dispatcher.UIThread.RunJobs();
 output.WriteLine($"# edit\tStackDock\t{input}\t{(int)stack.Orientation}");
}

RootDockDebug inspector=new();
Dock.Model.ReactiveUI.Factory factory=new();RootDock layout=new(){Factory=factory};
inspector.DataContext=layout;window.Content=inspector;window.UpdateLayout();Dispatcher.UIThread.RunJobs();
TabControl tabs=(TabControl)((DockPanel)inspector.Content!).Children[0];
if(tabs.Items.Count!=5||inspector.DataTemplates.Count!=18)throw new InvalidOperationException("Complete original inspector structure");
System.Collections.Generic.List<string> names=new();
((System.ComponentModel.INotifyPropertyChanged)inspector).PropertyChanged+=(_,e)=>names.Add(e.PropertyName!);
inspector.Filter="filter";inspector.Filter="filter";
if(string.Join(",",names)!="Filter,FilteredEvents")throw new InvalidOperationException("Original filter notifications");
inspector.Filter=null;
Tool loggedTool=new(){Title="Tool"};Dock.Model.ReactiveUI.Core.DockWindow loggedWindow=new(){Title="Window"};
factory.OnActiveDockableChanged(loggedTool);factory.OnFocusedDockableChanged(loggedTool);factory.OnDockableInit(loggedTool);
factory.OnDockableAdded(loggedTool);factory.OnDockableRemoved(loggedTool);factory.OnDockableClosed(loggedTool);factory.OnDockableMoved(loggedTool);
factory.OnDockableDocked(loggedTool,Dock.Model.Core.DockOperation.Right);factory.OnDockableUndocked(loggedTool,Dock.Model.Core.DockOperation.Left);factory.OnDockableSwapped(loggedTool);
factory.OnDockablePinned(loggedTool);factory.OnDockableUnpinned(loggedTool);factory.OnDockableHidden(loggedTool);factory.OnDockableRestored(loggedTool);
factory.OnWindowOpened(loggedWindow);factory.OnWindowClosing(loggedWindow);factory.OnWindowClosed(loggedWindow);factory.OnWindowAdded(loggedWindow);factory.OnWindowRemoved(loggedWindow);
factory.OnWindowMoveDragBegin(loggedWindow);factory.OnWindowMoveDrag(loggedWindow);factory.OnWindowMoveDragEnd(loggedWindow);factory.OnWindowActivated(loggedWindow);factory.OnDockableActivated(loggedTool);factory.OnWindowDeactivated(loggedWindow);factory.OnDockableDeactivated(loggedTool);
foreach(string line in inspector.Events){if(line.Length<13||line[12]!=' ')throw new InvalidOperationException("Original timestamp");output.WriteLine("# log\t"+line[13..]);}
if(inspector.Events.Count!=26)throw new InvalidOperationException("Original 26 subscriptions");
Dock.Model.ReactiveUI.Factory replacement=new();layout=new(){Factory=replacement};inspector.DataContext=layout;
factory.OnDockableAdded(loggedTool);if(inspector.Events.Count!=26)throw new InvalidOperationException("Original unsubscribe");replacement.OnDockableAdded(loggedTool);if(inspector.Events.Count!=27)throw new InvalidOperationException("Original replacement factory");
window.Content=null;replacement.OnDockableAdded(loggedTool);if(inspector.Events.Count!=28)throw new InvalidOperationException("Original detach retains logging");inspector.DataContext=null;
window.Content=inspector;inspector.DataContext=layout;tabs.SelectedIndex=3;Dispatcher.UIThread.RunJobs();window.UpdateLayout();
DockPanel eventsPanel=(DockPanel)((TabItem)tabs.Items[3]!).Content!;
StackPanel eventInputs=(StackPanel)eventsPanel.Children[0];TextBox filterInput=(TextBox)eventInputs.Children[1];ListBox logList=(ListBox)eventsPanel.Children[1];
filterInput.Text="DockableRemoved";Dispatcher.UIThread.RunJobs();
if(logList.ItemCount!=1)throw new InvalidOperationException("Original filtered UI count");
((Button)eventInputs.Children[2]).RaiseEvent(new RoutedEventArgs(Button.ClickEvent));Dispatcher.UIThread.RunJobs();
if(inspector.Events.Count!=0||logList.ItemCount!=0)throw new InvalidOperationException("Original clear updates filtered UI");
output.WriteLine("# main\tfiltered-clear-count\t"+logList.ItemCount);
inspector.DataContext=null;window.Content=null;

using(IDisposable attached=RootDockDebugExtensions.AttachDockDebug(window,()=>layout)){
 window.RaiseEvent(new Avalonia.Input.KeyEventArgs{RoutedEvent=Avalonia.Input.InputElement.KeyDownEvent,Key=Avalonia.Input.Key.F11});
 window.RaiseEvent(new Avalonia.Input.KeyEventArgs{RoutedEvent=Avalonia.Input.InputElement.KeyDownEvent,Key=Avalonia.Input.Key.F11});
}

// Measure the unchanged original header: its Auto grip receives only MinWidth.
Application.Current!.Styles.Add(new Avalonia.Themes.Fluent.FluentTheme());
Application.Current.Styles.Add(new Dock.Avalonia.Themes.Fluent.DockFluentTheme());
ToolDock headerDock=new(){ActiveDockable=loggedTool};
Dock.Avalonia.Controls.ToolChromeControl header=new(){DataContext=headerDock};
window.Width=600;window.Content=header;window.UpdateLayout();Dispatcher.UIThread.RunJobs();window.UpdateLayout();
Grid sourceGrip=header.GetVisualDescendants().OfType<Grid>().Single(grid=>grid.Name=="PART_Grid");
if(sourceGrip.Bounds.Width!=12)throw new InvalidOperationException("Original Auto grip width");
Console.WriteLine("Unchanged original Dock header: width="+header.Bounds.Width+", grip width="+sourceGrip.Bounds.Width);
window.Content=null;

int providerCalls=0;
System.Collections.Generic.List<Window> debugWindows=new();
IDisposable customDebug=RootDockDebugExtensions.AttachDockDebug(
 window,()=>{providerCalls++;return layout;},
 top=>{Window debug=new(){Width=512};debugWindows.Add(debug);return debug;},
 new Avalonia.Input.KeyGesture(Avalonia.Input.Key.F8,Avalonia.Input.KeyModifiers.Control));
void ToggleCustom()=>window.RaiseEvent(new Avalonia.Input.KeyEventArgs{
 RoutedEvent=Avalonia.Input.InputElement.KeyDownEvent,Key=Avalonia.Input.Key.F8,
 KeyModifiers=Avalonia.Input.KeyModifiers.Control});
if(providerCalls!=0)throw new InvalidOperationException("Original provider is lazy");
ToggleCustom();
if(providerCalls!=1||debugWindows.Count!=1||!debugWindows[0].IsVisible||debugWindows[0].Width!=512||
 debugWindows[0].Content is not RootDockDebug debugInspector||!ReferenceEquals(debugInspector.DataContext,layout))
 throw new InvalidOperationException("Original custom factory and current layout");
debugWindows[0].Close();ToggleCustom();
if(providerCalls!=2||debugWindows.Count!=2||!debugWindows[1].IsVisible)
 throw new InvalidOperationException("Original closed window reopens");
EventHandler<WindowClosingEventArgs> cancelClose=(_,e)=>e.Cancel=true;
debugWindows[1].Closing+=cancelClose;ToggleCustom();
if(providerCalls!=2||!debugWindows[1].IsVisible)throw new InvalidOperationException("Original canceled toggle retains window");
customDebug.Dispose();customDebug.Dispose();
if(!debugWindows[1].IsVisible)throw new InvalidOperationException("Original dispose honors canceled close");
ToggleCustom();if(providerCalls!=2)throw new InvalidOperationException("Original dispose removes handler");
debugWindows[1].Closing-=cancelClose;debugWindows[1].Close();
using(IDisposable closingDebug=RootDockDebugExtensions.AttachDockDebug(window,()=>layout,
 top=>{Window debug=new();debugWindows.Add(debug);return debug;},null)){
 window.RaiseEvent(new Avalonia.Input.KeyEventArgs{RoutedEvent=Avalonia.Input.InputElement.KeyDownEvent,Key=Avalonia.Input.Key.F11});
 window.Close();
 if(debugWindows[^1].IsVisible)throw new InvalidOperationException("Original main close closes debug window");
}
Console.WriteLine("Captured 18 original grids, 24 enum edits and 26 event messages; main inspector, filtered clear and F11/custom gesture lifecycle checks passed.");
