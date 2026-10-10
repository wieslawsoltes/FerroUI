// Upstream API extractor.
//
// Walks the upstream (Avalonia) source directories listed in projects.json and
// writes a JSON description of every file, type and member. Syntax trees only:
// no compilation, no semantic model, no restore of the upstream solution.
//
//   dotnet run -c Release --project scripts/api-extract -- \
//       --upstream /path/to/Avalonia --config scripts/api-extract/projects.json \
//       --out docs/porting/data
//
// The commit recorded in the output is the HEAD of the upstream directory, or
// the value of --commit when the directory is a plain export of a commit,
// which is how scripts/port-status/run.sh reads the tracked commit without
// touching the checkout. --xamlx-commit names the commit of the external/XamlX
// submodule of such an export.
//
// Output: <out>/upstream-api.json (index, and the whole data set when it is
// smaller than --split-threshold-mb), otherwise one file per project in
// <out>/upstream/<project>.json referenced from the index.

using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;
using Microsoft.CodeAnalysis;
using Microsoft.CodeAnalysis.CSharp;
using Microsoft.CodeAnalysis.CSharp.Syntax;

namespace ApiExtract;

internal static class Program
{
    private static readonly JsonSerializerOptions s_json = new()
    {
        WriteIndented = true,
        IndentSize = 1,
        Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
    };

