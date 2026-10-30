// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Globalization;
using System.Text.Json;
using Dock.Model.Core;
using Dock.Model.ReactiveUI.Controls;
using Dock.Serializer.SystemTextJson;

internal static class DockEnumReference
{
    internal static void Run(string prefix)
    {
        var serializer = new DockSerializer();
        using var values = new StreamWriter(prefix + ".values.tsv");
        void Value(int raw)
        {
            void Write<T>(string kind, T value)
            {
                var text = serializer.Serialize(value);
                values.WriteLine(kind + "\t" + raw.ToString(CultureInfo.InvariantCulture) + "\t" + value + "\t" + text);
                if (!Equals(serializer.Deserialize<T>(text), value)) throw new InvalidOperationException(kind);
            }
            Write("DOCK", (DockMode)raw);
            Write("PINNED", (PinnedDockDisplayMode)raw);
            Write("HOST", (DockFloatingWindowHostMode)raw);
        }
        foreach (var raw in new[] { int.MinValue, -19, -1, 0, 1, 2, 3, 4, 5, 9, 123, 999, int.MaxValue }) Value(raw);
        uint seed = 0x7fac2951;
        for (int i = 0; i < 128; ++i)
        {
            seed = unchecked(seed * 1664525 + 1013904223);
            Value(unchecked((int)seed));
        }
        var rows = new List<string[]>();
        void Read<T>(string kind, string json)
        {
            try
            {
                var value = serializer.Deserialize<T>(json);
                rows.Add(new[] { kind, json, "OK", serializer.Serialize(value) });
            }
            catch (Exception e) { rows.Add(new[] { kind, json, "ERR", e.GetType().Name }); }
        }
        foreach (var json in new[] { "0", "-0", "-19", "123", "2147483647", "-2147483648", "2147483648", "-2147483649", "1.0", "1e0", "null", "true", "[]", "{}", "\"1\"", "\"Default\"", "\"Overlay\"" })
        {
            Read<DockMode>("DOCK", json); Read<PinnedDockDisplayMode>("PINNED", json); Read<DockFloatingWindowHostMode>("HOST", json);
            Read<DockMode?>("OPTION_DOCK", json); Read<PinnedDockDisplayMode?>("OPTION_PINNED", json); Read<DockFloatingWindowHostMode?>("OPTION_HOST", json);
        }
        File.WriteAllText(prefix + ".reads.json", JsonSerializer.Serialize(rows, new JsonSerializerOptions { WriteIndented = true }));
    }
}
