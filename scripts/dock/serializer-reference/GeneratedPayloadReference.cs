// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Text.Json;
using System.Text.Json.Serialization;
using Dock.Serializer.SystemTextJson;

[assembly: DockJsonSourceGeneration]
[assembly: DockJsonSerializable(typeof(GeneratedHolder))]
[assembly: DockJsonSerializable(typeof(GeneratedPayload))]
[assembly: DockJsonSerializable(typeof(GeneratedGeneric<int>))]
[assembly: DockJsonSerializable(typeof(GeneratedGeneric<string>))]
[assembly: DockJsonSerializable(typeof(GeneratedTypeNamed))]
[assembly: DockJsonSerializable(typeof(GeneratedCustomHolder))]
[assembly: DockJsonSerializable(typeof(GeneratedReadonlyHolder))]
[assembly: DockJsonSerializable(typeof(GeneratedArrayPayload))]
[assembly: DockJsonSerializable(typeof(GeneratedWriteOnlyHolder))]

internal static class GeneratedPayloadReference
{
    internal static void Run(string path)
    {
        var serializer = DockSystemTextJsonGenerated.CreateSerializer();
        using var output = new StreamWriter(path + ".tsv");
        void Check(string name, bool ok) { if (!ok) throw new InvalidOperationException(name); output.WriteLine(name + "\t1"); }
        var payload = new GeneratedPayload { Name = "alpha" }; payload.Self = payload;
        var holder = new GeneratedHolder { Content = payload, Other = payload, Typed = payload };
        var text = serializer.Serialize(holder);
        File.WriteAllText(path + ".holder.json", text);
        var loaded = serializer.Deserialize<GeneratedHolder>(text)!;
        var first = (GeneratedPayload)loaded.Content!; var second = (GeneratedPayload)loaded.Other!;
        Check("generated-payload-actual-class", first.Name == "alpha");
        Check("generated-payload-inner-self-reference", ReferenceEquals(first, first.Self) && ReferenceEquals(second, second.Self));
        Check("generated-payload-scopes-are-independent", !ReferenceEquals(first, second) && !ReferenceEquals(first, loaded.Typed) && !ReferenceEquals(second, loaded.Typed));
        Check("generated-payload-outer-typed-self-reference", ReferenceEquals(loaded.Typed, loaded.Typed!.Self));
        using (var json = JsonDocument.Parse(text)) {
            var root = json.RootElement;
            Check("generated-payload-tag-before-inner-id", root.GetProperty("Content").EnumerateObject().First().Name == "$type");
            Check("generated-payload-fresh-reference-ids", root.GetProperty("Content").GetProperty("$id").GetString() == "1" && root.GetProperty("Other").GetProperty("$id").GetString() == "1" && root.GetProperty("Typed").GetProperty("$id").GetString() == "2");
        }
        var typed = new GeneratedHolder { Content = new GeneratedGeneric<int> { Value = 37 }, Other = new GeneratedGeneric<string> { Value = "text" } };
        text = serializer.Serialize(typed); File.WriteAllText(path + ".generic.json", text);
        loaded = serializer.Deserialize<GeneratedHolder>(text)!;
        Check("generated-payload-closed-generic-types", loaded.Content is GeneratedGeneric<int> { Value: 37 } && loaded.Other is GeneratedGeneric<string> { Value: "text" });
        var custom = new GeneratedCustomHolder { Content = "override" };
        text = serializer.Serialize(custom); File.WriteAllText(path + ".custom.json", text);
        Check("generated-payload-explicit-converter-wins", serializer.Deserialize<GeneratedCustomHolder>(text)!.Content is string value && value == "override");
        text = serializer.Serialize(new GeneratedReadonlyHolder()); File.WriteAllText(path + ".readonly.json", text);
        Check("generated-payload-readonly-included", JsonDocument.Parse(text).RootElement.GetProperty("Content").GetProperty("Name").GetString() == "readonly");
        Check("generated-payload-readonly-input-ignored", ((GeneratedPayload)serializer.Deserialize<GeneratedReadonlyHolder>("{\"Content\":{\"$type\":\"GeneratedPayload\",\"Name\":false}}")!.Content!).Name == "readonly");
        text = serializer.Serialize(new GeneratedHolder { Content = new GeneratedTypeNamed { Kind = "payload-type", Name = "named" } });
        File.WriteAllText(path + ".duplicate.json", text);
        loaded = serializer.Deserialize<GeneratedHolder>(text)!;
        Check("generated-payload-duplicate-tag-is-raw", loaded.Content is JsonElement e && e.EnumerateObject().Count(p => p.Name == "$type") == 2);
        Check("generated-payload-duplicate-tag-replay", serializer.Serialize(loaded) == text);
        bool Unsupported(object value) { try { serializer.Serialize(new GeneratedHolder { Content = value }); return false; } catch (NotSupportedException) { return true; } }
        Check("generated-payload-unregistered-class-rejected", Unsupported(new UnregisteredPayload()));
        Check("generated-payload-unregistered-primitive-rejected", Unsupported("primitive"));
        Check("generated-payload-nonobject-converter-rejected", Unsupported(new GeneratedArrayPayload()));
        using (var raw = JsonDocument.Parse("null")) {
            text = serializer.Serialize(new GeneratedHolder { Content = raw.RootElement.Clone() }); File.WriteAllText(path + ".null.json", text);
            Check("generated-payload-boxed-raw-null-is-written", JsonDocument.Parse(text).RootElement.GetProperty("Content").ValueKind == JsonValueKind.Null);
        }
        text = serializer.Serialize(new GeneratedTypeNamed { Kind = "payload-type", Name = "named" }); File.WriteAllText(path + ".named-root.json", text);
        bool rejected = false; try { serializer.Deserialize<GeneratedTypeNamed>(text); } catch (JsonException) { rejected = true; }
        Check("generated-concrete-type-named-metadata-rejected", rejected);
        var writeonly = serializer.Deserialize<GeneratedWriteOnlyHolder>("{\"Content\":{\"$type\":\"GeneratedPayload\",\"Name\":\"writeonly\"},\"Number\":37}")!;
        Check("generated-set-only-converter-reads-actual-payload",writeonly.Stored is GeneratedPayload p && p.Name=="writeonly");
        Check("generated-set-only-plain-property-reads",writeonly.StoredNumber==37);
        text=serializer.Serialize(writeonly);File.WriteAllText(path+".writeonly.json",text);
        Check("generated-set-only-properties-omitted-on-write",JsonDocument.Parse(text).RootElement.EnumerateObject().Count()==1);
        var reads = new List<object>();
        var tag = JsonSerializer.Serialize(typeof(GeneratedPayload).FullName);
        foreach (string content in new[] {
            "null", "true", "1e+02", "\"plain\"", "[1,null,{\"x\":2}]", "{}", "{\"$ref\":\"outer\"}",
            "{\"$type\":\"unknown\",\"Name\":\"x\"}", "{\"$type\":\"  \",\"Name\":\"x\"}", "{\"$type\":3,\"Name\":\"x\"}",
            "{\"$type\":" + tag + ",\"Name\":\"known\"}", "{\"Name\":\"late\",\"$type\":" + tag + "}",
            "{\"$id\":\"a\",\"$type\":" + tag + ",\"Name\":\"self\",\"Self\":{\"$ref\":\"a\"}}",
            "{\"$type\":\"unknown\",\"$type\":" + tag + ",\"Name\":\"duplicate\"}",
            "{\"$type\":" + tag + ",\"$type\":false,\"Name\":\"nonstring\"}",
            "{\"$type\":" + tag + ",\"Name\":false}", "{\"Name\":\"lateid\",\"$id\":\"a\",\"$type\":" + tag + "}",
            "{\"$type\":" + tag + ",\"$foo\":1}", "{\"$type\":" + tag + ",\"$values\":[]}",
            "{\"$type\":" + tag + ",\"$id\":\"a\",\"$ref\":\"a\"}", "{\"$type\":" + tag + ",\"$Type\":1}",
            "{\"$type\":" + tag + ",\"$$\":1}", "{\"$foo\":1}",
            "{\"$type\":\"\",\"$type\":" + tag + ",\"Name\":\"two\"}",
            "{\"$type\":false,\"$type\":" + tag + ",\"Name\":\"nonstring-first\"}"
        }) {
            var input = "{\"Content\":" + content + "}";
            string kind, replay = "";
            try { loaded = serializer.Deserialize<GeneratedHolder>(input)!; kind = loaded.Content switch { null => "null", GeneratedPayload => "payload", JsonElement => "raw", _ => throw new InvalidOperationException("unexpected payload") }; replay = serializer.Serialize(loaded); }
            catch (JsonException) { kind = "error-json"; }
            reads.Add(new { Input = input, Kind = kind, Replay = replay });
        }
        File.WriteAllText(path + ".read.json", JsonSerializer.Serialize(reads, new JsonSerializerOptions { WriteIndented = true }));
        Console.WriteLine("Original generated payload helper cases passed.");
    }
}
public sealed class GeneratedPayload { public string? Name { get; set; } public GeneratedPayload? Self { get; set; } }
public sealed class GeneratedGeneric<T> { public T Value { get; set; } = default!; }
public sealed class GeneratedTypeNamed { [JsonPropertyName("$type")] public string? Kind { get; set; } public string? Name { get; set; } }
public sealed class GeneratedHolder { public object? Content { get; set; } public object? Other { get; set; } public GeneratedPayload? Typed { get; set; } }
public sealed class GeneratedCustomHolder { [JsonConverter(typeof(GeneratedStringConverter))] public object? Content { get; set; } }
public sealed class GeneratedReadonlyHolder { public object? Content { get; } = new GeneratedPayload { Name = "readonly" }; }
public sealed class GeneratedWriteOnlyHolder { public object? Content {set=>Stored=value;} public int Number {set=>StoredNumber=value;} [JsonIgnore] public object? Stored {get;private set;} [JsonIgnore] public int StoredNumber {get;private set;} }
public sealed class UnregisteredPayload { }
[JsonConverter(typeof(GeneratedArrayConverter))]
public sealed class GeneratedArrayPayload { }
public sealed class GeneratedArrayConverter : JsonConverter<GeneratedArrayPayload>
{
    public override GeneratedArrayPayload Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) { using var document = JsonDocument.ParseValue(ref reader); return new(); }
    public override void Write(Utf8JsonWriter writer, GeneratedArrayPayload value, JsonSerializerOptions options) { writer.WriteStartArray(); writer.WriteNumberValue(1); writer.WriteEndArray(); }
}
public sealed class GeneratedStringConverter : JsonConverter<object?>
{
    public override object? Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options) => reader.GetString();
    public override void Write(Utf8JsonWriter writer, object? value, JsonSerializerOptions options) => writer.WriteStringValue((string?)value);
}
