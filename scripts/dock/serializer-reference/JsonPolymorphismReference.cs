// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Text.Json.Serialization.Metadata;
using Dock.Serializer.SystemTextJson;

internal static class JsonPolymorphismReference
{
    internal static void Run(string path)
    {
        var writes = new List<object>(); var reads = new List<object>();
        DockSerializer Serializer(JsonUnknownDerivedTypeHandling policy, bool ambiguity = false, bool exact = false, bool empty = false) {
            var resolver = new DefaultJsonTypeInfoResolver();
            resolver.Modifiers.Add(info => {
                if (info.Type != typeof(IPolyValue)) return;
                info.PolymorphismOptions = new JsonPolymorphismOptions { UnknownDerivedTypeHandling = policy, IgnoreUnrecognizedTypeDiscriminators = true };
                if (empty) return;
                if (ambiguity) {
                    info.PolymorphismOptions.DerivedTypes.Add(new JsonDerivedType(typeof(IPolyLeft), "left"));
                    info.PolymorphismOptions.DerivedTypes.Add(new JsonDerivedType(typeof(IPolyRight), "right"));
                    if (exact) info.PolymorphismOptions.DerivedTypes.Add(new JsonDerivedType(typeof(PolyBoth), "both"));
                } else {
                    info.PolymorphismOptions.DerivedTypes.Add(new JsonDerivedType(typeof(PolyA), "a"));
                    info.PolymorphismOptions.DerivedTypes.Add(new JsonDerivedType(typeof(PolyB), "b"));
                }
            });
            return new DockSerializer(resolver);
        }
        var empty=Serializer(JsonUnknownDerivedTypeHandling.FailSerialization,empty:true);
        string emptyWrite="",emptyRead="";
        try{empty.Serialize<IPolyValue>(new PolyA());}catch(InvalidOperationException){emptyWrite="invalid-config";}
        try{empty.Deserialize<IPolyValue>("null");}catch(InvalidOperationException){emptyRead="invalid-config";}
        writes.Add(new{policy="empty",policyValue=0,kind="a",wire="",error=emptyWrite});
        reads.Add(new{policy="empty",input="null",kind="",wire="",error=emptyRead,self=false});
        foreach (var policy in Enum.GetValues<JsonUnknownDerivedTypeHandling>()) {
            var serializer = Serializer(policy);
            foreach (var (kind,value) in new (string,IPolyValue)[] { ("a",new PolyA {Name="value",A=11}), ("b",new PolyB {Name="value",A=11,B=22}), ("c",new PolyC {Name="value",A=11,B=22,C=33}), ("x",new PolyX {Name="value",X=44}) }) {
                string wire="",error="";try {wire=serializer.Serialize<IPolyValue>(value);}catch(NotSupportedException){error="unsupported";}
                writes.Add(new{policy=policy.ToString(),policyValue=(int)policy,kind,wire,error});
            }
            foreach(string input in new[]{"null","{\"$type\":\"a\",\"Name\":\"read\",\"A\":11}","{\"$id\":\"one\",\"$type\":\"b\",\"B\":22,\"Name\":\"read\",\"Next\":{\"$ref\":\"one\"},\"A\":11}","{\"Name\":\"base\"}","{\"$type\":\"future\",\"Name\":\"base\"}"}) {
                string kind="",wire="",error="";bool self=false;
                try {var value=serializer.Deserialize<IPolyValue>(input);kind=value?.GetType().Name??"null";self=value is not null&&ReferenceEquals(value,value.Next);wire=serializer.Serialize<IPolyValue?>(value);}
                catch(NotSupportedException){error="unsupported";}catch(JsonException){error="json";}
                reads.Add(new{policy=policy.ToString(),input,kind,wire,error,self});
            }
        }
        var near=Serializer(JsonUnknownDerivedTypeHandling.FallBackToNearestAncestor);
        var original=new PolyC{Name="cycle",A=11,B=22,C=33};original.Next=original;
        var holder=new PolyHolder{First=original,Second=original};var text=near.Serialize(holder);
        File.WriteAllText(path+".holder.json",text);
        var loaded=near.Deserialize<PolyHolder>(text)!;
        using var checks=new StreamWriter(path+".tsv");
        void Check(string name,bool value){if(!value)throw new InvalidOperationException(name);checks.WriteLine(name+"\t1");}
        Check("polymorphic-nearest-registered-class",loaded.First is PolyB && loaded.First is not PolyC);
        Check("polymorphic-nearest-actual-shared-listener",ReferenceEquals(loaded.First,loaded.Second));
        Check("polymorphic-nearest-self-reference",ReferenceEquals(loaded.First,loaded.First!.Next));
        foreach(bool exact in new[]{false,true}) {
            var serializer=Serializer(JsonUnknownDerivedTypeHandling.FallBackToNearestAncestor,true,exact);string wire="",error="";
            try{wire=serializer.Serialize<IPolyValue>(new PolyBoth{Name="both",L=1,R=2});}catch(NotSupportedException){error="unsupported";}
            writes.Add(new{policy="ambiguity",policyValue=2,kind=exact?"both-exact":"both",wire,error});
            Check(exact?"polymorphic-exact-beats-ambiguous-ancestors":"polymorphic-ambiguous-interface-ancestors-rejected",exact?error==""&&wire.Contains("\"both\""):error=="unsupported");
        }
        File.WriteAllText(path+".json",JsonSerializer.Serialize(new{writes,reads},new JsonSerializerOptions{WriteIndented=true}));
        Console.WriteLine("Original polymorphic runtime policy cases passed.");
    }
}
public interface IPolyValue { string? Name {get;set;} IPolyValue? Next {get;set;} }
public class PolyA : IPolyValue { public string? Name {get;set;} public IPolyValue? Next {get;set;} public int A {get;set;} }
public class PolyB : PolyA { public int B {get;set;} }
public sealed class PolyC : PolyB { public int C {get;set;} }
public sealed class PolyX : IPolyValue { public string? Name {get;set;} public IPolyValue? Next {get;set;} public int X {get;set;} }
public interface IPolyLeft : IPolyValue {int L {get;set;}}
public interface IPolyRight : IPolyValue {int R {get;set;}}
public sealed class PolyBoth : IPolyLeft, IPolyRight { public string? Name {get;set;} public IPolyValue? Next {get;set;} public int L {get;set;} public int R {get;set;} }
public sealed class PolyHolder {public IPolyValue? First {get;set;} public IPolyValue? Second {get;set;}}
