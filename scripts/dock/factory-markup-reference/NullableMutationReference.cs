using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using Dock.Model.Core;
using Dock.Model.Controls;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class NullableMutationReference
{
    static bool NullFailure(Action action){try{action();return false;}catch(NullReferenceException){return true;}}
    internal static void Run(Action<string,bool> check)
    {
        var f=new MutationFactory();var dock=new DocumentDock{Id="docs"};
        f.InsertDockable(null!,null!,-1);f.InsertDockable(dock,null!,-1);
        check("nullable-insert-negative-no-dispatch",f.Trace.Count==0&&dock.VisibleDockables is null);
        new Factory().InsertDockable(null!,null!,-1);check("nullable-insert-default-negative-null-target",true);
        var list=new ObservableCollection<IDockable>();dock.VisibleDockables=list;
        list.CollectionChanged+=(_,e)=>{check("nullable-insert-change-payload",e.NewItems?.Count==1&&e.NewItems[0] is null&&e.NewStartingIndex==0);f.Trace.Add("list");};
        f.DockableAdded+=(_,e)=>{check("nullable-insert-added-payload",e.Dockable is null);f.Trace.Add("added:null");};
        f.InsertDockable(dock,null!,0);
        check("nullable-insert-order",string.Join(",",f.Trace)=="init:null:docs,list,added:null");
        check("nullable-insert-list-identity",ReferenceEquals(list,dock.VisibleDockables)&&list.Count==1&&list[0] is null);
        check("nullable-insert-empty-opened",dock.IsEmpty&&dock.OpenedDockablesCount==1);
        f=new MutationFactory();check("nullable-insert-null-target-after-init",NullFailure(()=>f.InsertDockable(null!,new Tool{Id="tool"},0))&&string.Join(",",f.Trace)=="init:tool:null");
        f.Trace.Clear();dock=new DocumentDock{Id="docs"};bool invalid=false;try{f.InsertDockable(dock,null!,2);}catch(ArgumentOutOfRangeException){invalid=true;}
        check("nullable-insert-invalid-creates-list",invalid&&dock.VisibleDockables?.Count==0&&string.Join(",",f.Trace)=="init:null:docs");
        var untouched=new DocumentDock();check("nullable-insert-default-null-before-list",NullFailure(()=>new Factory().InsertDockable(untouched,null!,0))&&untouched.VisibleDockables is null);
        f=new MutationFactory();check("nullable-remove-root-before-owner",NullFailure(()=>f.RemoveDockable(null!,true))&&string.Join(",",f.Trace)=="root:null");
        f.Trace.Clear();var pins=new List<IDockable>{null!};var root=new RootDock{LeftPinnedDockables=pins};f.NullRoot=root;
        check("nullable-remove-pin-override-before-owner",NullFailure(()=>f.RemoveDockable(null!,true))&&string.Join(",",f.Trace)=="root:null,pin:null");
        check("nullable-remove-failure-keeps-pin",pins.Count==1&&pins[0] is null);
        check("nullable-remove-default-failure",NullFailure(()=>new Factory().RemoveDockable(null!,true)));
        f=new MutationFactory();f.BasePin(null!);check("nullable-pin-core-no-root-noop",string.Join(",",f.Trace)=="root:null");
        f.Trace.Clear();f.NullRoot=new RootDock();f.BasePin(null!);check("nullable-pin-root-overload-receives-null",string.Join(",",f.Trace)=="root:null,pin-root:null");
        f.Trace.Clear();f.PinRootBase=true;check("nullable-pin-root-default-owner-failure",NullFailure(()=>f.BasePin(null!))&&string.Join(",",f.Trace)=="root:null,pin-root:null");
        f=new MutationFactory{RemovalNoop=true};root=new RootDock();dock=new DocumentDock{Id="docs",Owner=root,IsCollapsable=false};
        var item=new Tool{Id="trigger",Owner=dock};var original=new List<IDockable>{null!,item};dock.VisibleDockables=original;
        f.DockableMoved+=(_,e)=>f.Trace.Add("moved:"+MutationFactory.Name(e.Dockable));
        f.FloatAllDockables(item);
        check("nullable-float-snapshot-override-order",string.Join(",",f.Trace)=="remove:null,init:null:docs,moved:null,remove:trigger,init:trigger:docs,moved:trigger,window");
        check("nullable-float-target-values",f.Floated?.VisibleDockables?.Count==2&&f.Floated.VisibleDockables[0] is null&&ReferenceEquals(f.Floated.VisibleDockables[1],item));
        check("nullable-float-noop-remove-keeps-source",original.Count==2&&original[0] is null&&ReferenceEquals(original[1],item));
    }
    sealed class MutationFactory:Factory
    {
        internal readonly List<string> Trace=new();internal IRootDock? NullRoot;internal bool PinRootBase,RemovalNoop;internal IDock? Floated;
        internal static string Name(IDockable? item)=>item?.Id??"null";
        public override void InitDockable(IDockable item,IDockable? owner)=>Trace.Add("init:"+Name(item)+":"+Name(owner));
        public override IRootDock? FindRoot(IDockable item,Func<IRootDock,bool>? predicate=null){if(item is null){Trace.Add("root:null");return NullRoot;}return base.FindRoot(item,predicate);}
        protected override void PinDockableCore(IDockable item)=>Trace.Add("pin:"+Name(item));
        protected override void PinDockableCore(IDockable item,IRootDock root,IToolDock? unpin){Trace.Add("pin-root:"+Name(item));if(PinRootBase)base.PinDockableCore(item,root,unpin);}
        public override void RemoveDockable(IDockable item,bool collapse){if(RemovalNoop)Trace.Add("remove:"+Name(item));else base.RemoveDockable(item,collapse);}
        public override IDockWindow? CreateWindowFrom(IDockable item,DockWindowOptions? options){Trace.Add("window");Floated=(IDock)item;return null;}
        internal void BasePin(IDockable item)=>base.PinDockableCore(item);
    }
}
