// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
using Dock.Serializer.SystemTextJson;

internal static class AccessorFailureReference
{
    internal static void Run(string path)
    {
        var rows = new List<object>();
        foreach (var name in new[] {
            "get-error", "set-error", "invalid-token", "read-converter-error",
            "write-converter-error", "get-error-converter", "readonly-skip",
            "readonly-skip-converter", "writeonly-skip", "writeonly-skip-converter",
            "writeonly-set-error", "writeonly-set-error-converter",
            "readonly-get-error", "readonly-get-error-converter", "persistent-getter",
            "constructor-same-instance", "persistent-error"
        })
        {
            var model = new AccessorFailureModel(); var trace = new List<string>();
            var failure = new InvalidOperationException("original-accessor-failure");
            bool converted = name.Contains("converter"), readOnly = name.StartsWith("readonly"), writeOnly = name.StartsWith("writeonly");
            bool reading = name.StartsWith("set-") || name == "invalid-token" || name.StartsWith("read-converter") || name.Contains("-skip") && readOnly || name.Contains("set-error") || name=="constructor-same-instance";
            bool failGet = name.Contains("get-error") || name=="persistent-error", failSet = name.Contains("set-error");
            var resolver = new DefaultJsonTypeInfoResolver();
            resolver.Modifiers.Add(info => {
                if (info.Type != typeof(AccessorFailureModel)) return;
                info.CreateObject = () => { trace.Add("create"); if(name=="constructor-same-instance") model.Values[0]++;return model; };
                info.Properties.Clear();
                foreach (string member in new[] { "A", "B", "C" }) {
                    int sequence=0;
                    var property = info.CreateJsonPropertyInfo(typeof(int), member);
                    if (!(member == "B" && writeOnly)) property.Get = value => {
                        trace.Add("get:" + member);
                        if (member == "B" && failGet) throw failure;
                        if (member=="B" && name=="persistent-getter") return ++sequence;
                        return ((AccessorFailureModel)value).Values[member[0] - 'A'];
                    };
                    if (!(member == "B" && readOnly)) property.Set = (value, item) => {
                        trace.Add("set:" + member);
                        ((AccessorFailureModel)value).Values[member[0] - 'A'] = (int)item!;
                        if (member == "B" && failSet) throw failure;
                    };
                    if (member == "B" && converted) property.CustomConverter = new AccessorIntConverter(trace, failure, name);
                    info.Properties.Add(property);
                }
            });
            var serializer = new DockSerializer(resolver);
            bool success = true, sameError = false; string? wire = null;
            try {
                if(name=="constructor-same-instance") {serializer.Deserialize<AccessorFailureModel>("{\"B\":2}");serializer.Deserialize<AccessorFailureModel>("{\"C\":3}");}
                else if (reading) serializer.Deserialize<AccessorFailureModel>(readOnly ? "{\"A\":1,\"B\":{\"ignored\":true},\"C\":3}" : name == "invalid-token" ? "{\"A\":1,\"B\":\"bad\",\"C\":3}" : "{\"A\":1,\"B\":2,\"C\":3}");
                else if(name=="persistent-error") {try {serializer.Serialize(model);}catch(Exception error){if(!ReferenceEquals(error,failure))throw;}wire=serializer.Serialize(model);}
                else {if(name=="persistent-getter") serializer.Serialize(model);wire = serializer.Serialize(model);}
            } catch (Exception error) { success = false; sameError = ReferenceEquals(error, failure); }
            rows.Add(new { Name = name, Success = success, SameError = sameError, Trace = trace, Values = model.Values, Wire = wire });
        }
        File.WriteAllText(path, JsonSerializer.Serialize(rows, new JsonSerializerOptions { WriteIndented = true }));
    }
}
public sealed class AccessorFailureModel { public readonly int[] Values = new int[3]; }
internal sealed class AccessorIntConverter(List<string> trace, Exception failure, string name) : JsonConverter<int>
{
    public override int Read(ref Utf8JsonReader reader, Type type, JsonSerializerOptions options) {
        trace.Add("convert:read:B"); if (name == "read-converter-error") throw failure; return reader.GetInt32();
    }
    public override void Write(Utf8JsonWriter writer, int value, JsonSerializerOptions options) {
        trace.Add("convert:write:B"); if (name == "write-converter-error") throw failure; writer.WriteNumberValue(value);
    }
}
