using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Collections.Specialized;
using Avalonia.Controls;
using Avalonia.Threading;
using Dock.Model;
using Dock.Model.Core;
using Dock.Model.Controls;
using Dock.Model.Adapters;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class NullableWindowsReference
{
    private static bool NullFailure(Action action){try{action();return false;}catch(NullReferenceException){return true;}}
    private static string Name(IDockWindow? window)=>window?.Id??"null";
    internal static void Run(Action<string,bool> check,Func<string,object> load)
    {
        var ordinary=new Factory();ordinary.InsertWindow(null!,null!,-1);check("nullable-windows-negative-insert-guard",true);
        foreach(var pair in new (string,Action)[]{("add",()=>ordinary.AddWindow(null!,null!)),("insert",()=>ordinary.InsertWindow(null!,null!,0)),("remove",()=>ordinary.RemoveWindow(null!)),("close",()=>ordinary.CloseWindow(null!)),("init",()=>ordinary.InitDockWindow(null!,null)),("explicit",()=>ordinary.InitDockWindow(null!,null,null)),("resolve",()=>new WindowsFactory().BaseResolve(null!))})check("nullable-windows-default-"+pair.Item1,NullFailure(pair.Item2));
        var f=new WindowsFactory();var root=new RootDock();var list=new ObservableCollection<IDockWindow>();root.Windows=list;var payloads=new List<(int,object?)>();list.CollectionChanged+=(_,e)=>payloads.Add((e.NewStartingIndex,e.NewItems?[0]));f.AddWindow(root,null!);f.InsertWindow(root,null!,0);
        check("nullable-windows-add-insert-null-order",string.Join(",",f.Trace)=="added:null,init:null,added:null,init:null");check("nullable-windows-add-insert-null-list",ReferenceEquals(root.Windows,list)&&list.Count==2&&list[0]is null&&list[1]is null);check("nullable-windows-add-insert-payload",payloads.Count==2&&payloads[0]==(0,null)&&payloads[1]==(0,null));check("nullable-windows-add-insert-init-identity",f.Windows.Count==2&&f.Windows[0]is null&&f.Windows[1]is null&&ReferenceEquals(f.Owners[0],root)&&ReferenceEquals(f.Owners[1],root));
        ordinary=new Factory();root=new RootDock();int raised=0;var observed=root;ordinary.WindowAdded+=(_,e)=>{check("nullable-windows-default-added-null-payload",e.Window is null&&observed.Windows?[0]is null);raised++;};check("nullable-windows-default-commit-before-init-failure",NullFailure(()=>ordinary.AddWindow(root,null!))&&raised==1&&root.Windows?.Count==1&&root.Windows[0]is null);
        f=new WindowsFactory();root=new RootDock();bool range=false;try{f.InsertWindow(root,null!,1);}catch(ArgumentOutOfRangeException){range=true;}check("nullable-windows-insert-created-before-bounds",range&&root.Windows?.Count==0&&f.Trace.Count==0);f.AfterAdded=()=>throw new InvalidOperationException("added aborted");bool aborted=false;try{f.AddWindow(root,null!);}catch(InvalidOperationException e)when(e.Message=="added aborted"){aborted=true;}check("nullable-windows-added-failure-keeps-commit",aborted&&root.Windows?.Count==1&&root.Windows[0]is null&&string.Join(",",f.Trace)=="added:null"&&f.Windows.Count==0);
        f=new WindowsFactory();root=new RootDock();var old=new ObservableCollection<IDockWindow>();var replacement=new ObservableCollection<IDockWindow>();root.Windows=old;var liveRoot=root;f.AfterAdded=()=>liveRoot.Windows=replacement;f.AddWindow(root,null!);check("nullable-windows-added-replaces-live-list",old.Count==1&&old[0]is null&&replacement.Count==0&&ReferenceEquals(root.Windows,replacement));check("nullable-windows-init-keeps-owner-after-replacement",string.Join(",",f.Trace)=="added:null,init:null"&&f.Windows[0]is null&&ReferenceEquals(f.Owners[0],root));
        var hostFactory=new HostFactory{InitBase=true};var host=new CallbackHost();hostFactory.Host=host;root=new RootDock();hostFactory.InitDockWindow(null!,root);check("nullable-windows-host-before-explicit-override",string.Join(",",hostFactory.Trace)=="init:null,host:null,explicit:null"&&host.Calls==0&&ReferenceEquals(hostFactory.Owners[^1],root));hostFactory.Trace.Clear();hostFactory.ExplicitBase=true;check("nullable-windows-host-before-default-host-failure",NullFailure(()=>hostFactory.InitDockWindow(null!,root))&&string.Join(",",hostFactory.Trace)=="init:null,host:null,explicit:null"&&host.Calls==0);
        f=new WindowsFactory();root=new RootDock();var window=ordinary.CreateDockWindow();window.Id="second";list=new ObservableCollection<IDockWindow>{null!,window};root.Windows=list;f.BaseInitRoot(root);check("nullable-windows-root-init-null-order",string.Join(",",f.Trace)=="init:null,init:second");check("nullable-windows-root-init-shared-list",ReferenceEquals(root.Windows,list)&&f.Windows[0]is null&&ReferenceEquals(f.Windows[1],window));
        for(int mode=0;mode<2;mode++) {root=new RootDock();var first=new ProbeWindow();var last=new ProbeWindow();root.Windows=new List<IDockWindow>{first,null!,last};var adapter=new NavigateAdapter(root);check("nullable-windows-command-null-failure-"+mode,NullFailure(()=>{if(mode==0)adapter.ShowWindows();else adapter.ExitWindows();}));check("nullable-windows-command-retains-earlier-calls-"+mode,string.Join(",",first.Calls)==(mode==0?"Present":"Save,Exit")&&last.Calls.Count==0);}
        ordinary=new Factory();root=new RootDock();window=ordinary.CreateDockWindow();window.Layout=new RootDock();root.Windows=new List<IDockWindow>{null!,window};check("nullable-windows-search-first-null",NullFailure(()=>ordinary.FindDockable(root,_=>false)));check("nullable-windows-state-first-null",NullFailure(()=>new DockState().Save(root)));
        for(int mode=0;mode<2;mode++){f=new WindowsFactory{ExplicitBase=true};var a=new RootDock();var b=new RootDock();var owner=new ToolDock();b.Owner=owner;var probe=new ProbeWindow{Id="window",Sequence=new Queue<IRootDock?>(new IRootDock?[]{a,null,mode==0?b:null})};if(mode==0){f.InitDockWindow(probe,null,null);check("nullable-windows-layout-null-first-argument",f.InitItems.Count==1&&f.InitItems[0]is null&&ReferenceEquals(f.Owners[^1],owner));}else check("nullable-windows-layout-second-owner-failure",NullFailure(()=>f.InitDockWindow(probe,null,null))&&f.InitItems.Count==0);check("nullable-windows-layout-read-and-link-order-"+mode,probe.Reads==3&&probe.Factory is not null&&string.Join(",",probe.Calls)=="Host,Owner,Factory");}
        var nullRoot=new NullCollectionRootFactory();nullRoot.SplitToWindow(null!,null!,1,2,300,400);check("nullable-windows-null-collection-root-dispatch",nullRoot.Resolved==1&&nullRoot.Added==1&&nullRoot.Removed==1);check("nullable-windows-null-collection-root-window-identity",nullRoot.Result.X==1&&nullRoot.Result.Y==2&&nullRoot.Result.Width==300&&nullRoot.Result.Height==400&&nullRoot.Root.Windows is null);
        var panel=(StackPanel)load("CoreNullWindowsBindings");root=new RootDock();list=new ObservableCollection<IDockWindow>{null!};root.Windows=list;panel.DataContext=root;Dispatcher.UIThread.RunJobs();var count=(TextBlock)panel.Children[0];var content=(ContentControl)panel.Children[1];var items=(ItemsControl)panel.Children[2];check("nullable-windows-bound-initial-null",count.Text=="1"&&content.Content is null&&items.Items.Count==1);
        window=new Factory().CreateDockWindow();list[0]=window;Dispatcher.UIThread.RunJobs();check("nullable-windows-bound-indexer-stale-null",content.Content is null);panel.DataContext=null;panel.DataContext=root;Dispatcher.UIThread.RunJobs();check("nullable-windows-bound-window-rebind",ReferenceEquals(content.Content,window));list[0]=null!;list.Add(null!);Dispatcher.UIThread.RunJobs();check("nullable-windows-bound-live-count-stale-window",count.Text=="2"&&items.Items.Count==2&&ReferenceEquals(content.Content,window));panel.DataContext=null;panel.DataContext=root;Dispatcher.UIThread.RunJobs();check("nullable-windows-bound-null-rebind",content.Content is null);list.Remove(null!);Dispatcher.UIThread.RunJobs();check("nullable-windows-bound-null-remove",count.Text=="1"&&items.Items.Count==1);panel.DataContext=null;
    }
    private class WindowsFactory:Factory
    {
        internal readonly List<string> Trace=new();internal bool InitBase,ExplicitBase;internal readonly List<IDockWindow?> Windows=new();internal readonly List<IDockable?> Owners=new(),InitItems=new();internal Action? AfterAdded;
        public override void OnWindowAdded(IDockWindow? window){Trace.Add("added:"+Name(window));base.OnWindowAdded(window);AfterAdded?.Invoke();}
        public override void InitDockWindow(IDockWindow window,IDockable? owner){Trace.Add("init:"+Name(window));Windows.Add(window);Owners.Add(owner);if(InitBase)base.InitDockWindow(window,owner);}
        public override void InitDockWindow(IDockWindow window,IDockable? owner,IHostWindow? host){Trace.Add("explicit:"+Name(window));Windows.Add(window);Owners.Add(owner);if(ExplicitBase)base.InitDockWindow(window,owner,host);}
        public override void InitDockable(IDockable item,IDockable? owner){Trace.Add("dock-init:"+(item?.Id??"null"));InitItems.Add(item);Owners.Add(owner);}
        internal IRootDock BaseResolve(IRootDock root)=>base.ResolveWindowCollectionRoot(root);
        internal void BaseInitRoot(IRootDock root)=>base.InitDockable(root,null);
    }
    private sealed class HostFactory:WindowsFactory,IDockWindowHostFactory {internal IHostWindow? Host;public IHostWindow? CreateHostWindow(IDockWindow window){Trace.Add("host:"+Name(window));Windows.Add(window);return Host;}}
    private sealed class CallbackHost:IHostWindow
    {
        internal int Calls;public IHostWindowState? HostWindowState=>null;public bool IsTracked{get;set;}private IDockWindow? window;public IDockWindow? Window{get=>window;set{window=value;Calls++;}}
        public void SetActive(){}public void Present(bool dialog){}public void Exit(){}public void SetPosition(double x,double y){}public void GetPosition(out double x,out double y){x=y=0;}public void SetSize(double width,double height){}public void GetSize(out double width,out double height){width=height=0;}public void SetWindowState(DockWindowState state){}public DockWindowState GetWindowState()=>DockWindowState.Normal;public void SetTitle(string? title){}public void SetLayout(IDock layout){}
    }
    private sealed class NullCollectionRootFactory:Factory
    {
        internal readonly IRootDock Root=new RootDock();internal readonly IDockWindow Result=new Factory().CreateDockWindow();internal int Resolved,Added,Removed;
        public override IRootDock? FindRoot(IDockable item,Func<IRootDock,bool>? predicate=null)=>Root;
        protected override IRootDock ResolveWindowCollectionRoot(IRootDock root){if(!ReferenceEquals(root,Root))throw new InvalidOperationException("root identity");Resolved++;return null!;}
        public override void RemoveDockable(IDockable item,bool collapse){if(item is not null||!collapse)throw new InvalidOperationException("remove arguments");Removed++;}
        public override IDockWindow? CreateWindowFrom(IDockable item,DockWindowOptions? options){if(item is not null||options is not null)throw new InvalidOperationException("create arguments");return Result;}
        public override void AddWindow(IRootDock root,IDockWindow window){if(root is not null||!ReferenceEquals(window,Result))throw new InvalidOperationException("add arguments");Added++;}
    }
    internal sealed class ProbeWindow:IDockWindow
    {
        private readonly IDockWindow model=new Factory().CreateDockWindow();internal readonly List<string> Calls=new();internal Queue<IRootDock?>? Sequence;internal int Reads;
        public string Id{get=>model.Id;set=>model.Id=value;}public double X{get=>model.X;set=>model.X=value;}public double Y{get=>model.Y;set=>model.Y=value;}public double Width{get=>model.Width;set=>model.Width=value;}public double Height{get=>model.Height;set=>model.Height=value;}public DockWindowState WindowState{get=>model.WindowState;set=>model.WindowState=value;}public bool Topmost{get=>model.Topmost;set=>model.Topmost=value;}public string Title{get=>model.Title;set=>model.Title=value;}public DockWindowOwnerMode OwnerMode{get=>model.OwnerMode;set=>model.OwnerMode=value;}public IDockWindow? ParentWindow{get=>model.ParentWindow;set=>model.ParentWindow=value;}public bool IsModal{get=>model.IsModal;set=>model.IsModal=value;}public bool? ShowInTaskbar{get=>model.ShowInTaskbar;set=>model.ShowInTaskbar=value;}
        public IDockable? Owner{get=>model.Owner;set{Calls.Add("Owner");model.Owner=value;}}public IFactory? Factory{get=>model.Factory;set{Calls.Add("Factory");model.Factory=value;}}public IRootDock? Layout{get{Reads++;return Sequence is null?model.Layout:Sequence.Dequeue();}set=>model.Layout=value;}public IHostWindow? Host{get=>model.Host;set{Calls.Add("Host");model.Host=value;}}
        public bool OnClose()=>model.OnClose();public bool OnMoveDragBegin()=>model.OnMoveDragBegin();public void OnMoveDrag()=>model.OnMoveDrag();public void OnMoveDragEnd()=>model.OnMoveDragEnd();public void Save(){Calls.Add("Save");}public void Present(bool dialog){Calls.Add(dialog?"PresentModal":"Present");}public void Exit(){Calls.Add("Exit");}public void SetActive()=>model.SetActive();
    }
}
