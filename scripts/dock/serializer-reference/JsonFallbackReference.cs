// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
using Dock.Serializer.SystemTextJson;

internal static class JsonFallbackReference
{
    internal static void Run(string path)
    {
        var resolver = new DefaultJsonTypeInfoResolver();
        resolver.Modifiers.Add(info =>
        {
            if (info.Type == typeof(FallbackNode))
            {
                info.PolymorphismOptions = new JsonPolymorphismOptions
                {
                    UnknownDerivedTypeHandling = JsonUnknownDerivedTypeHandling.FallBackToBaseType,
                    IgnoreUnrecognizedTypeDiscriminators = true,
                    DerivedTypes = { new JsonDerivedType(typeof(RegisteredNode), "registered") }
                };
            }
        });
        var serializer = new DockSerializer(resolver);
        using var output = new StreamWriter(path);
        foreach (int length in new[] { 1, 63, 64, 65, 100 })
        {
            FallbackNode? root = null;
            for (int i = 0; i < length; i++) root = new UnregisteredNode { Next = root };
            FallbackNode.Reads = 0;
            bool success;
            try { serializer.Serialize<FallbackNode>(root!); success = true; }
            catch (JsonException) { success = false; }
            output.WriteLine($"depth\t{length}\t{(success ? 1 : 0)}\t{FallbackNode.Reads}");
        }
        var original = new UnregisteredNode();
        original.Next = original;
        var text = serializer.Serialize<FallbackNode>(original);
        File.WriteAllText(Path.ChangeExtension(path, ".json"), text);
        var loaded = serializer.Deserialize<FallbackNode>(text)!;
        if (loaded.GetType() != typeof(FallbackNode) || !ReferenceEquals(loaded, loaded.Next))
            throw new InvalidOperationException("Source fallback identity");
        output.WriteLine("base-self-reference\t1");
        loaded = serializer.Deserialize<FallbackNode>("{\"$type\":\"future\",\"Next\":null}")!;
        if (loaded.GetType() != typeof(FallbackNode) || loaded.Next is not null)
            throw new InvalidOperationException("Source unrecognized fallback");
        output.WriteLine("unrecognized-to-base\t1");
    }
}

public class FallbackNode
{
    public static int Reads;
    private FallbackNode? _next;
    public FallbackNode? Next { get { Reads++; return _next; } set { _next = value; } }
}
public sealed class RegisteredNode : FallbackNode { }
public sealed class UnregisteredNode : FallbackNode { }
