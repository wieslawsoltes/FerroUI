// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using System.IO;
using Avalonia;
using Avalonia.Controls;
using Avalonia.Headless;
using Avalonia.Markup.Xaml;
using Avalonia.MarkupExtension;
using Dock.Avalonia.Controls;
using Dock.Model.Controls;
using Dock.Model.Core;
using Dock.Model.ReactiveUI.Controls;
using Dock.Model.ReactiveUI.Core;
if(args.Length!=1)throw new ArgumentException("Pass output corpus path");
AppBuilder.Configure<Application>().UseHeadless(new AvaloniaHeadlessPlatformOptions()).SetupWithoutStarting();
using StreamWriter output=new(args[0]);
output.WriteLine("# Unchanged original Dock markup/factory assertions; .NET "+Environment.Version);
void Check(string name,bool result){if(!result)throw new InvalidOperationException(name);output.WriteLine(name+"\t1");}
var absolute=new Uri("avares://DockFactoryMarkupReference/LoadedView.axaml");
var context=new Context{BaseUri=new Uri("avares://DockFactoryMarkupReference/")};
var provider=new Provider(context,null);var load=new LoadExtension(absolute);
Check("load-constructor",load.Source==absolute);
Check("load-no-context",load.ProvideValue(new Provider(null,null)) is null);
Check("load-no-source",new LoadExtension().ProvideValue(provider) is null&&context.Reads==0);
var first=load.ProvideValue(provider);var second=load.ProvideValue(provider);
Check("load-object",first is TextBlock text&&text.Text=="Hello");
Check("load-fresh",!ReferenceEquals(first,second));
load.Source=new Uri("../LoadedView.axaml",UriKind.Relative);context.BaseUri=new Uri("avares://DockFactoryMarkupReference/Views/Host.axaml");
Check("load-relative",load.ProvideValue(provider) is TextBlock relative&&relative.Text=="Hello");
load.Source=absolute;context.BaseUri=null!;Check("load-null-base",load.ProvideValue(provider) is TextBlock);
load.Source=new Uri("Missing.axaml",UriKind.Relative);context.BaseUri=new Uri("avares://DockFactoryMarkupReference/");
context.OnRead=()=>load.Source=absolute;
Check("load-source-after-base",load.ProvideValue(provider) is TextBlock);
context.OnRead=()=>load.Source=null;
bool nullFailure=false;try{load.ProvideValue(provider);}catch(ArgumentNullException){nullFailure=true;}
Check("load-null-after-base-error",nullFailure);context.OnRead=null;
load.Source=new Uri("avares://DockFactoryMarkupReference/Missing.axaml");
bool missing=false;try{load.ProvideValue(provider);}catch{missing=true;}Check("load-missing-error",missing);
var button=new Button();var otherButton=new Button();var scope=new NameScope();var otherScope=new NameScope();
scope.Register("btn",button);otherScope.Register("btn",otherButton);var reference=new ReferenceExtension("btn");
Check("reference-constructor",reference.Name=="btn");
Check("reference-identity",ReferenceEquals(reference.ProvideValue(new Provider(null,scope)),button));
Check("reference-provider",ReferenceEquals(reference.ProvideValue(new Provider(null,otherScope)),otherButton));
Check("reference-no-scope",reference.ProvideValue(new Provider(null,null)) is null);
reference.Name=null;Check("reference-no-name",reference.ProvideValue(new Provider(null,scope)) is null);
reference.Name="later";Check("reference-missing",reference.ProvideValue(new Provider(null,scope)) is null);
scope.Register("later",button);Check("reference-immediate-next-call",ReferenceEquals(reference.ProvideValue(new Provider(null,scope)),button));
var namedModel=new Document();scope.Register("model",namedModel);reference.Name="model";
Check("reference-plain-model",ReferenceEquals(reference.ProvideValue(new Provider(null,scope)),namedModel));
var objectScope=new NameScope();var objectModel=new Document();var differentModel=new Document();
var pendingObject=objectScope.FindAsync("object");bool callbackRan=false;
pendingObject.OnCompleted(()=>{callbackRan=true;Check("scope-reentrant-find",ReferenceEquals(objectScope.Find("object"),objectModel));objectScope.Register("reentrant",differentModel);});
Check("scope-pending",!pendingObject.IsCompleted);
objectScope.Register("object",objectModel);
Check("scope-synchronous",callbackRan&&pendingObject.IsCompleted&&ReferenceEquals(pendingObject.GetResult(),objectModel));
objectScope.Register("object",objectModel);Check("scope-same-reference",ReferenceEquals(objectScope.Find("object"),objectModel));
bool duplicate=false;try{objectScope.Register("object",differentModel);}catch(ArgumentException){duplicate=true;}Check("scope-duplicate-object",duplicate);
bool mixedDuplicate=false;try{objectScope.Register("object",new Button());}catch(ArgumentException){mixedDuplicate=true;}Check("scope-mixed-duplicate",mixedDuplicate);
var missingObject=objectScope.FindAsync("missing");objectScope.Complete();Check("scope-complete-missing",missingObject.IsCompleted&&missingObject.GetResult() is null);
bool completedScope=false;try{objectScope.Register("object",objectModel);}catch(InvalidOperationException){completedScope=true;}Check("scope-completed-registration",completedScope);
var plainLoad=new LoadExtension(new Uri("avares://DockFactoryMarkupReference/PlainDocument.axaml"));
var plainFirst=plainLoad.ProvideValue(provider);var plainSecond=plainLoad.ProvideValue(provider);
Check("load-plain-root",plainFirst is Document plainDocument&&plainDocument.Title=="Plain root");
Check("load-plain-fresh",plainSecond is Document&& !ReferenceEquals(plainFirst,plainSecond));
var identityScope=new NameScope();IDockable modelInterface=namedModel;IDocument documentInterface=namedModel;
identityScope.Register("same",modelInterface);identityScope.Register("same",documentInterface);
Check("scope-interface-identity",ReferenceEquals(identityScope.Find("same"),namedModel));
Check("reference-model-interface-identity",ReferenceEquals(new ReferenceExtension("same").ProvideValue(new Provider(null,identityScope)),namedModel));
object?[] modelViews={namedModel,modelInterface,documentInterface,(DockableBase)namedModel,(ReactiveBase)namedModel,(ReactiveUI.ReactiveObject)namedModel};
foreach(var view in modelViews)identityScope.Register("same",view!);
Check("scope-inherited-identity",ReferenceEquals(identityScope.Find("same"),namedModel));
var referenceConverter=Dock.Avalonia.Converters.ReferenceEqualsMultiConverter.Instance;
Check("converter-inherited-identity",Equals(referenceConverter.Convert(modelViews,typeof(bool),null,System.Globalization.CultureInfo.InvariantCulture),true));
Check("converter-different-model",Equals(referenceConverter.Convert(new object?[]{namedModel,new Document()},typeof(bool),null,System.Globalization.CultureInfo.InvariantCulture),false));
object firstBox=42;object secondBox=42;
Check("converter-distinct-boxes",Equals(referenceConverter.Convert(new object?[]{firstBox,secondBox},typeof(bool),null,System.Globalization.CultureInfo.InvariantCulture),false));
var policyLoad=new LoadExtension(new Uri("avares://DockFactoryMarkupReference/CapabilityPolicyRoot.axaml"));
var policyRoot=(DockCapabilityPolicy)policyLoad.ProvideValue(provider)!;
Check("load-policy-root",policyRoot.CanClose==false&&policyRoot.CanFloat==true&&policyRoot.CanPin is null);
Check("load-policy-fresh",!ReferenceEquals(policyRoot,policyLoad.ProvideValue(provider)));
var overridesLoad=new LoadExtension(new Uri("avares://DockFactoryMarkupReference/CapabilityOverridesRoot.axaml"));
var overridesRoot=(DockCapabilityOverrides)overridesLoad.ProvideValue(provider)!;
Check("load-overrides-root",overridesRoot.CanPin==false&&overridesRoot.CanDrop==true&&overridesRoot.HasAnyOverride);
var policyEngine=new Dock.Model.ReactiveUI.Factory();var policyLayout=policyEngine.CreateLayout()!;
var policyDock=new DocumentDock();var policyDocument=new Document();
policyEngine.AddDockable(policyLayout,policyDock);policyEngine.AddDockable(policyDock,policyDocument);policyEngine.InitLayout(policyLayout);
policyLayout.RootDockCapabilityPolicy=policyRoot;
Check("loaded-root-engine-policy",!Dock.Model.DockCapabilityResolver.IsEnabled(policyDocument,DockCapability.Close));
overridesRoot.CanClose=true;policyDocument.DockCapabilityOverrides=overridesRoot;
Check("loaded-root-engine-override",Dock.Model.DockCapabilityResolver.IsEnabled(policyDocument,DockCapability.Close));
policyDocument.DockCapabilityOverrides=null;policyRoot.CanClose=true;
Check("loaded-root-live-policy",Dock.Model.DockCapabilityResolver.IsEnabled(policyDocument,DockCapability.Close));
var sdkDocumentLoad=new LoadExtension(new Uri("avares://DockFactoryMarkupReference/ReactiveDocumentRoot.axaml"));
var sdkDocument=(Document)sdkDocumentLoad.ProvideValue(provider)!;
var sdkTool=(Tool)new LoadExtension(new Uri("avares://DockFactoryMarkupReference/ReactiveToolRoot.axaml")).ProvideValue(provider)!;
var sdkDock=(DocumentDock)new LoadExtension(new Uri("avares://DockFactoryMarkupReference/ReactiveDocumentDockRoot.axaml")).ProvideValue(provider)!;
var sdkRoot=(RootDock)new LoadExtension(new Uri("avares://DockFactoryMarkupReference/ReactiveRootDockRoot.axaml")).ProvideValue(provider)!;
Check("sdk-document-root",sdkDocument.Title=="Source document"&&sdkDocument.IsOpen&&sdkDocument.MdiZIndex==7);
Check("sdk-tool-root",sdkTool.Title=="Source tool"&&sdkTool.IsSelected);
Check("sdk-document-dock-root",sdkDock.Title=="Source documents"&&!sdkDock.CanCreateDocument);
Check("sdk-root-dock-root",sdkRoot.Title=="Source root"&&sdkRoot.IsFocusableRoot);
Check("sdk-fresh-model",!ReferenceEquals(sdkDocument,sdkDocumentLoad.ProvideValue(provider)));
Check("sdk-original-base-classes",sdkDocument is DockableBase&&sdkRoot is DockBase&&sdkDocument is ReactiveBase);
var emptySdkDocument=new Document();var emptySdkDock=new DocumentDock();var emptySdkRoot=new RootDock();
Check("sdk-source-defaults",emptySdkDocument.Title==string.Empty&&!emptySdkDocument.IsOpen&&!emptySdkDock.CanCreateDocument&&emptySdkDock.CreateDocument is not null&&emptySdkDock.CascadeDocuments is not null&&emptySdkRoot.IsFocusableRoot&&emptySdkRoot.PinnedDockDisplayMode==PinnedDockDisplayMode.Overlay);
var sdkEngine=new Dock.Model.ReactiveUI.Factory{HideDocumentsOnClose=false};
sdkEngine.AddDockable(sdkRoot,sdkDock);sdkEngine.AddDockable(sdkDock,sdkDocument);sdkEngine.InitLayout(sdkRoot);
Check("sdk-original-engine-owner",ReferenceEquals(sdkDocument.Owner,sdkDock)&&sdkDock.VisibleDockables!.Count==1);
var sdkBindings=(StackPanel)new LoadExtension(new Uri("avares://DockFactoryMarkupReference/ReactiveModelBindings.axaml")).ProvideValue(provider)!;
sdkBindings.DataContext=sdkDocument;Avalonia.Threading.Dispatcher.UIThread.RunJobs();
var sdkText=(TextBlock)sdkBindings.Children[0];var sdkEdit=(TextBox)sdkBindings.Children[1];var sdkOpen=(CheckBox)sdkBindings.Children[2];
Check("sdk-compiled-title-binding",sdkText.Text=="Source document");
sdkDocument.Title="Changed engine";Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("sdk-live-model-title",sdkText.Text=="Changed engine");
sdkEdit.Text="Edited";Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("sdk-two-way-title",sdkDocument.Title=="Edited");
sdkOpen.IsChecked=false;Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("sdk-two-way-open",!sdkDocument.IsOpen);
sdkDock.CanCloseLastDockable=true;sdkEngine.CloseDockable(sdkDocument);Check("sdk-original-engine-close",sdkDock.VisibleDockables!.Count==0);
T LoadModelAsset<T>(string name)=>(T)new LoadExtension(new Uri("avares://DockFactoryMarkupReference/"+name+".axaml")).ProvideValue(provider)!;
var layoutTools=LoadModelAsset<ToolDock>("ReactiveToolDockRoot");
Check("layout-tool-dock-root",layoutTools.Title=="Source ToolDock"&&layoutTools.Alignment==Alignment.Right&&layoutTools.IsExpanded&&!layoutTools.AutoHide&&layoutTools.GripMode==GripMode.AutoHide);
var layoutProportional=LoadModelAsset<ProportionalDock>("ReactiveProportionalDockRoot");
Check("layout-proportional-root",layoutProportional.Title=="Source ProportionalDock"&&layoutProportional.Orientation==Dock.Model.Core.Orientation.Vertical);
var layoutStack=LoadModelAsset<StackDock>("ReactiveStackDockRoot");Check("layout-stack-root",layoutStack.Title=="Source StackDock"&&layoutStack.Orientation==Dock.Model.Core.Orientation.Vertical&&layoutStack.Spacing==8.5);
var layoutGrid=LoadModelAsset<GridDock>("ReactiveGridDockRoot");Check("layout-grid-root",layoutGrid.Title=="Source GridDock"&&layoutGrid.ColumnDefinitions=="2*,Auto,*"&&layoutGrid.RowDefinitions=="*,3*");
var layoutDock=LoadModelAsset<DockDock>("ReactiveDockDockRoot");Check("layout-dock-root",layoutDock.Title=="Source DockDock"&&!layoutDock.LastChildFill);
var layoutWrap=LoadModelAsset<WrapDock>("ReactiveWrapDockRoot");Check("layout-wrap-root",layoutWrap.Title=="Source WrapDock"&&layoutWrap.Orientation==Dock.Model.Core.Orientation.Vertical);
var layoutUniform=LoadModelAsset<UniformGridDock>("ReactiveUniformGridDockRoot");Check("layout-uniform-root",layoutUniform.Title=="Source UniformGridDock"&&layoutUniform.Rows==2&&layoutUniform.Columns==3);
var layoutSplit=LoadModelAsset<SplitViewDock>("ReactiveSplitViewDockRoot");Check("layout-split-view-root",layoutSplit.Title=="Source SplitViewDock"&&layoutSplit.CompactPaneLength==56&&layoutSplit.OpenPaneLength==400&&layoutSplit.IsPaneOpen&&layoutSplit.UseLightDismissOverlayMode&&layoutSplit.DisplayMode==Dock.Model.Core.SplitViewDisplayMode.CompactOverlay&&layoutSplit.PanePlacement==Dock.Model.Core.SplitViewPanePlacement.Bottom&&layoutSplit.PaneDockable is Tool{Title:"Pane"}&&layoutSplit.ContentDockable is Document{Title:"Content"});
var layoutSplitter=LoadModelAsset<ProportionalDockSplitter>("ReactiveProportionalDockSplitterRoot");Check("layout-proportional-splitter-root",layoutSplitter.Title=="Source ProportionalDockSplitter"&&!layoutSplitter.CanResize&&layoutSplitter.ResizePreview);
var layoutGridSplitter=LoadModelAsset<GridDockSplitter>("ReactiveGridDockSplitterRoot");Check("layout-grid-splitter-root",layoutGridSplitter.Title=="Source GridDockSplitter"&&layoutGridSplitter.ResizeDirection==Dock.Model.Core.GridResizeDirection.Rows);
Check("layout-tool-source-defaults",new ToolDock() is {Alignment:Alignment.Unset,AutoHide:true,GripMode:GripMode.Visible,IsExpanded:false});
Check("layout-source-defaults",new DockDock().LastChildFill&&new StackDock().Spacing==0&&new UniformGridDock() is {Rows:0,Columns:0}&&new GridDock() is {ColumnDefinitions:null,RowDefinitions:null}&&new ProportionalDockSplitter() is {CanResize:true,ResizePreview:false}&&new GridDockSplitter().ResizeDirection==Dock.Model.Core.GridResizeDirection.Columns);
Check("layout-split-source-defaults",new SplitViewDock() is {CompactPaneLength:48,OpenPaneLength:320,IsPaneOpen:false,UseLightDismissOverlayMode:false,PaneDockable:null,ContentDockable:null,DisplayMode:Dock.Model.Core.SplitViewDisplayMode.Overlay,PanePlacement:Dock.Model.Core.SplitViewPanePlacement.Left});
var layoutRoot=new RootDock();var layoutEngine=new Dock.Model.ReactiveUI.Factory();layoutEngine.AddDockable(layoutRoot,layoutProportional);layoutEngine.AddDockable(layoutProportional,layoutTools);layoutEngine.InitLayout(layoutRoot);
var addedTool=LoadModelAsset<Tool>("ReactiveToolRoot");layoutTools.AddTool(addedTool);
Check("layout-original-add-tool",layoutTools.VisibleDockables!.Count==1&&ReferenceEquals(layoutTools.ActiveDockable,addedTool)&&ReferenceEquals(layoutRoot.FocusedDockable,addedTool)&&ReferenceEquals(addedTool.Owner,layoutTools));
var toolBindings=LoadModelAsset<StackPanel>("ReactiveLayoutBindings");toolBindings.DataContext=layoutTools;Avalonia.Threading.Dispatcher.UIThread.RunJobs();
var layoutTitle=(TextBlock)toolBindings.Children[0];var layoutAlignment=(TextBox)toolBindings.Children[1];
Check("layout-compiled-class-binding",layoutTitle.Text=="Source ToolDock"&&layoutAlignment.Text=="Right");
layoutAlignment.Text="Left";((CheckBox)toolBindings.Children[2]).IsChecked=false;((CheckBox)toolBindings.Children[3]).IsChecked=true;Avalonia.Threading.Dispatcher.UIThread.RunJobs();
Check("layout-two-way-enum-bool",layoutTools.Alignment==Alignment.Left&&!layoutTools.IsExpanded&&layoutTools.AutoHide);
layoutTools.Title="Engine tools";layoutTools.Alignment=Alignment.Bottom;Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("layout-live-source-notification",layoutTitle.Text=="Engine tools"&&layoutAlignment.Text=="Bottom");
var splitBindings=LoadModelAsset<StackPanel>("ReactiveSplitViewBindings");splitBindings.DataContext=layoutSplit;Avalonia.Threading.Dispatcher.UIThread.RunJobs();var splitOpen=(CheckBox)splitBindings.Children[0];
layoutSplit.ClosePane();Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("layout-close-pane-method",!layoutSplit.IsPaneOpen&&splitOpen.IsChecked==false);
layoutSplit.OpenPane();layoutSplit.TogglePane();Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("layout-open-toggle-pane-methods",!layoutSplit.IsPaneOpen&&splitOpen.IsChecked==false);
((Slider)splitBindings.Children[1]).Value=200;splitOpen.IsChecked=true;Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("layout-split-two-way",layoutSplit.OpenPaneLength==200&&layoutSplit.IsPaneOpen);
var paneContent=(ContentControl)splitBindings.Children[2];Check("layout-pane-identity",ReferenceEquals(paneContent.Content,layoutSplit.PaneDockable));layoutSplit.PaneDockable=null;Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("layout-pane-null",paneContent.Content is null);
var coreValues=LoadModelAsset<StackPanel>("CoreModelEnumAssembly");Check("layout-original-core-assembly",((ContentControl)coreValues.Children[0]).Content is Dock.Model.Core.SplitViewDisplayMode.CompactOverlay&&((ContentControl)coreValues.Children[1]).Content is Dock.Model.Core.SplitViewPanePlacement.Bottom&&((ContentControl)coreValues.Children[2]).Content is Dock.Model.Core.Orientation.Vertical);
var contractConsumer=LoadModelAsset<FactoryReference.ContractConsumer>("ContractConsumerRoot");
Check("layout-foreign-contract-assignments",contractConsumer.Policy is DockCapabilityOverrides{CanClose:false}&&contractConsumer.Optional is DockCapabilityOverrides{CanFloat:false});
var classFactory=LoadModelAsset<Dock.Model.ReactiveUI.Factory>("ReactiveFactoryRoot");
Check("factory-class-root",classFactory.HideToolsOnClose&&!classFactory.HideDocumentsOnClose);
Check("factory-class-defaults",new Dock.Model.ReactiveUI.Factory() is {HideToolsOnClose:false,HideDocumentsOnClose:false});
Check("factory-source-creation-overrides",classFactory.CreateRootDock() is RootDock&&classFactory.CreateProportionalDock() is ProportionalDock&&classFactory.CreateDockDock() is DockDock&&classFactory.CreateStackDock() is StackDock&&classFactory.CreateGridDock() is GridDock&&classFactory.CreateWrapDock() is WrapDock&&classFactory.CreateUniformGridDock() is UniformGridDock&&classFactory.CreateProportionalDockSplitter() is ProportionalDockSplitter&&classFactory.CreateGridDockSplitter() is GridDockSplitter&&classFactory.CreateToolDock() is ToolDock&&classFactory.CreateDocumentDock() is DocumentDock&&classFactory.CreateSplitViewDock() is SplitViewDock&&classFactory.CreateDocument() is Document&&classFactory.CreateTool() is Tool&&classFactory.CreateDockWindow() is DockWindow);
var createdRoot=classFactory.CreateRootDock();Check("factory-pinned-lists",createdRoot.LeftPinnedDockables is {Count:0}&&createdRoot.RightPinnedDockables is {Count:0}&&createdRoot.TopPinnedDockables is {Count:0}&&createdRoot.BottomPinnedDockables is {Count:0}&&!ReferenceEquals(createdRoot.LeftPinnedDockables,createdRoot.RightPinnedDockables)&&!ReferenceEquals(createdRoot.TopPinnedDockables,createdRoot.BottomPinnedDockables));
Check("factory-layout-creation",classFactory.CreateLayout() is RootDock{LeftPinnedDockables:not null});
var registry=classFactory.VisibleRootControls;var registryItem=new Document();registry.Add(registryItem,"native value");Check("factory-shared-dictionary",ReferenceEquals(registry,classFactory.VisibleRootControls)&&registry.Count==1);registry.Clear();Check("factory-registry-clear",classFactory.VisibleRootControls.Count==0);
Check("factory-fresh-registries",!ReferenceEquals(classFactory.VisibleRootControls,classFactory.PinnedRootControls)&&!ReferenceEquals(classFactory.DockControls,new Dock.Model.ReactiveUI.Factory().DockControls));
var classWindow=LoadModelAsset<DockWindow>("ReactiveDockWindowRoot");
Check("window-class-root",classWindow.Id=="source-window"&&classWindow.Title=="Source window"&&classWindow.X==42.25&&classWindow.Y==-12&&classWindow.Width==800&&classWindow.Height==600&&classWindow.WindowState==DockWindowState.Maximized&&classWindow.Topmost&&classWindow.OwnerMode==DockWindowOwnerMode.RootWindow&&classWindow.IsModal&&classWindow.ShowInTaskbar==false&&classWindow.Layout?.Title=="Window layout");
Check("window-class-defaults",new DockWindow() is {Id:"IDockWindow",Title:"IDockWindow",X:0,Y:0,Width:0,Height:0,WindowState:DockWindowState.Normal,Topmost:false,IsModal:false,ShowInTaskbar:null,OwnerMode:DockWindowOwnerMode.Default,Layout:null,Host:null,Factory:null,Owner:null,ParentWindow:null});
Check("window-source-callback-defaults",classWindow.OnClose()&&classWindow.OnMoveDragBegin());classWindow.OnMoveDrag();classWindow.OnMoveDragEnd();classWindow.Save();classWindow.SetActive();classWindow.Exit();
var factoryClassBindings=LoadModelAsset<StackPanel>("ReactiveFactoryBindings");factoryClassBindings.DataContext=classFactory;Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("factory-compiled-class-bindings",((CheckBox)factoryClassBindings.Children[0]).IsChecked==true&&((TextBlock)factoryClassBindings.Children[2]).Text=="0"&&((TextBlock)factoryClassBindings.Children[3]).Text=="0"&&((TextBlock)factoryClassBindings.Children[4]).Text=="0");
((CheckBox)factoryClassBindings.Children[0]).IsChecked=false;((CheckBox)factoryClassBindings.Children[1]).IsChecked=true;Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("factory-two-way-class-inputs",!classFactory.HideToolsOnClose&&classFactory.HideDocumentsOnClose);
var windowClassBindings=LoadModelAsset<StackPanel>("ReactiveDockWindowBindings");windowClassBindings.DataContext=classWindow;Avalonia.Threading.Dispatcher.UIThread.RunJobs();var windowClassTitle=(TextBlock)windowClassBindings.Children[0];var windowClassState=(TextBox)windowClassBindings.Children[4];Check("window-compiled-class-bindings",windowClassTitle.Text=="Source window"&&windowClassState.Text=="Maximized");
((TextBox)windowClassBindings.Children[1]).Text="Edited window";((Slider)windowClassBindings.Children[2]).Value=700;((CheckBox)windowClassBindings.Children[3]).IsChecked=false;windowClassState.Text="Normal";Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("window-two-way-class-inputs",classWindow.Title=="Edited window"&&classWindow.Width==700&&!classWindow.Topmost&&classWindow.WindowState==DockWindowState.Normal);classWindow.Title="Engine window";Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("window-source-class-notification",windowClassTitle.Text=="Engine window");
var windowGraph=LoadModelAsset<StackPanel>("ReactiveFactoryWindowGraph");var graphFactory=(Dock.Model.ReactiveUI.Factory)((ContentControl)windowGraph.Children[0]).Content!;var graphParent=(DockWindow)((ContentControl)windowGraph.Children[1]).Content!;var graphOwner=(RootDock)((ContentControl)windowGraph.Children[2]).Content!;var graphWindow=(DockWindow)((ContentControl)windowGraph.Children[3]).Content!;
Check("window-class-composition-references",ReferenceEquals(graphWindow.Factory,graphFactory)&&ReferenceEquals(graphWindow.ParentWindow,graphParent)&&ReferenceEquals(graphWindow.Owner,graphOwner)&&ReferenceEquals(graphWindow.Layout,graphOwner));graphFactory.InitDockWindow(graphWindow,graphOwner);Check("window-class-factory-initialization",ReferenceEquals(graphWindow.Factory,graphFactory)&&graphOwner.Window is null&&ReferenceEquals(graphOwner.Factory,graphFactory)&&graphWindow.Host is null);
IFactory? absent=null;
Check("factory-null-root",absent!.GetActiveRoot() is null);Check("factory-null-document",absent!.GetCurrentDocument() is null);
Check("factory-null-windows",absent!.GetCurrentDockWindow() is null&&absent!.GetCurrentHostWindow() is null);
absent!.CloseFocusedDockable();Check("factory-null-close",true);
var factory=new ProbeFactory();var inactive=new RootDock();var active=new RootDock{IsActive=true};var later=new RootDock{IsActive=true};
foreach(var root in new[]{inactive,active,later})factory.DockControls.Add(new DockControl{InitializeLayout=false,Layout=root});
Check("factory-first-active",ReferenceEquals(factory.GetActiveRoot(),active));
factory.Track(null,inactive,null);Check("factory-tracked-inactive",ReferenceEquals(factory.GetActiveRoot(),inactive));
var focus=new Document();var current=new Document();inactive.FocusedDockable=focus;
factory.Track(current,inactive,null);Check("factory-current-document",ReferenceEquals(factory.GetCurrentDocument(),current));
var splitter=new ProportionalDockSplitter();factory.Track(splitter,inactive,null);
Check("factory-focused-fallback",ReferenceEquals(factory.GetCurrentDocument(),focus));
factory.CloseFocusedDockable();Check("factory-close-current",ReferenceEquals(factory.Calls[0],splitter));
factory.Track(null,inactive,null);factory.CloseFocusedDockable();Check("factory-close-fallback",ReferenceEquals(factory.Calls[1],focus));
inactive.FocusedDockable=null;factory.CloseFocusedDockable();Check("factory-close-empty",factory.Calls.Count==2);
var tool=new Tool();factory.Track(tool,inactive,null);Check("factory-tool-is-document",ReferenceEquals(factory.GetCurrentDocument(),tool));
var window=new DockWindow();factory.Track(null,null,window);Check("factory-window",ReferenceEquals(factory.GetCurrentDockWindow(),window));
var host=new HostWindow();window.Host=host;Check("factory-live-host",ReferenceEquals(factory.GetCurrentHostWindow(),host));
window.Host=null;Check("factory-cleared-host",factory.GetCurrentHostWindow() is null);
var engine=new Dock.Model.ReactiveUI.Factory{HideDocumentsOnClose=false};var document=new Document();
var documentDock=new DocumentDock{CanCloseLastDockable=true,IsCollapsable=false,VisibleDockables=new ObservableCollection<IDockable>{document},ActiveDockable=document};
var layout=new RootDock{VisibleDockables=new ObservableCollection<IDockable>{documentDock},ActiveDockable=documentDock,FocusedDockable=document};
engine.InitLayout(layout);engine.CloseFocusedDockable();Check("factory-real-close",documentDock.VisibleDockables!.Count==0);

