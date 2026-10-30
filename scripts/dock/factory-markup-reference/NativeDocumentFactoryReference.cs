using System;
using Avalonia.Controls;
using Avalonia.Threading;
using Dock.Model.Controls;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class NativeDocumentFactoryReference
{
    internal static void Run(Action<string,bool> check,Func<string,object> load)
    {
        var producer=(LocatorFactory)load("NativeDocumentFactoryProducer");
        var dock=(DocumentDock)load("ReactiveDocumentFactoryRoot");
        var panel=(StackPanel)load("CoreDocumentFactoryBindings");
        check("document-factory-markup-deferred",producer.DockableCalls==0&&producer.DocumentFactory is not null);
        var callback=producer.DocumentFactory!; IDocumentDockFactory contract=dock;
        contract.DocumentFactory=callback;
        check("document-factory-source-identity",ReferenceEquals(dock.DocumentFactory,callback));
        check("document-factory-assignment-deferred",producer.DockableCalls==0);
        panel.DataContext=contract;Dispatcher.UIThread.RunJobs();
        check("document-factory-bound-identity",ReferenceEquals(((ContentControl)panel.Children[0]).Content,callback));
        ((Button)panel.Children[1]).Command!.Execute(null);
        check("document-factory-bound-invoke",producer.DockableCalls==1);
        var f=new Factory();dock.Factory=f;dock.VisibleDockables=f.CreateList<IDockable>();
        dock.CreateDocument!.Execute(null);
        check("document-factory-engine-invoke",producer.DockableCalls==2);
        check("document-factory-engine-visible",ReferenceEquals(dock.VisibleDockables[0],producer.DocumentValue));
        check("document-factory-engine-active",ReferenceEquals(dock.ActiveDockable,producer.DocumentValue));
        check("document-factory-engine-owner",ReferenceEquals(producer.DocumentValue.Owner,dock));
        panel.DataContext=null;contract.DocumentFactory=null;
        check("document-factory-null",dock.DocumentFactory is null);producer.DocumentFactory=null;
        Func<IDockable?> locator=()=>producer.DocumentValue;
        contract.DocumentFactory=locator!;
        check("document-factory-clr-cross-nullability-identity",ReferenceEquals(contract.DocumentFactory,locator));
        check("document-factory-clr-cross-nullability-invoke",ReferenceEquals(contract.DocumentFactory!(),producer.DocumentValue));
        contract.DocumentFactory=null;
        var assignment=(StackPanel)load("CoreFactoryAssignmentBindings");
        var target=new DocumentDock(); IDocumentDockFactory targetContract=target;
        assignment.DataContext=targetContract;Dispatcher.UIThread.RunJobs();
        int calls=0;Func<IDockable?> original=()=>{calls++;return producer.DocumentValue;};
        var content=(ContentControl)assignment.Children[0];content.Content=original;Dispatcher.UIThread.RunJobs();
        check("delegate-two-way-deferred",calls==0);
        check("delegate-two-way-source-identity",ReferenceEquals(target.DocumentFactory,original));
        var factory=new Factory();factory.DockableLocator=new System.Collections.Generic.Dictionary<string,Func<IDockable?>>();
        factory.DockableLocator["callback"]=target.DocumentFactory!;
        check("delegate-locator-roundtrip-identity",ReferenceEquals(factory.DockableLocator["callback"],original));
        target.Factory=factory;target.VisibleDockables=factory.CreateList<IDockable>();target.CreateDocument!.Execute(null);
        check("delegate-engine-once",calls==1);
        check("delegate-engine-result",ReferenceEquals(target.VisibleDockables[0],producer.DocumentValue));
        assignment.DataContext=null;assignment.DataContext=targetContract;Dispatcher.UIThread.RunJobs();
        check("delegate-source-rebind-identity",ReferenceEquals(content.Content,original));
        Func<IDockable> required=()=>producer.DocumentValue;target.DocumentFactory=required;factory.DockableLocator["required"]=target.DocumentFactory;target.DocumentFactory=factory.DockableLocator["required"]!;
        check("delegate-required-roundtrip",ReferenceEquals(target.DocumentFactory,required));
        Func<IDockable?> empty=()=>null;target.DocumentFactory=empty!;
        check("delegate-null-native-result",target.DocumentFactory!() is null);
        check("delegate-null-native-identity",ReferenceEquals(target.DocumentFactory,empty));
        assignment.DataContext=null;content.Content=null;target.DocumentFactory=null;factory.DockableLocator.Clear();
        int emptyCalls=0;var detached=new DocumentDock{DocumentFactory=()=>{emptyCalls++;return null!;}};
        detached.CreateDocument!.Execute(null);
        check("delegate-null-engine-without-factory-noop",emptyCalls==1&&detached.VisibleDockables is null);
        var overriding=new NullDocumentOverride{DocumentFactory=()=>null!};
        overriding.CreateDocument!.Execute(null);
        check("delegate-null-engine-virtual-dispatch",overriding.NullAdds==1);
        var commands=(StackPanel)load("CoreNullDocumentBindings");int commandCalls=0;
        var plain=new DocumentDock{DocumentFactory=()=>{commandCalls++;return null!;}};
        commands.DataContext=plain;Dispatcher.UIThread.RunJobs();
        for(int index=0;index<3;index++)((Button)commands.Children[index]).Command!.Execute(null);
        check("nullable-command-no-factory-once",commandCalls==1);
        check("nullable-command-no-factory-graph",plain.VisibleDockables is null);
        var derived=new NullDocumentOverride{DocumentFactory=()=>null!};commands.DataContext=derived;Dispatcher.UIThread.RunJobs();
        for(int index=0;index<3;index++)((Button)commands.Children[index]).Command!.Execute(null);
        check("nullable-command-document-virtual",derived.NullAdds==2);
        check("nullable-command-tool-virtual",derived.NullTools==1);commands.DataContext=null;
        var plainTool=new ToolDock();plainTool.AddTool(null!);
        check("nullable-tool-no-factory-graph",plainTool.VisibleDockables is null);


    }
}

internal sealed class NullDocumentOverride:DocumentDock
{
    internal int NullAdds {get;private set;}
    internal int NullTools {get;private set;}
    public override void AddTool(IDockable tool){if(tool is null){NullTools++;return;}base.AddTool(tool);}
    public override void AddDocument(IDockable document)
    {
        if(document is null){NullAdds++;return;}
        base.AddDocument(document);
    }
}