    private static int Main(string[] args)
    {
        string? upstream = null, config = null, outDir = null, commitArg = null, xamlxCommitArg = null;
        double splitMb = 15;
        for (var i = 0; i < args.Length; i++)
        {
            switch (args[i])
            {
                case "--upstream": upstream = args[++i]; break;
                case "--config": config = args[++i]; break;
                case "--out": outDir = args[++i]; break;
                case "--commit": commitArg = args[++i]; break;
                case "--xamlx-commit": xamlxCommitArg = args[++i]; break;
                case "--split-threshold-mb": splitMb = double.Parse(args[++i]); break;
                default:
                    Console.Error.WriteLine($"unknown argument {args[i]}");
                    return 2;
            }
        }

        if (upstream is null || config is null || outDir is null)
        {
            Console.Error.WriteLine("usage: api-extract --upstream <dir> --config <projects.json> --out <dir> [--commit <sha>] [--xamlx-commit <sha>] [--split-threshold-mb 15]");
            return 2;
        }

        upstream = Path.GetFullPath(upstream);
        var sw = Stopwatch.StartNew();
        var cfg = JsonNode.Parse(File.ReadAllText(config), documentOptions: new JsonDocumentOptions { CommentHandling = JsonCommentHandling.Skip, AllowTrailingCommas = true })!.AsObject();
        var symbols = cfg["preprocessorSymbols"]!.AsArray().Select(x => x!.GetValue<string>()).ToArray();
        var excludeDirs = cfg["excludeDirs"]!.AsArray().Select(x => x!.GetValue<string>()).ToHashSet(StringComparer.OrdinalIgnoreCase);
        var excludeFiles = cfg["excludeFiles"]!.AsArray().Select(x => GlobToRegex(x!.GetValue<string>())).ToArray();
        var parseOptions = new CSharpParseOptions(LanguageVersion.Preview, DocumentationMode.None, SourceCodeKind.Regular, symbols);

        var projects = new List<JsonObject>();
        foreach (var entry in cfg["projects"]!.AsArray().Select(x => x!.AsObject()))
        {
            foreach (var (path, isGlob) in Expand(upstream, entry["path"]!.GetValue<string>()))
            {
                var dir = Path.Combine(upstream, path);
                if (!Directory.Exists(dir))
                {
                    Console.Error.WriteLine($"warning: {path} does not exist in the upstream checkout, skipped");
                    continue;
                }

                var detail = entry["detail"]?.GetValue<string>() ?? "full";
                var name = !isGlob && entry["name"] is { } n ? n.GetValue<string>() : Path.GetFileName(path);
                var project = new JsonObject
                {
                    ["name"] = name,
                    ["path"] = path,
                    ["detail"] = detail,
                    ["scope"] = entry["scope"]?.GetValue<string>() ?? "in",
                };
                foreach (var key in new[] { "rust", "crate", "phase", "priority", "rustIdl" })
                {
                    if (entry[key] is { } v && !(isGlob && key is "rust" or "crate"))
                        project[key] = v.DeepClone();
                }

                if (detail == "files")
                {
                    var patterns = entry["patterns"]!.AsArray().Select(x => GlobToRegex(x!.GetValue<string>())).ToArray();
                    var list = new JsonArray();
                    foreach (var f in Walk(dir, excludeDirs).Where(f => patterns.Any(p => p.IsMatch(Path.GetFileName(f)))))
                        list.Add(new JsonObject { ["path"] = Rel(dir, f), ["lines"] = CountLines(f) });
                    project["files"] = list;
                }
                else
                {
                    var csFiles = Walk(dir, excludeDirs)
                        .Where(f => f.EndsWith(".cs", StringComparison.Ordinal))
                        .Where(f => !excludeFiles.Any(p => p.IsMatch(Path.GetFileName(f))))
                        .ToArray();
                    var results = new ConcurrentDictionary<string, JsonObject?>();
                    Parallel.ForEach(csFiles, f =>
                    {
                        results[f] = ExtractFile(dir, f, parseOptions, detail == "full");
                    });
                    var files = new JsonArray();
                    foreach (var f in csFiles)
                    {
                        if (results[f] is { } o)
                            files.Add(o);
                    }

                    project["files"] = files;
                }

                if (entry["extraFiles"] is JsonArray extras)
                {
                    var extraObj = new JsonObject();
                    foreach (var extra in extras.Select(x => x!.AsObject()))
                    {
                        var patterns = extra["patterns"]!.AsArray().Select(x => GlobToRegex(x!.GetValue<string>())).ToArray();
                        var list = new JsonArray();
                        foreach (var f in Walk(dir, excludeDirs).Where(f => patterns.Any(p => p.IsMatch(Path.GetFileName(f)))))
                            list.Add(Rel(dir, f));
                        extraObj[extra["label"]!.GetValue<string>()] = list;
                    }

                    project["extraFiles"] = extraObj;
                }

                if (entry["idl"] is JsonArray idls)
                {
                    var idlArr = new JsonArray();
                    foreach (var idl in idls.Select(x => x!.GetValue<string>()))
                    {
                        var idlPath = Path.Combine(dir, idl);
                        if (File.Exists(idlPath))
                            idlArr.Add(IdlExtractor.Extract(idl, File.ReadAllText(idlPath)));
                    }

                    project["idl"] = idlArr;
                }

                projects.Add(project);
            }
        }

        projects.Sort((a, b) => string.CompareOrdinal(a["path"]!.GetValue<string>(), b["path"]!.GetValue<string>()));

        var commit = commitArg ?? GitHead(upstream);
        var submoduleCommits = new JsonObject();
        if (Directory.Exists(Path.Combine(upstream, "external", "XamlX")))
            submoduleCommits["external/XamlX"] = xamlxCommitArg ?? GitHead(Path.Combine(upstream, "external", "XamlX"));

        Directory.CreateDirectory(outDir);
        var serialized = projects.Select(p => (p, text: p.ToJsonString(s_json))).ToList();
        var total = serialized.Sum(x => (long)Encoding.UTF8.GetByteCount(x.text));
        var split = total > splitMb * 1024 * 1024;
        var index = new JsonObject
        {
            ["schema"] = 1,
            ["upstreamCommit"] = commit,
            ["submoduleCommits"] = submoduleCommits,
            ["split"] = split,
            ["solutionLayout"] = SolutionLayout(upstream, excludeDirs),
        };
        var perProjectDir = Path.Combine(outDir, "upstream");
        if (split)
        {
            if (Directory.Exists(perProjectDir))
            {
                foreach (var old in Directory.GetFiles(perProjectDir, "*.json"))
                    File.Delete(old);
            }

            Directory.CreateDirectory(perProjectDir);
            var arr = new JsonArray();
            foreach (var (p, text) in serialized)
            {
                var fileName = SafeName(p["name"]!.GetValue<string>()) + ".json";
                File.WriteAllText(Path.Combine(perProjectDir, fileName), text + "\n");
                arr.Add(new JsonObject
                {
                    ["name"] = p["name"]!.DeepClone(),
                    ["path"] = p["path"]!.DeepClone(),
                    ["detail"] = p["detail"]!.DeepClone(),
                    ["file"] = "upstream/" + fileName,
                    ["bytes"] = Encoding.UTF8.GetByteCount(text) + 1,
                });
            }

            index["projects"] = arr;
        }
        else
        {
            var arr = new JsonArray();
            foreach (var (p, _) in serialized)
                arr.Add(p);
            index["projects"] = arr;
        }

        File.WriteAllText(Path.Combine(outDir, "upstream-api.json"), index.ToJsonString(s_json) + "\n");

        var fileCount = projects.Sum(p => p["files"]!.AsArray().Count);
        Console.Error.WriteLine($"api-extract: {projects.Count} projects, {fileCount} files, {total / 1024.0 / 1024.0:F1} MB JSON ({(split ? "split per project" : "single file")}), {sw.Elapsed.TotalSeconds:F1}s, upstream {commit}");
        return 0;
    }

