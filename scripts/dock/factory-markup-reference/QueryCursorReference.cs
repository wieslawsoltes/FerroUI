// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using System;
using System.Collections.Generic;
using Dock.Avalonia.Controls;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
namespace DockFactoryMarkupReference;
internal static class QueryCursorReference
{
 internal static void Run(Action<string,bool> check)
 {
  var expected=new Dictionary<string,string>();
  foreach(var line in System.IO.File.ReadAllLines(System.IO.Path.Combine(AppContext.BaseDirectory,"query-cursor-reference.tsv")))
  {
   if(line.StartsWith("#"))continue;
   var parts=line.Split('\t');expected.Add(string.Join(" ",parts[0],parts[1],parts[2],parts[3]),parts[4]);
  }
  var beforeFactory=new Factory();var beforeRoot=new RootDock{Id="before"};var before=beforeFactory.Find(beforeRoot,_=>true).GetEnumerator();
  before.Dispose();check("query-cursor-dispose-before-start",!before.MoveNext()&&before.Current is null);
  before.Dispose();check("query-cursor-dispose-before-start-repeat",!before.MoveNext()&&before.Current is null);
  foreach(bool all in new[]{false,true})
  foreach(string target in new[]{"root","nested","leaf","tail"})
  foreach(bool match in new[]{false,true})
  foreach(bool twice in new[]{false,true})
  {
   var f=new Factory();var root=new RootDock{Id="root"};var nested=new RootDock{Id="nested"};var leaf=new Document{Id="leaf"};var tail=new Document{Id="tail"};
   nested.VisibleDockables=new List<IDockable>{leaf};root.VisibleDockables=new List<IDockable>{nested,tail};
   if(all)f.DockControls.Add(new DockControl{InitializeLayout=false,Layout=root});
   var trace=new List<string>();IEnumerator<IDockable>? cursor=null;bool disposed=false;
   Func<IDockable,bool> predicate=item=>{
    trace.Add("predicate:"+item.Id);
    if(item.Id==target&&!disposed){disposed=true;trace.Add("before:"+(cursor!.Current?.Id??"null"));cursor.Dispose();trace.Add("after:"+(cursor.Current?.Id??"null"));if(twice){trace.Add("reentry:"+cursor.MoveNext());cursor.Dispose();}}
    return match;
   };
   cursor=(all?f.Find(predicate):f.Find(root,predicate)).GetEnumerator();
   try{for(int i=0;i<9;i++){bool moved=cursor.MoveNext();trace.Add("move:"+moved+":"+(cursor.Current?.Id??"null"));if(!moved)break;}}catch(Exception e){trace.Add("error:"+e.GetType().Name);}
   for(int i=0;i<2;i++){try{trace.Add("post:"+cursor.MoveNext()+":"+(cursor.Current?.Id??"null"));}catch(Exception e){trace.Add("post-error:"+e.GetType().Name);}}cursor.Dispose();trace.Add("disposed-end:"+(cursor.Current?.Id??"null"));
   check("query-cursor-dispose-"+all+"-"+target+"-"+match+"-"+twice,string.Join("|",trace)==expected[all+" "+target+" "+match+" "+twice]);
  }
 }
}
