// Source-model application event owner for the original XAML compiler boundary.
namespace DockFactoryMarkupReference;
public sealed class FactoryEventConsumer {
 public Dock.Model.ReactiveUI.Factory? Factory {get;set;}
 public int Calls {get;private set;}
 public object? Sender {get;private set;}
 public void OnClosing(object? sender,Dock.Model.Core.Events.DockableClosingEventArgs args){Calls++;Sender=sender;args.Cancel=true;}
}
