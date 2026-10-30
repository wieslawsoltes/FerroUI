// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Globalization;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
using Dock.Model.Core;
using Dock.Serializer.SystemTextJson;

internal static class CoreValueReference
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
        var point = new DockPoint(-0d, double.PositiveInfinity);
        var pointText = serializer.Serialize(point);
        File.WriteAllText(prefix + ".point.json", pointText);
        var readPoint = serializer.Deserialize<DockPoint>(pointText);
        Check("point-readonly-source-default-construction", readPoint.X == 0 && readPoint.Y == 0);
        Check("point-write-getters-no-reference-id", !pointText.Contains("$id") && pointText.Contains("Infinity"));
        Check("point-readonly-input-values-skipped", serializer.Deserialize<DockPoint>("{\"X\":null,\"Y\":{\"not\":\"a number\"}}").Y == 0);
        var options = new DockManagerOptions();
        Check("manager-read-constructor-retains-enabled-default", serializer.Deserialize<DockManagerOptions>("{}")!.IsDockingEnabled);
        options.IsDockingEnabled = false;
        File.WriteAllText(prefix + ".manager.json", serializer.Serialize(options));
        var holder = new CoreValueHolder
        {
            Options = options, OtherOptions = options, Point = point,
            State = (DockingWindowState)unchecked((int)0x80000004),
            Mask = (DockOperationMask)(-1), Action = (DragAction)int.MaxValue
        };
        var holderText = serializer.Serialize(holder);
        File.WriteAllText(prefix + ".holder.json", holderText);
        var read = serializer.Deserialize<CoreValueHolder>(holderText)!;
        File.WriteAllText(prefix + ".holder.roundtrip.json", serializer.Serialize(read));
        Check("manager-shared-original-instance", ReferenceEquals(read.Options, read.OtherOptions));
        read.Options!.IsDockingEnabled = true;
        Check("manager-shared-original-mutation", read.OtherOptions!.IsDockingEnabled);
        Check("flags-undefined-signed-bits-retained", (int)read.State == unchecked((int)0x80000004) && (int)read.Mask == -1 && (int)read.Action == int.MaxValue);
        Check("flags-default-format-is-numeric", serializer.Serialize(DockOperationMask.Fill | DockOperationMask.Left) == "3");
        Check("nullable-point-source-null", serializer.Deserialize<DockPoint?>("null") is null);
        var custom = new DockSerializer(new CoreValueIntResolver());
        Check("flags-independent-of-int32-codec",custom.Serialize(42)=="\"custom-int32\""&&custom.Deserialize<int>("42")==123&&custom.Serialize((DockOperationMask)(-1))=="-1"&&(int)custom.Deserialize<DockOperationMask>("-1")==-1);

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
        foreach (var json in new[] { "{}", "null", "[]", "17", "{\"X\":-0,\"Y\":1.25}", "{\"X\":null}", "{\"X\":\"bad\"}", "{\"Y\":{\"not\":\"a number\"}}", "{\"x\":12}", "{\"X\":1,\"X\":2}", "{\"$id\":\"a\",\"X\":3}", "{\"$id\":17}", "{\"X\":3,\"$id\":\"a\"}", "{\"$ref\":\"a\"}", "{\"$values\":[]}", "{\"$type\":\"x\"}", "{\"$other\":1}" }) Read<DockPoint>("POINT", json);
        foreach (var json in new[] { "{}", "null", "[]", "17", "{\"IsDockingEnabled\":false}", "{\"IsDockingEnabled\":null}", "{\"IsDockingEnabled\":0}", "{\"IsDockingEnabled\":\"true\"}", "{\"IsDockingEnabled\":false,\"IsDockingEnabled\":true}", "{\"isDockingEnabled\":false}", "{\"$id\":\"a\",\"IsDockingEnabled\":false}", "{\"$other\":1}" }) Read<DockManagerOptions>("MANAGER", json);
        void Flags(string json)
        {
            Read<DockingWindowState>("STATE", json);
            Read<DockOperationMask>("MASK", json);
            Read<DragAction>("ACTION", json);
        }
        foreach (var json in new[] { "0", "-0", "1", "3", "7", "31", "63", "64", "-1", "2147483647", "-2147483648", "2147483648", "-2147483649", "1.0", "1e0", "null", "true", "[]", "{}", "\"1\"", "\"None\"", "\"Fill, Left\"", "\"NaN\"" }) Flags(json);
        uint seed = 0x19af3471;
        for (int i = 0; i < 128; ++i)
        {
            seed = unchecked(seed * 1664525 + 1013904223);
            Flags(unchecked((int)seed).ToString(CultureInfo.InvariantCulture));
        }
        Read<DockPoint?>("OPTION_POINT", "null");
        Read<DockPoint?>("OPTION_POINT", "{\"X\":7,\"Y\":8}");
        Read<DockingWindowState?>("OPTION_STATE", "null");
        Read<DockingWindowState?>("OPTION_STATE", "-1");
        Read<DockOperationMask?>("OPTION_MASK", "null");
        Read<DockOperationMask?>("OPTION_MASK", "-2147483648");
        Read<DragAction?>("OPTION_ACTION", "null");
        Read<DragAction?>("OPTION_ACTION", "2147483647");
        File.WriteAllText(prefix + ".reads.json", JsonSerializer.Serialize(rows, new JsonSerializerOptions { WriteIndented = true }));
    }
}
internal sealed class CoreValueIntConverter : JsonConverter<int>
{
    public override int Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => 123;
    public override void Write(Utf8JsonWriter writer, int value, JsonSerializerOptions options) => writer.WriteStringValue("custom-int32");
}
internal sealed class CoreValueIntResolver : IJsonTypeInfoResolver
{
    private readonly DockModelPolymorphicTypeResolver fallback = new();
    public JsonTypeInfo? GetTypeInfo(Type type, JsonSerializerOptions options) => type==typeof(int)
        ? JsonMetadataServices.CreateValueInfo<int>(options,new CoreValueIntConverter()) : fallback.GetTypeInfo(type,options);
}
public sealed class CoreValueHolder
{
    public DockManagerOptions? Options { get; set; }
    public DockManagerOptions? OtherOptions { get; set; }
    public DockPoint? Point { get; set; }
    public DockingWindowState State { get; set; }
    public DockOperationMask Mask { get; set; }
    public DragAction Action { get; set; }
}
