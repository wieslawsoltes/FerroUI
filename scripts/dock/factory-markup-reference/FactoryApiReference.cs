using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using Dock.Model.Core;
using Dock.Model.ReactiveUI;

namespace DockFactoryMarkupReference;

internal static class FactoryApiReference
{
    internal static void Run(Action<string, bool> check)
    {
        var factory = new Factory();
        var root = factory.CreateRootDock(); root.Id = "root";
        var left = factory.CreateDocumentDock(); left.Id = "left";
        var right = factory.CreateDocumentDock(); right.Id = "right";
        factory.AddDockable(root, left); factory.AddDockable(root, right);
        var a = factory.CreateDocument(); a.Id = "a";
        var b = factory.CreateDocument(); b.Id = "b";
        var c = factory.CreateDocument(); c.Id = "c";
        factory.AddDockable(left, a); factory.AddDockable(left, b);
        factory.InitLayout(root);
        factory.InsertDockable(left, c, 1);
        check("factory-api-insert", Ids(left) == "a,c,b" && ReferenceEquals(c.Owner, left));
        factory.MoveDockable(left, right, c, null);
        check("factory-api-move-between", Ids(left) == "a,b" && Ids(right) == "c");
        factory.SwapDockable(left, right, a, c);
        check("factory-api-swap-between", Ids(left) == "c,b" && Ids(right) == "a");
        factory.RemoveDockable(b, false);
        check("factory-api-remove", Ids(left) == "c");
        factory.MoveDockable(right, a, a);
        check("factory-api-move-self", Ids(right) == "a");
        factory.SwapDockable(right, a, a);
        check("factory-api-swap-self", Ids(right) == "a");

        var searchRoot = factory.CreateRootDock(); searchRoot.Id = "root";
        var nested = factory.CreateDocumentDock(); nested.Id = "nested";
        var item = factory.CreateDocument(); item.Id = "item";
        factory.AddDockable(searchRoot, nested); factory.AddDockable(nested, item);
        int calls = 0;
        IEnumerable<IDockable> query = factory.Find(searchRoot, _ => { calls++; return true; });
        check("factory-api-find-deferred", calls == 0);
        using var first = query.GetEnumerator(); using var second = query.GetEnumerator();
        check("factory-api-enumerators-deferred", calls == 0 && first.Current is null);
        check("factory-api-cursor-first", first.MoveNext() && ReferenceEquals(first.Current, searchRoot));
        check("factory-api-independent-cursor", second.MoveNext() && ReferenceEquals(second.Current, searchRoot));
        check("factory-api-nested-first", first.MoveNext() && ReferenceEquals(first.Current, nested));
        check("factory-api-nested-duplicate", first.MoveNext() && ReferenceEquals(first.Current, nested));
        check("factory-api-original-query-item", first.MoveNext() && ReferenceEquals(first.Current, item));
        check("factory-api-cursor-end-current", !first.MoveNext() && ReferenceEquals(first.Current, item));
        bool resetFails = false; try { ((IEnumerator)first).Reset(); } catch (NotSupportedException) { resetFails = true; }
        check("factory-api-reset-unsupported", resetFails);
        first.Dispose();
        check("factory-api-disposed-current", !first.MoveNext() && ReferenceEquals(first.Current, item));
        var later = factory.CreateTool(); later.Id = "later"; factory.AddDockable(searchRoot, later);
        check("factory-api-query-rereads-tree", string.Join(",", query.Select(x => x.Id)) == "root,nested,nested,item,later");
        check("factory-api-find-dockable", ReferenceEquals(factory.FindDockable(searchRoot, x => x.Id == "item"), item));
        check("factory-api-find-root", ReferenceEquals(factory.FindRoot(item), searchRoot));
        int roots = 0;
        check("factory-api-find-root-predicate", factory.FindRoot(item, _ => { roots++; return false; }) is null && roots == 2);
        check("factory-api-find-all-empty", !factory.Find(_ => true).Any());
        int faultCalls = 0;
        using var faultCursor = factory.Find(searchRoot, _ => { if (++faultCalls > 1) throw new InvalidOperationException("source predicate failure"); return true; }).GetEnumerator();
        check("factory-api-fault-first", faultCursor.MoveNext() && ReferenceEquals(faultCursor.Current, searchRoot));
        bool faulted = false; try { faultCursor.MoveNext(); } catch (InvalidOperationException) { faulted = true; }
        check("factory-api-fault-current", faulted && ReferenceEquals(faultCursor.Current, searchRoot));
        check("factory-api-fault-ended", !faultCursor.MoveNext() && faultCalls == 2);

        var ownerRoot = factory.CreateRootDock();
        var window = factory.CreateDockWindow(); window.Layout = root; root.Window = window;
        factory.InitDockWindow(window, ownerRoot, null);
        check("factory-api-window-init", ReferenceEquals(window.Factory, factory) && ReferenceEquals(window.Owner, ownerRoot));
        var extraWindow = factory.CreateDockWindow();
        factory.AddWindow(ownerRoot, window); factory.InsertWindow(ownerRoot, extraWindow, 0);
        check("factory-api-window-insert", ownerRoot.Windows!.Count == 2 && ReferenceEquals(ownerRoot.Windows[0], extraWindow));
        factory.RemoveWindow(extraWindow);
        check("factory-api-window-remove", ownerRoot.Windows.Count == 1 && ReferenceEquals(ownerRoot.Windows[0], window));
        factory.InitActiveDockable(a, right);
        check("factory-api-init-active", ReferenceEquals(factory.CurrentDockable, a));
        factory.ActivateWindow(a);
        check("factory-api-activate-window", ReferenceEquals(factory.CurrentDockWindow, window));

        var tools = factory.CreateToolDock();factory.AddDockable(root,tools);
        var toolA = factory.CreateTool(); toolA.Id = "a"; var toolB = factory.CreateTool(); toolB.Id = "b";
        factory.AddDockable(tools,toolA);factory.AddDockable(tools,toolB);factory.InitLayout(root);
        factory.PinDockable(toolA);factory.PinDockable(toolB);
        check("factory-api-pinned-default-root",factory.IsDockablePinned(toolA));
        check("factory-api-pinned-explicit-root",factory.IsDockablePinned(toolA,root));
        factory.MovePinnedDockable(toolA,toolB);
        check("factory-api-pinned-order",string.Join(",",root.LeftPinnedDockables!.Select(x=>x.Id))=="b,a");
    }

    private static string Ids(IDock dock) => string.Join(",", dock.VisibleDockables!.Select(x => x.Id));
}