    /// <summary>Top-level layout of the upstream solution: every project directory with its source file count.</summary>
    private static JsonObject SolutionLayout(string upstream, HashSet<string> excludeDirs)
    {
        var layout = new JsonObject();
        foreach (var top in new[] { "src", "native", "external", "samples", "tests" })
        {
            var dir = Path.Combine(upstream, top);
            if (!Directory.Exists(dir))
                continue;
            var arr = new JsonArray();
            var projectDirs = Directory.GetFiles(dir, "*.csproj", SearchOption.AllDirectories)
                .Concat(Directory.GetFiles(dir, "*.proj", SearchOption.AllDirectories))
                .Where(f => !Path.GetRelativePath(upstream, f).Split(Path.DirectorySeparatorChar).Any(excludeDirs.Contains))
                .Select(f => Path.GetDirectoryName(f)!)
                .Distinct()
                .OrderBy(x => x, StringComparer.Ordinal);
            foreach (var d in projectDirs)
            {
                arr.Add(new JsonObject
                {
                    ["path"] = Path.GetRelativePath(upstream, d).Replace('\\', '/'),
                    ["csFiles"] = Walk(d, excludeDirs).Count(f => f.EndsWith(".cs", StringComparison.Ordinal)),
                });
            }

            layout[top] = arr;
        }

        return layout;
    }

    private static IEnumerable<(string path, bool isGlob)> Expand(string upstream, string path)
    {
        if (!path.EndsWith("/*", StringComparison.Ordinal))
        {
            yield return (path, false);
            yield break;
        }

        var parent = path[..^2];
        var dir = Path.Combine(upstream, parent);
        if (!Directory.Exists(dir))
            yield break;
        foreach (var sub in Directory.GetDirectories(dir).OrderBy(x => x, StringComparer.Ordinal))
        {
            if (Directory.GetFiles(sub, "*.csproj").Length > 0)
                yield return (parent + "/" + Path.GetFileName(sub), true);
        }
    }

    private static IEnumerable<string> Walk(string dir, HashSet<string> excludeDirs)
    {
        foreach (var f in Directory.GetFiles(dir).OrderBy(x => x, StringComparer.Ordinal))
            yield return f;
        foreach (var d in Directory.GetDirectories(dir).OrderBy(x => x, StringComparer.Ordinal))
        {
            if (excludeDirs.Contains(Path.GetFileName(d)))
                continue;
            foreach (var f in Walk(d, excludeDirs))
                yield return f;
        }
    }

    private static string Rel(string dir, string file) => Path.GetRelativePath(dir, file).Replace('\\', '/');

    private static int CountLines(string file)
    {
        var n = 0;
        foreach (var _ in File.ReadLines(file))
            n++;
        return n;
    }

    private static Regex GlobToRegex(string glob) =>
        new("^" + Regex.Escape(glob).Replace("\\*", ".*").Replace("\\?", ".") + "$", RegexOptions.CultureInvariant);

    private static string SafeName(string name) => Regex.Replace(name, "[^A-Za-z0-9_.-]+", "_").Trim('_');

    private static string GitHead(string dir)
    {
        try
        {
            var psi = new ProcessStartInfo("git", "rev-parse HEAD")
            {
                WorkingDirectory = dir,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
            };
            using var p = Process.Start(psi)!;
            var s = p.StandardOutput.ReadToEnd().Trim();
            p.WaitForExit();
            return p.ExitCode == 0 ? s : "unknown";
        }
        catch
        {
            return "unknown";
        }
    }

    private static JsonObject? ExtractFile(string projectDir, string file, CSharpParseOptions options, bool full)
    {
        var text = File.ReadAllText(file);
        var head = text.Length > 600 ? text[..600] : text;
        if (head.Contains("<auto-generated", StringComparison.OrdinalIgnoreCase) || head.Contains("<autogenerated", StringComparison.OrdinalIgnoreCase))
            return null;

        var tree = CSharpSyntaxTree.ParseText(text, options);
        var root = tree.GetCompilationUnitRoot();
        var types = new JsonArray();
        var extractor = new TypeExtractor(full);
        extractor.CollectTypes(root.Members, "", null, types);
        var lines = 1;
        foreach (var c in text)
        {
            if (c == '\n')
                lines++;
        }

        return new JsonObject
        {
            ["path"] = Rel(projectDir, file),
            ["lines"] = lines,
            ["types"] = types,
        };
    }
}

internal sealed class TypeExtractor(bool full)
{
    public void CollectTypes(SyntaxList<MemberDeclarationSyntax> members, string ns, string? outer, JsonArray output)
    {
        foreach (var m in members)
        {
            switch (m)
            {
                case BaseNamespaceDeclarationSyntax nsDecl:
                    CollectTypes(nsDecl.Members, ns.Length == 0 ? nsDecl.Name.ToString() : ns + "." + nsDecl.Name, outer, output);
                    break;
                case BaseTypeDeclarationSyntax or DelegateDeclarationSyntax:
                    if (ExtractType(m, ns, outer, "internal") is { } t)
                        output.Add(t);
                    break;
            }
        }
    }

