using System;
using System.Collections.Generic;
using Dock.Model.Core;
using Dock.Model.Controls;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;

namespace DockFactoryMarkupReference;

internal static class NullableTargetReference
{
    private static bool NullFailure(Action action)
    {
        try { action(); return false; }
        catch (NullReferenceException) { return true; }
    }

    internal static void Run(Action<string, bool> check)
    {
        var f = new TargetFactory();
        var item = new Tool { Id = "item" };
        check("nullable-target-add-after-init", NullFailure(() => f.AddDockable(null!, item)) && string.Join(",", f.Trace) == "init:item:null");
        f.Trace.Clear();
        check("nullable-target-add-null-item-after-init", NullFailure(() => f.AddDockable(null!, null!)) && string.Join(",", f.Trace) == "init:null:null");
        var defaultFactory = new Factory();
        item = new Tool { Owner = new DocumentDock() };
        check("nullable-target-add-default-init-first", NullFailure(() => defaultFactory.AddDockable(null!, item)) && item.Owner is null && ReferenceEquals(item.Factory, defaultFactory));
        check("nullable-target-add-default-null-item", NullFailure(() => defaultFactory.AddDockable(null!, null!)));
        for (int index = 0; index < 7; index++)
        {
            int operation = index;
            check("nullable-target-helper-first-dereference-" + index, NullFailure(() => f.Helper(operation)));
        }

        var dock = new DocumentDock();
        check("nullable-target-selection-empty", f.BaseSelect(dock, null!, -1) is null);
        var child = new Document();
        var list = new List<IDockable> { null!, child, new ProportionalDockSplitter() };
        dock.VisibleDockables = list;
        check("nullable-target-selection-null-entry", f.BaseSelect(dock, null!, 1) is null);
        check("nullable-target-selection-same-child", ReferenceEquals(f.BaseSelect(dock, null!, 2), child));
        check("nullable-target-selection-splitter", f.BaseSelect(dock, null!, 3) is null);
        bool invalid = false;
        try { f.BaseSelect(dock, null!, 5); }
        catch (ArgumentOutOfRangeException) { invalid = true; }
        check("nullable-target-selection-invalid-keeps-list", invalid && ReferenceEquals(list, dock.VisibleDockables) && list.Count == 3);
        f = new TargetFactory();
        check("nullable-target-selection-override-values", ReferenceEquals(f.ProbeSelect(), f.Selection) && string.Join(",", f.Trace) == "selection:null:null:2");
        f.Trace.Clear();
        f.ProbePin();
        check("nullable-target-pin-override-values", string.Join(",", f.Trace) == "pin:null:null");

        f = new TargetFactory();
        item = new Tool { Id = "item" };
        f.BasePin(item);
        check("nullable-target-pin-no-owner-noop", string.Join(",", f.Trace) == "root:item");
        f.Trace.Clear();
        item.Owner = new ToolDock();
        f.BasePin(item);
        check("nullable-target-pin-invisible-noop", string.Join(",", f.Trace) == "root:item");
        f.Trace.Clear();
        check("nullable-target-pin-null-item-first", NullFailure(() => f.BasePin(null!)) && f.Trace.Count == 0);
        foreach (bool itemBounds in new[] { true, false })
        {
            f = new TargetFactory();
            var owner = new ToolDock();
            item = new Tool { Id = "item", Owner = owner };
            owner.SetVisibleBounds(10, 20, 210, 160);
            if (itemBounds) item.SetVisibleBounds(40, 50, 310, 260);
            list = new List<IDockable> { item };
            owner.VisibleDockables = list;
            bool failed = NullFailure(() => f.BasePin(item));
            item.GetPinnedBounds(out double x, out double y, out double width, out double height);
            check("nullable-target-pin-bounds-before-root-" + itemBounds, failed && x == 0 && y == 0 && width == (itemBounds ? 310 : 210) && height == (itemBounds ? 260 : 160));
            check("nullable-target-pin-failure-keeps-list-" + itemBounds, ReferenceEquals(list, owner.VisibleDockables) && list.Count == 1 && ReferenceEquals(list[0], item) && string.Join(",", f.Trace) == "root:item");
        }
        foreach (bool toolOwner in new[] { true, false })
        {
            f = new TargetFactory();
            var owner = new ToolDock();
            item = new Tool { Id = "item", Owner = toolOwner ? owner : null };
            list = new List<IDockable> { item };
            owner.VisibleDockables = list;
            var pins = new List<IDockable> { item };
            f.QueryRoot = new RootDock { LeftPinnedDockables = pins };
            bool failed = NullFailure(() => f.BasePin(item));
            item.GetPinnedBounds(out _, out _, out double width, out double height);
            check("nullable-target-pin-fallback-keeps-null-root-" + toolOwner, failed && string.Join(",", f.Trace) == "root:item" && pins.Count == 1 && list.Count == 1 && double.IsNaN(width) && double.IsNaN(height));
        }
    }

    private sealed class TargetFactory : Factory
    {
        internal readonly List<string> Trace = new();
        internal IRootDock? QueryRoot;
        internal readonly IDockable Selection = new Document();
        private static string Name(IDockable? item) => item?.Id ?? "null";
        public override void InitDockable(IDockable item, IDockable? owner) => Trace.Add("init:" + Name(item) + ":" + Name(owner));
        public override IRootDock? FindRoot(IDockable item, Func<IRootDock, bool>? predicate = null)
        {
            Trace.Add("root:" + Name(item));
            return QueryRoot;
        }
        protected override IDockable? GetActiveDockableAfterRemoval(IDock dock, IDockable removed, int index)
        {
            Trace.Add($"selection:{Name(dock)}:{Name(removed)}:{index}");
            return Selection;
        }
        protected override void PinDockableCore(IDockable item, IRootDock root, IToolDock? unpin) => Trace.Add("pin:" + Name(item) + ":" + Name(root));
        internal IDockable? BaseSelect(IDock dock, IDockable removed, int index) => base.GetActiveDockableAfterRemoval(dock, removed, index);
        internal IDockable? ProbeSelect() => GetActiveDockableAfterRemoval(null!, null!, 2);
        internal void BasePin(IDockable item) => base.PinDockableCore(item, null!, null);
        internal void ProbePin() => PinDockableCore(null!, null!, null);
        internal void Helper(int operation)
        {
            switch (operation)
            {
                case 0: AddVisibleDockable(null!, null!); break;
                case 1: InsertVisibleDockable(null!, -1, null!); break;
                case 2: RemoveVisibleDockable(null!, null!); break;
                case 3: RemoveAllVisibleDockables(null!); break;
                case 4: RemoveVisibleDockableAt(null!, -1); break;
                case 5: CleanupOrphanedSplitters(null!); break;
                case 6: base.GetActiveDockableAfterRemoval(null!, null!, -1); break;
                default: throw new ArgumentOutOfRangeException(nameof(operation));
            }
        }
    }
}
