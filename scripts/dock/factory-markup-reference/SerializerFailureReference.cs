// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System;
using System.IO;
using Dock.Model;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;
namespace DockFactoryMarkupReference;
internal static class SerializerFailureReference{
 internal static void Run(Action<string,bool> check){
  var serializer=new Probe();var factory=new Factory();var root=factory.CreateRootDock();serializer.Value=root;var manager=new DockWorkspaceManager(serializer);
  var workspace=manager.Capture("A",root,false);manager.TrackFactory(factory);factory.OnDockableMoved(null);
  var failure=new InvalidOperationException("source serializer failure");serializer.Failure=failure;
  Exception? Capture(Action action){try{action();return null;}catch(Exception e){return e;}}
  check("serializer-result-capture-source-error-identity",ReferenceEquals(Capture(()=>manager.Capture("A",root,false)),failure));
  check("serializer-result-restore-source-error-identity",ReferenceEquals(Capture(()=>manager.Restore(workspace)),failure));
  check("serializer-result-lookup-source-error-identity",ReferenceEquals(Capture(()=>manager.TryRestore("A",out _)),failure));
  check("serializer-result-payload-unmodified",workspace.Layout=="snapshot-1");
  check("serializer-result-dirty-unmodified",manager.IsDirty&&workspace.IsDirty);
  check("serializer-result-active-unmodified",ReferenceEquals(manager.ActiveWorkspace,workspace));
  check("serializer-result-missing-lookup-skips-serializer",!manager.TryRestore("missing",out _));
 }
 private sealed class Probe:IDockSerializer{
  internal IDock? Value;internal Exception? Failure;
  public string Serialize<T>(T value){if(Failure is not null)throw Failure;return "snapshot-1";}
  public T? Deserialize<T>(string text){if(Failure is not null)throw Failure;return (T?)(object?)Value;}
  public T? Load<T>(Stream stream)=>throw new InvalidOperationException("unused");
  public void Save<T>(Stream stream,T value)=>throw new InvalidOperationException("unused");
 }
}