    private JsonObject? ExtractType(MemberDeclarationSyntax decl, string ns, string? outer, string defaultAccess)
    {
        var modifiers = decl.Modifiers;
        var access = Accessibility(modifiers, defaultAccess);
        if (access is "private" or "file")
            return null;

        string name;
        string kind;
        TypeParameterListSyntax? typeParams = null;
        BaseListSyntax? baseList = null;
        switch (decl)
        {
            case ClassDeclarationSyntax c: kind = "class"; name = c.Identifier.Text; typeParams = c.TypeParameterList; baseList = c.BaseList; break;
            case StructDeclarationSyntax s: kind = "struct"; name = s.Identifier.Text; typeParams = s.TypeParameterList; baseList = s.BaseList; break;
            case InterfaceDeclarationSyntax i: kind = "interface"; name = i.Identifier.Text; typeParams = i.TypeParameterList; baseList = i.BaseList; break;
            case RecordDeclarationSyntax r:
                kind = r.ClassOrStructKeyword.IsKind(SyntaxKind.StructKeyword) ? "record struct" : "record";
                name = r.Identifier.Text; typeParams = r.TypeParameterList; baseList = r.BaseList; break;
            case EnumDeclarationSyntax e: kind = "enum"; name = e.Identifier.Text; baseList = e.BaseList; break;
            case DelegateDeclarationSyntax d: kind = "delegate"; name = d.Identifier.Text; typeParams = d.TypeParameterList; break;
            default: return null;
        }

        var arity = typeParams?.Parameters.Count ?? 0;
        var display = name + (arity > 0 ? "<" + string.Join(", ", typeParams!.Parameters.Select(p => p.Identifier.Text)) + ">" : "");
        var qualified = outer is null ? display : outer + "." + display;
        var attrs = Attributes(decl.AttributeLists);
        var mods = new JsonArray();
        foreach (var mod in modifiers)
        {
            if (mod.Kind() is SyntaxKind.StaticKeyword or SyntaxKind.AbstractKeyword or SyntaxKind.SealedKeyword or SyntaxKind.PartialKeyword
                or SyntaxKind.ReadOnlyKeyword or SyntaxKind.RefKeyword or SyntaxKind.UnsafeKeyword or SyntaxKind.NewKeyword)
                mods.Add(mod.Text);
        }

        var obj = new JsonObject
        {
            ["name"] = name,
            ["kind"] = kind,
            ["namespace"] = ns,
            ["fullName"] = (ns.Length == 0 ? "" : ns + ".") + qualified,
            ["accessibility"] = access,
            ["line"] = Line(decl),
        };
        if (outer is not null)
            obj["outer"] = outer;
        if (arity > 0)
        {
            obj["arity"] = arity;
            obj["typeParameters"] = new JsonArray(typeParams!.Parameters.Select(p => (JsonNode)p.Identifier.Text).ToArray());
        }

        if (mods.Count > 0)
            obj["modifiers"] = mods;
        if (modifiers.Any(SyntaxKind.PartialKeyword))
            obj["partial"] = true;
        if (baseList is not null)
            obj["bases"] = new JsonArray(baseList.Types.Select(t => (JsonNode)Normalize(t.ToString())).ToArray());
        if (attrs.Count > 0)
        {
            obj["attributes"] = attrs;
            if (HasAttribute(decl.AttributeLists, "Flags"))
                obj["flags"] = true;
        }

        var members = new JsonArray();
        var nested = new JsonArray();
        var privateCount = 0;
        var isInterface = kind == "interface";

        switch (decl)
        {
            case DelegateDeclarationSyntax d:
                obj["signature"] = Normalize($"{d.ReturnType} {display}{d.ParameterList}");
                break;
            case EnumDeclarationSyntax e:
                foreach (var em in e.Members)
                {
                    var mo = new JsonObject
                    {
                        ["kind"] = "enum member",
                        ["name"] = em.Identifier.Text,
                        ["signature"] = Normalize(em.Identifier.Text + (em.EqualsValue is null ? "" : " " + em.EqualsValue)),
                        ["accessibility"] = "public",
                    };
                    AddAttributes(mo, em.AttributeLists);
                    members.Add(mo);
                }

                break;
            case TypeDeclarationSyntax t:
                if (t.ParameterList is { } primary)
                {
                    members.Add(new JsonObject
                    {
                        ["kind"] = "ctor",
                        ["name"] = ".ctor",
                        ["signature"] = Normalize(name + primary),
                        ["accessibility"] = "public",
                        ["params"] = primary.Parameters.Count,
                        ["primary"] = true,
                        ["line"] = Line(t),
                    });
                    if (t is RecordDeclarationSyntax)
                    {
                        foreach (var p in primary.Parameters)
                        {
                            var po = new JsonObject
                            {
                                ["kind"] = "property",
                                ["name"] = p.Identifier.Text,
                                ["signature"] = Normalize($"{p.Type} {p.Identifier.Text} {{ get; init; }}"),
                                ["accessibility"] = "public",
                                ["type"] = Normalize(p.Type?.ToString() ?? ""),
                                ["accessors"] = new JsonObject { ["get"] = "public", ["init"] = "public" },
                                ["positional"] = true,
                            };
                            members.Add(po);
                        }
                    }
                }

                foreach (var m in t.Members)
                {
                    if (m is BaseTypeDeclarationSyntax or DelegateDeclarationSyntax)
                    {
                        // Nested types of an interface are public by default.
                        var n = ExtractNested(m, ns, qualified, isInterface);
                        if (n is null)
                            privateCount++;
                        else
                            nested.Add(n);
                        continue;
                    }

                    ExtractMember(m, name, isInterface, members, ref privateCount);
                }

                break;
        }

        if (full)
        {
            obj["members"] = members;
        }
        else
        {
            obj["memberCount"] = members.Count;
        }

        if (privateCount > 0)
            obj["privateMembers"] = privateCount;
        if (nested.Count > 0)
            obj["nestedTypes"] = nested;
        return obj;
    }

