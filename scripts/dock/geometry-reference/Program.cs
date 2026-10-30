using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using Dock.Model.Core;

// Compile the actual frozen geometry sources and capture their invariant output.
// This program has no dependency on a Dock binary or a UI framework.
if (args.Length != 1)
{
    throw new ArgumentException("Pass the output TSV path.");
}

SortedSet<ulong> values = new();
if (!new DockPoint(double.NaN, -0.0).Equals(new DockPoint(double.NaN, 0.0))
    || !new DockRect(double.NaN, double.NaN, -0.0, double.PositiveInfinity).Equals(new DockRect(double.NaN, double.NaN, 0.0, double.PositiveInfinity)))
{
    throw new InvalidOperationException("The reference value-type equality contract changed.");
}

void Add(double value) => values.Add(BitConverter.DoubleToUInt64Bits(value));
foreach (double value in new[] { 0.0, -0.0, double.NaN, double.PositiveInfinity, double.NegativeInfinity, double.MaxValue, double.MinValue, double.Epsilon, -double.Epsilon })
{
    Add(value);
}

for (int exponent = -323; exponent <= 308; exponent++)
{
    double value = Math.Pow(10, exponent);
    foreach (double candidate in new[] { value, Math.BitIncrement(value), Math.BitDecrement(value) })
    {
        Add(candidate);
        Add(-candidate);
    }
}

ulong random = 0xD0C0_F3A0_2026_1020;
for (int index = 0; index < 4096; index++)
{
    unchecked
    {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
    }
    values.Add(random);
}

using StreamWriter writer = new(args[0]);
writer.WriteLine("# Dock geometry reference; source baseline 0be0ab3118c6d24be5c4d78befda881d0e539409; .NET " + Environment.Version);
writer.WriteLine("# IEEE-754 double bits (hex)\tDockPoint(value, 0).ToString() first component; invariant culture");
foreach (ulong bits in values)
{
    double value = BitConverter.UInt64BitsToDouble(bits);
    string point = new DockPoint(value, 0).ToString();
    string text = point.Substring(0, point.IndexOf(", ", StringComparison.Ordinal));
    string rectangle = new DockRect(value, 0, 0, 0).ToString();
    if (rectangle != text + ", 0, 0, 0")
    {
        throw new InvalidOperationException("The two source types format the same value differently.");
    }
    writer.WriteLine(bits.ToString("X16", CultureInfo.InvariantCulture) + "\t" + text);
}
