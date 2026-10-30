using System;
using System.Collections.Generic;
using System.Linq;
using Dock.Model;
using Dock.Model.Core;
using Dock.Model.Controls;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class NullableMoveReference
{
    private static string Name(IDockable? item) => item?.Id ?? "null";
    private static string Shape(IList<IDockable> list) => string.Join(",", list.Select(Name));
    private static bool NullFailure(Action action) { try { action(); return false; } catch (NullReferenceException) { return true; } }
    private static Document Item(string id) => new() { Id = id };
    private static MoveFactory Scene()
    {
        var f = new MoveFactory();
        f.DockableAdded += (_,e) => f.Trace.Add("added:" + Name(e.Dockable));
        f.DockableRemoved += (_,e) => f.Trace.Add("removed:" + Name(e.Dockable));
        f.DockableMoved += (_,e) => f.Trace.Add("moved:" + Name(e.Dockable));
        f.DockableSwapped += (_,e) => f.Trace.Add("swapped:" + Name(e.Dockable));
        f.DockableUndocked += (_,e) => f.Trace.Add("undocked:" + Name(e.Dockable));
        f.DockableDocked += (_,e) => f.Trace.Add("docked:" + Name(e.Dockable));
        return f;
    }
    internal static void Run(Action<string,bool> check)
    {
        var f=Scene();var dock=new DocumentDock();var a=Item("a");var b=Item("b");var list=new List<IDockable>{null!,a,b};dock.VisibleDockables=list;
        f.MoveDockable(dock,null!,b);
        check("nullable-move-same-null-source-order",string.Join(",",f.Trace)=="removed:null,undocked:null,added:null,moved:null,docked:null");
        check("nullable-move-same-null-source-values",Shape(list)=="a,b,null"&&ReferenceEquals(list,dock.VisibleDockables)&&dock.ActiveDockable is null);
        f.Trace.Clear();f.MoveDockable(dock,b,null!);
        check("nullable-move-same-null-target-values",Shape(list)=="a,null,b"&&ReferenceEquals(dock.ActiveDockable,b));
        check("nullable-move-same-null-target-order",string.Join(",",f.Trace)=="removed:b,root:b,root:b,root:b,root:b,root:b,undocked:b,added:b,root:b,root:b,root:b,root:b,root:b,moved:b,docked:b");
        f.Trace.Clear();f.MoveDockable(dock,null!,null!);f.MoveDockable(new DocumentDock(),null!,null!);
        check("nullable-move-same-absent-noop",f.Trace.Count==0);
        check("nullable-move-same-null-dock",NullFailure(()=>f.MoveDockable(null!,null!,null!)));
        f=Scene();dock=new DocumentDock();b=Item("b");var c=Item("c");list=new List<IDockable>{null!,b};dock.VisibleDockables=list;
        var reentryList=list;var reentryFactory=f;
        EventHandler<Dock.Model.Core.Events.DockableRemovedEventArgs> handler=(_,e)=>{check("nullable-swap-reentry-before-second-write",e.Dockable?.Id=="b"&&Shape(reentryList)=="null,null");reentryList[0]=c;reentryFactory.Trace.Add("reentered");};
        f.DockableRemoved+=handler;f.SwapDockable(dock,null!,b);f.DockableRemoved-=handler;
        check("nullable-swap-reentry-original-values",Shape(list)=="b,null"&&ReferenceEquals(list,dock.VisibleDockables)&&ReferenceEquals(dock.ActiveDockable,b));
        check("nullable-swap-reentry-order",string.Join(",",f.Trace)=="removed:b,reentered,root:b,root:b,root:b,root:b,root:b,added:null,added:b,root:b,root:b,root:b,root:b,root:b,swapped:null,swapped:b");
        f.Trace.Clear();f.SwapDockable(dock,null!,null!);check("nullable-swap-same-null-noop",f.Trace.Count==0);
        check("nullable-swap-same-null-dock",NullFailure(()=>f.SwapDockable(null!,null!,null!)));
        foreach(bool custom in new[]{true,false})
        {
            f=Scene();var source=new DocumentDock{Id="source"};var target=new DocumentDock{Id="target"};b=Item("b");var left=new List<IDockable>{null!};var right=new List<IDockable>{b};source.VisibleDockables=left;target.VisibleDockables=right;
            if(custom)
            {
                f.SwapDockable(source,target,null!,b);
                check("nullable-swap-between-custom-init-order",string.Join(",",f.Trace)=="init:null:target,init:b:source,swapped:b,swapped:null");
                check("nullable-swap-between-custom-selection",ReferenceEquals(source.ActiveDockable,b)&&target.ActiveDockable is null);
            }
            else check("nullable-swap-between-default-init-failure",NullFailure(()=>new Factory().SwapDockable(source,target,null!,b))&&b.Factory is null&&source.ActiveDockable is null);
            check("nullable-swap-between-committed-lists-"+custom,Shape(left)=="b"&&Shape(right)=="null"&&ReferenceEquals(left,source.VisibleDockables)&&ReferenceEquals(right,target.VisibleDockables));
        }
        f=Scene();dock=new DocumentDock{Id="same",VisibleDockables=new List<IDockable>{null!}};f.SwapDockable(dock,dock,null!,null!);
        check("nullable-swap-between-same-null-still-inits",string.Join(",",f.Trace)=="init:null:same,init:null:same,swapped:null,swapped:null");
        f=Scene();var owner=new DocumentDock();a=Item("a");b=Item("b");list=new List<IDockable>{null!,a,b};owner.VisibleDockables=list;owner.ActiveDockable=a;
        f.MoveDockable(owner,owner,null!,b);
        check("nullable-move-between-same-null-order",string.Join(",",f.Trace)=="root:null,added:null,removed:null,undocked:null,moved:null,docked:null");
        check("nullable-move-between-same-null-values",Shape(list)=="a,b,null"&&ReferenceEquals(owner.ActiveDockable,a));
        f.Trace.Clear();var destination=new DocumentDock{Id="target"};var destinationList=new List<IDockable>();destination.VisibleDockables=destinationList;f.MoveDockable(owner,destination,null!,null);
        check("nullable-move-between-custom-remove-order",string.Join(",",f.Trace)=="root:null,remove:null,undocked:null,init:null:target,added:null,moved:null,docked:null");
        check("nullable-move-between-custom-remove-values",Shape(list)=="a,b,null"&&Shape(destinationList)=="null"&&ReferenceEquals(destinationList,destination.VisibleDockables)&&destination.IsEmpty&&destination.OpenedDockablesCount==1);
        f=new MoveFactory();check("nullable-move-between-null-target-after-unpin",NullFailure(()=>f.MoveDockable(null!,null!,null!,null))&&string.Join(",",f.Trace)=="root:null");
        f.Trace.Clear();destination=new DocumentDock();check("nullable-move-between-null-source-after-target-list",NullFailure(()=>f.MoveDockable(null!,destination,null!,null))&&destination.VisibleDockables?.Count==0&&string.Join(",",f.Trace)=="root:null");
        f.Trace.Clear();owner=new DocumentDock();f.SwapDockable(owner,null!,null!,null!);check("nullable-swap-between-short-circuit",f.Trace.Count==0);owner.VisibleDockables=new List<IDockable>();check("nullable-swap-between-null-target",NullFailure(()=>f.SwapDockable(owner,null!,null!,null!)));check("nullable-swap-between-null-source",NullFailure(()=>f.SwapDockable(null!,null!,null!,null!)));
        f=Scene();var root=new RootDock();a=Item("a");var pins=new List<IDockable>{null!,a};root.LeftPinnedDockables=pins;f.QueryRoot=root;f.MovePinnedDockable(null!,a);
        check("nullable-move-pinned-order",string.Join(",",f.Trace)=="root:null,moved:null");check("nullable-move-pinned-values",Shape(pins)=="a,null"&&ReferenceEquals(pins,root.LeftPinnedDockables));
        f.Trace.Clear();root.LeftPinnedDockables=new List<IDockable>{null!};root.RightPinnedDockables=new List<IDockable>{a};f.MovePinnedDockable(null!,a);check("nullable-move-pinned-first-strip-only",string.Join(",",f.Trace)=="root:null");
        f.QueryRoot=null;f.Trace.Clear();f.MovePinnedDockable(null!,null!);check("nullable-move-pinned-null-root-noop",string.Join(",",f.Trace)=="root:null");
        f=new MoveFactory();f.UnpinDockable(null!);check("nullable-move-unpin-null-no-root",string.Join(",",f.Trace)=="root:null");f.Trace.Clear();pins=new List<IDockable>{null!};f.QueryRoot=new RootDock{LeftPinnedDockables=pins};f.UnpinDockable(null!);check("nullable-move-unpin-null-virtual-pin",string.Join(",",f.Trace)=="root:null,pin:null"&&pins.Count==1);
        check("nullable-move-unpin-default-null-failure",NullFailure(()=>new Factory().UnpinDockable(null!)));check("nullable-move-pin-default-null-failure",NullFailure(()=>new Factory().PinDockable(null!)));
        check("nullable-move-capability-evaluate-null",NullFailure(()=>DockCapabilityResolver.Evaluate(null!,DockCapability.Pin)));check("nullable-move-capability-context-null",NullFailure(()=>DockCapabilityResolver.Evaluate(null!,DockCapability.Pin,null)));check("nullable-move-capability-enabled-null",NullFailure(()=>DockCapabilityResolver.IsEnabled(null!,DockCapability.Pin,new DocumentDock())));check("nullable-move-capability-drop-null",NullFailure(()=>DockCapabilityResolver.ResolveDropTargetDock(null!)));
    }
    private sealed class MoveFactory:Factory
    {
        internal readonly List<string> Trace=new();internal IRootDock? QueryRoot;
        public override IRootDock? FindRoot(IDockable item,Func<IRootDock,bool>? predicate=null){Trace.Add("root:"+Name(item));return QueryRoot;}
        public override void PinDockable(IDockable item)=>Trace.Add("pin:"+Name(item));
        public override void RemoveDockable(IDockable item,bool collapse)=>Trace.Add("remove:"+Name(item));
        public override void InitDockable(IDockable item,IDockable? owner)=>Trace.Add("init:"+Name(item)+":"+Name(owner));
    }
}
