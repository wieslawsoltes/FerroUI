// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Text.Json;
using Dock.Model.Core;
using Dock.Serializer.SystemTextJson;
internal static class CoreModelReference
{
 internal static void Run(string prefix){
  var serializer=new DockSerializer();
  using var checks=new StreamWriter(prefix+".tsv");
  void Check(string name,bool value){if(!value)throw new InvalidOperationException(name);checks.WriteLine(name+"\t1");}
  var rect=new DockRect(-0d,0.125,double.PositiveInfinity,double.NaN);
  var rectText=serializer.Serialize(rect);File.WriteAllText(prefix+".rect.json",rectText);
  var rectRead=serializer.Deserialize<DockRect>(rectText);
  Check("rect-value-no-reference-id",!rectText.Contains("$id"));
  Check("rect-components-and-signed-zero",BitConverter.DoubleToInt64Bits(rectRead.X)==long.MinValue&&rectRead.Y==0.125&&double.IsPositiveInfinity(rectRead.Width)&&double.IsNaN(rectRead.Height));
  var policy=new DockCapabilityPolicy{CanClose=false,CanPin=true};
  var overrides=new DockCapabilityOverrides{CanFloat=false,CanDrop=true};
  var policyText=serializer.Serialize(policy);var overrideText=serializer.Serialize(overrides);
  File.WriteAllText(prefix+".policy.json",policyText);File.WriteAllText(prefix+".overrides.json",overrideText);
  Check("policy-null-values-omitted",!policyText.Contains("CanDrag")&&policyText.Contains("false"));
  Check("override-derived-readonly-property-ignored",!overrideText.Contains("HasAnyOverride"));
  Check("policy-default-serialization",serializer.Serialize(new DockCapabilityPolicy())=="{\n  \"$id\": \"1\"\n}");
  var holder=new CoreCapabilityHolder{Overrides=overrides,Policy=overrides,Contract=overrides,Bounds=rect,OtherBounds=rect};
  var holderText=serializer.Serialize(holder);File.WriteAllText(prefix+".holder.json",holderText);
  var held=serializer.Deserialize<CoreCapabilityHolder>(holderText)!;
  Check("override-upcast-shared-reference",ReferenceEquals(held.Overrides,held.Policy)&&ReferenceEquals(held.Policy,held.Contract));
  held.Policy!.CanClose=true;Check("same-original-override-state",held.Overrides!.CanClose==true&&held.Overrides.HasAnyOverride);
  Check("nullable-bounds-are-value-copies",held.Bounds.HasValue&&held.OtherBounds.HasValue&&BitConverter.DoubleToInt64Bits(held.Bounds.Value.X)==long.MinValue);
  IDockCapabilityPolicy contract=policy;
  File.WriteAllText(prefix+".contract.json",serializer.Serialize(contract));
  Check("policy-interface-read-has-no-fabricated-constructor",Throws<NotSupportedException>(()=>serializer.Deserialize<IDockCapabilityPolicy>("{}")));
  var baseFirst=new CoreBaseFirst{Policy=overrides,Overrides=overrides};
  File.WriteAllText(prefix+".base-first.json",serializer.Serialize(baseFirst));
  Check("base-first-cannot-recover-derived-class",Throws<InvalidOperationException>(()=>serializer.Deserialize<CoreBaseFirst>(serializer.Serialize(baseFirst)))||Throws<JsonException>(()=>serializer.Deserialize<CoreBaseFirst>(serializer.Serialize(baseFirst))));
  var reads=new List<string[]>();
  void Read<T>(string kind,string json){try{var value=serializer.Deserialize<T>(json);reads.Add(new[]{kind,json,"OK",serializer.Serialize(value)});}catch(Exception e){reads.Add(new[]{kind,json,"ERR",e.GetType().Name});}}
  foreach(var json in new[]{"{}","null","[]","17","{\"X\":-0}","{\"x\":12}","{\"X\":1,\"X\":2}","{\"Width\":\"Infinity\",\"Height\":\"NaN\"}","{\"X\":null}","{\"X\":\"1\"}","{\"extra\":{\"nested\":[null,1]}}","{\"$id\":\"a\",\"X\":3}","{\"$id\":17,\"X\":3}","{\"X\":3,\"$id\":\"a\"}","{\"$id\":\"a\",\"$id\":\"b\"}","{\"$ref\":\"a\"}","{\"$values\":[1,2]}","{\"$type\":\"unknown\",\"X\":5}","{\"$other\":1}","{\"$id\":null}"})Read<DockRect>("RECT",json);
  Read<CoreDollarProperty>("DOLLAR","{\"$other\":7}");
  foreach(var json in new[]{"{}","null","[]","{\"CanClose\":false}","{\"CanClose\":true,\"CanClose\":null}","{\"CanClose\":1}","{\"CanClose\":\"true\"}","{\"canClose\":true}","{\"HasAnyOverride\":true,\"CanPin\":false}","{\"$id\":\"a\",\"CanClose\":true}","{\"$ref\":\"a\"}","{\"$type\":\"x\"}","{\"$other\":1}","{\"$values\":[]}"}){Read<DockCapabilityPolicy>("POLICY",json);Read<DockCapabilityOverrides>("OVERRIDE",json);}
  File.WriteAllText(prefix+".reads.json",JsonSerializer.Serialize(reads,new JsonSerializerOptions{WriteIndented=true}));
 }
 static bool Throws<T>(Action action)where T:Exception{try{action();return false;}catch(T){return true;}}
}
public sealed class CoreCapabilityHolder{
 public DockCapabilityOverrides? Overrides{get;set;}public DockCapabilityPolicy? Policy{get;set;}public IDockCapabilityPolicy? Contract{get;set;}public DockRect? Bounds{get;set;}public DockRect? OtherBounds{get;set;}}
public sealed class CoreBaseFirst{public DockCapabilityPolicy? Policy{get;set;}public DockCapabilityOverrides? Overrides{get;set;}}

public sealed class CoreDollarProperty{[System.Text.Json.Serialization.JsonPropertyName("$other")]public int Value{get;set;}}