    private JsonObject? ExtractNested(MemberDeclarationSyntax m, string ns, string outer, bool outerIsInterface) =>
        // Nested types are private by default, except inside interfaces.
        ExtractType(m, ns, outer, outerIsInterface ? "public" : "private");

    private void ExtractMember(MemberDeclarationSyntax m, string typeName, bool isInterface, JsonArray output, ref int privateCount)
    {
        var defaultAccess = isInterface ? "public" : "private";
        switch (m)
        {
            case FieldDeclarationSyntax f:
            {
                var access = Accessibility(f.Modifiers, defaultAccess);
                foreach (var v in f.Declaration.Variables)
                {
                    if (access == "private") { privateCount++; continue; }
                    var type = Normalize(f.Declaration.Type.ToString());
                    var o = Member("field", v.Identifier.Text, access, f.Modifiers, f.AttributeLists, m);
                    o["type"] = type;
                    o["signature"] = Normalize($"{ModifierText(f.Modifiers)}{type} {v.Identifier.Text}");
                    Flag(o, type, v.Identifier.Text, v.Initializer?.Value);
                    output.Add(o);
                }

                break;
            }
            case EventFieldDeclarationSyntax ef:
            {
                var access = Accessibility(ef.Modifiers, defaultAccess);
                foreach (var v in ef.Declaration.Variables)
                {
                    if (access == "private") { privateCount++; continue; }
                    var type = Normalize(ef.Declaration.Type.ToString());
                    var o = Member("event", v.Identifier.Text, access, ef.Modifiers, ef.AttributeLists, m);
                    o["type"] = type;
                    o["signature"] = Normalize($"{ModifierText(ef.Modifiers)}event {type} {v.Identifier.Text}");
                    output.Add(o);
                }

                break;
            }
            case EventDeclarationSyntax e:
            {
                var explicitIface = e.ExplicitInterfaceSpecifier?.Name.ToString();
                var access = explicitIface is null ? Accessibility(e.Modifiers, defaultAccess) : "explicit";
                if (access == "private") { privateCount++; break; }
                var type = Normalize(e.Type.ToString());
                var o = Member("event", e.Identifier.Text, access, e.Modifiers, e.AttributeLists, m);
                o["type"] = type;
                if (explicitIface is not null)
                    o["explicitInterface"] = Normalize(explicitIface);
                o["signature"] = Normalize($"{ModifierText(e.Modifiers)}event {type} {(explicitIface is null ? "" : explicitIface + ".")}{e.Identifier.Text}");
                output.Add(o);
                break;
            }
            case PropertyDeclarationSyntax p:
            {
                var explicitIface = p.ExplicitInterfaceSpecifier?.Name.ToString();
                var access = explicitIface is null ? Accessibility(p.Modifiers, defaultAccess) : "explicit";
                if (access == "private") { privateCount++; break; }
                var type = Normalize(p.Type.ToString());
                var o = Member("property", p.Identifier.Text, access, p.Modifiers, p.AttributeLists, m);
                o["type"] = type;
                if (explicitIface is not null)
                    o["explicitInterface"] = Normalize(explicitIface);
                var accessors = Accessors(p.AccessorList, p.ExpressionBody is not null, access);
                o["accessors"] = accessors;
                o["signature"] = Normalize($"{ModifierText(p.Modifiers)}{type} {(explicitIface is null ? "" : explicitIface + ".")}{p.Identifier.Text} {AccessorText(accessors, access)}");
                Flag(o, type, p.Identifier.Text, p.Initializer?.Value);
                output.Add(o);
                break;
            }
            case IndexerDeclarationSyntax ix:
            {
                var explicitIface = ix.ExplicitInterfaceSpecifier?.Name.ToString();
                var access = explicitIface is null ? Accessibility(ix.Modifiers, defaultAccess) : "explicit";
                if (access == "private") { privateCount++; break; }
                var type = Normalize(ix.Type.ToString());
                var o = Member("indexer", "this[]", access, ix.Modifiers, ix.AttributeLists, m);
                o["type"] = type;
                if (explicitIface is not null)
                    o["explicitInterface"] = Normalize(explicitIface);
                var accessors = Accessors(ix.AccessorList, ix.ExpressionBody is not null, access);
                o["accessors"] = accessors;
                o["params"] = ix.ParameterList.Parameters.Count;
                o["signature"] = Normalize($"{ModifierText(ix.Modifiers)}{type} {(explicitIface is null ? "" : explicitIface + ".")}this{ix.ParameterList} {AccessorText(accessors, access)}");
                output.Add(o);
                break;
            }
            case MethodDeclarationSyntax me:
            {
                var explicitIface = me.ExplicitInterfaceSpecifier?.Name.ToString();
                var access = explicitIface is null ? Accessibility(me.Modifiers, defaultAccess) : "explicit";
                if (access == "private") { privateCount++; break; }
                var o = Member("method", me.Identifier.Text, access, me.Modifiers, me.AttributeLists, m);
                o["type"] = Normalize(me.ReturnType.ToString());
                if (explicitIface is not null)
                    o["explicitInterface"] = Normalize(explicitIface);
                o["params"] = me.ParameterList.Parameters.Count;
                if (me.TypeParameterList is { } tp)
                    o["arity"] = tp.Parameters.Count;
                if (me.ParameterList.Parameters.Count > 0 && me.ParameterList.Parameters[0].Modifiers.Any(SyntaxKind.ThisKeyword))
                    o["extension"] = true;
                if (isInterface && (me.Body is not null || me.ExpressionBody is not null))
                    o["defaultImplementation"] = true;
                var constraints = me.ConstraintClauses.Count > 0 ? " " + string.Join(" ", me.ConstraintClauses.Select(c => c.ToString())) : "";
                o["signature"] = Normalize($"{ModifierText(me.Modifiers)}{me.ReturnType} {(explicitIface is null ? "" : explicitIface + ".")}{me.Identifier.Text}{me.TypeParameterList}{me.ParameterList}{constraints}");
                output.Add(o);
                break;
            }
            case ConstructorDeclarationSyntax c:
            {
                var isStatic = c.Modifiers.Any(SyntaxKind.StaticKeyword);
                var access = isStatic ? "static" : Accessibility(c.Modifiers, defaultAccess);
                if (access == "private") { privateCount++; break; }
                var o = Member(isStatic ? "static ctor" : "ctor", isStatic ? ".cctor" : ".ctor", access, c.Modifiers, c.AttributeLists, m);
                o["params"] = c.ParameterList.Parameters.Count;
                o["signature"] = Normalize($"{ModifierText(c.Modifiers)}{typeName}{c.ParameterList}");
                output.Add(o);
                break;
            }
            case DestructorDeclarationSyntax d:
            {
                var o = Member("finalizer", "~" + typeName, "protected", d.Modifiers, d.AttributeLists, m);
                o["signature"] = $"~{typeName}()";
                output.Add(o);
                break;
            }
            case OperatorDeclarationSyntax op:
            {
                var access = Accessibility(op.Modifiers, defaultAccess);
                if (access == "private") { privateCount++; break; }
                var o = Member("operator", "operator " + op.OperatorToken.Text, access, op.Modifiers, op.AttributeLists, m);
                o["type"] = Normalize(op.ReturnType.ToString());
                o["params"] = op.ParameterList.Parameters.Count;
                o["paramTypes"] = new JsonArray(op.ParameterList.Parameters.Select(p => (JsonNode)Normalize(p.Type?.ToString() ?? "")).ToArray());
                o["signature"] = Normalize($"{ModifierText(op.Modifiers)}{op.ReturnType} operator {op.OperatorToken.Text}{op.ParameterList}");
                output.Add(o);
                break;
            }
            case ConversionOperatorDeclarationSyntax conv:
            {
                var access = Accessibility(conv.Modifiers, defaultAccess);
                if (access == "private") { privateCount++; break; }
                var o = Member("operator", conv.ImplicitOrExplicitKeyword.Text + " operator", access, conv.Modifiers, conv.AttributeLists, m);
                o["type"] = Normalize(conv.Type.ToString());
                o["params"] = 1;
                o["paramTypes"] = new JsonArray(conv.ParameterList.Parameters.Select(p => (JsonNode)Normalize(p.Type?.ToString() ?? "")).ToArray());
                o["conversion"] = conv.ImplicitOrExplicitKeyword.Text;
                o["signature"] = Normalize($"{ModifierText(conv.Modifiers)}{conv.ImplicitOrExplicitKeyword.Text} operator {conv.Type}{conv.ParameterList}");
                output.Add(o);
                break;
            }
        }
    }

