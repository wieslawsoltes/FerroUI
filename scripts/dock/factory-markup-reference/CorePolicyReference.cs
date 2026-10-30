using System;
using Avalonia.Controls;
using Avalonia.Threading;
using Dock.Model;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;

namespace DockFactoryMarkupReference;

internal static class CorePolicyReference
{
    internal static void Run(Action<string, bool> check, Func<string, object> load)
    {
        var policy = (DockCapabilityPolicy)load("CoreCapabilityPolicyRoot");
        check("core-policy-loaded", policy.CanClose == false && policy.CanFloat == true && policy.CanPin is null);
        var overrides = (DockCapabilityOverrides)load("CoreCapabilityOverridesRoot");
        check("core-overrides-loaded", overrides.CanPin == false && overrides.CanDrop == true && overrides.HasAnyOverride);
        var factory = new Factory(); var root = factory.CreateRootDock(); var dock = factory.CreateDocumentDock(); var item = factory.CreateDocument();
        factory.AddDockable(root, dock); factory.AddDockable(dock, item); factory.InitLayout(root);
        root.RootDockCapabilityPolicy = policy;
        var denied = DockCapabilityResolver.Evaluate(item, DockCapability.Close);
        check("core-policy-engine-root", !denied.EffectiveValue && denied.EffectiveSource == DockCapabilityValueSource.RootPolicy);
        item.DockCapabilityOverrides = overrides; overrides.CanClose = true;
        var allowed = DockCapabilityResolver.Evaluate(item, DockCapability.Close);
        check("core-policy-engine-override", allowed.EffectiveValue && allowed.EffectiveSource == DockCapabilityValueSource.DockableOverride);

        var setters = new (string Name, DockCapability Capability, Action<IDockCapabilityPolicy, bool?> Set, Func<IDockCapabilityPolicy, bool?> Get)[] {
            ("Close",DockCapability.Close,(p,v)=>p.CanClose=v,p=>p.CanClose),
            ("Pin",DockCapability.Pin,(p,v)=>p.CanPin=v,p=>p.CanPin),
            ("Float",DockCapability.Float,(p,v)=>p.CanFloat=v,p=>p.CanFloat),
            ("Drag",DockCapability.Drag,(p,v)=>p.CanDrag=v,p=>p.CanDrag),
            ("Drop",DockCapability.Drop,(p,v)=>p.CanDrop=v,p=>p.CanDrop),
            ("DockAsDocument",DockCapability.DockAsDocument,(p,v)=>p.CanDockAsDocument=v,p=>p.CanDockAsDocument)
        };
        int kind = 0;
        foreach (DockCapabilityPolicy current in new DockCapabilityPolicy[] {new(),new DockCapabilityOverrides()}) {
            foreach (var property in setters) {
                int index = 0;
                foreach (bool? value in new bool?[] {null,false,true,null}) {
                    property.Set(current,value);
                    check($"core-policy-values-{kind}-{property.Name}-{index++}",property.Get(current)==value&&current.Get(property.Capability)==value&&(current is not DockCapabilityOverrides o||o.HasAnyOverride==value.HasValue));
                }
            }
            kind++;
        }
        var probe = new PolicyProbe {CanClose=false};
        check("core-policy-virtual-get",probe.Get(DockCapability.Close)==true&&probe.Calls==1&&probe.CanClose==false);
        root.RootDockCapabilityPolicy = probe; item.DockCapabilityOverrides = null;
        check("core-policy-resolver-virtual-get",DockCapabilityResolver.Evaluate(item,DockCapability.Close).EffectiveValue&&probe.Calls==2);

        var options = (DockWindowOptions)load("CoreWindowOptionsRoot");
        check("core-options-loaded",options.OwnerMode==DockWindowOwnerMode.ParentWindow&&options.IsModal&&options.ShowInTaskbar==false&&options.ParentWindow?.Title=="Parent title");
        var optionsView = (StackPanel)load("CoreWindowOptionsBindings"); optionsView.DataContext=options; Dispatcher.UIThread.RunJobs();
        check("core-options-parent-binding",((TextBlock)optionsView.Children[0]).Text=="Parent title");
        options.ParentWindow!.Title="Changed parent";Dispatcher.UIThread.RunJobs();check("core-options-parent-live-binding",((TextBlock)optionsView.Children[0]).Text=="Changed parent");
        ((CheckBox)optionsView.Children[1]).IsChecked=false;((CheckBox)optionsView.Children[2]).IsChecked=null;Dispatcher.UIThread.RunJobs();
        check("core-options-two-way",!options.IsModal&&options.ShowInTaskbar is null);
        var window=factory.CreateWindowFrom(item,options)!;
        check("core-options-factory-window",window.OwnerMode==DockWindowOwnerMode.ParentWindow&&!window.IsModal&&ReferenceEquals(window.ParentWindow,options.ParentWindow));
        int setting=0;
        foreach(bool? value in new bool?[]{null,false,true,null}) {
            options.ShowInTaskbar=value;window.ShowInTaskbar=true;options.ApplyTo(window);
            check($"core-options-apply-{setting++}",window.ShowInTaskbar==(value??true)&&ReferenceEquals(window.ParentWindow,options.ParentWindow));
        }
        bool nullRejected=false;try{options.ApplyTo(null!);}catch(ArgumentNullException error){nullRejected=error.ParamName=="window";}check("core-options-null-window",nullRejected);

        var evaluation=(DockCapabilityEvaluation)load("CoreCapabilityEvaluationRoot");
        check("core-evaluation-loaded",evaluation.Capability==DockCapability.Pin&&!evaluation.BaseValue&&evaluation.RootPolicyValue is null&&evaluation.DockPolicyValue==true&&evaluation.DockableOverrideValue==false&&evaluation.EffectiveValue&&evaluation.EffectiveSource==DockCapabilityValueSource.DockPolicy&&evaluation.DiagnosticMessage=="Supplied diagnostics");
        var evaluationView=(StackPanel)load("CoreCapabilityEvaluationBindings");evaluationView.DataContext=evaluation;Dispatcher.UIThread.RunJobs();
        check("core-evaluation-bindings",((TextBlock)evaluationView.Children[0]).Text=="Supplied diagnostics"&&((CheckBox)evaluationView.Children[1]).IsChecked==true&&((CheckBox)evaluationView.Children[2]).IsChecked==true);
        int tuple=0;
        foreach(bool? rootValue in new bool?[]{null,false,true}) foreach(bool? dockValue in new bool?[]{null,false,true}) foreach(bool? overValue in new bool?[]{null,false,true}) {
            var value=new DockCapabilityEvaluation(DockCapability.Pin,false,rootValue,dockValue,overValue,true,DockCapabilityValueSource.DockPolicy,"unchanged diagnostics");
            check($"core-evaluation-constructor-{tuple++}",value.Capability==DockCapability.Pin&&!value.BaseValue&&value.RootPolicyValue==rootValue&&value.DockPolicyValue==dockValue&&value.DockableOverrideValue==overValue&&value.EffectiveValue&&value.EffectiveSource==DockCapabilityValueSource.DockPolicy&&value.DiagnosticMessage=="unchanged diagnostics");
        }
        var rootPolicy=new DockCapabilityPolicy();var dockPolicy=new DockCapabilityPolicy();var local=new DockCapabilityOverrides();root.RootDockCapabilityPolicy=rootPolicy;dock.DockCapabilityPolicy=dockPolicy;item.DockCapabilityOverrides=local;
        tuple=0;
        foreach(bool? rootValue in new bool?[]{null,false,true}) foreach(bool? dockValue in new bool?[]{null,false,true}) foreach(bool? overValue in new bool?[]{null,false,true}) {
            rootPolicy.CanClose=rootValue;dockPolicy.CanClose=dockValue;local.CanClose=overValue;
            var implicitContext=DockCapabilityResolver.Evaluate(item,DockCapability.Close);var explicitNull=DockCapabilityResolver.Evaluate(item,DockCapability.Close,null);
            bool implicitValue=overValue??dockValue??rootValue??item.CanClose;bool explicitValue=overValue??rootValue??item.CanClose;
            check($"core-resolver-contexts-{tuple}",implicitContext.EffectiveValue==implicitValue&&explicitNull.EffectiveValue==explicitValue&&implicitContext.RootPolicyValue==rootValue&&implicitContext.DockPolicyValue==dockValue&&explicitNull.DockPolicyValue is null&&implicitContext.DockableOverrideValue==overValue);
            check($"core-resolver-enabled-{tuple++}",DockCapabilityResolver.IsEnabled(item,DockCapability.Close)==explicitValue&&DockCapabilityResolver.IsEnabled(item,DockCapability.Close,dock)==implicitValue);
        }
        check("core-resolver-operation-dock",ReferenceEquals(DockCapabilityResolver.ResolveOperationDock(item),dock));
        check("core-resolver-target-dock",ReferenceEquals(DockCapabilityResolver.ResolveDropTargetDock(item),dock));
    }
    private sealed class PolicyProbe:DockCapabilityPolicy {
        public int Calls {get;private set;}
        public override bool? Get(DockCapability capability){Calls++;return true;}
    }
}
