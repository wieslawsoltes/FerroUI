using System;
using System.Collections.Generic;
using Dock.Model.Core;
using Dock.Model.Controls;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class NullableFloatingReference
{
    private static bool NullFailure(Action action){try{action();return false;}catch(NullReferenceException){return true;}}
    private static bool PrepareFailure(Action action){try{action();return false;}catch(InvalidOperationException e)when(e.Message=="prepare aborted"){return true;}}
    private static string Name(IDockable? item)=>item?.Id??"null";
    private static IDockWindow Window()=>new Factory().CreateDockWindow();
    internal static void Run(Action<string,bool> check)
    {
        var ordinary=new Factory();check("nullable-floating-default-plain-null",ordinary.CreateWindowFrom(null!)is null);check("nullable-floating-default-options-null",ordinary.CreateWindowFrom(null!,null)is null);
        var options=new DockWindowOptions();check("nullable-floating-default-ownership-guard",ordinary.CreateWindowFrom(null!,options)is null&&options.ParentWindow is null);
        options.OwnerMode=DockWindowOwnerMode.DockableWindow;check("nullable-floating-default-required-root-failure",NullFailure(()=>ordinary.CreateWindowFrom(null!,options)));var parent=Window();options.ParentWindow=parent;check("nullable-floating-default-existing-parent-guard",ordinary.CreateWindowFrom(null!,options)is null&&ReferenceEquals(options.ParentWindow,parent));
        var f=new WindowFactory{Result=Window(),Root=new RootDock()};check("nullable-floating-plain-no-creation",f.BasePlain(null!)is null&&f.BasePlain(new StackDock())is null&&f.Trace.Count==0);
        f=new WindowFactory();f.FloatDockable(null!);f.FloatAllDockables(null!);f.SplitToWindow(null!,null!,1,2,300,400);
        check("nullable-floating-wrapper-overload-order",string.Join(",",f.Trace)=="float:null,all:null,split:null:null");check("nullable-floating-wrapper-original-null-coordinates",f.LastItem is null&&f.LastDock is null&&f.LastOptions is null&&f.Coordinates==(1d,2d,300d,400d));
        check("nullable-floating-default-float-null",NullFailure(()=>new Factory().FloatDockable(null!)));check("nullable-floating-default-all-null",NullFailure(()=>new Factory().FloatAllDockables(null!)));check("nullable-floating-default-split-null",NullFailure(()=>new Factory().SplitToWindow(null!,null!,0,0,1,1)));
        f=new WindowFactory();f.BaseSplit(null!,null!,0,0,1,1,null);check("nullable-floating-custom-no-root-split",string.Join(",",f.Trace)=="find:null");
        parent=Window();var root=new RootDock{Window=parent};f=new WindowFactory{Root=root,PrepareBase=true};options=new DockWindowOptions{OwnerMode=DockWindowOwnerMode.DockableWindow};
        check("nullable-floating-prepare-no-result",f.BaseOptions(null!,options)is null&&string.Join(",",f.Trace)=="prepare:null,find:null,plain:null");check("nullable-floating-prepare-shared-parent-options",ReferenceEquals(options.ParentWindow,parent)&&ReferenceEquals(f.LastOptions,options)&&f.LastWindow is null);
        var window=Window();f=new WindowFactory{Result=window,AbortPrepare=true};options=new DockWindowOptions();check("nullable-floating-prepare-aborts-before-create",PrepareFailure(()=>f.BaseOptions(null!,options))&&string.Join(",",f.Trace)=="prepare:null"&&!window.IsModal);
        f.Trace.Clear();f.AbortPrepare=false;f.MutateModal=true;var result=f.BaseOptions(null!,options);check("nullable-floating-prepare-create-apply-order",string.Join(",",f.Trace)=="prepare:null,plain:null,apply");check("nullable-floating-applied-original-handles",ReferenceEquals(result,window)&&window.IsModal&&ReferenceEquals(f.LastOptions,options)&&ReferenceEquals(f.LastWindow,result));
        f.Trace.Clear();f.MutateModal=false;check("nullable-floating-null-options-still-apply-hook",ReferenceEquals(f.BaseOptions(null!,null),result)&&window.IsModal&&f.LastOptions is null&&string.Join(",",f.Trace)=="prepare:null,plain:null,apply");
        parent=Window();window=Window();f=new WindowFactory{Root=new RootDock{Window=parent},PrepareBase=true,Result=window};options=new DockWindowOptions{OwnerMode=DockWindowOwnerMode.DockableWindow,IsModal=true,ShowInTaskbar=false};result=f.BaseOptions(null!,options);
        check("nullable-floating-parent-capture-create-apply-order",string.Join(",",f.Trace)=="prepare:null,find:null,plain:null,apply");check("nullable-floating-parent-result-identity",ReferenceEquals(result,window)&&ReferenceEquals(options.ParentWindow,parent)&&ReferenceEquals(window.ParentWindow,parent));check("nullable-floating-parent-presentation-options",window.OwnerMode==DockWindowOwnerMode.DockableWindow&&window.IsModal&&window.ShowInTaskbar==false);
        f=new WindowFactory{Root=new RootDock()};options=new DockWindowOptions();var activeFactory=f;f.DockableUndocked+=(_,e)=>{check("nullable-floating-split-null-undocked-payload",e.Dockable is null&&e.Operation==DockOperation.Window);activeFactory.Trace.Add("undocked:null");};f.BaseSplit(null!,null!,1,2,300,400,options);
        check("nullable-floating-split-null-lifecycle-order",string.Join(",",f.Trace)=="find:null,prepare:null,remove:null,undocked:null,create-options:null");check("nullable-floating-split-null-original-options-collapse",f.Collapse.Count==1&&f.Collapse[0]&&ReferenceEquals(f.LastOptions,options));
        f=new WindowFactory();var owner=new ToolDock{Id="owner"};owner.SetPointerScreenPosition(double.NaN,200);owner.SetVisibleBounds(30,40,640,480);var item=new Tool{Id="item",Owner=owner};item.SetVisibleBounds(11,12,8,80);options=new DockWindowOptions();f.BaseFloat(item,options);
        check("nullable-floating-ordinary-coordinate-fallback",f.Coordinates==(11d,200d,640d,80d));check("nullable-floating-ordinary-overload-identities",ReferenceEquals(f.LastItem,item)&&ReferenceEquals(f.LastDock,owner)&&ReferenceEquals(f.LastOptions,options));check("nullable-floating-ordinary-overload-order",string.Join(",",f.Trace)=="find:item,prepare:item,split:owner:item");
    }
    private sealed class WindowFactory:Factory
    {
        internal readonly List<string> Trace=new();internal IRootDock? Root;internal IDockWindow? Result,LastWindow;internal bool PrepareBase,AbortPrepare,MutateModal;internal DockWindowOptions? LastOptions;internal IDockable? LastItem;internal IDock? LastDock;internal (double,double,double,double) Coordinates;internal readonly List<bool> Collapse=new();
        public override IRootDock? FindRoot(IDockable item,Func<IRootDock,bool>? predicate=null){Trace.Add("find:"+Name(item));return Root;}
        public override IDockWindow? CreateWindowFrom(IDockable item){Trace.Add("plain:"+Name(item));LastItem=item;return Result;}
        public override IDockWindow? CreateWindowFrom(IDockable item,DockWindowOptions? options){Trace.Add("create-options:"+Name(item));LastItem=item;LastOptions=options;return Result;}
        public override void FloatDockable(IDockable item,DockWindowOptions? options){Trace.Add("float:"+Name(item));LastItem=item;LastOptions=options;}
        public override void FloatAllDockables(IDockable item,DockWindowOptions? options){Trace.Add("all:"+Name(item));LastItem=item;LastOptions=options;}
        public override void SplitToWindow(IDock dock,IDockable item,double x,double y,double width,double height,DockWindowOptions? options){Trace.Add("split:"+Name(dock)+":"+Name(item));LastDock=dock;LastItem=item;LastOptions=options;Coordinates=(x,y,width,height);}
        public override void RemoveDockable(IDockable item,bool collapse){Trace.Add("remove:"+Name(item));Collapse.Add(collapse);}
        protected override void PrepareWindowOptionsForDockable(IDockable item,DockWindowOptions? options){Trace.Add("prepare:"+Name(item));LastOptions=options;if(AbortPrepare)throw new InvalidOperationException("prepare aborted");if(MutateModal)options!.IsModal=true;if(PrepareBase)base.PrepareWindowOptionsForDockable(item,options);}
        protected override void ApplyWindowOptions(IDockWindow window,DockWindowOptions? options){Trace.Add("apply");LastOptions=options;LastWindow=window;base.ApplyWindowOptions(window,options);}
        internal IDockWindow? BasePlain(IDockable item)=>base.CreateWindowFrom(item);
        internal IDockWindow? BaseOptions(IDockable item,DockWindowOptions? options)=>base.CreateWindowFrom(item,options);
        internal void BaseSplit(IDock dock,IDockable item,double x,double y,double width,double height,DockWindowOptions? options)=>base.SplitToWindow(dock,item,x,y,width,height,options);
        internal void BaseFloat(IDockable item,DockWindowOptions? options)=>base.FloatDockable(item,options);
    }
}