    private static JsonObject Member(string kind, string name, string access, SyntaxTokenList modifiers, SyntaxList<AttributeListSyntax> attributes, SyntaxNode node)
    {
        var o = new JsonObject
        {
            ["kind"] = kind,
            ["name"] = name,
            ["accessibility"] = access,
        };
        var mods = new JsonArray();
        foreach (var mod in modifiers)
        {
            if (mod.Kind() is SyntaxKind.StaticKeyword or SyntaxKind.VirtualKeyword or SyntaxKind.AbstractKeyword or SyntaxKind.OverrideKeyword
                or SyntaxKind.SealedKeyword or SyntaxKind.ReadOnlyKeyword or SyntaxKind.ConstKeyword or SyntaxKind.NewKeyword
                or SyntaxKind.ExternKeyword or SyntaxKind.AsyncKeyword or SyntaxKind.RequiredKeyword or SyntaxKind.UnsafeKeyword
                or SyntaxKind.PartialKeyword or SyntaxKind.VolatileKeyword)
                mods.Add(mod.Text);
        }

        if (mods.Count > 0)
            o["modifiers"] = mods;
        AddAttributes(o, attributes);
        o["line"] = Line(node);
        return o;
    }

    private static void Flag(JsonObject o, string type, string name, ExpressionSyntax? initializer)
    {
        var bare = type.TrimEnd('?');
        var lt = bare.IndexOf('<');
        var head = lt < 0 ? bare : bare[..lt];
        var dot = head.LastIndexOf('.');
        if (dot >= 0)
            head = head[(dot + 1)..];
        var isStatic = o["modifiers"] is JsonArray mods && mods.Any(x => x!.GetValue<string>() == "static");
        if (!isStatic)
            return;
        switch (head)
        {
            case "StyledProperty": o["avaloniaProperty"] = "styled"; break;
            case "DirectProperty": o["avaloniaProperty"] = "direct"; break;
            case "AttachedProperty": o["avaloniaProperty"] = "attached"; break;
            case "AvaloniaProperty": o["avaloniaProperty"] = "untyped"; break;
            case "RoutedEvent": o["routedEvent"] = true; break;
        }

        if (o["avaloniaProperty"] is not null && initializer is not null)
        {
            // Registration call (Register / RegisterAttached / RegisterDirect / AddOwner) and the XAML-visible name.
            var text = initializer.ToString();
            var m = Regex.Match(text, @"\b(Register(?:Attached|Direct)?|AddOwner)\s*<");
            if (m.Success)
                o["registration"] = m.Groups[1].Value;
            var nm = Regex.Match(text, "\\(\\s*(?:nameof\\((\\w+)\\)|\"([^\"]+)\")");
            if (nm.Success)
                o["registeredName"] = nm.Groups[1].Success ? nm.Groups[1].Value : nm.Groups[2].Value;
        }
    }

