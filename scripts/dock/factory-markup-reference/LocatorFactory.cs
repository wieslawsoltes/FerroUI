namespace DockFactoryMarkupReference;
public sealed class LocatorFactory:Dock.Model.ReactiveUI.Factory {
 public object ContextValue {get;}=new string("Located context".ToCharArray());
 public Dock.Model.Core.IHostWindow HostValue {get;}=new Dock.Avalonia.Controls.HostWindow();
 public Dock.Model.Core.IDockable DocumentValue {get;}=new Dock.Model.ReactiveUI.Controls.Document();
 public System.Func<Dock.Model.Core.IDockable,bool>? SearchPredicate {get;set;}
 public System.Func<Dock.Model.Controls.IRootDock,bool>? RootPredicate {get;set;}
 public int PredicateCalls {get;private set;}
 public bool AcceptDockable(Dock.Model.Core.IDockable value){PredicateCalls++;return value.Id=="item";}
 public bool AcceptRoot(Dock.Model.Controls.IRootDock value){PredicateCalls++;return value.Id=="root";}
 public System.Func<Dock.Model.Core.IDockable>? DocumentFactory {get;set;}
 public Dock.Model.Core.IDockable CreateDocumentValue(){DockableCalls++;return DocumentValue;}
 public int ContextCalls {get;private set;}
 public int HostCalls {get;private set;}
 public int DockableCalls {get;private set;}
 public object? CreateContext(){ContextCalls++;return ContextValue;}
 public Dock.Model.Core.IHostWindow? CreateHost(){HostCalls++;return HostValue;}
 public Dock.Model.Core.IDockable? CreateDockable(){DockableCalls++;return DocumentValue;}
}

public sealed class TransientHostFactory:Dock.Model.ReactiveUI.Factory {
 public int Reads {get;private set;}
 public override System.Func<Dock.Model.Core.IHostWindow?>? DefaultHostWindowLocator {
  get{Reads++;return Reads==1?()=>null:null;}
  set=>base.DefaultHostWindowLocator=value;
 }
}
