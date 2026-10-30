using System;
using System.Collections.Generic;
using System.ComponentModel;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;
namespace DockFactoryMarkupReference;
internal static class NullableWindowOptionsReference
{
    private static bool ArgumentFailure(Action action){try{action();return false;}catch(ArgumentNullException e){return e.ParamName=="window";}}
    internal static void Run(Action<string,bool> check)
    {
        var options=new DockWindowOptions();var parent=new Factory().CreateDockWindow();options.ParentWindow=parent;options.IsModal=true;
        check("nullable-window-options-direct-null-argument",ArgumentFailure(()=>options.ApplyTo(null!)));
        var f=new OptionsFactory();f.BaseApply(null,null);check("nullable-window-options-null-options-null-window-guard",f.Calls==0);
        check("nullable-window-options-base-null-window-argument",ArgumentFailure(()=>f.BaseApply(null,options)));
        check("nullable-window-options-failure-preserves-options",options.IsModal&&ReferenceEquals(options.ParentWindow,parent));
        var window=new Factory().CreateDockWindow();window.OwnerMode=DockWindowOwnerMode.ParentWindow;window.ParentWindow=parent;window.IsModal=true;window.ShowInTaskbar=false;int changes=0;((INotifyPropertyChanged)window).PropertyChanged+=(_,_)=>changes++;
        f.BaseApply(window,null);check("nullable-window-options-no-options-preserve-window",changes==0&&window.OwnerMode==DockWindowOwnerMode.ParentWindow&&ReferenceEquals(window.ParentWindow,parent)&&window.IsModal&&window.ShowInTaskbar==false);
        f.Apply(null,options);check("nullable-window-options-custom-hook-null",f.Calls==1&&f.Window is null&&ReferenceEquals(f.Options,options));f.Delegate=true;
        check("nullable-window-options-hook-base-argument-failure",ArgumentFailure(()=>f.Apply(null,options))&&f.Calls==2&&ReferenceEquals(f.Options,options));
        window=new Factory().CreateDockWindow();f.Apply(window,options);check("nullable-window-options-hook-original-window-options",f.Calls==3&&ReferenceEquals(f.Window,window)&&ReferenceEquals(f.Options,options)&&window.IsModal);
        for(int mode=0;mode<2;mode++)
        {
            window=new Factory().CreateDockWindow();var first=new Factory().CreateDockWindow();var second=new Factory().CreateDockWindow();options=new DockWindowOptions{OwnerMode=DockWindowOwnerMode.DockableWindow,ParentWindow=first,IsModal=true,ShowInTaskbar=false};window.ShowInTaskbar=true;
            var log=new List<string>();var live=options;bool remove=mode==1;
            ((INotifyPropertyChanged)window).PropertyChanged+=(_,e)=>{log.Add(e.PropertyName!);switch(e.PropertyName){case "OwnerMode":live.OwnerMode=DockWindowOwnerMode.ParentWindow;live.ParentWindow=second;live.IsModal=false;break;case "ParentWindow":live.IsModal=true;break;case "IsModal":live.ShowInTaskbar=remove?null:false;break;}};
            f.BaseApply(window,options);
            check("nullable-window-options-live-setter-order-"+mode,string.Join(",",log)=="OwnerMode,ParentWindow,IsModal"+(remove?"":",ShowInTaskbar"));
            check("nullable-window-options-live-later-values-"+mode,window.OwnerMode==DockWindowOwnerMode.DockableWindow&&options.OwnerMode==DockWindowOwnerMode.ParentWindow&&window.IsModal&&window.ShowInTaskbar==remove);
            check("nullable-window-options-live-parent-identity-"+mode,ReferenceEquals(window.ParentWindow,second)&&ReferenceEquals(options.ParentWindow,window.ParentWindow));
        }
    }
    private sealed class OptionsFactory:Factory
    {
        internal bool Delegate;internal int Calls;internal IDockWindow? Window;internal DockWindowOptions? Options;
        protected override void ApplyWindowOptions(IDockWindow window,DockWindowOptions? options){Calls++;Window=window;Options=options;if(Delegate)base.ApplyWindowOptions(window,options);}
        internal void BaseApply(IDockWindow? window,DockWindowOptions? options)=>base.ApplyWindowOptions(window!,options);
        internal void Apply(IDockWindow? window,DockWindowOptions? options)=>ApplyWindowOptions(window!,options);
    }
}
