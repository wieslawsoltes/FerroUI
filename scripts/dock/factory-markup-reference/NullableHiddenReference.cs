using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using Dock.Model.Core;
using Dock.Model.Controls;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class NullableHiddenReference
{
    private static bool NullFailure(Action action){try{action();return false;}catch(NullReferenceException){return true;}}
    internal static void Run(Action<string,bool> check)
    {
        var f=new HiddenFactory();f.HideDockable((IDockable)null!);check("nullable-hidden-hide-no-root-order",string.Join(",",f.Trace)=="find:null:default,find:null:all");f.Trace.Clear();f.RestoreDockable((IDockable)null!);check("nullable-hidden-restore-no-root-order",string.Join(",",f.Trace)=="find:null:all");
        check("nullable-hidden-hide-default-failure",NullFailure(()=>new Factory().HideDockable((IDockable)null!)));check("nullable-hidden-restore-default-failure",NullFailure(()=>new Factory().RestoreDockable((IDockable)null!)));
        f=new HiddenFactory();var first=new RootDock();var second=new RootDock();var pins=new List<IDockable>{null!};first.LeftPinnedDockables=pins;f.Root=first;f.AfterPin=second;
        check("nullable-hidden-hide-pin-reroot-order",NullFailure(()=>f.HideDockable((IDockable)null!))&&string.Join(",",f.Trace)=="find:null:default,pin:null,find:null:all");
        check("nullable-hidden-hide-list-created-before-owner",first.HiddenDockables is null&&second.HiddenDockables?.Count==0&&pins.Count==1&&pins[0] is null);
        for(int mode=0;mode<4;mode++)
        {
            f=new HiddenFactory();var root=new RootDock();f.Root=root;
            if(mode>0)root.HiddenDockables=mode==1?new List<IDockable>():mode==2?new List<IDockable>{new Tool()}:new List<IDockable>{null!};
            var current=f;var observed=root.HiddenDockables;f.DockableRestored+=(_,e)=>{check("nullable-hidden-restore-null-event",e.Dockable is null&&observed?.Count==0);current.Trace.Add("restored:null");};
            if(mode==3)check("nullable-hidden-restore-null-committed-removal",NullFailure(()=>f.RestoreDockable((IDockable)null!))&&root.HiddenDockables?.Count==0&&string.Join(",",f.Trace)=="find:null:all,restored:null");
            else{f.RestoreDockable((IDockable)null!);check("nullable-hidden-restore-short-circuit-"+mode,string.Join(",",f.Trace)=="find:null:all"&&root.HiddenDockables?.Count==(mode==0?(int?)null:mode==1?0:1));}
        }
        f=new HiddenFactory();var eventRoot=new RootDock();var old=new ObservableCollection<IDockable>{null!,null!};var replacement=new List<IDockable>{new Tool()};eventRoot.HiddenDockables=old;f.Root=eventRoot;var eventFactory=f;
        old.CollectionChanged+=(_,e)=>{check("nullable-hidden-remove-null-collection-payload",e.OldItems?.Count==1&&e.OldItems[0] is null&&e.OldStartingIndex==0);eventFactory.Trace.Add("list:null:0");};
        f.DockableRestored+=(_,e)=>{check("nullable-hidden-restore-reentry-event-state",e.Dockable is null&&old.Count==1);eventFactory.Trace.Add("restored:null");eventRoot.HiddenDockables=replacement;};
        check("nullable-hidden-restore-reentry-order",NullFailure(()=>f.RestoreDockable((IDockable)null!))&&string.Join(",",f.Trace)=="find:null:all,list:null:0,restored:null");
        check("nullable-hidden-restore-reentry-keeps-both-lists",old.Count==1&&old[0] is null&&ReferenceEquals(replacement,eventRoot.HiddenDockables)&&replacement.Count==1);
        foreach(bool replace in new[]{true,false})
        {
            var s=Scene.Create();check("nullable-hidden-hide-initial-list-absent-"+replace,s.Root.HiddenDockables is null);var newList=new List<IDockable>();IList<IDockable>? captured=null;var trace=new List<string>();
            s.Factory.DockableRemoved+=(_,e)=>{captured=s.Root.HiddenDockables;s.Root.HiddenDockables=replace?newList:null;trace.Add("removed");};s.Factory.DockableHidden+=(_,e)=>trace.Add("hidden");
            if(replace){s.Factory.HideDockable(s.Item);check("nullable-hidden-hide-live-new-list",ReferenceEquals(newList,s.Root.HiddenDockables)&&newList.Count==1&&ReferenceEquals(newList[0],s.Item)&&string.Join(",",trace)=="removed,hidden");}
            else check("nullable-hidden-hide-live-null-list",NullFailure(()=>s.Factory.HideDockable(s.Item))&&s.Root.HiddenDockables is null&&string.Join(",",trace)=="removed");
            check("nullable-hidden-hide-live-partial-state-"+replace,captured?.Count==0&&s.Visible.Count==0&&ReferenceEquals(s.Visible,s.Owner.VisibleDockables)&&ReferenceEquals(s.Item.Owner,s.Root)&&ReferenceEquals(s.Item.OriginalOwner,s.Owner));
        }
        {
            var s=Scene.Create();s.Factory.HideDockable(s.Item);var hidden=s.Root.HiddenDockables!;
            s.Factory.DockableRestored+=(_,e)=>{check("nullable-hidden-redirect-observer-before-owner-write",ReferenceEquals(e.Dockable,s.Item)&&ReferenceEquals(s.Item.Owner,s.Root));s.Item.OriginalOwner=s.Alternate;};
            s.Factory.RestoreDockable(s.Item);
            check("nullable-hidden-redirect-shared-lists",hidden.Count==0&&ReferenceEquals(hidden,s.Root.HiddenDockables)&&s.Visible.Count==0&&s.OtherVisible.Count==1&&ReferenceEquals(s.OtherVisible,s.Alternate.VisibleDockables)&&ReferenceEquals(s.OtherVisible[0],s.Item));
            check("nullable-hidden-redirect-owner-factory",ReferenceEquals(s.Item.Owner,s.Alternate)&&s.Item.OriginalOwner is null&&ReferenceEquals(s.Item.Factory,s.Factory));
        }
        {
            var dispatch=new DispatchFactory();dispatch.HideDockable((IDockable)null!);dispatch.RestoreDockable((IDockable)null!);check("nullable-hidden-virtual-overrides-receive-null",string.Join(",",dispatch.Trace)=="hide,restore");
        }
        {
            var factory=new Factory();var root=new RootDock{Id="root",VisibleDockables=new List<IDockable>{null!,new Tool{Id="wanted"}}};var laterItem=new Tool{Id="wanted"};var later=new RootDock{VisibleDockables=new List<IDockable>{laterItem},HiddenDockables=new List<IDockable>{laterItem}};
            var control=new Dock.Avalonia.Controls.DockControl{InitializeLayout=false,InitializeFactory=false,Layout=root};var next=new Dock.Avalonia.Controls.DockControl{InitializeLayout=false,InitializeFactory=false,Layout=later};factory.DockControls.Add(control);factory.DockControls.Add(next);
            check("nullable-hidden-id-hide-first-null-failure",NullFailure(()=>factory.HideDockable("wanted"))&&root.VisibleDockables.Count==2&&root.HiddenDockables is null&&later.HiddenDockables.Count==1);
            root.HiddenDockables=new List<IDockable>{null!,new Tool{Id="wanted"}};
            check("nullable-hidden-id-restore-first-null-failure",NullFailure(()=>factory.RestoreDockable("wanted"))&&root.HiddenDockables.Count==2&&later.HiddenDockables.Count==1);
        }
    }
    private sealed class HiddenFactory:Factory
    {
        internal IRootDock? Root,AfterPin;internal readonly List<string> Trace=new();
        public override IRootDock? FindRoot(IDockable item,Func<IRootDock,bool>? predicate=null){if(item is not null)throw new InvalidOperationException("nullable fixture");Trace.Add("find:null:"+(predicate is null?"default":"all"));return Root;}
        protected override void PinDockableCore(IDockable item){if(item is not null)throw new InvalidOperationException("nullable fixture");Trace.Add("pin:null");if(AfterPin is not null)Root=AfterPin;}
    }
    private sealed class DispatchFactory:Factory
    {
        internal readonly List<string> Trace=new();
        public override void HideDockable(IDockable item){if(item is not null)throw new InvalidOperationException("nullable fixture");Trace.Add("hide");}
        public override void RestoreDockable(IDockable item){if(item is not null)throw new InvalidOperationException("nullable fixture");Trace.Add("restore");}
    }
    private sealed record Scene(Factory Factory,IRootDock Root,IToolDock Owner,IToolDock Alternate,IDockable Item,IList<IDockable> Visible,IList<IDockable> OtherVisible)
    {
        internal static Scene Create(){var f=new Factory();var root=f.CreateRootDock();var owner=f.CreateToolDock();var alternate=f.CreateToolDock();var item=f.CreateTool();item.Id="item";f.AddDockable(root,owner);f.AddDockable(root,alternate);f.AddDockable(owner,item);f.InitLayout(root);return new(f,root,owner,alternate,item,owner.VisibleDockables!,alternate.VisibleDockables!);}
    }
}
