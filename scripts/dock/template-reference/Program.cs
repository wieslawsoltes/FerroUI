using System.Globalization;
using Avalonia.Data.Converters;
foreach (var direction in Enum.GetValues<Dock.Model.Core.GridResizeDirection>()) {
 var result=DefaultValueConverter.Instance.Convert(direction,typeof(Avalonia.Controls.GridResizeDirection),null,CultureInfo.InvariantCulture);
 Console.WriteLine($"{direction}: {result?.GetType().FullName}: {result}");
}
var template=new Avalonia.Controls.Templates.FuncDataTemplate<IDisposable>((_,_)=>null);
Console.WriteLine($"Null matches: {template.Match(null)}");
