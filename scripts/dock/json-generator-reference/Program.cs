// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Collections.Immutable;
using System.Text.Json;
using Dock.Serializer.SystemTextJson.Generators;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;

if (args.Length != 1) throw new ArgumentException("Pass the output corpus path");
var results = new List<object>();
var prefix = "using Dock.Serializer.SystemTextJson; using Dock.Model.ReactiveUI.Controls; using Dock.Model.ReactiveUI.Core; ";
var activation = "[assembly: DockJsonSourceGeneration] ";
var cases = new (string Name, string Source, bool Missing)[] {
    ("inactive",prefix+"namespace Example { public class Doc : Document {} }",false),
    ("no-activation-order",prefix+"[assembly: DockJsonSerializable(typeof(Example.Payload<>))] [assembly: DockJsonSerializable(typeof(Example.Payload<int>))] namespace Example { public class Payload<T> {} }",false),
    ("actual-models",prefix+activation+"[assembly: DockJsonSerializable(typeof(RootDock))] [assembly: DockJsonSerializable(typeof(DocumentDock))] [assembly: DockJsonSerializable(typeof(Document))] [assembly: DockJsonSerializable(typeof(DockWindow))] namespace Example {}",false),
    ("activation-only",prefix+activation+"namespace Example { public class Ordinary {} }",false),
    ("discovery",prefix+activation+"namespace Example { public class Root : RootDock {} public class Docs : DocumentDock {} public class Tools : ToolDock {} public class Doc : Document {} internal class ToolItem : Tool {} public class Window : DockWindow {} public class Template : Dock.Model.Controls.IDocumentTemplate { public object? Content {get;set;} } public class ToolTemplate : Dock.Model.Controls.IToolTemplate {public object? Content {get;set;}} public abstract class AbstractDoc : Document {} public class Open<T> : Document {} public static class Container { protected internal class Nested : Document {} private class Hidden : Document {} } }",false),
    ("invalid-registrations",prefix+activation+"[assembly: DockJsonSerializable(typeof(int))] [assembly: DockJsonSerializable(typeof(string))] [assembly: DockJsonSerializable(typeof(int[]))] [assembly: DockJsonSerializable(typeof(Action))] [assembly: DockJsonSerializable(typeof(Example.Abstract))] [assembly: DockJsonSerializable(typeof(Example.Open<>))] [assembly: DockJsonSerializable(typeof(Dock.Model.Core.IDockable))] namespace Example { public abstract class Abstract {} public class Open<T> {} }",false),
    ("closed-generics",prefix+activation+"[assembly: DockJsonSerializable(typeof(Example.Payload<int>))] [assembly: DockJsonSerializable(typeof(Example.Payload<string>))] [assembly: DockJsonSerializable(typeof(Example.Payload<int[]>))] [assembly: DockJsonSerializable(typeof(Example.Payload<int[,]>))] [assembly: DockJsonSerializable(typeof(Example.Outer<int>.Nested<string>))] namespace Example { public class Payload<T> {public T Value {get;set;}=default!;} public class Outer<T> {public class Nested<U> {public T A {get;set;}=default!;public U B {get;set;}=default!;}} }",false),
    ("ignored-members",prefix+activation+"namespace Example { public abstract class Base : Document { [System.Runtime.Serialization.IgnoreDataMember] public virtual string? Secret {get;set;} [System.Runtime.Serialization.IgnoreDataMember] public string? Hidden {get;set;} } public class Doc : Base {public override string? Secret {get;set;} public new string? Hidden {get;set;} public System.Windows.Input.ICommand? Command {get;set;} [System.Runtime.Serialization.IgnoreDataMember] private string? Private {get;set;} [System.Runtime.Serialization.IgnoreDataMember] public static string? Static {get;set;} public string? Category {get;set;} } }",false),
    ("unique-context",prefix+activation+"namespace A { public class DockSerializerGeneratedJsonContext {} } namespace B {public class DockSerializerGeneratedJsonContext_1 {}} namespace Example { public class Doc : Document {} }",false),
    ("duplicate-registration",prefix+activation+"[assembly: DockJsonSerializable(typeof(Example.Doc))] [assembly: DockJsonSerializable(typeof(Example.Doc))] namespace Example {public class Doc : Document {}}",false),
    ("missing-contracts","using Dock.Serializer.SystemTextJson; "+activation+"[assembly: DockJsonSerializable(typeof(Example.Payload))] namespace Example {public class Payload {}}",true),
    ("non-named-registration",prefix+activation+"[assembly: DockJsonSerializable(typeof(dynamic))] [assembly: DockJsonSerializable(typeof(delegate*<int,void>))] namespace Example {}",false),
    ("registration-domain",prefix+activation+"[assembly: DockJsonSerializable(typeof(object))] [assembly: DockJsonSerializable(typeof(int?))] [assembly: DockJsonSerializable(typeof(System.Collections.Generic.List<int>))] [assembly: DockJsonSerializable(typeof(Example.Template))] namespace Example {public struct Template : Dock.Model.Controls.IDocumentTemplate {public object? Content {get;set;}}}",false),
    ("interface-ignore-union",prefix+activation+"namespace Example {public class IgnoredTemplate : Dock.Model.Controls.IDocumentTemplate {[System.Runtime.Serialization.IgnoreDataMember] public object? Content {get;set;}} public class RegularTemplate : Dock.Model.Controls.IDocumentTemplate {public object? Content {get;set;}} public class Command : System.Windows.Input.ICommand {public event EventHandler? CanExecuteChanged;public bool CanExecute(object? p)=>true;public void Execute(object? p){}} public class Doc : Document {public Command? SpecificCommand {get;set;}}}",false),
};
foreach (var item in cases) Run(item.Name,item.Source,item.Missing,Array.Empty<MetadataReference>());
Run("generated-payloads",File.ReadAllText("../serializer-reference/GeneratedPayloadReference.cs"),false,Array.Empty<MetadataReference>());
var duplicate = Compilation("DuplicateReference", "namespace Example { public class DuplicateDocument : Dock.Model.ReactiveUI.Controls.Document {} }",false,Array.Empty<MetadataReference>());
using(var stream = new MemoryStream()) {
    var emitted = duplicate.Emit(stream); if (!emitted.Success) throw new Exception(string.Join("\n",emitted.Diagnostics));
    var alias = MetadataReference.CreateFromImage(stream.ToArray(), MetadataReferenceProperties.Assembly.WithAliases(ImmutableArray.Create("RefA")));
    Run("duplicate-discriminator","extern alias RefA; "+prefix+activation+"[assembly: DockJsonSerializable(typeof(RefA::Example.DuplicateDocument))] namespace Example {public class DuplicateDocument : Document {}}",false,new[]{alias});
}
foreach(bool friend in new[]{true,false}) {
    var code=(friend?"[assembly: System.Runtime.CompilerServices.InternalsVisibleTo(\"DockGeneratorConsumer\")] ":"")+"namespace Foreign {internal class InternalDoc : Dock.Model.ReactiveUI.Controls.Document {}}";
    var library=Compilation(friend?"FriendLibrary":"PrivateLibrary",code,false,Array.Empty<MetadataReference>());
    using var bytes=new MemoryStream();var emitted=library.Emit(bytes);if(!emitted.Success)throw new Exception(string.Join("\n",emitted.Diagnostics));
    var reference=MetadataReference.CreateFromImage(bytes.ToArray(),MetadataReferenceProperties.Assembly.WithAliases(ImmutableArray.Create("External")));
    Run(friend?"friend-registration":"inaccessible-registration","extern alias External; "+prefix+activation+"[assembly: DockJsonSerializable(typeof(External::Foreign.InternalDoc))] namespace Example {}",false,new[]{reference});
}
var foreign=Compilation("ForeignLibrary","namespace Foreign {public class PublicDoc : Dock.Model.ReactiveUI.Controls.Document {}}",false,Array.Empty<MetadataReference>());
using(var bytes=new MemoryStream()) {var emitted=foreign.Emit(bytes);if(!emitted.Success)throw new Exception(string.Join("\n",emitted.Diagnostics));Run("referenced-types-not-auto-discovered",prefix+activation+"namespace Example {}",false,new[]{MetadataReference.CreateFromImage(bytes.ToArray())});}
File.WriteAllText(args[0],JsonSerializer.Serialize(results,new JsonSerializerOptions{WriteIndented=true}));
Console.WriteLine($"Original compiler generator cases: {results.Count}");

