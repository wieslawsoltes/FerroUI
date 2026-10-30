// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System.Text;
using Dock.Serializer.SystemTextJson;
internal static class SerializerFailures
{
 internal static void Run(string path){
  using var output=new StreamWriter(path);void Check(string name,bool value){if(!value)throw new InvalidOperationException(name);output.WriteLine(name+"\t1");}
  Exception? Capture(Action action){try{action();return null;}catch(Exception e){return e;}}
  var serializer=new DockSerializer();
  var read=new IOException("read");var close=new IOException("close");var write=new IOException("write");var flush=new IOException("flush");
  var stream=new Probe{ReadError=read};Check("load-read-error-identity",ReferenceEquals(Capture(()=>serializer.Load<string>(stream)),read));Check("load-read-error-closes",stream.Closes==1);
  stream=new Probe{ReadError=read,CloseError=close};Check("load-close-error-overrides-read",ReferenceEquals(Capture(()=>serializer.Load<string>(stream)),close));
  stream=new Probe{CloseError=close};stream.Write(Encoding.UTF8.GetBytes("{"));stream.Position=0;Check("load-close-error-overrides-json",ReferenceEquals(Capture(()=>serializer.Load<string>(stream)),close));
  stream=new Probe{WriteError=write};Check("save-write-error-identity",ReferenceEquals(Capture(()=>serializer.Save(stream,"value")),write));Check("save-write-error-closes",stream.Closes==1);
  stream=new Probe{FlushError=flush};Check("save-flush-error-identity",ReferenceEquals(Capture(()=>serializer.Save(stream,"value")),flush));Check("save-flush-error-closes",stream.Closes==1);
  stream=new Probe{WriteError=write,CloseError=close};Check("save-close-error-overrides-write",ReferenceEquals(Capture(()=>serializer.Save(stream,"value")),close));
  stream=new Probe{Readable=false};Check("load-unreadable-constructor-does-not-close",Capture(()=>serializer.Load<string>(stream)) is ArgumentException&&stream.Closes==0);
  stream=new Probe{Writable=false};Check("save-unwritable-constructor-does-not-close",Capture(()=>serializer.Save(stream,"value")) is ArgumentException&&stream.Closes==0);
  var getter=new InvalidOperationException("getter");Check("save-serialize-precedes-null-stream",ReferenceEquals(Capture(()=>serializer.Save<Throwing>(null!,new Throwing{Failure=getter})),getter));
 }
 private sealed class Throwing{public Exception Failure=null!;public string Value=>throw Failure;}
 private sealed class Probe:MemoryStream{
  internal Exception? ReadError,WriteError,FlushError,CloseError;internal bool Readable=true,Writable=true;internal int Closes;
  public override bool CanRead=>Readable&&base.CanRead;public override bool CanWrite=>Writable&&base.CanWrite;
  public override int Read(byte[] buffer,int offset,int count){if(ReadError is not null)throw ReadError;return base.Read(buffer,offset,count);}
  public override void Write(byte[] buffer,int offset,int count){if(WriteError is not null)throw WriteError;base.Write(buffer,offset,count);}
  public override void Flush(){if(FlushError is not null)throw FlushError;base.Flush();}
  protected override void Dispose(bool disposing){Closes++;base.Dispose(disposing);if(CloseError is not null)throw CloseError;}
 }
}
