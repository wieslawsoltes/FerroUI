using System;
using System.Collections.Generic;
using Dock.Model.Core;
using Dock.Model.Controls;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class NullableStateReference
{
    static bool Fails(Action action){try{action();return false;}catch(NullReferenceException){return true;}}
    internal static void Run(Action<string,bool> check)
    {
        var f=new StateFactory();var child=new Document{Id="child"};var pane=new Tool{Id="pane"};
        var split=new SplitViewDock{Id="split",VisibleDockables=new List<IDockable>{null!,child},PaneDockable=pane,ContentDockable=pane};
        f.Walk(split);
        check("nullable-state-recursive-virtual-order",string.Join(",",f.Trace)=="update:split,recurse:null,update:null,recurse:child,update:child,recurse:pane,update:pane");
        f.Trace.Clear();check("nullable-state-update-null-assignment",Fails(()=>f.Update(null!)));
        check("nullable-state-resolve-before-null-assignment",string.Join(",",f.Trace)=="resolve:null");
        f.Trace.Clear();((IDockingWindowStateSync)f).OnDockingWindowStatePropertyChanged(new Tool{Id="tool"},DockingWindowStateProperty.IsSelected);
        check("nullable-state-scope-unwinds",f.Trace.Count>0&&f.Trace[0]=="resolve:tool");
        f=new StateFactory{InterceptNullRoot=true,NullRoot=new RootDock()};var expected=f.NullRoot;
        check("nullable-root-override-null",ReferenceEquals(f.FindRoot(null!),expected));
        var first=new RootDock();var leaf=new Tool{Id="leaf",Owner=first};f.Trace.Clear();
        var found=f.FindRoot(leaf,_=>{leaf.Owner=null;return false;});
        check("nullable-root-reread-null-override",ReferenceEquals(found,expected)&&string.Join(",",f.Trace)=="root:leaf,root:null");
        check("nullable-root-default-null-failure",Fails(()=>new Factory().FindRoot(null!)));
        f=new StateFactory();check("nullable-state-default-root-failure",Fails(()=>f.Resolve(null!))&&string.Join(",",f.Trace)=="root:null");
        f.InterceptNullRoot=true;f.Trace.Clear();check("nullable-state-missing-root-failure",Fails(()=>f.Resolve(null!))&&string.Join(",",f.Trace)=="root:null");
        var root=new RootDock{LeftPinnedDockables=new List<IDockable>{null!}};f.NullRoot=root;
        check("nullable-pin-explicit-root",f.IsDockablePinned(null!,root));
        check("nullable-pin-virtual-root",f.IsDockablePinned(null!));
        check("nullable-state-pinned-null-failure",Fails(()=>f.Resolve(null!)));
        root.LeftPinnedDockables=null;check("nullable-pin-cleared",!f.IsDockablePinned(null!,root));
    }
    sealed class StateFactory:Factory
    {
        internal readonly List<string> Trace=new();
        internal bool InterceptNullRoot;internal IRootDock? NullRoot;
        static string Name(IDockable? item)=>item?.Id??"null";
        protected override void UpdateDockingWindowState(IDockable item)=>Trace.Add("update:"+Name(item));
        protected override void UpdateDockingWindowStateRecursive(IDockable item){Trace.Add("recurse:"+Name(item));base.UpdateDockingWindowStateRecursive(item);}
        protected override DockingWindowState ResolveDockingWindowState(IDockable item){Trace.Add("resolve:"+Name(item));return DockingWindowState.Docked;}
        public override IRootDock? FindRoot(IDockable item,Func<IRootDock,bool>? predicate=null){Trace.Add("root:"+Name(item));return item is null&&InterceptNullRoot?NullRoot:base.FindRoot(item,predicate);}
        internal void Walk(IDockable item)=>base.UpdateDockingWindowStateRecursive(item);
        internal void Update(IDockable item)=>base.UpdateDockingWindowState(item);
        internal DockingWindowState Resolve(IDockable item)=>base.ResolveDockingWindowState(item);
    }
}
