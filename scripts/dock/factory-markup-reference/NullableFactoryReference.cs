using System;
using System.Collections.Generic;
using Avalonia.Controls;
using Avalonia.Threading;
using Dock.Model.Controls;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class NullableFactoryReference
{
    internal static void Run(Action<string,bool> check,Func<string,object> load)
    {
        var trace=new List<string>();var f=new NullFactory("f",trace);
        var d=new DocumentDock{Factory=f,DocumentFactory=()=>null!};
        d.CreateDocument!.Execute(null);d.AddDocument(null!);d.AddTool(null!);
        var tool=new ToolDock{Factory=f};tool.AddTool(null!);
        check("nullable-factory-order",string.Join(",",trace)==string.Join(",",System.Linq.Enumerable.Repeat("f:add,f:active,f:focus",4)));
        check("nullable-factory-no-graph",d.VisibleDockables is null&&tool.VisibleDockables is null);
        check("nullable-factory-owner",ReferenceEquals(f.LastDock,tool));
        foreach(string remove in new[]{"add","active","never"})
        {
            trace.Clear();var first=new NullFactory("first",trace);var second=new NullFactory("second",trace);
            var owner=new DocumentDock{Factory=first};
            first.OnAdd=()=>owner.Factory=remove=="add"?null:second;
            if(remove=="active")second.OnActive=()=>owner.Factory=null;
            owner.AddDocument(null!);
            string expected=remove=="add"?"first:add":remove=="active"?"first:add,second:active":"first:add,second:active,second:focus";
            check("nullable-factory-reread-"+remove,string.Join(",",trace)==expected);
        }
        trace.Clear();f.InitDockable(null!,d);f.AbortInit=true;
        bool stopped=false;try{f.AddViaBase(d,null!);}catch(InvalidOperationException e)when(e.Message=="init sentinel"){stopped=true;}
        check("nullable-factory-init-dispatch",stopped&&string.Join(",",trace)=="f:init,f:init");
        check("nullable-factory-init-owner",ReferenceEquals(f.LastDock,d));
        check("nullable-factory-init-before-graph",d.VisibleDockables is null);
        var original=new Factory();bool init=false,active=false;
        try{original.InitDockable(null!,null);}catch(NullReferenceException){init=true;}
        try{original.SetActiveDockable(null!);}catch(NullReferenceException){active=true;}
        check("nullable-factory-default-init",init);check("nullable-factory-default-active",active);
        trace.Clear();var panel=(StackPanel)load("CoreNullDocumentBindings");int calls=0;
        var bound=new DocumentDock{Factory=f,DocumentFactory=()=>{calls++;return null!;}};
        panel.DataContext=bound;Dispatcher.UIThread.RunJobs();
        for(int i=0;i<3;i++)((Button)panel.Children[i]).Command!.Execute(null);
        check("nullable-factory-bound-order",string.Join(",",trace)=="f:add,f:active,f:focus,f:add,f:active,f:focus,f:add,f:active,f:focus");
        check("nullable-factory-bound-once",calls==1&&bound.VisibleDockables is null);panel.DataContext=null;
        // Matched Rust base Add with the same null-aware initialization override.
        f.AbortInit=false;trace.Clear();var listOwner=new DocumentDock();f.AddViaBase(listOwner,null!);
        check("nullable-base-add-list-element",listOwner.VisibleDockables?.Count==1&&listOwner.VisibleDockables[0] is null);
    }
}
internal sealed class NullFactory:Factory
{
    private readonly string _label;private readonly List<string> _trace;
    internal Action? OnAdd,OnActive;internal IDock? LastDock;internal bool AbortInit;
    internal NullFactory(string label,List<string> trace){_label=label;_trace=trace;}
    private void Record(string name,IDockable item){if(item is not null)throw new InvalidOperationException("null fixture");_trace.Add(_label+":"+name);}
    public override void AddDockable(IDock dock,IDockable dockable){Record("add",dockable);LastDock=dock;OnAdd?.Invoke();}
    public override void SetActiveDockable(IDockable dockable){Record("active",dockable);OnActive?.Invoke();}
    public override void SetFocusedDockable(IDock dock,IDockable? dockable){Record("focus",dockable!);LastDock=dock;}
    public override void InitDockable(IDockable dockable,IDockable? owner){Record("init",dockable);LastDock=owner as IDock;if(AbortInit)throw new InvalidOperationException("init sentinel");}
    internal void AddViaBase(IDock dock,IDockable item)=>base.AddDockable(dock,item);
}
