// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
using Dock.Model.Core;
using Dock.Model.Controls;
using Dock.Model.ReactiveUI;
using Dock.Model.ReactiveUI.Controls;
using Dock.Serializer.SystemTextJson;
using DockReactiveUISample.ViewModels.Documents;
using DockReactiveUISample.ViewModels.Docks;
using DockReactiveUISample.ViewModels.Tools;
using DockReactiveUISample.ViewModels.Views;
internal static class SampleLayoutReference
{
    internal static void Run(string path)
    {
        var serializer = new DockSerializer();
        using var checks = new StreamWriter(path + ".tsv");
        void Check(string name, bool value) {
            if (!value) throw new InvalidOperationException(name);
            checks.WriteLine(name + "\t1");
        }
        void Record<T>(T model, string name) {
            var wire = serializer.Serialize(model);
            File.WriteAllText(path + "." + name + ".json", wire);
            Check("sample-default-" + name, serializer.Serialize(serializer.Deserialize<T>(wire)) == wire);
        }
        Record(new DocumentViewModel(), "document");
        Record(new CustomDocumentDock(), "documents");
        Record(new HomeViewModel(), "home");
        Record(new DashboardViewModel(), "dashboard");
        Record(new Tool1ViewModel(), "tool1");
        Record(new Tool2ViewModel(), "tool2");
        Record(new Tool3ViewModel(), "tool3");
        Record(new Tool4ViewModel(), "tool4");
        Record(new Tool5ViewModel(), "tool5");
        Record(new Tool6ViewModel(), "tool6");
        Record(new Tool7ViewModel(), "tool7");
        Record(new Tool8ViewModel(), "tool8");
        var factory = new Factory();
        var documents = new CustomDocumentDock { Id = "Documents", CanCreateDocument = true };
        var document = new DocumentViewModel { Id = "Document1", Title = "Editor" };
        documents.VisibleDockables = factory.CreateList<IDockable>(document);
        documents.ActiveDockable = document;
        var tools = new ToolDock { Id = "Tools", VisibleDockables = factory.CreateList<IDockable>(new Tool1ViewModel { Id = "Tool1" }, new Tool8ViewModel { Id = "Tool8" }) };
        var home = new HomeViewModel { Id = "Home", VisibleDockables = factory.CreateList<IDockable>(documents, tools) };
        var root = new RootDock { Id = "Root", VisibleDockables = factory.CreateList<IDockable>(new DashboardViewModel { Id = "Dashboard" }, home), ActiveDockable = home, DefaultDockable = home };
        factory.InitLayout(root);
        var wire = serializer.Serialize<IRootDock>(root);
        File.WriteAllText(path + ".graph.json", wire);
        var loaded = serializer.Deserialize<IRootDock>(wire)!;
        Check("sample-wire-stable", serializer.Serialize(loaded) == wire);
        var loadedHome = (HomeViewModel)loaded.VisibleDockables![1];
        var loadedDocuments = (CustomDocumentDock)loadedHome.VisibleDockables![0];
        Check("sample-original-classes", loaded.VisibleDockables[0] is DashboardViewModel && loadedDocuments.VisibleDockables![0] is DocumentViewModel);
        Check("sample-original-tool-classes", ((ToolDock)loadedHome.VisibleDockables[1]).VisibleDockables![0] is Tool1ViewModel && ((ToolDock)loadedHome.VisibleDockables[1]).VisibleDockables![1] is Tool8ViewModel);
        Check("sample-root-alias", ReferenceEquals(loaded.ActiveDockable, loadedHome) && ReferenceEquals(loaded.DefaultDockable, loadedHome));
        Check("sample-owner-alias", ReferenceEquals(loadedDocuments.Owner, loadedHome));
        var restoredFactory = new Factory();
        restoredFactory.InitLayout(loaded);
        loadedDocuments.CreateDocument!.Execute(null);
        Check("sample-original-constructor-command", loadedDocuments.VisibleDockables!.Count == 2 && loadedDocuments.VisibleDockables[1] is DocumentViewModel && loadedDocuments.ActiveDockable!.Id == "Document2");
    }
}
