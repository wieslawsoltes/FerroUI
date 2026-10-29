// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::*;
use ferrodock_model::core::IFactoryEvents;
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, ProportionalDock, RootDock, SplitViewDock},
    Factory,
};
use ferroui_base::threading::CancellationToken;
use ferroui_controls::{AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_system::events::Event;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "command-bar-manager",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
pub(crate) fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                f()
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
pub(crate) fn item(title: &str, order: i32, group: Option<&str>) -> Rc<DockCommandBarItem> {
    let item = DockCommandBarItem::new(Some(Rc::new(title.to_owned())));
    item.set_order(order);
    item.set_group_id(group.map(str::to_owned));
    item
}
pub(crate) fn bar(
    id: &str,
    kind: DockCommandBarKind,
    mode: DockCommandBarMergeMode,
    order: i32,
    items: Vec<Rc<DockCommandBarItem>>,
) -> Rc<DockCommandBarDefinition> {
    let bar = DockCommandBarDefinition::new(id.into(), kind);
    bar.set_merge_mode(mode);
    bar.set_order(order);
    bar.set_items(Some(ReadOnlyListHandle::from_items(items)));
    bar
}
pub(crate) fn list<T: Clone + 'static>(values: Vec<T>) -> ReadOnlyListHandle<T> {
    ReadOnlyListHandle::from_items(values)
}
fn titles(bar: &DockCommandBarDefinition) -> Vec<String> {
    bar.items()
        .unwrap()
        .iter()
        .map(|i| {
            i.header()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap()
                .clone()
        })
        .collect()
}
#[test]
fn replace_wins_per_kind_and_stable_order_retains_shallow_identity() {
    use DockCommandBarKind::*;
    use DockCommandBarMergeMode::*;
    let original = item("original", 0, None);
    let replacement = item("replacement", 0, None);
    let base = bar("base", Menu, Append, 0, vec![original.clone()]);
    let toolbar = bar("tools", ToolBar, Replace, 0, vec![original]);
    let first = bar("first", Menu, Replace, 2, vec![replacement.clone()]);
    let second = bar("second", Menu, Replace, 2, vec![]);
    let early = bar("early", Menu, Replace, -1, vec![]);
    let content: ferrodock_model::BoxedValue = Rc::new(Cell::new(1));
    first.set_content(Some(content.clone()));
    let result = DockCommandBarManager::merge_bars(
        &list(vec![base, toolbar.clone()]),
        &list(vec![
            first.clone(),
            bar("ignored", Menu, Append, -9, vec![]),
            second,
            early,
        ]),
    );
    assert_eq!(
        result.iter().map(|b| b.id()).collect::<Vec<_>>(),
        ["early", "first", "second", "tools"]
    );
    assert!(!Rc::ptr_eq(&result[1], &first));
    assert!(!Rc::ptr_eq(&result[3], &toolbar));
    assert!(Rc::ptr_eq(&result[1].content().unwrap(), &content));
    assert!(Rc::ptr_eq(&result[1].items().unwrap().get(0), &replacement));
    replacement.set_header(Some(Rc::new("edited".to_owned())));
    assert_eq!(titles(&result[1]), ["edited"]);
    result[1].set_items(None);
    assert_eq!(first.items().unwrap().count(), 1);
}
#[test]
fn append_uses_first_matching_id_or_last_bar_preserving_base_order() {
    use DockCommandBarKind::*;
    use DockCommandBarMergeMode::*;
    let a = bar("a", Menu, Replace, 0, vec![item("base-a", 99, None)]);
    let duplicate = bar("a", Menu, Replace, 1, vec![item("duplicate", 0, None)]);
    let z = bar("z", Menu, Replace, 2, vec![item("base-z", 0, None)]);
    let result = DockCommandBarManager::merge_bars(
        &list(vec![a.clone(), duplicate, z]),
        &list(vec![
            bar(
                "a",
                Menu,
                Append,
                0,
                vec![
                    item("late", 2, None),
                    item("first", 1, None),
                    item("tie", 1, None),
                ],
            ),
            bar("missing", Menu, Append, 1, vec![item("last", 0, None)]),
        ]),
    );
    assert_eq!(result.len(), 3);
    assert_eq!(titles(&result[0]), ["base-a", "first", "tie", "late"]);
    assert_eq!(titles(&result[1]), ["duplicate"]);
    assert_eq!(titles(&result[2]), ["base-z", "last"]);
    assert_eq!(titles(&a), ["base-a"]);
    let empty = bar("empty", Menu, Append, 0, vec![]);
    let added = bar("new", Menu, Append, 0, vec![item("new", 0, None)]);
    assert_eq!(
        DockCommandBarManager::merge_bars(&list(vec![empty.clone()]), &list(vec![added.clone()]))
            .len(),
        2
    );
    empty.set_items(None);
    assert_eq!(
        DockCommandBarManager::merge_bars(&list(vec![]), &list(vec![empty, added])).len(),
        2
    );
}
#[test]
fn grouped_merge_inserts_after_last_match_in_first_bar_and_falls_back_for_missing_groups() {
    use DockCommandBarKind::*;
    use DockCommandBarMergeMode::*;
    let base = bar(
        "base",
        Menu,
        Replace,
        0,
        vec![
            item("a", 0, Some("g")),
            item("middle", 0, None),
            item("b", 0, Some("g")),
            item("end", 0, None),
        ],
    );
    let other = bar("other", Menu, Replace, 1, vec![item("other", 0, Some("g"))]);
    let incoming = bar(
        "incoming",
        Menu,
        MergeByGroup,
        0,
        vec![item("later", 2, None), item("earlier", 1, None)],
    );
    incoming.set_group_id(Some("g".into()));
    let result =
        DockCommandBarManager::merge_bars(&list(vec![base.clone(), other]), &list(vec![incoming]));
    assert_eq!(
        titles(&result[0]),
        ["a", "middle", "b", "earlier", "later", "end"]
    );
    assert_eq!(titles(&result[1]), ["other"]);
    assert_eq!(titles(&base), ["a", "middle", "b", "end"]);
    for group in [None, Some(""), Some(" \t\u{a0}"), Some("missing")] {
        let added = bar("new", Menu, MergeByGroup, 0, vec![item("new", 0, None)]);
        added.set_group_id(group.map(str::to_owned));
        assert_eq!(
            DockCommandBarManager::merge_bars(&list(vec![base.clone()]), &list(vec![added])).len(),
            2
        );
    }
    assert_eq!(
        DockCommandBarManager::find_group_insert_index(&[item("x", 0, None)], "missing"),
        1
    );
}
#[test]
fn active_resolution_preserves_one_level_scope_document_priority_and_split_pane_order() {
    let root = RootDock::new();
    let outer = ProportionalDock::new();
    let inner = DocumentDock::new();
    let doc = Document::new();
    root.set_active_dockable(Some(outer.clone()));
    outer.set_active_dockable(Some(inner.clone()));
    inner.set_active_dockable(Some(doc.clone()));
    let expected: Rc<dyn IDockable> = inner.clone();
    assert!(Rc::ptr_eq(
        &DockCommandBarManager::resolve_active_dockable(
            root.clone(),
            DockCommandBarMergingScope::ActiveDockable
        )
        .unwrap(),
        &expected
    ));
    // A document dock's active document wins even when absent from VisibleDockables.
    let expected: Rc<dyn IDockable> = doc.clone();
    assert!(Rc::ptr_eq(
        &DockCommandBarManager::find_active_document(inner.clone()).unwrap(),
        &expected
    ));
    root.set_visible_dockables(Some(ferrodock_model::collections::ListHandle::from_plain(
        vec![Some(inner)],
    )));
    assert!(Rc::ptr_eq(
        &DockCommandBarManager::resolve_active_dockable(
            root.clone(),
            DockCommandBarMergingScope::ActiveDocument
        )
        .unwrap(),
        &expected
    ));
    let split = SplitViewDock::new();
    let pane = DocumentDock::new();
    pane.set_active_dockable(Some(doc));
    let content = DocumentDock::new();
    let content_doc = Document::new();
    content.set_active_dockable(Some(content_doc.clone()));
    split.set_pane_dockable(Some(pane));
    split.set_content_dockable(Some(content.clone()));
    assert!(Rc::ptr_eq(
        &DockCommandBarManager::find_active_document(split.clone()).unwrap(),
        &expected
    ));
    split.set_pane_dockable(None);
    let expected: Rc<dyn IDockable> = content_doc;
    assert!(Rc::ptr_eq(
        &DockCommandBarManager::find_active_document(split).unwrap(),
        &expected
    ));
    content.set_active_dockable(None);
    assert!(DockCommandBarManager::find_active_document(content).is_none());
}
struct Provider {
    changed: Event<()>,
    bars: RefCell<ReadOnlyListHandle<Rc<DockCommandBarDefinition>>>,
    reads: Cell<usize>,
}
impl Provider {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            changed: Event::new(),
            bars: RefCell::new(list(vec![bar(
                "tools",
                DockCommandBarKind::ToolBar,
                DockCommandBarMergeMode::Replace,
                0,
                vec![item("action", 0, None)],
            )])),
            reads: Cell::new(0),
        })
    }
}
impl IDockCommandBarProvider for Provider {
    fn get_command_bars(&self) -> ReadOnlyListHandle<Rc<DockCommandBarDefinition>> {
        self.reads.set(self.reads.get() + 1);
        self.bars.borrow().clone()
    }
    fn command_bars_changed(&self) -> &Event<()> {
        &self.changed
    }
}
struct SettingsGuard(bool, DockCommandBarMergingScope);
impl SettingsGuard {
    fn new() -> Self {
        let guard = Self(
            DockSettings::command_bar_merging_enabled(),
            DockSettings::command_bar_merging_scope(),
        );
        DockSettings::set_command_bar_merging_enabled(true);
        DockSettings::set_command_bar_merging_scope(DockCommandBarMergingScope::ActiveDockable);
        guard
    }
}
impl Drop for SettingsGuard {
    fn drop(&mut self) {
        DockSettings::set_command_bar_merging_enabled(self.0);
        DockSettings::set_command_bar_merging_scope(self.1);
    }
}
#[test]
fn manager_tracks_original_factory_and_provider_events_and_detach_releases_it_with_dependencies_alive(
) {
    run(|| {
        let _settings = SettingsGuard::new();
        let factory = Factory::new();
        let root = RootDock::new();
        root.set_factory(Some(factory.clone()));
        let doc = Document::new();
        let provider = Provider::new();
        let contract: Rc<dyn IDockCommandBarProvider> = provider.clone();
        doc.set_context(Some(Rc::new(contract)));
        root.set_active_dockable(Some(doc.clone()));
        let host = DockCommandBarHost::new();
        let manager = DockCommandBarManager::new(host.clone(), None);
        manager.attach(root.clone());
        assert!(host.is_visible());
        assert_eq!(host.tool_bars().unwrap().count(), 1);
        assert_eq!(provider.reads.get(), 1);
        provider.changed.raise(&());
        assert_eq!(provider.reads.get(), 2);
        manager.attach(root.clone());
        assert_eq!(provider.reads.get(), 3);
        factory.on_active_dockable_changed(Some(doc.clone()));
        assert_eq!(provider.reads.get(), 4);
        DockSettings::set_command_bar_merging_enabled(false);
        provider.changed.raise(&());
        assert!(!host.is_visible());
        assert!(host.tool_bars().is_none());
        assert_eq!(provider.reads.get(), 4);
        DockSettings::set_command_bar_merging_enabled(true);
        provider.changed.raise(&());
        assert!(host.is_visible());
        assert_eq!(provider.reads.get(), 5);
        let next = Provider::new();
        let contract: Rc<dyn IDockCommandBarProvider> = next.clone();
        doc.set_context(Some(Rc::new(contract)));
        factory.on_active_dockable_changed(Some(doc.clone()));
        assert_eq!(next.reads.get(), 1);
        provider.changed.raise(&());
        assert_eq!(next.reads.get(), 1);
        next.changed.raise(&());
        assert_eq!(next.reads.get(), 2);
        // Source Attach(same layout) refreshes bars but does not resubscribe a changed Factory.
        let replacement_factory = Factory::new();
        root.set_factory(Some(replacement_factory.clone()));
        manager.attach(root.clone());
        let count = next.reads.get();
        replacement_factory.on_active_dockable_changed(Some(doc.clone()));
        assert_eq!(next.reads.get(), count);
        factory.on_active_dockable_changed(Some(doc));
        assert_eq!(next.reads.get(), count + 1);
        let weak = Rc::downgrade(&manager);
        drop(manager);
        assert!(weak.upgrade().is_some());
        let manager = weak.upgrade().unwrap();
        manager.detach();
        assert!(!host.is_visible());
        assert!(host.menu_bars().is_none());
        assert!(host.ribbon_bars().is_none());
        let count = next.reads.get();
        drop(manager);
        assert!(weak.upgrade().is_none());
        factory.on_active_dockable_changed(None);
        next.changed.raise(&());
        assert_eq!(next.reads.get(), count);
        assert!(root.factory().is_some());
    });
}
#[test]
fn native_host_peer_and_model_metadata_preserve_original_members_and_event_disposal() {
    run(|| {
        use crate::command_bars::{
            DockCommandBarDefinitionMetadata, DockCommandBarItemMetadata,
            IDockCommandBarProviderMetadata,
        };
        use ferroui_base::{
            metadata::{MarkupDelegate, MarkupTyped},
            BoxedValue,
        };
        use ferroui_controls::automation::{
            peers::{AutomationControlType, ControlAutomationPeer},
            AutomationProperties,
        };
        let host = DockCommandBarHost::new();
        assert!(!host.is_visible());
        assert!(host.base_command_bars().is_none());
        let peer = ControlAutomationPeer::create_peer_for_element(&host);
        assert_eq!(peer.get_class_name(), "DockCommandBarHost");
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::ToolBar
        );
        assert_eq!(peer.get_name(), "Dock command bars");
        assert_eq!(
            peer.get_help_text(),
            "Visible=false; MenuBars=0; ToolBars=0; RibbonBars=0; BaseDefinitions=0"
        );
        AutomationProperties::set_name(&host, Some("My commands"));
        assert_eq!(peer.get_name(), "My commands");
        let definition = bar(
            "id",
            DockCommandBarKind::Menu,
            DockCommandBarMergeMode::Append,
            4,
            vec![],
        );
        host.set_base_command_bars(Some(list(vec![definition.clone()])));
        assert!(peer.get_help_text().ends_with("BaseDefinitions=1"));
        let metadata = <dyn DockCommandBarDefinitionMetadata>::MARKUP;
        assert_eq!(metadata.properties.len(), 7);
        let boxed: BoxedValue = Rc::new(definition.clone());
        let property = metadata.find_property("Order").unwrap();
        (property.set.unwrap())(&[Some(boxed.clone()), Some(Rc::new(17_i32))]).unwrap();
        assert_eq!(definition.order(), 17);
        assert!(metadata.find_property("Id").unwrap().set.is_none());
        assert!(metadata.find_property("Kind").unwrap().set.is_none());
        let item = item("header", 0, None);
        let metadata = <dyn DockCommandBarItemMetadata>::MARKUP;
        assert_eq!(metadata.properties.len(), 8);
        let boxed: BoxedValue = Rc::new(item.clone());
        let set = metadata.find_property("Header").unwrap().set.unwrap();
        set(&[Some(boxed.clone()), Some(Rc::new("changed".to_owned()))]).unwrap();
        assert_eq!(
            ferrodock_controls_recycling::from_recycling_value(&item.header().unwrap())
                .downcast_ref::<String>()
                .unwrap(),
            "changed"
        );
        set(&[Some(boxed), None]).unwrap();
        assert!(item.header().is_none());
        let provider = Provider::new();
        let contract: Rc<dyn IDockCommandBarProvider> = provider.clone();
        let expected = contract.clone();
        let boxed: BoxedValue = Rc::new(contract);
        let metadata = <dyn IDockCommandBarProviderMetadata>::MARKUP;
        let method = metadata.find_methods("GetCommandBars").next().unwrap();
        let returned = (method.invoke)(&[Some(boxed.clone())]).unwrap().unwrap();
        assert!(
            returned
                .downcast_ref::<ReadOnlyListHandle<Rc<DockCommandBarDefinition>>>()
                .unwrap()
                == &*provider.bars.borrow()
        );
        let calls = Rc::new(Cell::new(0));
        let count = calls.clone();
        let handler = MarkupDelegate::new(move |args| {
            assert_eq!(args.len(), 2);
            let sender = args[0]
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<dyn IDockCommandBarProvider>>()
                .unwrap();
            assert!(Rc::ptr_eq(sender, &expected));
            assert!(args[1].as_ref().unwrap().downcast_ref::<()>().is_some());
            count.set(count.get() + 1);
            None
        });
        let event = metadata.find_event("CommandBarsChanged").unwrap();
        assert_eq!(event.arguments.len(), 2);
        let result = (event.add)(&[Some(boxed), Some(Rc::new(handler.clone()))]).unwrap();
        assert!(result.is_none());
        assert_eq!(provider.changed.handler_count(), 1);
        provider.changed.raise(&());
        assert_eq!(calls.get(), 1);
        // Shared event metadata exposes add only; remove the actual System token.
        let token = provider.changed.snapshot().iter().next().unwrap().0;
        provider.changed.remove(token);
        let contract: Rc<dyn IDockCommandBarProvider> = provider.clone();
        let subscription = crate::command_bars::model_metadata::subscribe(&contract, handler);
        assert_eq!(provider.changed.handler_count(), 1);
        subscription.dispose();
        assert_eq!(provider.changed.handler_count(), 0);
        provider.changed.raise(&());
        assert_eq!(calls.get(), 1);
    });
}
#[test]
fn base_bars_without_provider_and_missing_factory_follow_original_host_rules() {
    run(|| {
        let _settings = SettingsGuard::new();
        let root = RootDock::new();
        let host = DockCommandBarHost::new();
        host.set_base_command_bars(Some(list(vec![bar(
            "base",
            DockCommandBarKind::Menu,
            DockCommandBarMergeMode::Replace,
            0,
            vec![item("File", 0, None)],
        )])));
        let manager = DockCommandBarManager::new(host.clone(), None);
        manager.attach(root.clone());
        assert!(!host.is_visible());
        let factory = Factory::new();
        root.set_factory(Some(factory.clone()));
        manager.attach(root.clone());
        assert!(host.is_visible());
        assert_eq!(host.menu_bars().unwrap().count(), 1);
        assert!(host.tool_bars().is_none());
        root.set_factory(None);
        manager.attach(root);
        assert!(!host.is_visible());
        manager.detach();
    });
}
struct ProviderDockable {
    base: ferrodock_model_reactiveui::core::DockableBase,
    provider: Rc<Provider>,
}
ferroui_reactiveui::reactive_object!(ProviderDockable, base);
impl ferrodock_model_reactiveui::core::DockableBaseImpl for ProviderDockable {}
ferrodock_model_reactiveui::impl_dockable!(
    ProviderDockable,
    fn as_command_bar_provider(self: Rc<Self>) -> Option<Rc<dyn IDockCommandBarProvider>> {
        Some(self)
    }
);
impl IDockCommandBarProvider for ProviderDockable {
    fn get_command_bars(&self) -> ReadOnlyListHandle<Rc<DockCommandBarDefinition>> {
        self.provider.get_command_bars()
    }
    fn command_bars_changed(&self) -> &Event<()> {
        &self.provider.changed
    }
}
#[test]
fn dockable_provider_takes_precedence_over_context_and_plain_context_is_not_a_provider() {
    let own = Provider::new();
    let model = Rc::new_cyclic(|weak: &std::rc::Weak<ProviderDockable>| ProviderDockable {
        base: ferrodock_model_reactiveui::core::DockableBase::construct(weak.clone()),
        provider: own.clone(),
    });
    let context = Provider::new();
    let contract: Rc<dyn IDockCommandBarProvider> = context.clone();
    model.set_context(Some(Rc::new(contract)));
    let expected: Rc<dyn IDockCommandBarProvider> = model.clone();
    assert!(Rc::ptr_eq(
        &DockCommandBarManager::find_provider(model).unwrap(),
        &expected
    ));
    assert_eq!(context.reads.get(), 0);
    let doc = Document::new();
    doc.set_context(Some(Rc::new("ordinary context".to_owned())));
    assert!(DockCommandBarManager::find_provider(doc.clone()).is_none());
    let contract: Rc<dyn IDockCommandBarProvider> = context;
    doc.set_context(Some(ferrodock_controls_recycling::to_recycling_value(
        Rc::new(contract.clone()),
    )));
    assert!(Rc::ptr_eq(
        &DockCommandBarManager::find_provider(doc.clone()).unwrap(),
        &contract
    ));
    doc.set_context(None);
    assert!(DockCommandBarManager::find_provider(doc).is_none());
}
struct ReentrantAdapter {
    manager: RefCell<std::rc::Weak<DockCommandBarManager>>,
    calls: Cell<usize>,
}
impl PartialEq for ReentrantAdapter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl IDockCommandBarAdapter for ReentrantAdapter {
    fn build_bars(
        &self,
        _kind: DockCommandBarKind,
        _definitions: &ReadOnlyListHandle<Rc<DockCommandBarDefinition>>,
    ) -> ReadOnlyListHandle<Ref<ferroui_controls::Control>> {
        self.calls.set(self.calls.get() + 1);
        if let Some(manager) = self.manager.borrow().upgrade() {
            manager.detach();
        }
        list(vec![])
    }
}
#[test]
fn adapter_can_detach_during_same_layout_refresh_without_borrow_panics() {
    run(|| {
        let _settings = SettingsGuard::new();
        let root = RootDock::new();
        let factory = Factory::new();
        root.set_factory(Some(factory.clone()));
        let adapter = Rc::new(ReentrantAdapter {
            manager: RefCell::new(std::rc::Weak::new()),
            calls: Cell::new(0),
        });
        let manager = DockCommandBarManager::new(DockCommandBarHost::new(), Some(adapter.clone()));
        manager.attach(root.clone());
        adapter.manager.replace(Rc::downgrade(&manager));
        manager.attach(root);
        assert_eq!(adapter.calls.get(), 6);
        assert!(manager.layout.borrow().is_none());
        assert!(manager.factory.borrow().is_none());
    });
}
