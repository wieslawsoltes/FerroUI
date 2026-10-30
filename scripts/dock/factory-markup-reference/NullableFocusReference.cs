using System;
using System.Collections.Generic;
using Dock.Model.Core;
using Dock.Model.Controls;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Core;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class NullableFocusReference
{
    private static bool NullFailure(Action action){try{action();return false;}catch(NullReferenceException){return true;}}
    private static string Name(IDockable? item)=>item?.Id??"null";
    internal static void Run(Action<string,bool> check)
    {
        var ordinary=new Factory();check("nullable-focus-default-null-focus-lookup-failure",NullFailure(()=>ordinary.SetFocusedDockable(null!,null)));check("nullable-focus-default-null-window-lookup-failure",NullFailure(()=>ordinary.ActivateWindow(null!)));
        int events=0;ordinary.ActiveDockableChanged+=(_,e)=>{check("nullable-focus-null-active-event-payload",e.Dockable is null);events++;};check("nullable-focus-null-owner-final-selection-failure",NullFailure(()=>ordinary.InitActiveDockable(null,null!))&&events==1);
        ordinary=new Factory();var doc=new Document{Owner=new ToolDock()};check("nullable-focus-default-init-effects-before-lookup-failure",NullFailure(()=>ordinary.InitActiveDockable(doc,null!))&&doc.Owner is null&&ReferenceEquals(doc.Factory,ordinary));
        var f=new FocusFactory();f.BaseFocus(null!,null);f.ActivateWindow(null!);check("nullable-focus-lookup-null-custom-no-root",string.Join(",",f.Trace)=="find:null:focus,find:null:all");f.InitActiveDockable(null,new ToolDock());check("nullable-focus-null-item-valid-owner-no-focus",f.Trace[^1]=="active:null");
        for(int mode=0;mode<2;mode++)
        {
            f=new FocusFactory();var active=f;bool suppress=mode==1;var item=new SelectedItem(f.Trace,()=>active.AllowFocus=!suppress){Id="item"};
            check("nullable-focus-init-final-null-owner-"+mode,NullFailure(()=>active.InitActiveDockable(item,null!)));
            check("nullable-focus-init-selected-gate-order-"+mode,string.Join(",",f.Trace)=="active:item,find:item:focus,find:item:focus,init:item:null,find:item:focus,find:item:focus,selected,gate:item:null"+(suppress?"":",focus:null:item"));
            check("nullable-focus-init-original-item-owner-"+mode,item.Owner is null&&ReferenceEquals(item.Factory,f)&&f.LastDock is null&&(suppress?f.LastItem is null:ReferenceEquals(f.LastItem,item)));
        }
        f=new FocusFactory();var abort=new SelectedItem(f.Trace,()=>throw new InvalidOperationException("selected aborted")){Id="item"};bool aborted=false;try{f.InitActiveDockable(abort,null!);}catch(InvalidOperationException e)when(e.Message=="selected aborted"){aborted=true;}
        check("nullable-focus-selected-failure-aborts-before-policy",aborted&&string.Join(",",f.Trace)=="active:item,find:item:focus,find:item:focus,init:item:null,find:item:focus,find:item:focus,selected"&&ReferenceEquals(abort.Factory,f));
        for(int mode=0;mode<2;mode++)
        {
            f=new FocusFactory();var root=new RootDock();var window=new Factory().CreateDockWindow();var next=new Factory().CreateDockWindow();next.Id="next";root.Window=window;f.Root=root;bool replace=mode==1;int calls=0;
            window.Host=new CallbackHost(()=>{calls++;root.Window=replace?next:null;});IDockWindow? received=window;int raised=0;f.WindowActivated+=(_,e)=>{received=e.Window;raised++;};f.ActivateWindow(null!);
            check("nullable-focus-activation-live-window-order-"+mode,string.Join(",",f.Trace)=="find:null:all,window:"+(replace?"next":"null")&&calls==1);
            check("nullable-focus-activation-live-event-"+mode,raised==1&&ReferenceEquals(received,replace?next:null)&&window.Host is not null);
        }
        f=new FocusFactory{Root=new RootDock()};doc=new Document{Id="doc"};check("nullable-focus-root-override-final-null-owner-failure",NullFailure(()=>f.BaseFocus(null!,doc)));check("nullable-focus-root-override-commits-focus",ReferenceEquals(f.Root.FocusedDockable,doc)&&f.Trace[0]=="find:null:focus");
        f=new FocusFactory();check("nullable-focus-default-policy-null-inputs",f.BaseGate(null!,null!));
    }
    private sealed class FocusFactory:Factory
    {
        internal readonly List<string> Trace=new();internal IRootDock? Root;internal bool AllowFocus=true;internal IDock? LastDock;internal IDockable? LastItem;
        public override IRootDock? FindRoot(IDockable item,Func<IRootDock,bool>? predicate=null){Trace.Add("find:"+Name(item)+":"+(predicate is null?"all":"focus"));return Root;}
        public override void OnActiveDockableChanged(IDockable? item){Trace.Add("active:"+Name(item));base.OnActiveDockableChanged(item);}
        public override void InitDockable(IDockable item,IDockable? owner){Trace.Add("init:"+Name(item)+":"+Name(owner));base.InitDockable(item,owner);}
        public override void SetFocusedDockable(IDock dock,IDockable? item){Trace.Add("focus:"+Name(dock)+":"+Name(item));LastDock=dock;LastItem=item;}
        public override void OnWindowActivated(IDockWindow? window){Trace.Add("window:"+(window?.Id??"null"));base.OnWindowActivated(window);}
        protected override bool ShouldFocusActiveDockable(IDockable item,IDock owner){Trace.Add("gate:"+Name(item)+":"+Name(owner));return AllowFocus;}
        internal void BaseFocus(IDock dock,IDockable? item)=>base.SetFocusedDockable(dock,item);
        internal bool BaseGate(IDockable item,IDock owner)=>base.ShouldFocusActiveDockable(item,owner);
    }
    private sealed class SelectedItem:DockableBase
    {
        private readonly List<string> trace;private readonly Action action;
        internal SelectedItem(List<string> trace,Action action){this.trace=trace;this.action=action;}
        public override void OnSelected(){trace.Add("selected");action();}
    }
    private sealed class CallbackHost:IHostWindow
    {
        private readonly Action active;internal CallbackHost(Action active){this.active=active;}
        public IHostWindowState? HostWindowState=>null;public bool IsTracked{get;set;}public IDockWindow? Window{get;set;}
        public void SetActive()=>active();public void Present(bool dialog){}public void Exit(){}public void SetPosition(double x,double y){}public void GetPosition(out double x,out double y){x=y=0;}public void SetSize(double width,double height){}public void GetSize(out double width,out double height){width=height=0;}public void SetWindowState(DockWindowState state){}public DockWindowState GetWindowState()=>DockWindowState.Normal;public void SetTitle(string? title){}public void SetLayout(IDock layout){}
    }
}
