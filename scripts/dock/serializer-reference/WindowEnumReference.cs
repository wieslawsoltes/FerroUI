// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Globalization;
using System.Text.Json;
using Dock.Model.Core;
using Dock.Model.ReactiveUI.Core;
using Dock.Serializer.SystemTextJson;

internal static class WindowEnumReference
{
    internal static void Run(string prefix)
    {
        var serializer = new DockSerializer();
        using var checks = new StreamWriter(prefix + ".tsv");
        void Check(string name, bool value)
        {
            if (!value) throw new InvalidOperationException(name);
            checks.WriteLine(name + "\t1");
        }
        var options = new DockWindowOptions { OwnerMode = (DockWindowOwnerMode)(-19), IsModal = true, ShowInTaskbar = false };
        var optionsText = serializer.Serialize(options);
        File.WriteAllText(prefix + ".options.json", optionsText);
        var read = serializer.Deserialize<DockWindowOptions>(optionsText)!;
        Check("window-options-undefined-owner-value", (int)read.OwnerMode == -19);
        Check("window-options-original-constructor-and-flags", read.ParentWindow is null && read.IsModal && read.ShowInTaskbar == false);
        var window = new DockWindow();
        read.ApplyTo(window);
        Check("window-options-apply-original-unknown-owner", (int)window.OwnerMode == -19 && window.IsModal && window.ShowInTaskbar == false);
        var defaults = serializer.Deserialize<DockWindowOptions>("{}")!;
        Check("window-options-source-default-values", defaults.OwnerMode == DockWindowOwnerMode.Default && defaults.ParentWindow is null && !defaults.IsModal && defaults.ShowInTaskbar is null);
        Check("window-enum-unknown-values-have-numeric-names", ((DockWindowState)(-19)).ToString() == "-19" && ((DockWindowOwnerMode)123).ToString() == "123");
        var parent = new DockWindow { Id = "parent", Title = "Parent", WindowState = (DockWindowState)(-17), OwnerMode = (DockWindowOwnerMode)123, ShowInTaskbar = true };
        options.ParentWindow = parent;
        var pairText = serializer.Serialize(new WindowOptionsPair { A = options, B = options, Window = parent });
        File.WriteAllText(prefix + ".pair.json",pairText);
        var pair = serializer.Deserialize<WindowOptionsPair>(pairText)!;
        Check("window-options-shared-options-and-parent",ReferenceEquals(pair.A,pair.B)&&ReferenceEquals(pair.A!.ParentWindow,pair.Window));
        pair.A!.ApplyTo(window);
        Check("window-options-apply-retains-original-parent",ReferenceEquals(window.ParentWindow,pair.Window)&&(int)window.OwnerMode==-19);

        using var values = new StreamWriter(prefix + ".values.tsv");
        void Value(int raw)
        {
            var owner = (DockWindowOwnerMode)raw;
            var state = (DockWindowState)raw;
            values.WriteLine("OWNER\t" + raw.ToString(CultureInfo.InvariantCulture) + "\t" + owner + "\t" + serializer.Serialize(owner));
            values.WriteLine("STATE\t" + raw.ToString(CultureInfo.InvariantCulture) + "\t" + state + "\t" + serializer.Serialize(state));
            if ((int)serializer.Deserialize<DockWindowOwnerMode>(serializer.Serialize(owner)) != raw || (int)serializer.Deserialize<DockWindowState>(serializer.Serialize(state)) != raw) throw new InvalidOperationException("window enum roundtrip");
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
        foreach (var json in new[] { "0", "-0", "-19", "123", "2147483647", "-2147483648", "2147483648", "-2147483649", "1.0", "1e0", "null", "true", "[]", "{}", "\"1\"", "\"Default\"", "\"FullScreen\"" })
        {
            Read<DockWindowOwnerMode>("OWNER", json);
            Read<DockWindowState>("STATE", json);
        }
        foreach (var json in new[] { "{}", "null", "[]", "{\"OwnerMode\":-19}", "{\"OwnerMode\":null}", "{\"OwnerMode\":\"Default\"}", "{\"IsModal\":true,\"ShowInTaskbar\":false}", "{\"ParentWindow\":null}", "{\"ShowInTaskbar\":true,\"ShowInTaskbar\":null}", "{\"$id\":\"a\",\"OwnerMode\":123}", "{\"$other\":1}", "{\"$ref\":\"missing\"}" }) Read<DockWindowOptions>("OPTIONS", json);
        Read<DockWindowOwnerMode?>("OPTION_OWNER", "null");
        Read<DockWindowOwnerMode?>("OPTION_OWNER", "-19");
        Read<DockWindowState?>("OPTION_STATE", "null");
        Read<DockWindowState?>("OPTION_STATE", "123");
        File.WriteAllText(prefix + ".reads.json", JsonSerializer.Serialize(rows, new JsonSerializerOptions { WriteIndented = true }));
    }
}
public sealed class WindowOptionsPair
{
    public DockWindowOptions? A { get; set; }
    public DockWindowOptions? B { get; set; }
    public IDockWindow? Window { get; set; }
}