static CSharpCompilation Compilation(string name,string source,bool missing,MetadataReference[] extra) {
    var paths = ((string)AppContext.GetData("TRUSTED_PLATFORM_ASSEMBLIES")!).Split(Path.PathSeparator).ToList();
    paths.Add(typeof(Dock.Model.ReactiveUI.Controls.Document).Assembly.Location);paths.Add(typeof(Dock.Serializer.SystemTextJson.DockJsonSourceGenerationAttribute).Assembly.Location);
    var references=paths.Distinct(StringComparer.OrdinalIgnoreCase).Where(p=>!missing||!Path.GetFileName(p).StartsWith("Dock.Model",StringComparison.Ordinal)).Select(p=>MetadataReference.CreateFromFile(p)).Cast<MetadataReference>().Concat(extra);
    var tree=CSharpSyntaxTree.ParseText(source,new CSharpParseOptions(LanguageVersion.CSharp13),path:name+".cs");
    return CSharpCompilation.Create(name,new[]{tree},references,new CSharpCompilationOptions(OutputKind.DynamicallyLinkedLibrary));
}
void Run(string name,string source,bool missing,MetadataReference[] extra) {
    var compilation=Compilation("DockGeneratorConsumer",source,missing,extra);
    GeneratorDriver driver=CSharpGeneratorDriver.Create(new[]{new DockJsonSourceGenerator().AsSourceGenerator()},parseOptions:(CSharpParseOptions)compilation.SyntaxTrees[0].Options);
    driver=driver.RunGeneratorsAndUpdateCompilation(compilation,out _,out _);
    var run=driver.GetRunResult().Results.Single();
    var context=run.GeneratedSources.FirstOrDefault(s=>s.HintName=="DockSystemTextJsonContext.g.cs");
    var generated=run.GeneratedSources.FirstOrDefault(s=>s.HintName=="DockSystemTextJsonGenerated.g.cs");
    bool enabled=context.SourceText is not null;
    string contextName="";string[] contexts=Array.Empty<string>();var ignored=new List<object>();var polymorphisms=new List<object>();
    if(enabled){
        var syntax=context.SyntaxTree.GetRoot();contextName=syntax.DescendantNodes().OfType<ClassDeclarationSyntax>().Single().Identifier.ValueText;
        contexts=syntax.DescendantNodes().OfType<AttributeSyntax>().Where(a=>a.Name.ToString().EndsWith("JsonSerializable",StringComparison.Ordinal)).Select(a=>((TypeOfExpressionSyntax)a.ArgumentList!.Arguments[0].Expression).Type.ToString()).ToArray();
        var root=generated.SyntaxTree.GetRoot();
        var method=root.DescendantNodes().OfType<MethodDeclarationSyntax>().Single(m=>m.Identifier.ValueText=="CreateIgnoredMembers");
        foreach(var assignment in method.DescendantNodes().OfType<AssignmentExpressionSyntax>()) {
            if(assignment.Left is ElementAccessExpressionSyntax left){var expr=left.DescendantNodes().OfType<TypeOfExpressionSyntax>().First().Type.ToString();var names=assignment.Right.DescendantNodes().OfType<InitializerExpressionSyntax>().Single().Expressions.Cast<LiteralExpressionSyntax>().Select(x=>(string)x.Token.Value!).ToArray();ignored.Add(new{expression=expr,names});}
        }
        foreach(string kind in new[]{"Dockable","Dock","RootDock","DockWindow","DocumentTemplate","ToolTemplate","Object"}) {
            string[] types;string handling;bool ignore;
            if(kind=="Object") {var map=root.DescendantNodes().OfType<MethodDeclarationSyntax>().Single(m=>m.Identifier.ValueText=="CreateObjectPayloadDiscriminators");types=map.DescendantNodes().OfType<AssignmentExpressionSyntax>().Where(a=>a.Left is ElementAccessExpressionSyntax).Select(a=>a.Left.DescendantNodes().OfType<TypeOfExpressionSyntax>().First().Type.ToString()).ToArray();handling="FailSerialization";ignore=false;}
            else {var options=root.DescendantNodes().OfType<MethodDeclarationSyntax>().Single(m=>m.Identifier.ValueText=="Create"+kind+"Options");types=options.DescendantNodes().OfType<ObjectCreationExpressionSyntax>().Where(n=>n.Type.ToString().EndsWith("JsonDerivedType",StringComparison.Ordinal)).Select(n=>((TypeOfExpressionSyntax)n.ArgumentList!.Arguments[0].Expression).Type.ToString()).ToArray();var assignments=options.DescendantNodes().OfType<AssignmentExpressionSyntax>().ToArray();handling=((MemberAccessExpressionSyntax)assignments.Single(a=>a.Left.ToString()=="UnknownDerivedTypeHandling").Right).Name.Identifier.ValueText;ignore=assignments.Single(a=>a.Left.ToString()=="IgnoreUnrecognizedTypeDiscriminators").Right.IsKind(SyntaxKind.TrueLiteralExpression);}
            polymorphisms.Add(new{kind,types,handling,ignore});
        }
    }
    var snapshot=new Snapshot(compilation);
    var input=snapshot.Export();
    results.Add(new{name,input,expected=new{enabled,contextName,contexts,ignored,polymorphisms,diagnostics=run.Diagnostics.Where(d=>d.Id.StartsWith("DSTJ",StringComparison.Ordinal)).Select(d=>new{id=d.Id,message=d.GetMessage(),location=Snapshot.Location(d.Location)}).ToArray()},nameCases=snapshot.NameCases});
}

