using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.Linq;
using Avalonia.Controls;
using Avalonia.Threading;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class NullableListReference
{
    internal static void Run(Action<string,bool> check,Func<string,object> load)
    {
        var factory=new NullInitOnlyFactory();
        foreach(var dock in new IDock[]{new DocumentDock(),new RootDock()})
        {
            int added=0;bool nullPayload=false;
            EventHandler<Dock.Model.Core.Events.DockableAddedEventArgs> handler=(_,e)=>{added++;nullPayload=e.Dockable is null;};
            factory.DockableAdded+=handler;factory.AddDockable(dock,null!);factory.DockableAdded-=handler;
            string kind=dock is RootDock?"root":"document";
            check("nullable-list-base-add-"+kind,dock.VisibleDockables?.Count==1&&dock.VisibleDockables[0] is null);
            check("nullable-list-empty-"+kind,dock.IsEmpty);
            check("nullable-list-opened-"+kind,dock.OpenedDockablesCount==(dock is RootDock?0:1));
            check("nullable-list-added-event-"+kind,added==1&&nullPayload);
        }
        var child=new Document();var root=new DocumentDock{VisibleDockables=new List<IDockable>{null!,child}};
        int calls=0;var query=factory.Find(root,_=>{calls++;return true;});
        check("nullable-query-deferred",calls==0);
        using(var cursor=query.GetEnumerator())
        {
            check("nullable-query-root",cursor.MoveNext()&&ReferenceEquals(cursor.Current,root));
            check("nullable-query-null-result",cursor.MoveNext()&&cursor.Current is null);
            check("nullable-query-after-null",cursor.MoveNext()&&ReferenceEquals(cursor.Current,child));
            check("nullable-query-end",!cursor.MoveNext()&&calls==3);
        }
        check("nullable-query-repeatable",query.Count()==3);
        check("nullable-query-null-predicate",factory.Find(root,value=>value is null).SequenceEqual(new IDockable[]{null!}));
        var observed=new List<IDockable>();
        Func<IDockable,bool> inspect=value=>{observed.Add(value);return true;};
        _=factory.Find(root,inspect).ToArray();
        check("nullable-query-delegate-original-values",observed.Count==3&&ReferenceEquals(observed[0],root)&&observed[1] is null&&ReferenceEquals(observed[2],child));
        var normalized=new RootDock{VisibleDockables=new List<IDockable>{null!,child}};
        var plain=normalized.VisibleDockables;factory.InitDockable(normalized,null);
        check("nullable-normalized-new-list",!ReferenceEquals(plain,normalized.VisibleDockables)&&normalized.VisibleDockables is System.Collections.Specialized.INotifyCollectionChanged);
        check("nullable-normalized-values",normalized.VisibleDockables?.Count==2&&normalized.VisibleDockables[0] is null&&ReferenceEquals(normalized.VisibleDockables[1],child));
        var same=normalized.VisibleDockables;factory.InitDockable(normalized,null);
        check("nullable-normalized-notifying-identity",ReferenceEquals(same,normalized.VisibleDockables));
        var split=new ProportionalDockSplitter();var cleanup=new RootDock{VisibleDockables=new List<IDockable>{split,null!,child},ActiveDockable=split};
        factory.Cleanup(cleanup);
        check("nullable-cleanup-values",cleanup.VisibleDockables.Count==2&&cleanup.VisibleDockables[0] is null&&ReferenceEquals(cleanup.VisibleDockables[1],child));
        check("nullable-cleanup-first-fallback",cleanup.ActiveDockable is null);
        var panel=(StackPanel)load("CoreNullListBindings");var list=new ObservableCollection<IDockable>{null!};
        var bound=new DocumentDock{VisibleDockables=list};panel.DataContext=bound;Dispatcher.UIThread.RunJobs();
        var count=(TextBlock)panel.Children[0];var content=(ContentControl)panel.Children[1];var items=(ItemsControl)panel.Children[2];
        check("nullable-bound-null-slot",count.Text=="1"&&content.Content is null&&items.Items.Count==1);
        list[0]=child;Dispatcher.UIThread.RunJobs();
        check("nullable-bound-initial-null-indexer-does-not-refresh",content.Content is null);
        panel.DataContext=null;panel.DataContext=bound;Dispatcher.UIThread.RunJobs();
        check("nullable-bound-nonnull-rebind",ReferenceEquals(content.Content,child));
        list[0]=null!;list.Add(null!);Dispatcher.UIThread.RunJobs();
        check("nullable-bound-nonnull-indexer-retains-value-until-rebind",ReferenceEquals(content.Content,child));
        panel.DataContext=null;panel.DataContext=bound;Dispatcher.UIThread.RunJobs();
        check("nullable-bound-null-replace-and-add",count.Text=="2"&&content.Content is null&&items.Items.Count==2);
        list.Remove(null!);Dispatcher.UIThread.RunJobs();
        check("nullable-bound-null-remove",count.Text=="1"&&content.Content is null&&items.Items.Count==1);
        panel.DataContext=null;
    }
}
internal sealed class NullInitOnlyFactory:Factory
{
    public override void InitDockable(IDockable dockable,IDockable? owner)
    {
        if(dockable is not null)base.InitDockable(dockable,owner);
    }
    internal void Cleanup(IDock dock)=>CleanupOrphanedSplitters(dock);
}
