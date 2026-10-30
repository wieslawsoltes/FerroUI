// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Text;
using System.Text.Json;
using Dock.Serializer.SystemTextJson;
if(args.Length!=2)throw new ArgumentException("Pass stream and source-only JSON corpus paths");
using var output=new StreamWriter(args[0]);
void Check(string name,bool value){if(!value)throw new InvalidOperationException(name);output.WriteLine(name+"\t1");}
bool Throws<T>(Action action)where T:Exception{try{action();return false;}catch(T){return true;}}
var stream=new MemoryStream();var alias=stream;
Check("memory-new-capabilities",stream.CanRead&&stream.CanWrite&&stream.CanSeek&&stream.Length==0&&stream.Position==0);
stream.Write(new byte[]{1,2,3});Check("memory-write-position-length",stream.Position==3&&stream.Length==3);
stream.Seek(0,SeekOrigin.Begin);Check("memory-read-position",stream.ReadByte()==1&&stream.Position==1);
stream.Seek(5,SeekOrigin.Begin);stream.WriteByte(9);Check("memory-expand-zero-gap",stream.ToArray().SequenceEqual(new byte[]{1,2,3,0,0,9})&&stream.Position==6);
stream.SetLength(2);Check("memory-truncate-clamps-position",stream.Position==2&&stream.Length==2);
stream.SetLength(5);Check("memory-regrow-zeroes",stream.ToArray().SequenceEqual(new byte[]{1,2,0,0,0}));
stream.Position=4;Check("memory-negative-seek-keeps-position",Throws<IOException>(()=>stream.Seek(-5,SeekOrigin.Current))&&stream.Position==4);
stream.Close();Check("memory-close-shared-capabilities",!alias.CanRead&&!alias.CanWrite&&!alias.CanSeek);
Check("memory-close-read-rejected",Throws<ObjectDisposedException>(()=>alias.ReadByte()));
Check("memory-close-write-rejected",Throws<ObjectDisposedException>(()=>alias.WriteByte(1)));
Check("memory-close-seek-rejected",Throws<ObjectDisposedException>(()=>alias.Seek(0,SeekOrigin.Begin)));
Check("memory-close-length-rejected",Throws<ObjectDisposedException>(()=>{var x=alias.Length;}));
Check("memory-close-position-rejected",Throws<ObjectDisposedException>(()=>{var x=alias.Position;}));
Check("memory-close-set-length-rejected",Throws<NotSupportedException>(()=>alias.SetLength(0)));
Check("memory-closed-negative-position-check-first",Throws<ArgumentOutOfRangeException>(()=>alias.Position=-1));
Check("memory-closed-overflow-position-check-after-close",Throws<ObjectDisposedException>(()=>alias.Position=(long)int.MaxValue+1));
Check("memory-closed-invalid-length-check-first",Throws<ArgumentOutOfRangeException>(()=>alias.SetLength(-1)));
Check("memory-close-snapshot-retained",alias.ToArray().SequenceEqual(new byte[]{1,2,0,0,0}));
alias.Close();alias.Dispose();alias.Flush();Check("memory-close-repeat-flush",!alias.CanRead&&alias.ToArray().Length==5);
var bytes=new byte[]{3,4,5};stream=new MemoryStream(bytes,true);stream.WriteByte(8);Check("memory-fixed-buffer-original-identity",bytes[0]==8);
bytes[1]=7;Check("memory-fixed-buffer-live-read",stream.ReadByte()==7);
Check("memory-fixed-buffer-expand-rejected",Throws<NotSupportedException>(()=>stream.SetLength(4)));
stream.SetLength(1);stream.SetLength(3);Check("memory-fixed-regrow-zeroes-original-buffer",bytes.SequenceEqual(new byte[]{8,0,0}));
var readOnly=new MemoryStream(new byte[]{1,2},false);Check("memory-readonly-capabilities",readOnly.CanRead&&!readOnly.CanWrite&&readOnly.CanSeek);
Check("memory-readonly-write-rejected",Throws<NotSupportedException>(()=>readOnly.WriteByte(9)));
Check("memory-readonly-set-length-rejected",Throws<NotSupportedException>(()=>readOnly.SetLength(1)));
readOnly.Flush();Check("memory-readonly-flush-allowed",readOnly.Position==0);
stream=new MemoryStream();stream.Position=5;stream.Write(Array.Empty<byte>());Check("memory-empty-write-extends",stream.Length==5&&stream.Position==5&&stream.ToArray().All(b=>b==0));
stream=new MemoryStream();stream.Position=5;Check("memory-past-end-empty-read",stream.Read(Array.Empty<byte>())==0&&stream.Position==5&&stream.Length==0);
Check("memory-max-position-without-allocation",stream.Seek(int.MaxValue,SeekOrigin.Begin)==int.MaxValue&&stream.Length==0);
Check("memory-position-overflow-rejected",Throws<ArgumentOutOfRangeException>(()=>stream.Position=(long)int.MaxValue+1)&&stream.Position==int.MaxValue);
Check("memory-seek-overflow-rejected",Throws<ArgumentOutOfRangeException>(()=>stream.Seek((long)int.MaxValue+1,SeekOrigin.Begin))&&stream.Position==int.MaxValue);
Check("memory-length-overflow-rejected",Throws<ArgumentOutOfRangeException>(()=>stream.SetLength((long)int.MaxValue+1))&&stream.Length==0);
var contract=new StreamContractProbe();Dock.Model.Core.IDockSerializer api=contract;var argument=new Payload{Name="same"};
api.Load<Payload>(null!);Check("interface-load-null-dispatch",contract.SeenStream is null&&contract.SeenType==typeof(Payload));
api.Save<Payload>(null!,argument);Check("interface-save-null-dispatch",contract.SeenStream is null&&contract.SeenType==typeof(Payload)&&ReferenceEquals(contract.SeenValue,argument));
var original=new MemoryStream();api.Load<string>(original);Check("interface-load-original-shared-stream",ReferenceEquals(contract.SeenStream,original)&&contract.SeenType==typeof(string));
api.Save<int>(original,17);Check("interface-save-original-stream-and-generic-payload",ReferenceEquals(contract.SeenStream,original)&&contract.SeenType==typeof(int)&&Equals(contract.SeenValue,17));
output.Flush();
using var audit=new StreamWriter(args[1]);
void Audit(string name,bool value){if(!value)throw new InvalidOperationException(name);audit.WriteLine(name+"\t1\tMATCHED");}
JsonModelReference.Run(Path.ChangeExtension(args[1],".model.tsv"));
CoreLayoutCatalogReference.Run(Path.ChangeExtension(args[1],".layout"));
SampleLayoutReference.Run(Path.ChangeExtension(args[1],".sample-layout"));
JsonFallbackReference.Run(Path.ChangeExtension(args[1],".fallback.tsv"));
JsonNumberReference.Run(Path.ChangeExtension(args[1],".numbers.tsv"));
GeneratedPayloadReference.Run(Path.ChangeExtension(args[1],".generated"));
JsonPolymorphismReference.Run(Path.ChangeExtension(args[1],".polymorphic"));
AttributeReference.Run(Path.ChangeExtension(args[1],".attributes.tsv"));
CoreModelReference.Run(Path.ChangeExtension(args[1],".core"));
CoreValueReference.Run(Path.ChangeExtension(args[1],".values"));
WindowEnumReference.Run(Path.ChangeExtension(args[1],".window"));
DockEnumReference.Run(Path.ChangeExtension(args[1],".dock-enums"));
AccessorFailureReference.Run(Path.ChangeExtension(args[1],".accessors.json"));
var serializer=new DockSerializer();var payload=new Payload{Name="alpha"};payload.Self=payload;
var text=serializer.Serialize(payload);using(var doc=JsonDocument.Parse(text)){
 Audit("json-readonly-property-is-included",doc.RootElement.GetProperty("ReadOnly").GetInt32()==7);
 Audit("json-self-reference-preserved-on-wire",doc.RootElement.GetProperty("Self").GetProperty("$ref").GetString()==doc.RootElement.GetProperty("$id").GetString());
 Audit("json-null-property-omitted",!doc.RootElement.TryGetProperty("Other",out _));}