internal sealed class Snapshot(CSharpCompilation compilation) {
    private readonly List<ITypeSymbol> types=new();private readonly Dictionary<ITypeSymbol,int> typeIds=new(SymbolEqualityComparer.Default);
    private readonly List<IPropertySymbol> properties=new();private readonly Dictionary<IPropertySymbol,int> propertyIds=new(SymbolEqualityComparer.Default);
    private readonly List<IAssemblySymbol> assemblies=new();private readonly Dictionary<IAssemblySymbol,int> assemblyIds=new(SymbolEqualityComparer.Default);
    private readonly HashSet<INamedTypeSymbol> owners=new(SymbolEqualityComparer.Default);
    public List<object> NameCases {get;}=new();
    public static string? Location(Location? location)=>location is null||location==Microsoft.CodeAnalysis.Location.None?null:location.IsInSource?$"{location.SourceSpan.Start}:{location.SourceSpan.Length}":"metadata";
    private int Type(ITypeSymbol t){if(typeIds.TryGetValue(t,out int id))return id;id=types.Count;typeIds.Add(t,id);types.Add(t);return id;}
    private int Property(IPropertySymbol p){if(propertyIds.TryGetValue(p,out int id))return id;id=properties.Count;propertyIds.Add(p,id);properties.Add(p);return id;}
    private int? Assembly(IAssemblySymbol? a){if(a is null)return null;if(assemblyIds.TryGetValue(a,out int id))return id;id=assemblies.Count;assemblyIds.Add(a,id);assemblies.Add(a);return id;}
    private void Owner(INamedTypeSymbol t){if(!owners.Add(t))return;Type(t);if(t.BaseType is not null)Owner(t.BaseType);foreach(var i in t.AllInterfaces)Owner(i);}
    private static IEnumerable<INamedTypeSymbol> Named(INamespaceSymbol ns){foreach(var t in ns.GetTypeMembers())foreach(var n in Nested(t))yield return n;foreach(var child in ns.GetNamespaceMembers())foreach(var n in Named(child))yield return n;}
    private static IEnumerable<INamedTypeSymbol> Nested(INamedTypeSymbol t){yield return t;foreach(var child in t.GetTypeMembers())foreach(var n in Nested(child))yield return n;}
    public object Export(){
        var names=new[]{"Dock.Model.Core.IDockable","Dock.Model.Core.IDock","Dock.Model.Controls.IRootDock","Dock.Model.Core.IDockWindow","Dock.Model.Controls.IDocumentTemplate","Dock.Model.Controls.IToolTemplate","System.Windows.Input.ICommand","System.Runtime.Serialization.IgnoreDataMemberAttribute"};
        var required=names.Select(n=>compilation.GetTypeByMetadataName(n)).ToArray();foreach(var t in required)if(t is not null)Owner(t);
        var source=Named(compilation.Assembly.GlobalNamespace).ToArray();foreach(var t in source)Owner(t);
        var attrs=compilation.Assembly.GetAttributes();var registrations=new List<object>();
        foreach(var a in attrs.Where(a=>a.AttributeClass?.ToDisplayString()=="Dock.Serializer.SystemTextJson.DockJsonSerializableAttribute")){
            var arg=a.ConstructorArguments.Length==1?a.ConstructorArguments[0]:default;var t=arg.Value as ITypeSymbol;if(t is INamedTypeSymbol named)Owner(named);
            registrations.Add(new{type_id=t is null?(int?)null:Type(t),valid_type_constant=a.ConstructorArguments.Length==1&&arg.Kind==TypedConstantKind.Type,display_name=t?.ToDisplayString(SymbolDisplayFormat.CSharpErrorMessageFormat)??"<unknown>",location=Location(a.ApplicationSyntaxReference?.GetSyntax().GetLocation())});
        }
        var intType=compilation.GetSpecialType(SpecialType.System_Int32);var stringType=compilation.GetSpecialType(SpecialType.System_String);
        var payload=compilation.GetTypeByMetadataName("RuntimePayload`1")!;
        void Name(INamedTypeSymbol symbol,Type runtime){NameCases.Add(new{type_id=Type(symbol),expected=runtime.FullName});}
        Name(payload.Construct(intType),typeof(RuntimePayload<int>));Name(payload.Construct(stringType),typeof(RuntimePayload<string>));
        Name(payload.Construct(compilation.CreateArrayTypeSymbol(intType)),typeof(RuntimePayload<int[]>));Name(payload.Construct(compilation.CreateArrayTypeSymbol(intType,2)),typeof(RuntimePayload<int[,]>));
        Name(payload.Construct(compilation.CreateTupleTypeSymbol(ImmutableArray.Create<ITypeSymbol>(intType,stringType))),typeof(RuntimePayload<(int,string)>));
        Name(payload.Construct(payload.Construct(intType)),typeof(RuntimePayload<RuntimePayload<int>>));
        Name(payload.Construct(compilation.CreateArrayTypeSymbol(payload.Construct(intType),2)),typeof(RuntimePayload<RuntimePayload<int>[,]>));
        Name(compilation.GetTypeByMetadataName("RuntimeOuter`1")!.Construct(intType).GetTypeMembers("Nested").Single().Construct(stringType),typeof(RuntimeOuter<int>.Nested<string>));
        var typeRows=new List<object>();var propertyRows=new List<object>();int ti=0,pi=0;
        while(ti<types.Count||pi<properties.Count){
            if(ti<types.Count){var t=types[ti++];var n=t as INamedTypeSymbol;var array=t as IArrayTypeSymbol;var pointer=t as IPointerTypeSymbol;
                typeRows.Add(new{expression=t.ToDisplayString(SymbolDisplayFormat.FullyQualifiedFormat),display_name=t.ToDisplayString(SymbolDisplayFormat.CSharpErrorMessageFormat),metadata_name=t.MetadataName,ns=t.ContainingNamespace?.IsGlobalNamespace==false?t.ContainingNamespace.ToDisplayString():"",kind=t.TypeKind.ToString(),special=t.SpecialType!=SpecialType.None,is_abstract=t.IsAbstract,accessibility=t.DeclaredAccessibility.ToString(),assembly=Assembly(t.ContainingAssembly),containing_type=t.ContainingType is null?(int?)null:Type(t.ContainingType),base_type=n?.BaseType is null?(int?)null:Type(n.BaseType),all_interfaces=n?.AllInterfaces.Select(Type).ToArray()??Array.Empty<int>(),type_arguments=n?.TypeArguments.Select(Type).ToArray()??Array.Empty<int>(),arity=n?.Arity??0,is_unbound_generic=n?.IsUnboundGenericType??false,tuple_underlying_type=n?.TupleUnderlyingType is null?(int?)null:Type(n.TupleUnderlyingType),element_type=array is not null?(int?)Type(array.ElementType):pointer is not null?Type(pointer.PointedAtType):null,array_rank=array?.Rank??0,properties=n is not null&&owners.Contains(n)?n.GetMembers().OfType<IPropertySymbol>().Select(Property).ToArray():Array.Empty<int>(),location=Location(t.Locations.FirstOrDefault())});
            }else{var p=properties[pi++];propertyRows.Add(new{name=p.Name,type_id=Type(p.Type),is_public=p.DeclaredAccessibility==Accessibility.Public,is_static=p.IsStatic,ignore_data_member=p.GetAttributes().Any(a=>a.AttributeClass?.ToDisplayString()=="System.Runtime.Serialization.IgnoreDataMemberAttribute"),overridden_property=p.OverriddenProperty is null?(int?)null:Property(p.OverriddenProperty)});}
        }
        return new{activated=attrs.Any(a=>a.AttributeClass?.ToDisplayString()=="Dock.Serializer.SystemTextJson.DockJsonSourceGenerationAttribute"),registrations,source_types=source.Select(Type).ToArray(),required_symbols=required.Select(t=>t is null?(int?)null:Type(t)).ToArray(),occupied_type_names=compilation.GetSymbolsWithName(_=>true,SymbolFilter.Type).Select(s=>s.Name).Distinct().ToArray(),assemblies=assemblies.Select(a=>new{name=a.Identity.Name,version=a.Identity.Version.ToString(),culture=a.Identity.CultureName,public_key_token=a.Identity.PublicKeyToken.ToArray().Select(v=>(int)v).ToArray(),grants_internal_access=a.GivesAccessTo(compilation.Assembly)}).ToArray(),types=typeRows,properties=propertyRows,context_roots=new{obj="global::System.Object",dockable_list="global::System.Collections.Generic.IList<global::Dock.Model.Core.IDockable>",window_list="global::System.Collections.Generic.IList<global::Dock.Model.Core.IDockWindow>"}};
    }
}
public sealed class RuntimePayload<T> { }
public sealed class RuntimeOuter<T> { public sealed class Nested<U> { } }