// Exact source events, without reflection or substituted models.
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.ActiveDockableChangedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.ActiveDockableChanged+=handler;f.OnActiveDockableChanged(eventDocument);Check("event-ActiveDockableChanged",calls==1&&valid);
 f.ActiveDockableChanged-=handler;f.OnActiveDockableChanged(eventDocument);Check("event-disposed-ActiveDockableChanged",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.FocusedDockableChangedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.FocusedDockableChanged+=handler;f.OnFocusedDockableChanged(eventDocument);Check("event-FocusedDockableChanged",calls==1&&valid);
 f.FocusedDockableChanged-=handler;f.OnFocusedDockableChanged(eventDocument);Check("event-disposed-FocusedDockableChanged",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.GlobalDockTrackingChangedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&e.Current.Dockable is not null&&e.Reason==DockTrackingChangeReason.ActiveDockableChanged;};
 f.GlobalDockTrackingChanged+=handler;f.OnActiveDockableChanged(eventDocument);Check("event-GlobalDockTrackingChanged",calls==1&&valid);
 f.GlobalDockTrackingChanged-=handler;f.OnActiveDockableChanged(eventDocument);Check("event-disposed-GlobalDockTrackingChanged",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableInitEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableInit+=handler;f.OnDockableInit(eventDocument);Check("event-DockableInit",calls==1&&valid);
 f.DockableInit-=handler;f.OnDockableInit(eventDocument);Check("event-disposed-DockableInit",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableAddedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableAdded+=handler;f.OnDockableAdded(eventDocument);Check("event-DockableAdded",calls==1&&valid);
 f.DockableAdded-=handler;f.OnDockableAdded(eventDocument);Check("event-disposed-DockableAdded",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableRemovedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableRemoved+=handler;f.OnDockableRemoved(eventDocument);Check("event-DockableRemoved",calls==1&&valid);
 f.DockableRemoved-=handler;f.OnDockableRemoved(eventDocument);Check("event-disposed-DockableRemoved",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableClosingEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableClosing+=handler;f.OnDockableClosing(eventDocument);Check("event-DockableClosing",calls==1&&valid);
 f.DockableClosing-=handler;f.OnDockableClosing(eventDocument);Check("event-disposed-DockableClosing",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableClosedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableClosed+=handler;f.OnDockableClosed(eventDocument);Check("event-DockableClosed",calls==1&&valid);
 f.DockableClosed-=handler;f.OnDockableClosed(eventDocument);Check("event-disposed-DockableClosed",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableMovedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableMoved+=handler;f.OnDockableMoved(eventDocument);Check("event-DockableMoved",calls==1&&valid);
 f.DockableMoved-=handler;f.OnDockableMoved(eventDocument);Check("event-disposed-DockableMoved",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableDockedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableDocked+=handler;f.OnDockableDocked(eventDocument,DockOperation.Left);Check("event-DockableDocked",calls==1&&valid);
 f.DockableDocked-=handler;f.OnDockableDocked(eventDocument,DockOperation.Left);Check("event-disposed-DockableDocked",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableUndockedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableUndocked+=handler;f.OnDockableUndocked(eventDocument,DockOperation.Left);Check("event-DockableUndocked",calls==1&&valid);
 f.DockableUndocked-=handler;f.OnDockableUndocked(eventDocument,DockOperation.Left);Check("event-disposed-DockableUndocked",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableSwappedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableSwapped+=handler;f.OnDockableSwapped(eventDocument);Check("event-DockableSwapped",calls==1&&valid);
 f.DockableSwapped-=handler;f.OnDockableSwapped(eventDocument);Check("event-disposed-DockableSwapped",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockablePinnedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockablePinned+=handler;f.OnDockablePinned(eventDocument);Check("event-DockablePinned",calls==1&&valid);
 f.DockablePinned-=handler;f.OnDockablePinned(eventDocument);Check("event-disposed-DockablePinned",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableUnpinnedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableUnpinned+=handler;f.OnDockableUnpinned(eventDocument);Check("event-DockableUnpinned",calls==1&&valid);
 f.DockableUnpinned-=handler;f.OnDockableUnpinned(eventDocument);Check("event-disposed-DockableUnpinned",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableHiddenEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableHidden+=handler;f.OnDockableHidden(eventDocument);Check("event-DockableHidden",calls==1&&valid);
 f.DockableHidden-=handler;f.OnDockableHidden(eventDocument);Check("event-disposed-DockableHidden",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableRestoredEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableRestored+=handler;f.OnDockableRestored(eventDocument);Check("event-DockableRestored",calls==1&&valid);
 f.DockableRestored-=handler;f.OnDockableRestored(eventDocument);Check("event-disposed-DockableRestored",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.WindowOpenedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Window,eventWindow);};
 f.WindowOpened+=handler;f.OnWindowOpened(eventWindow);Check("event-WindowOpened",calls==1&&valid);
 f.WindowOpened-=handler;f.OnWindowOpened(eventWindow);Check("event-disposed-WindowOpened",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.WindowClosingEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Window,eventWindow);};
 f.WindowClosing+=handler;f.OnWindowClosing(eventWindow);Check("event-WindowClosing",calls==1&&valid);
 f.WindowClosing-=handler;f.OnWindowClosing(eventWindow);Check("event-disposed-WindowClosing",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.WindowClosedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Window,eventWindow);};
 f.WindowClosed+=handler;f.OnWindowClosed(eventWindow);Check("event-WindowClosed",calls==1&&valid);
 f.WindowClosed-=handler;f.OnWindowClosed(eventWindow);Check("event-disposed-WindowClosed",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.WindowAddedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Window,eventWindow);};
 f.WindowAdded+=handler;f.OnWindowAdded(eventWindow);Check("event-WindowAdded",calls==1&&valid);
 f.WindowAdded-=handler;f.OnWindowAdded(eventWindow);Check("event-disposed-WindowAdded",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.WindowRemovedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Window,eventWindow);};
 f.WindowRemoved+=handler;f.OnWindowRemoved(eventWindow);Check("event-WindowRemoved",calls==1&&valid);
 f.WindowRemoved-=handler;f.OnWindowRemoved(eventWindow);Check("event-disposed-WindowRemoved",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.WindowMoveDragBeginEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Window,eventWindow);};
 f.WindowMoveDragBegin+=handler;f.OnWindowMoveDragBegin(eventWindow);Check("event-WindowMoveDragBegin",calls==1&&valid);
 f.WindowMoveDragBegin-=handler;f.OnWindowMoveDragBegin(eventWindow);Check("event-disposed-WindowMoveDragBegin",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.WindowMoveDragEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Window,eventWindow);};
 f.WindowMoveDrag+=handler;f.OnWindowMoveDrag(eventWindow);Check("event-WindowMoveDrag",calls==1&&valid);
 f.WindowMoveDrag-=handler;f.OnWindowMoveDrag(eventWindow);Check("event-disposed-WindowMoveDrag",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.WindowMoveDragEndEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Window,eventWindow);};
 f.WindowMoveDragEnd+=handler;f.OnWindowMoveDragEnd(eventWindow);Check("event-WindowMoveDragEnd",calls==1&&valid);
 f.WindowMoveDragEnd-=handler;f.OnWindowMoveDragEnd(eventWindow);Check("event-disposed-WindowMoveDragEnd",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.WindowActivatedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Window,eventWindow);};
 f.WindowActivated+=handler;f.OnWindowActivated(eventWindow);Check("event-WindowActivated",calls==1&&valid);
 f.WindowActivated-=handler;f.OnWindowActivated(eventWindow);Check("event-disposed-WindowActivated",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableActivatedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableActivated+=handler;f.OnDockableActivated(eventDocument);Check("event-DockableActivated",calls==1&&valid);
 f.DockableActivated-=handler;f.OnDockableActivated(eventDocument);Check("event-disposed-DockableActivated",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.WindowDeactivatedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Window,eventWindow);};
 f.WindowDeactivated+=handler;f.OnWindowDeactivated(eventWindow);Check("event-WindowDeactivated",calls==1&&valid);
 f.WindowDeactivated-=handler;f.OnWindowDeactivated(eventWindow);Check("event-disposed-WindowDeactivated",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventRoot=f.CreateRootDock();var eventDocument=f.CreateDocument();eventDocument.Owner=eventRoot;var eventWindow=f.CreateDockWindow();eventWindow.Layout=eventRoot;eventRoot.Window=eventWindow;
 int calls=0;bool valid=false;EventHandler<Dock.Model.Core.Events.DockableDeactivatedEventArgs> handler=(sender,e)=>{calls++;valid=ReferenceEquals(sender,f)&&ReferenceEquals(e.Dockable,eventDocument);};
 f.DockableDeactivated+=handler;f.OnDockableDeactivated(eventDocument);Check("event-DockableDeactivated",calls==1&&valid);
 f.DockableDeactivated-=handler;f.OnDockableDeactivated(eventDocument);Check("event-disposed-DockableDeactivated",calls==1);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.ActiveDockableChangedEventArgs(eventDocument,eventRoot,eventWindow);Check("event-argument-ActiveDockableChanged",ReferenceEquals(e.Dockable,eventDocument)&&ReferenceEquals(e.RootDock,eventRoot)&&ReferenceEquals(e.Window,eventWindow)&&e.HostWindow is null);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableActivatedEventArgs(eventDocument);Check("event-argument-DockableActivated",ReferenceEquals(e.Dockable,eventDocument));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableAddedEventArgs(eventDocument);Check("event-argument-DockableAdded",ReferenceEquals(e.Dockable,eventDocument));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableClosedEventArgs(eventDocument);Check("event-argument-DockableClosed",ReferenceEquals(e.Dockable,eventDocument));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableClosingEventArgs(eventDocument);Check("event-argument-DockableClosing",ReferenceEquals(e.Dockable,eventDocument)&&!e.Cancel);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableDeactivatedEventArgs(eventDocument);Check("event-argument-DockableDeactivated",ReferenceEquals(e.Dockable,eventDocument));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableDockedEventArgs(eventDocument,DockOperation.Right);Check("event-argument-DockableDocked",ReferenceEquals(e.Dockable,eventDocument)&&e.Operation==DockOperation.Right);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableHiddenEventArgs(eventDocument);Check("event-argument-DockableHidden",ReferenceEquals(e.Dockable,eventDocument));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableInitEventArgs(eventDocument);Check("event-argument-DockableInit",ReferenceEquals(e.Dockable,eventDocument)&&e.Context is null);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableMovedEventArgs(eventDocument);Check("event-argument-DockableMoved",ReferenceEquals(e.Dockable,eventDocument));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableOwnerChangingEventArgs(eventDocument,null);Check("event-argument-DockableOwnerChanging",ReferenceEquals(e.OldOwner,eventDocument)&&e.NewOwner is null);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockablePinnedEventArgs(eventDocument);Check("event-argument-DockablePinned",ReferenceEquals(e.Dockable,eventDocument));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableRemovedEventArgs(eventDocument);Check("event-argument-DockableRemoved",ReferenceEquals(e.Dockable,eventDocument));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableRestoredEventArgs(eventDocument);Check("event-argument-DockableRestored",ReferenceEquals(e.Dockable,eventDocument));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableSwappedEventArgs(eventDocument);Check("event-argument-DockableSwapped",ReferenceEquals(e.Dockable,eventDocument));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableUndockedEventArgs(eventDocument,DockOperation.Right);Check("event-argument-DockableUndocked",ReferenceEquals(e.Dockable,eventDocument)&&e.Operation==DockOperation.Right);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.DockableUnpinnedEventArgs(eventDocument);Check("event-argument-DockableUnpinned",ReferenceEquals(e.Dockable,eventDocument));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.FocusedDockableChangedEventArgs(eventDocument,eventRoot,eventWindow);Check("event-argument-FocusedDockableChanged",ReferenceEquals(e.Dockable,eventDocument)&&ReferenceEquals(e.RootDock,eventRoot)&&ReferenceEquals(e.Window,eventWindow)&&e.HostWindow is null);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.GlobalDockTrackingChangedEventArgs(GlobalDockTrackingState.Empty,new GlobalDockTrackingState(eventDocument,eventRoot,eventWindow),DockTrackingChangeReason.WindowActivated);Check("event-argument-GlobalDockTrackingChanged",e.Previous.Dockable is null&&ReferenceEquals(e.Current.Dockable,eventDocument)&&e.Reason==DockTrackingChangeReason.WindowActivated);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.WindowActivatedEventArgs(eventWindow);Check("event-argument-WindowActivated",ReferenceEquals(e.Window,eventWindow));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.WindowAddedEventArgs(eventWindow);Check("event-argument-WindowAdded",ReferenceEquals(e.Window,eventWindow));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.WindowClosedEventArgs(eventWindow);Check("event-argument-WindowClosed",ReferenceEquals(e.Window,eventWindow));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.WindowClosingEventArgs(eventWindow);Check("event-argument-WindowClosing",ReferenceEquals(e.Window,eventWindow)&&!e.Cancel);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.WindowDeactivatedEventArgs(eventWindow);Check("event-argument-WindowDeactivated",ReferenceEquals(e.Window,eventWindow));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.WindowMoveDragBeginEventArgs(eventWindow);Check("event-argument-WindowMoveDragBegin",ReferenceEquals(e.Window,eventWindow)&&!e.Cancel);}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.WindowMoveDragEndEventArgs(eventWindow);Check("event-argument-WindowMoveDragEnd",ReferenceEquals(e.Window,eventWindow));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.WindowMoveDragEventArgs(eventWindow);Check("event-argument-WindowMoveDrag",ReferenceEquals(e.Window,eventWindow));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.WindowOpenedEventArgs(eventWindow);Check("event-argument-WindowOpened",ReferenceEquals(e.Window,eventWindow));}
{var f=new Dock.Model.ReactiveUI.Factory();var eventDocument=f.CreateDocument();var eventRoot=f.CreateRootDock();var eventWindow=f.CreateDockWindow();var e=new Dock.Model.Core.Events.WindowRemovedEventArgs(eventWindow);Check("event-argument-WindowRemoved",ReferenceEquals(e.Window,eventWindow));}
{
 var f=new Dock.Model.ReactiveUI.Factory();f.DockableClosing+=(_,e)=>e.Cancel=true;f.WindowClosing+=(_,e)=>e.Cancel=true;f.WindowMoveDragBegin+=(_,e)=>e.Cancel=true;
 Check("event-cancel-document",!f.OnDockableClosing(f.CreateDocument()));Check("event-cancel-window",!f.OnWindowClosing(f.CreateDockWindow()));Check("event-cancel-drag",!f.OnWindowMoveDragBegin(f.CreateDockWindow()));
 var d=f.CreateDocument();object contextObject=new();f.DockableInit+=(_,e)=>e.Context=contextObject;f.OnDockableInit(d);Check("event-context-replacement",ReferenceEquals(d.Context,contextObject));
}
{
 var eventArgument=LoadModelAsset<Dock.Model.Core.Events.DockableClosingEventArgs>("FactoryEventArgumentRoot");Check("event-compiled-argument-root",eventArgument.Cancel&&eventArgument.Dockable?.Title=="Event document");
 var state=LoadModelAsset<GlobalDockTrackingState>("FactoryTrackingStateRoot");Check("event-compiled-tracking-root",state.Dockable is null&&state.RootDock is null&&state.Window is null);
 var owner=LoadModelAsset<DockFactoryMarkupReference.FactoryEventConsumer>("FactoryEventConsumer");var f=owner.Factory!;Check("event-compiled-handler",!f.OnDockableClosing(f.CreateDocument())&&owner.Calls==1&&ReferenceEquals(owner.Sender,f));
}
{
 var trackingReasonText=LoadModelAsset<TextBlock>("FactoryTrackingEventBindings");
 foreach(var reason in new[]{DockTrackingChangeReason.Unknown,DockTrackingChangeReason.ActiveDockableChanged,DockTrackingChangeReason.FocusedDockableChanged,DockTrackingChangeReason.WindowActivated,DockTrackingChangeReason.WindowDeactivated,DockTrackingChangeReason.DockableActivated,DockTrackingChangeReason.DockableDeactivated,DockTrackingChangeReason.WindowClosed,DockTrackingChangeReason.WindowRemoved,DockTrackingChangeReason.LayoutInitialized}){
  trackingReasonText.DataContext=new Dock.Model.Core.Events.GlobalDockTrackingChangedEventArgs(GlobalDockTrackingState.Empty,GlobalDockTrackingState.Empty,reason);Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("event-compiled-reason-"+reason,trackingReasonText.Text==reason.ToString());
 }
}
{
 var f=new Dock.Model.ReactiveUI.Factory();var explicitRoot=f.CreateRootDock();var explicitDocument=f.CreateDocument();var explicitWindow=f.CreateDockWindow();explicitDocument.Owner=explicitRoot;explicitRoot.FocusedDockable=explicitDocument;
 f.OnWindowActivated(explicitWindow,explicitRoot);Check("event-explicit-root-overload",ReferenceEquals(f.CurrentRootDock,explicitRoot)&&f.CurrentDockWindow is null&&ReferenceEquals(f.CurrentDockable,explicitDocument));
}
{
 var locatorFactory=LoadModelAsset<DockFactoryMarkupReference.LocatorFactory>("FactoryLocatorConsumer");
 Check("locator-compiled-deferred",locatorFactory.ContextCalls==0&&locatorFactory.HostCalls==0);
 Check("locator-default-context",ReferenceEquals(locatorFactory.GetContext("missing"),locatorFactory.ContextValue));
 Check("locator-default-host",ReferenceEquals(locatorFactory.GetHostWindow("missing"),locatorFactory.HostValue));
 var contexts=new Dictionary<string,Func<object?>>{{"null",null!},{"none",()=>null},{"context",locatorFactory.CreateContext}};
 locatorFactory.ContextLocator=contexts;
 Check("locator-null-entry",locatorFactory.GetContext("null") is null&&locatorFactory.ContextCalls==1);
 Check("locator-null-return",locatorFactory.GetContext("none") is null&&locatorFactory.ContextCalls==1);
 Check("locator-empty-id",locatorFactory.GetContext("") is null&&locatorFactory.ContextCalls==1);
 Check("locator-dictionary-context",ReferenceEquals(locatorFactory.GetContext("context"),locatorFactory.ContextValue)&&locatorFactory.ContextCalls==2);
 var hosts=new Dictionary<string,Func<IHostWindow?>>{{"null",null!},{"none",()=>null},{"host",locatorFactory.CreateHost}};
 locatorFactory.HostWindowLocator=hosts;
 Check("locator-host-null-entry",locatorFactory.GetHostWindow("null") is null&&locatorFactory.HostCalls==1);
 Check("locator-host-null-return",locatorFactory.GetHostWindow("none") is null&&locatorFactory.HostCalls==1);
 Check("locator-dictionary-host",ReferenceEquals(locatorFactory.GetHostWindow("host"),locatorFactory.HostValue)&&locatorFactory.HostCalls==2);
 locatorFactory.DockableLocator=new Dictionary<string,Func<IDockable?>>{{"null",null!},{"none",()=>null},{"document",locatorFactory.CreateDockable}};
 Check("locator-dockable-null-entry",locatorFactory.GetDockable<IDockable>("null") is null&&locatorFactory.DockableCalls==0);
 Check("locator-dockable-null-return",locatorFactory.GetDockable<IDockable>("none") is null&&locatorFactory.DockableCalls==0);
 Check("locator-dockable-reference",ReferenceEquals(locatorFactory.GetDockable<IDockable>("document"),locatorFactory.DocumentValue)&&locatorFactory.DockableCalls==1);
 Check("locator-dockable-wrong-contract",locatorFactory.GetDockable<IToolDock>("document") is null&&locatorFactory.DockableCalls==2);
 bool duplicateKey=false;try{contexts.Add("context",locatorFactory.CreateContext);}catch(ArgumentException){duplicateKey=true;}Check("locator-dictionary-duplicate",duplicateKey&&contexts.Count==3);
 bool missingKey=false;try{_=contexts["missing"];}catch(KeyNotFoundException){missingKey=true;}Check("locator-dictionary-missing-key",missingKey);
 contexts["null"]=locatorFactory.CreateContext;Check("locator-replaced-null",ReferenceEquals(locatorFactory.GetContext("null"),locatorFactory.ContextValue));
 Check("locator-remove-null",hosts.Remove("null")&&!hosts.Remove("null"));
 locatorFactory.DocumentValue.Title="Tracked model";var trackedRoot=locatorFactory.CreateRootDock();locatorFactory.AddDockable(trackedRoot,locatorFactory.DocumentValue);locatorFactory.OnActiveDockableChanged(locatorFactory.DocumentValue);
 var bindingPanel=LoadModelAsset<StackPanel>("CoreFactoryBindings");bindingPanel.DataContext=locatorFactory;Avalonia.Threading.Dispatcher.UIThread.RunJobs();
 Check("locator-core-tracking-title",((TextBlock)bindingPanel.Children[0]).Text=="Tracked model");
 Check("locator-core-bindings",((TextBlock)bindingPanel.Children[2]).Text=="3");
 ((CheckBox)bindingPanel.Children[1]).IsChecked=true;Avalonia.Threading.Dispatcher.UIThread.RunJobs();Check("locator-core-two-way",locatorFactory.HideDocumentsOnClose);
}
{
 var transientFactory=new DockFactoryMarkupReference.TransientHostFactory();transientFactory.InitLayout(transientFactory.CreateRootDock());Check("locator-transient-virtual-default",transientFactory.Reads>=2&&transientFactory.HostWindowLocator!.ContainsKey("IDockWindow")&&transientFactory.HostWindowLocator["IDockWindow"] is null);
}
{
 var searchFactory=LoadModelAsset<DockFactoryMarkupReference.LocatorFactory>("FactorySearchConsumer");
 Check("factory-api-method-predicates-deferred",searchFactory.PredicateCalls==0);
 var searchRoot=searchFactory.CreateRootDock();searchRoot.Id="root";var searchDocument=searchFactory.DocumentValue;searchDocument.Id="item";searchFactory.AddDockable(searchRoot,searchDocument);
 var searchQuery=searchFactory.Find(searchRoot,searchFactory.SearchPredicate!);Check("factory-api-method-query-deferred",searchFactory.PredicateCalls==0);
 using var searchCursor=searchQuery.GetEnumerator();Check("factory-api-method-enumerator-deferred",searchFactory.PredicateCalls==0);
 Check("factory-api-method-query-item",searchCursor.MoveNext()&&ReferenceEquals(searchCursor.Current,searchDocument));
 Check("factory-api-method-query-end",!searchCursor.MoveNext()&&searchFactory.PredicateCalls==2);
 Check("factory-api-method-root-predicate",ReferenceEquals(searchFactory.FindRoot(searchDocument,searchFactory.RootPredicate),searchRoot));
}
{
 var apiFactory=new Dock.Model.ReactiveUI.Factory();var apiRoot=apiFactory.CreateRootDock();var apiDocument=apiFactory.CreateDocument();apiFactory.AddDockable(apiRoot,apiDocument);apiFactory.InitLayout(apiRoot);
 var apiWindow=apiFactory.CreateDockWindow();apiWindow.Layout=apiRoot;apiRoot.Window=apiWindow;apiFactory.InitDockWindow(apiWindow,apiRoot);
 var apiButton=LoadModelAsset<Button>("CoreFactoryApiBindings");apiButton.DataContext=apiFactory;Avalonia.Threading.Dispatcher.UIThread.RunJobs();apiButton.Command!.Execute(apiDocument);
 Check("factory-api-compiled-command-window",ReferenceEquals(apiFactory.CurrentDockWindow,apiWindow));
}
DockFactoryMarkupReference.FactoryApiReference.Run(Check);
DockFactoryMarkupReference.CorePolicyReference.Run(Check, name=>LoadModelAsset<object>(name));
DockFactoryMarkupReference.NumericEnumReference.Run(Check, name=>LoadModelAsset<object>(name));
DockFactoryMarkupReference.CoreContractReference.Run(Check, name=>LoadModelAsset<object>(name));
DockFactoryMarkupReference.CoreControlsReference.Run(Check, name=>LoadModelAsset<object>(name));
DockFactoryMarkupReference.NativeDocumentFactoryReference.Run(Check, name=>LoadModelAsset<object>(name));
DockFactoryMarkupReference.NullableFactoryReference.Run(Check, name=>LoadModelAsset<object>(name));
DockFactoryMarkupReference.NullableListReference.Run(Check, name=>LoadModelAsset<object>(name));
DockFactoryMarkupReference.NullableStateReference.Run(Check);
DockFactoryMarkupReference.NullableMutationReference.Run(Check);
DockFactoryMarkupReference.NullableTargetReference.Run(Check);
DockFactoryMarkupReference.NullableMoveReference.Run(Check);
DockFactoryMarkupReference.NullablePreviewReference.Run(Check);
DockFactoryMarkupReference.NullableHiddenReference.Run(Check);
DockFactoryMarkupReference.NullableFloatingReference.Run(Check);
DockFactoryMarkupReference.NullableFocusReference.Run(Check);
DockFactoryMarkupReference.NullableWindowOptionsReference.Run(Check);
DockFactoryMarkupReference.NullableWindowsReference.Run(Check, name=>LoadModelAsset<object>(name));
DockFactoryMarkupReference.NullableSearchReference.Run(Check);
DockFactoryMarkupReference.QueryCursorReference.Run(Check);
DockFactoryMarkupReference.SerializerFailureReference.Run(Check);
DockFactoryMarkupReference.CoreHostReference.Run(Check, name=>LoadModelAsset<object>(name));
Console.WriteLine("Original Load/Reference and factory API reference branches passed.");
sealed class Context:IUriContext{
 private Uri _baseUri=null!;public int Reads;public Action? OnRead;public Uri BaseUri{get{Reads++;OnRead?.Invoke();return _baseUri;}set=>_baseUri=value;}
}
sealed class Provider:IServiceProvider{
 private readonly IUriContext? _context;private readonly INameScope? _scope;
 public Provider(IUriContext? context,INameScope? scope){_context=context;_scope=scope;}
 public object? GetService(Type type)=>type==typeof(IUriContext)?_context:type==typeof(INameScope)?_scope:null;
}
sealed class ProbeFactory:Dock.Model.ReactiveUI.Factory{
 public List<IDockable?> Calls{get;}=new();
 public void Track(IDockable? value,IRootDock? root,IDockWindow? window)=>UpdateGlobalDockTracking(value,root,window,DockTrackingChangeReason.LayoutInitialized);
 public override void CloseDockable(IDockable? value)=>Calls.Add(value);
}

namespace FactoryReference {
 public sealed class ContractConsumer {
  public IDockCapabilityPolicy Policy {get;set;}=new DockCapabilityOverrides();
  public IDockCapabilityPolicy? Optional {get;set;}
 }
}
