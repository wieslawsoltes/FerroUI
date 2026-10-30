using System;
using System.Collections.Generic;
using Dock.Model.Core;
using Dock.Model.Controls;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class NullablePreviewReference
{
    private static bool NullFailure(Action action) { try { action(); return false; } catch (NullReferenceException) { return true; } }
    private static string Name(IDockable? item) => item?.Id ?? "null";
    private static List<IDockable> Install(PreviewFactory f, RootDock root, ToolDock preview, params IDockable[] items)
    {
        var list=new List<IDockable>(items);preview.VisibleDockables=list;root.PinnedDock=preview;f.Root=root;return list;
    }
    internal static void Run(Action<string,bool> check)
    {
        var f=new PreviewFactory();f.PreviewPinnedDockable(null!);f.TogglePreviewPinnedDockable(null!);
        check("nullable-preview-no-root-noop",string.Join(",",f.Trace)=="find:null,find:null");
        f.Trace.Clear();((IFactory)f).PreviewPinnedDockable((object?)null);((IFactory)f).TogglePreviewPinnedDockable((object?)null);
        check("nullable-preview-object-null-no-lookup",f.Trace.Count==0);
        check("nullable-preview-hide-null-root",NullFailure(()=>f.HidePreviewingDockables(null!)));
        check("nullable-preview-default-null-item",NullFailure(()=>new Factory().PreviewPinnedDockable(null!)));
        check("nullable-preview-default-null-toggle",NullFailure(()=>new Factory().TogglePreviewPinnedDockable(null!)));
        var root=new RootDock();f.HidePreviewingDockables(root);var preview=new ToolDock();root.PinnedDock=preview;f.HidePreviewingDockables(root);
        check("nullable-preview-hide-missing-list-keeps-preview",ReferenceEquals(root.PinnedDock,preview));
        f=new PreviewFactory();root=new RootDock();preview=new ToolDock();var item=new Tool{KeepPinnedDockableVisible=true,Owner=preview};preview.ActiveDockable=item;var list=Install(f,root,preview,item);
        check("nullable-preview-null-item-after-cleanup",NullFailure(()=>f.PreviewPinnedDockable(null!))&&root.PinnedDock is null&&list.Count==0&&ReferenceEquals(list,preview.VisibleDockables)&&preview.ActiveDockable is null);
        check("nullable-preview-null-item-restored-owner",ReferenceEquals(item.Owner,root)&&item.OriginalOwner is null&&string.Join(",",f.Trace)=="find:null");
        f=new PreviewFactory();root=new RootDock();f.Sequence.Enqueue(root);f.Sequence.Enqueue(null);f.TogglePreviewPinnedDockable(null!);
        check("nullable-preview-toggle-fixed-relookup",string.Join(",",f.Trace)=="find:null,find:null");
        f.Trace.Clear();preview=new ToolDock();list=Install(f,root,preview,new IDockable[]{null!});
        check("nullable-preview-toggle-null-snapshot-failure",NullFailure(()=>f.TogglePreviewPinnedDockable(null!))&&list.Count==1&&list[0] is null&&string.Join(",",f.Trace)=="find:null");
        foreach(bool keep in new[]{true,false})
        {
            f=new PreviewFactory();root=new RootDock();preview=new ToolDock();item=new Tool{KeepPinnedDockableVisible=keep,Owner=preview};var later=new Tool();preview.ActiveDockable=item;list=Install(f,root,preview,item,null!,later);
            check("nullable-preview-hide-null-snapshot-"+keep,NullFailure(()=>f.HidePreviewingDockables(root))&&list.Count==(keep?3:2)&&ReferenceEquals(preview.ActiveDockable,item)&&ReferenceEquals(item.Owner,keep?(IDockable)preview:root)&&ReferenceEquals(root.PinnedDock,preview));
        }
        foreach(bool clear in new[]{true,false})
        {
            f=new PreviewFactory();root=new RootDock();var old=new ToolDock();var replacement=new ToolDock();var a=new Tool{Owner=old};var b=new Tool{Owner=old};old.ActiveDockable=a;var oldList=Install(f,root,old,a,b);var newList=new List<IDockable>{a,b};replacement.VisibleDockables=newList;
            var liveRoot=root;bool once=false;a.PropertyChanged+=(_,e)=>{if(e.PropertyName=="Owner"&&!once){once=true;liveRoot.PinnedDock=clear?null:replacement;}};
            bool failed=NullFailure(()=>f.HidePreviewingDockables(root));
            check("nullable-preview-hide-live-removal-"+clear,clear?failed&&root.PinnedDock is null&&newList.Count==2&&ReferenceEquals(b.Owner,old):!failed&&ReferenceEquals(root.PinnedDock,replacement)&&newList.Count==0&&ReferenceEquals(b.Owner,root));
            check("nullable-preview-hide-cached-final-state-"+clear,oldList.Count==2&&ReferenceEquals(oldList,old.VisibleDockables)&&ReferenceEquals(old.ActiveDockable,a)&&ReferenceEquals(a.Owner,root));
        }
        foreach(int behavior in new[]{0,1,2})
        {
            f=new PreviewFactory();root=new RootDock{Id="root"};var owner=new ToolDock{Owner=root};item=new Tool{Id="item",Owner=owner};var old=new ToolDock();var restored=new ToolDock();root.PinnedDock=old;f.Root=root;f.Restore=restored;f.InitMode=behavior==2?2:0;
            var liveRoot=root;old.PropertyChanged+=(_,e)=>{if(e.PropertyName=="VisibleDockables")liveRoot.PinnedDock=null;};
            bool failed=NullFailure(()=>{if(behavior==1)new Factory().PreviewPinnedDockable(item);else f.PreviewPinnedDockable(item);});
            check("nullable-preview-init-null-outcome-"+behavior,behavior==2?!failed&&ReferenceEquals(root.PinnedDock,restored)&&ReferenceEquals(restored.ActiveDockable,item):failed&&root.PinnedDock is null);
            check("nullable-preview-init-null-trace-"+behavior,behavior==1?f.Trace.Count==0:string.Join(",",f.Trace)=="find:item,init:null:root");
            check("nullable-preview-init-null-committed-list-"+behavior,old.VisibleDockables?.Count==1&&ReferenceEquals(old.VisibleDockables[0],item)&&ReferenceEquals(item.OriginalOwner,owner)&&old.ActiveDockable is null);
        }
        f=new PreviewFactory();root=new RootDock{Id="root"};f.Root=root;f.InitMode=1;item=new Tool{Id="item"};
        check("nullable-preview-init-clears-live-root",NullFailure(()=>f.PreviewPinnedDockable(item))&&root.PinnedDock is null&&f.LastInit is IToolDock saved&&saved.VisibleDockables?.Count==1&&ReferenceEquals(saved.VisibleDockables[0],item)&&saved.ActiveDockable is null&&string.Join(",",f.Trace)=="find:item,init:some:root");
        f=new PreviewFactory();root=new RootDock();preview=new ToolDock();var splitter=new ProportionalDockSplitter{KeepPinnedDockableVisible=true};item=new Tool{KeepPinnedDockableVisible=true};list=Install(f,root,preview,splitter,item);f.HidePreviewingDockables(root);
        check("nullable-preview-keep-reselects-first-nonsplitter",list.Count==2&&ReferenceEquals(list,preview.VisibleDockables)&&ReferenceEquals(preview.ActiveDockable,item)&&ReferenceEquals(root.PinnedDock,preview));
    }
    private sealed class PreviewFactory:Factory
    {
        internal IRootDock? Root;internal readonly Queue<IRootDock?> Sequence=new();internal readonly List<string> Trace=new();internal int InitMode;internal IToolDock? Restore;internal IDockable? LastInit;
        public override IRootDock? FindRoot(IDockable item,Func<IRootDock,bool>? predicate=null){Trace.Add("find:"+Name(item));return Sequence.Count==0?Root:Sequence.Dequeue();}
        public override void InitDockable(IDockable item,IDockable? owner)
        {
            Trace.Add("init:"+(item is null?"null":"some")+":"+Name(owner));LastInit=item;
            if(InitMode!=0)((IRootDock)owner!).PinnedDock=InitMode==1?null:Restore;
        }
    }
}