    private static JsonObject Accessors(AccessorListSyntax? list, bool expressionBodied, string propertyAccess)
    {
        var o = new JsonObject();
        if (expressionBodied || list is null)
        {
            o["get"] = propertyAccess;
            return o;
        }

        foreach (var a in list.Accessors)
        {
            var key = a.Keyword.Text; // get / set / init / add / remove
            o[key] = a.Modifiers.Count > 0 ? Accessibility(a.Modifiers, propertyAccess) : propertyAccess;
        }

        return o;
    }

    private static string AccessorText(JsonObject accessors, string propertyAccess)
    {
        var sb = new StringBuilder("{ ");
        foreach (var (k, v) in accessors)
        {
            var access = v!.GetValue<string>();
            sb.Append(access == propertyAccess ? "" : access + " ").Append(k).Append("; ");
        }

        return sb.Append('}').ToString();
    }

    private static string ModifierText(SyntaxTokenList modifiers)
    {
        var sb = new StringBuilder();
        foreach (var m in modifiers)
        {
            if (m.Kind() is SyntaxKind.StaticKeyword or SyntaxKind.VirtualKeyword or SyntaxKind.AbstractKeyword or SyntaxKind.OverrideKeyword
                or SyntaxKind.SealedKeyword or SyntaxKind.ReadOnlyKeyword or SyntaxKind.ConstKeyword or SyntaxKind.NewKeyword
                or SyntaxKind.AsyncKeyword or SyntaxKind.RequiredKeyword)
                sb.Append(m.Text).Append(' ');
        }

        return sb.ToString();
    }

    private static string Accessibility(SyntaxTokenList modifiers, string defaultAccess)
    {
        bool pub = false, prot = false, intern = false, priv = false, file = false;
        foreach (var m in modifiers)
        {
            switch (m.Kind())
            {
                case SyntaxKind.PublicKeyword: pub = true; break;
                case SyntaxKind.ProtectedKeyword: prot = true; break;
                case SyntaxKind.InternalKeyword: intern = true; break;
                case SyntaxKind.PrivateKeyword: priv = true; break;
                case SyntaxKind.FileKeyword: file = true; break;
            }
        }

        if (file) return "file";
        if (pub) return "public";
        if (prot && intern) return "protected internal";
        if (prot && priv) return "private protected";
        if (prot) return "protected";
        if (intern) return "internal";
        if (priv) return "private";
        return defaultAccess;
    }

