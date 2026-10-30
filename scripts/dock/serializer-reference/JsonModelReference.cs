// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Collections.ObjectModel;
using System.Text.Json;
using System.Text.Json.Serialization.Metadata;
using Dock.Model.Controls;
using Dock.Model;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
using Dock.Model.ReactiveUI.Core;
using Dock.Serializer.SystemTextJson;
internal static class JsonModelReference
{
 internal static void Run(string path){
  // Load the actual model assembly before source polymorphic templates initialize.
  var factory=new Factory();var root=(RootDock)factory.CreateRootDock();root.Id="root";root.Title="Root";
  var docs=new DocumentDock{Id="docs",Title="Documents"};var document=new Document{Id="doc",Title="Document"};
  docs.VisibleDockables=factory.CreateList<IDockable>(document);docs.ActiveDockable=document;docs.DefaultDockable=document;
  document.Proportion=0.125;document.MinWidth=128.5;document.MaxWidth=double.PositiveInfinity;document.MinHeight=-0d;
  docs.CollapsedProportion=0.25;
  var policy=new DockCapabilityPolicy{CanClose=false,CanPin=true};
  root.RootDockCapabilityPolicy=policy;docs.DockCapabilityPolicy=policy;
  document.DockCapabilityOverrides=new DockCapabilityOverrides{CanClose=true,CanFloat=false};
  document.PinnedBounds=new DockRect(-0d,0.125,512.5,256.25);
  document.AllowedDockOperations=DockOperationMask.Fill|DockOperationMask.Left;
  document.AllowedDropOperations=(DockOperationMask)unchecked((int)0x80000001);
  root.Dock=(DockMode)(-19);root.PinnedDockDisplayMode=(PinnedDockDisplayMode)123;root.FloatingWindowHostMode=(DockFloatingWindowHostMode)(-17);
  docs.Dock=(DockMode)int.MinValue;document.Dock=(DockMode)123;
  docs.PinnedDockDisplayModeOverride=PinnedDockDisplayMode.Inline;document.PinnedDockDisplayModeOverride=(PinnedDockDisplayMode)int.MinValue;
  root.VisibleDockables=factory.CreateList<IDockable>(docs);root.DefaultDockable=docs;factory.InitLayout(root);
  var floating=new RootDock{Id="float",Title="Floating",VisibleDockables=factory.CreateList<IDockable>()};
  var window=new DockWindow{Id="window",Title="Window",Layout=floating,X=-0d,Y=1e-5,Width=1024.25,Height=768};
  root.Windows=factory.CreateList<IDockWindow>(window,window);root.Window=new DockWindow{Id="main",Title="Main",Layout=root};
  new DockWindowOptions{OwnerMode=(DockWindowOwnerMode)(-19),ParentWindow=root.Window,IsModal=true,ShowInTaskbar=false}.ApplyTo(window);
  window.WindowState=(DockWindowState)123;window.Topmost=true;
  using var output=new StreamWriter(path);void Check(string name,bool value){if(!value)throw new InvalidOperationException(name);output.WriteLine(name+"\t1");}
  DockSerializer Serializer(Type list){
   var resolver=new DockModelPolymorphicTypeResolver();
   var allowed=new HashSet<string>{"Id","Title","Owner","VisibleDockables","ActiveDockable","DefaultDockable","Window","Windows","Layout","ParentWindow","A","B","Item","Proportion","CollapsedProportion","MinWidth","MaxWidth","MinHeight","MaxHeight","X","Y","Width","Height","PinnedBounds","DockCapabilityOverrides","DockCapabilityPolicy","RootDockCapabilityPolicy","CanClose","CanPin","CanFloat","CanDrag","CanDrop","CanDockAsDocument","DockingState","AllowedDockOperations","AllowedDropOperations","WindowState","Topmost","OwnerMode","IsModal","ShowInTaskbar","Dock","PinnedDockDisplayModeOverride","PinnedDockDisplayMode","FloatingWindowHostMode"};
   resolver.Modifiers.Add(info=>{if(info.Kind==JsonTypeInfoKind.Object){for(int i=info.Properties.Count-1;i>=0;i--)if(!allowed.Contains(info.Properties[i].Name))info.Properties.RemoveAt(i);}});
   return new DockSerializer(list,resolver);
  }
  var serializer=Serializer(typeof(ObservableCollection<>));var text=serializer.Serialize(root);
  File.WriteAllText(Path.ChangeExtension(path,".root.json"),text);
  var loaded=serializer.Deserialize<RootDock>(text)!;var loadedDocs=(DocumentDock)loaded.VisibleDockables![0];var loadedDocument=(Document)loadedDocs.VisibleDockables![0];
  Check("model-concrete-root-no-discriminator",!JsonDocument.Parse(text).RootElement.TryGetProperty("$type",out _));
  Check("model-derived-document-type-tag",text.Contains("Dock.Model.ReactiveUI.Controls.Document\""));
  Check("model-root-concrete-to-owner-interface-reference",ReferenceEquals(loadedDocs.Owner,loaded));
  Check("model-dock-concrete-to-owner-interface-reference",ReferenceEquals(loadedDocument.Owner,loadedDocs));
  Check("model-root-default-same-visible",ReferenceEquals(loaded.DefaultDockable,loadedDocs));
  Check("model-document-active-same-visible",ReferenceEquals(loadedDocs.ActiveDockable,loadedDocument));
  Check("model-document-default-same-visible",ReferenceEquals(loadedDocs.DefaultDockable,loadedDocument));
  Check("model-window-root-interface-reference",ReferenceEquals(loaded.Window!.Layout,loaded));
  Check("model-window-shared-reference",ReferenceEquals(loaded.Windows![0],loaded.Windows[1]));
  Check("model-window-parent-reference-alias",ReferenceEquals(loaded.Windows[0].ParentWindow,loaded.Window));
  Check("model-window-undefined-state-and-owner-retained",(int)loaded.Windows[0].WindowState==123&&(int)loaded.Windows[0].OwnerMode==-19);
  Check("model-window-presentation-flags-retained",loaded.Windows[0].Topmost&&loaded.Windows[0].IsModal&&loaded.Windows[0].ShowInTaskbar==false&&loaded.Window.ShowInTaskbar is null);
  Check("model-root-undefined-dock-mode",(int)loaded.Dock==-19);
  Check("model-root-undefined-pinned-and-host-mode",(int)loaded.PinnedDockDisplayMode==123&&(int)loaded.FloatingWindowHostMode==-17);
  Check("model-dock-undefined-signed-boundary",(int)loadedDocs.Dock==int.MinValue);
  Check("model-document-undefined-dock-mode",(int)loadedDocument.Dock==123);
  Check("model-pinned-override-null-and-unknown",loaded.PinnedDockDisplayModeOverride is null&&loadedDocs.PinnedDockDisplayModeOverride==PinnedDockDisplayMode.Inline&&(int?)loadedDocument.PinnedDockDisplayModeOverride==int.MinValue);
  Check("model-default-nan-proportions-and-constraints",double.IsNaN(loaded.Proportion)&&double.IsNaN(loaded.MinWidth)&&double.IsNaN(loadedDocument.MaxHeight));
  Check("model-finite-proportion-and-collapsed-values",loadedDocument.Proportion==0.125&&loadedDocs.CollapsedProportion==0.25&&loadedDocument.MinWidth==128.5);
  Check("model-named-infinite-size",double.IsPositiveInfinity(loadedDocument.MaxWidth));
  Check("model-negative-zero-size",BitConverter.DoubleToInt64Bits(loadedDocument.MinHeight)==long.MinValue);
  // The source setter considers its initial +0 equal to incoming -0 and keeps +0.
  Check("model-window-coordinate-and-size-wire",BitConverter.DoubleToInt64Bits(loaded.Windows[0].X)==0&&loaded.Windows[0].Y==1e-5&&loaded.Windows[0].Width==1024.25&&loaded.Windows[0].Height==768);
  Check("model-observable-dockable-list",loaded.VisibleDockables is ObservableCollection<IDockable>);
  Check("model-observable-window-list",loaded.Windows is ObservableCollection<IDockWindow>);
  var plain=Serializer(typeof(List<>)).Deserialize<RootDock>(text)!;
  Check("model-configured-plain-dockable-list",plain.VisibleDockables is List<IDockable>);
  Check("model-configured-plain-window-list",plain.Windows is List<IDockWindow>);
  Check("model-policy-alias-reference",ReferenceEquals(loaded.RootDockCapabilityPolicy,loadedDocs.DockCapabilityPolicy));
  Check("model-docking-state-numeric-flags",loadedDocument.DockingState==DockingWindowState.Document);
  Check("model-undefined-operation-mask-retained",(int)loadedDocument.AllowedDropOperations==unchecked((int)0x80000001));
  Check("model-masks-use-original-operation-rules",loadedDocument.AllowedDockOperations.Allows(DockOperation.OuterLeft)&&!loadedDocument.AllowedDockOperations.Allows(DockOperation.Right)&&loadedDocument.AllowedDropOperations.Allows(DockOperation.Fill)&&!loadedDocument.AllowedDropOperations.Allows(DockOperation.Window));
  Check("model-pinned-bounds-original-accessors",loadedDocument.PinnedBounds is DockRect pinned&&BitConverter.DoubleToInt64Bits(pinned.X)==long.MinValue&&pinned.Y==0.125&&pinned.Width==512.5&&pinned.Height==256.25);
  Check("model-override-values-and-ignored-computed-state",loadedDocument.DockCapabilityOverrides is {CanClose:true,CanFloat:false,HasAnyOverride:true}&&!text.Contains("HasAnyOverride"));
  factory.InitLayout(loaded);Check("model-original-engine-initializes-deserialized-layout",ReferenceEquals(loadedDocument.Factory,factory)&&ReferenceEquals(loadedDocument.Owner,loadedDocs));
  Check("model-original-engine-retains-dock-and-pinned-modes",(int)loadedDocument.Dock==123&&(int?)loadedDocument.PinnedDockDisplayModeOverride==int.MinValue&&(int)loaded.FloatingWindowHostMode==-17);
  var capability=DockCapabilityResolver.Evaluate(loadedDocument,DockCapability.Close);
  Check("model-restored-original-capability-precedence",capability.EffectiveValue&&capability.RootPolicyValue==false&&capability.DockPolicyValue==false&&capability.DockableOverrideValue==true&&capability.EffectiveSource==DockCapabilityValueSource.DockableOverride);
  loaded.RootDockCapabilityPolicy!.CanPin=false;
  Check("model-policy-mutation-same-original-instance",loadedDocs.DockCapabilityPolicy!.CanPin==false&&!DockCapabilityResolver.IsEnabled(loadedDocument,DockCapability.Pin,loadedDocs));
  loaded.RootDockCapabilityPolicy.CanPin=true;
  var list=new List<IDockable>{document,null!};var holder=new JsonListHolder{A=list,B=list,Item=document};
  var holderText=serializer.Serialize(holder);File.WriteAllText(Path.ChangeExtension(path,".list.json"),holderText);
  var held=serializer.Deserialize<JsonListHolder>(holderText)!;
  Check("model-list-alias-reference",ReferenceEquals(held.A,held.B));
  Check("model-list-item-alias-reference",ReferenceEquals(held.A![0],held.Item));
  Check("model-list-null-entry-retained",held.A!.Count==2&&held.A[1]is null);
  var custom=Serializer(typeof(CountedList<>)).Deserialize<JsonListHolder>(holderText)!;
  Check("model-custom-list-constructor",custom.A is CountedList<IDockable>);
  Check("model-custom-list-alias-same-constructor-instance",ReferenceEquals(custom.A,custom.B));
  var direct=serializer.Serialize<IDockable>(document);var restored=serializer.Deserialize<IDockable>(direct);
  Check("model-interface-root-actual-concrete-class",restored is Document);
  Check("model-interface-root-owner-cycle",restored is Document d&&ReferenceEquals(((DocumentDock)d.Owner!).VisibleDockables![0],d));
  bool Failure(string json){try{serializer.Deserialize<IDockable>(json);return false;}catch(NotSupportedException){return true;}catch(JsonException){return true;}}
  Check("model-unknown-discriminator-interface-not-fabricated",Failure("{\"$type\":\"future\",\"Id\":\"unknown\"}"));
  Check("model-late-discriminator-rejected",Failure("{\"Id\":\"x\",\"$type\":\"Dock.Model.ReactiveUI.Controls.Document\"}"));
  Check("model-duplicate-discriminator-rejected",Failure("{\"$type\":\"x\",\"$type\":\"y\"}"));
  WeakReference<DockWindow> SetWindow(RootDock target){var value=new DockWindow();target.Window=value;return new WeakReference<DockWindow>(value);}
  WeakReference<DocumentDock> SetOwner(Document target){var value=new DocumentDock();target.Owner=value;return new WeakReference<DocumentDock>(value);}
  WeakReference<DockWindow> SetParent(DockWindow target){var value=new DockWindow();target.ParentWindow=value;return new WeakReference<DockWindow>(value);}
  var retainedRoot=new RootDock();var retainedDoc=new Document();var retainedWindow=new DockWindow();
  var windowRef=SetWindow(retainedRoot);var ownerRef=SetOwner(retainedDoc);var parentRef=SetParent(retainedWindow);
  GC.Collect();GC.WaitForPendingFinalizers();GC.Collect();
  Check("model-window-source-owns-reference",windowRef.TryGetTarget(out var keptWindow)&&ReferenceEquals(keptWindow,retainedRoot.Window));
  Check("model-owner-source-owns-reference",ownerRef.TryGetTarget(out var keptOwner)&&ReferenceEquals(keptOwner,retainedDoc.Owner));
  Check("model-parent-window-source-owns-reference",parentRef.TryGetTarget(out var keptParent)&&ReferenceEquals(keptParent,retainedWindow.ParentWindow));
  retainedRoot.Window=null;retainedDoc.Owner=null;retainedWindow.ParentWindow=null;
  Check("model-source-reference-clear-is-explicit",retainedRoot.Window is null&&retainedDoc.Owner is null&&retainedWindow.ParentWindow is null);
  GC.KeepAlive(retainedRoot);GC.KeepAlive(retainedDoc);GC.KeepAlive(retainedWindow);
  Console.WriteLine("Actual model projection and reference assertions passed.");
 }
}
public sealed class JsonListHolder{public IList<IDockable>? A{get;set;}public IList<IDockable>? B{get;set;}public IDockable? Item{get;set;}}
public sealed class CountedList<T>:List<T>{}
