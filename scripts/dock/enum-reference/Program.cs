using Dock.Model.Core;
using System.Globalization;
if (args.Length != 1) throw new ArgumentException("Pass the output corpus path.");
using StreamWriter writer = new(args[0]);
writer.WriteLine("# Actual source DockingWindowState.ToString using .NET " + Environment.Version + "; bits\ttext");
void Write(int bits) => writer.WriteLine(bits.ToString(CultureInfo.InvariantCulture) + "\t" + ((DockingWindowState)bits).ToString());
for (int bits = -32; bits <= 63; bits++) Write(bits);
Write(int.MinValue);
Write(int.MaxValue);
