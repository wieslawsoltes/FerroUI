// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using Dock.Serializer.SystemTextJson;
internal static class AttributeReference
{
    internal static void Run(string path)
    {
        using var output = new StreamWriter(path);
        void Check(string name, bool value) { if (!value) throw new InvalidOperationException(name); output.WriteLine(name + "\t1"); }
        var activation = new DockJsonSourceGenerationAttribute();
        var alias = activation;
        Check("attribute-activation-shared-alias", ReferenceEquals(activation, alias));
        Check("attribute-activation-distinct-instances", !ReferenceEquals(activation, new DockJsonSourceGenerationAttribute()));
        Type type = typeof(List<int?>);
        var registration = new DockJsonSerializableAttribute(type);
        Check("attribute-registration-exact-closed-type", ReferenceEquals(registration.Type, type) && registration.Type != typeof(List<int>));
        var registrationAlias = registration;
        Check("attribute-registration-shared-alias", ReferenceEquals(registration, registrationAlias));
        Check("attribute-registration-distinct-instances", !ReferenceEquals(registration, new DockJsonSerializableAttribute(type)));
        Check("attribute-registration-null-retained", new DockJsonSerializableAttribute(null!).Type is null);
        var noConstruction = new DockJsonSerializableAttribute(typeof(ConstructorProbe));
        Check("attribute-registration-no-type-construction", noConstruction.Type == typeof(ConstructorProbe) && ConstructorProbe.Calls == 0);
        Console.WriteLine("Original attribute constructor/type cases passed.");
    }
    private sealed class ConstructorProbe
    {
        internal static int Calls;
        private ConstructorProbe() { Calls++; throw new InvalidOperationException("must not construct"); }
    }
}