var restored=serializer.Deserialize<Payload>(text);Audit("json-self-reference-restored",ReferenceEquals(restored,restored!.Self));
var saved=new MemoryStream();serializer.Save(saved,payload);Audit("json-save-closes-stream",!saved.CanRead&&!saved.CanWrite);
Audit("json-save-emits-utf8-preamble",saved.ToArray().Take(3).SequenceEqual(new byte[]{239,187,191}));
var loaded=new MemoryStream(saved.ToArray());var loadedPayload=serializer.Load<Payload>(loaded);Audit("json-load-closes-stream",!loaded.CanRead&&loadedPayload!.Name=="alpha");
var empty=new MemoryStream();Audit("json-empty-load-throws",Throws<JsonException>(()=>serializer.Load<Payload>(empty)));Audit("json-empty-load-still-closes",!empty.CanRead);
var malformed=new MemoryStream(Encoding.UTF8.GetBytes("{"));Audit("json-malformed-load-throws",Throws<JsonException>(()=>serializer.Load<Payload>(malformed)));Audit("json-malformed-load-still-closes",!malformed.CanRead);
Audit("json-null-load-rejected",Throws<ArgumentNullException>(()=>serializer.Load<Payload>(null!)));
Audit("json-null-save-rejected",Throws<ArgumentNullException>(()=>serializer.Save<Payload>(null!,payload)));

var wires=new (string Name,string Value)[]{("ascii","alpha"),("quote","\""),("slash","\\/"),("html","<>&'+`"),("controls","\n\r\t\b\f\0"),("unicode","é界🙂")};
using(var wire=new StreamWriter(Path.ChangeExtension(args[1],".wire.tsv"))){foreach(var item in wires)wire.WriteLine(item.Name+"\t"+serializer.Serialize(item.Value));wire.WriteLine("payload\t"+Convert.ToBase64String(Encoding.UTF8.GetBytes(text)));}

File.WriteAllText(Path.ChangeExtension(args[1],".payload.json"),text);
SerializerFailures.Run(Path.ChangeExtension(args[1],".failures.tsv"));
Console.WriteLine("Source stream contract and JSON audit assertions passed.");
public sealed class Payload{public string? Name{get;set;}public Payload? Self{get;set;}public Payload? Other{get;set;}public int ReadOnly=>7;}

public sealed class StreamContractProbe:Dock.Model.Core.IDockSerializer
{
 public Stream? SeenStream;public Type? SeenType;public object? SeenValue;
 public string Serialize<T>(T value)=>throw new InvalidOperationException("unused contract probe member");
 public T? Deserialize<T>(string text)=>throw new InvalidOperationException("unused contract probe member");
 public T? Load<T>(Stream stream){SeenStream=stream;SeenType=typeof(T);return default;}
 public void Save<T>(Stream stream,T value){SeenStream=stream;SeenType=typeof(T);SeenValue=value;}
}