    private static JsonArray Attributes(SyntaxList<AttributeListSyntax> lists)
    {
        var arr = new JsonArray();
        foreach (var l in lists)
        {
            foreach (var a in l.Attributes)
                arr.Add(Normalize((l.Target is null ? "" : l.Target.Identifier.Text + ": ") + a));
        }

        return arr;
    }

    private static bool HasAttribute(SyntaxList<AttributeListSyntax> lists, string name)
    {
        foreach (var l in lists)
        {
            foreach (var a in l.Attributes)
            {
                var n = a.Name.ToString();
                var dot = n.LastIndexOf('.');
                if (dot >= 0)
                    n = n[(dot + 1)..];
                if (n == name || n == name + "Attribute")
                    return true;
            }
        }

        return false;
    }

    private static void AddAttributes(JsonObject o, SyntaxList<AttributeListSyntax> lists)
    {
        var attrs = Attributes(lists);
        if (attrs.Count > 0)
            o["attributes"] = attrs;
    }

    private static int Line(SyntaxNode node) => node.GetLocation().GetLineSpan().StartLinePosition.Line + 1;

    private static readonly Regex s_ws = new(@"\s+", RegexOptions.Compiled);
    private static readonly Regex s_comment = new(@"/\*.*?\*/|//[^\n]*", RegexOptions.Compiled | RegexOptions.Singleline);

    private static string Normalize(string s)
    {
        if (s.Contains('/'))
            s = s_comment.Replace(s, " ");
        s = s_ws.Replace(s, " ").Trim();
        return s.Replace("( ", "(").Replace(" )", ")").Replace(" ,", ",");
    }
}

/// <summary>Regex pass over a MicroCom IDL file: interfaces with their methods, enums, structs.</summary>
internal static class IdlExtractor
{
    public static JsonObject Extract(string file, string text)
    {
        text = Regex.Replace(text, @"/\*.*?\*/", " ", RegexOptions.Singleline);
        text = Regex.Replace(text, @"//[^\n]*", " ");
        var interfaces = new JsonArray();
        var enums = new JsonArray();
        var structs = new JsonArray();

        foreach (Match m in Regex.Matches(text, @"(?<attrs>(?:\[[^\]]*\]\s*)*)\b(?<kind>interface|enum|struct)\s+(?<name>\w+)\s*(?::\s*(?<base>\w+))?\s*\{(?<body>[^{}]*)\}"))
        {
            var kind = m.Groups["kind"].Value;
            var name = m.Groups["name"].Value;
            var body = m.Groups["body"].Value;
            var attrs = Regex.Matches(m.Groups["attrs"].Value, @"\[([^\]]*)\]").Select(a => a.Groups[1].Value.Trim()).ToArray();
            switch (kind)
            {
                case "interface":
                {
                    var methods = new JsonArray();
                    foreach (var raw in body.Split(';'))
                    {
                        var decl = Regex.Replace(raw, @"\s+", " ").Trim();
                        if (decl.Length == 0)
                            continue;
                        var mm = Regex.Match(decl, @"(\w+)\s*\(");
                        methods.Add(new JsonObject
                        {
                            ["name"] = mm.Success ? mm.Groups[1].Value : decl,
                            ["signature"] = decl,
                        });
                    }

                    var o = new JsonObject { ["name"] = name };
                    if (m.Groups["base"].Success)
                        o["base"] = m.Groups["base"].Value;
                    var uuid = attrs.Select(a => Regex.Match(a, @"uuid\(([^)]+)\)")).FirstOrDefault(x => x.Success);
                    if (uuid is not null)
                        o["uuid"] = uuid.Groups[1].Value;
                    o["methods"] = methods;
                    interfaces.Add(o);
                    break;
                }
                case "enum":
                {
                    var members = new JsonArray();
                    foreach (var raw in body.Split(','))
                    {
                        var decl = Regex.Replace(raw, @"\s+", " ").Trim();
                        if (decl.Length == 0)
                            continue;
                        members.Add(Regex.Match(decl, @"^\w+").Value);
                    }

                    var o = new JsonObject { ["name"] = name, ["members"] = members };
                    if (attrs.Length > 0)
                        o["attributes"] = new JsonArray(attrs.Select(a => (JsonNode)a).ToArray());
                    enums.Add(o);
                    break;
                }
                default:
                {
                    var fields = new JsonArray();
                    foreach (var raw in body.Split(';'))
                    {
                        var decl = Regex.Replace(raw, @"\s+", " ").Trim();
                        if (decl.Length == 0)
                            continue;
                        fields.Add(decl);
                    }

                    structs.Add(new JsonObject { ["name"] = name, ["fields"] = fields });
                    break;
                }
            }
        }

        return new JsonObject
        {
            ["file"] = file,
            ["interfaces"] = interfaces,
            ["enums"] = enums,
            ["structs"] = structs,
        };
    }
}
