use crate::controls::*;
use crate::core::{DockableBase, DockableBaseImpl};
use crate::factory_metadata_tests::{DocumentMetadata, ToolMetadata};
use crate::Factory;
use ferrodock_model::collections::ListHandle;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::{BoxedValue, DockState};
use ferroui_reactiveui::reactive_object;
use std::cell::{Cell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::rc::{Rc, Weak};

type Callback = Rc<dyn Fn()>;
pub(crate) struct Content {
    base: DockableBase,
    tool: bool,
    document: bool,
    value: RefCell<Option<BoxedValue>>,
    reads: Cell<usize>,
    on_get: RefCell<Option<Callback>>,
    on_set: RefCell<Option<Callback>>,
}
reactive_object!(Content, base);
impl DockableBaseImpl for Content {}
crate::impl_dockable!(
    Content,
    fn as_tool_content(self: Rc<Self>) -> Option<Rc<dyn IToolContent>> {
        if self.tool {
            Some(self)
        } else {
            None
        }
    },
    fn as_document_content(self: Rc<Self>) -> Option<Rc<dyn IDocumentContent>> {
        if self.document {
            Some(self)
        } else {
            None
        }
    }
);
impl Content {
    pub(crate) fn new(id: &str, tool: bool, document: bool, value: Option<BoxedValue>) -> Rc<Self> {
        let model = Rc::new_cyclic(|runtime: &Weak<Self>| Self {
            base: DockableBase::construct(runtime.clone()),
            tool,
            document,
            value: RefCell::new(value),
            reads: Cell::new(0),
            on_get: RefCell::new(None),
            on_set: RefCell::new(None),
        });
        model.set_id(id.into());
        model
    }
    fn get(&self) -> Option<BoxedValue> {
        self.reads.set(self.reads.get() + 1);
        let value = self.value.borrow().clone();
        let callback = self.on_get.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
        value
    }
    fn set(&self, value: Option<BoxedValue>) {
        let previous = self.value.replace(value);
        drop(previous);
        let callback = self.on_set.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}
impl IToolContent for Content {
    fn content(&self) -> Option<BoxedValue> {
        self.get()
    }
    fn set_content(&self, value: Option<BoxedValue>) {
        self.set(value);
    }
}
impl IDocumentContent for Content {
    fn content(&self) -> Option<BoxedValue> {
        self.get()
    }
    fn set_content(&self, value: Option<BoxedValue>) {
        self.set(value);
    }
}
pub(crate) struct Template(pub(crate) RefCell<Option<BoxedValue>>);
impl IDocumentTemplate for Template {
    fn content(&self) -> Option<BoxedValue> {
        self.0.borrow().clone()
    }
    fn set_content(&self, value: Option<BoxedValue>) {
        *self.0.borrow_mut() = value;
    }
}
impl IToolTemplate for Template {
    fn content(&self) -> Option<BoxedValue> {
        self.0.borrow().clone()
    }
    fn set_content(&self, value: Option<BoxedValue>) {
        *self.0.borrow_mut() = value;
    }
}
fn list(items: Vec<Rc<dyn IDockable>>) -> Option<ListHandle<Option<Rc<dyn IDockable>>>> {
    Some(ListHandle::from_items(
        (items).into_iter().map(Some).collect(),
    ))
}

#[test]
fn state_roundtrips_content_in_hidden_four_pin_strips_preview_windows_and_visible_children() {
    let factory = Factory::new();
    let root = factory.create_root_dock();
    let nodes: Vec<_> = (0..8)
        .map(|index| {
            Content::new(
                &index.to_string(),
                index % 2 == 0,
                index % 2 != 0,
                Some(Rc::new(index)),
            )
        })
        .collect();
    root.set_hidden_dockables(list(vec![nodes[0].clone()]));
    root.set_left_pinned_dockables(list(vec![nodes[1].clone()]));
    root.set_right_pinned_dockables(list(vec![nodes[2].clone()]));
    root.set_top_pinned_dockables(list(vec![nodes[3].clone()]));
    root.set_bottom_pinned_dockables(list(vec![nodes[4].clone()]));
    let preview = factory.create_tool_dock();
    preview.set_visible_dockables(list(vec![nodes[5].clone()]));
    root.set_pinned_dock(Some(preview));
    let window = factory.create_dock_window();
    let layout = factory.create_root_dock();
    layout.set_visible_dockables(list(vec![nodes[6].clone()]));
    window.set_layout(Some(layout));
    root.set_windows(Some(factory.create_list(vec![Some(window)])));
    root.set_visible_dockables(list(vec![nodes[7].clone()]));
    let state = DockState::new();
    state.save(root.clone());
    for node in &nodes {
        node.set(None);
    }
    state.restore(root);
    for (index, node) in nodes.iter().enumerate() {
        assert_eq!(
            node.get().unwrap().downcast_ref::<i32>(),
            Some(&(index as i32))
        );
    }
}
#[test]
fn state_restores_tool_and_document_templates_before_content_and_saved_null_suppresses_fallback() {
    for tool in [false, true] {
        let factory = Factory::new();
        let root = factory.create_root_dock();
        let content = Content::new("item", tool, !tool, None);
        let fallback: BoxedValue = Rc::new("fallback".to_owned());
        let template = Rc::new(Template(RefCell::new(Some(fallback.clone()))));
        let dock: Rc<dyn IDock> = if tool {
            let dock = ToolMetadata::new();
            dock.set_tool_template(Some(template.clone()));
            dock
        } else {
            let dock = DocumentMetadata::new();
            dock.set_document_template(Some(template.clone()));
            dock
        };
        dock.set_id("dock".into());
        dock.set_visible_dockables(list(vec![content.clone()]));
        root.set_visible_dockables(list(vec![dock.clone()]));
        factory.init_layout(root.clone());
        let state = DockState::new();
        state.save(root.clone());
        if tool {
            dock.clone()
                .as_tool_dock_content()
                .unwrap()
                .set_tool_template(None);
        } else {
            dock.clone()
                .as_document_dock_content()
                .unwrap()
                .set_document_template(None);
        }
        content.set(Some(Rc::new("changed".to_owned())));
        state.restore(root.clone());
        assert!(content.get().is_none());
        if tool {
            assert!(Rc::ptr_eq(
                &dock
                    .clone()
                    .as_tool_dock_content()
                    .unwrap()
                    .tool_template()
                    .unwrap(),
                &(template.clone() as Rc<dyn IToolTemplate>)
            ));
        } else {
            assert!(Rc::ptr_eq(
                &dock
                    .clone()
                    .as_document_dock_content()
                    .unwrap()
                    .document_template()
                    .unwrap(),
                &(template.clone() as Rc<dyn IDocumentTemplate>)
            ));
        }
        state.reset();
        state.restore(root);
        assert!(Rc::ptr_eq(&content.get().unwrap(), &fallback));
    }
}
#[test]
fn state_duplicate_ids_keep_last_visit_but_tool_document_and_template_namespaces_remain_separate() {
    let root = RootDock::new();
    let first = Content::new("same", true, false, Some(Rc::new(1_i32)));
    let second = Content::new("same", true, false, Some(Rc::new(2_i32)));
    let document = Content::new("same", false, true, Some(Rc::new(3_i32)));
    root.set_hidden_dockables(list(vec![first.clone()]));
    root.set_visible_dockables(list(vec![second.clone(), document.clone()]));
    let state = DockState::new();
    state.save(root.clone());
    for node in [&first, &second, &document] {
        node.set(None);
    }
    state.restore(root);
    assert_eq!(first.get().unwrap().downcast_ref::<i32>(), Some(&2));
    assert_eq!(second.get().unwrap().downcast_ref::<i32>(), Some(&2));
    assert_eq!(document.get().unwrap().downcast_ref::<i32>(), Some(&3));
}
#[test]
fn state_skips_empty_ids_and_saves_tool_content_before_document_content_on_multi_contract_objects()
{
    let root = RootDock::new();
    let empty = Content::new("", true, false, Some(Rc::new(1_i32)));
    let both = Content::new("same", true, true, Some(Rc::new(2_i32)));
    root.set_visible_dockables(list(vec![empty.clone(), both.clone()]));
    let state = DockState::new();
    state.save(root.clone());
    assert_eq!(empty.reads.get(), 0);
    let document = Content::new("same", false, true, None);
    root.set_visible_dockables(list(vec![document.clone()]));
    state.restore(root.clone());
    assert!(document.get().is_none());
    let tool = Content::new("same", true, false, None);
    root.set_visible_dockables(list(vec![tool.clone()]));
    state.restore(root);
    assert_eq!(tool.get().unwrap().downcast_ref::<i32>(), Some(&2));
}
#[test]
fn state_split_view_extras_are_not_visited_twice_when_visible_or_shared_between_slots() {
    let split = SplitViewDock::new();
    let content = Content::new("content", false, true, Some(Rc::new(5_i32)));
    split.set_pane_dockable(Some(content.clone()));
    split.set_content_dockable(Some(content.clone()));
    let state = DockState::new();
    state.save(split.clone());
    assert_eq!(content.reads.get(), 1);
    content.reads.set(0);
    split.set_visible_dockables(list(vec![content.clone()]));
    state.save(split.clone());
    assert_eq!(content.reads.get(), 1);
    content.set(None);
    state.restore(split);
    assert_eq!(content.get().unwrap().downcast_ref::<i32>(), Some(&5));
}
#[test]
fn state_does_not_cache_the_entry_dock_template_itself_and_only_restores_listed_children() {
    let dock = ToolMetadata::new();
    dock.set_id("dock".into());
    let template: Rc<dyn IToolTemplate> = Rc::new(Template(RefCell::new(None)));
    dock.set_tool_template(Some(template));
    let state = DockState::new();
    state.save(dock.clone());
    dock.set_tool_template(None);
    state.restore(dock);
    assert!(state_tool_template_is_absent(&state));
    fn state_tool_template_is_absent(state: &DockState) -> bool {
        let root = RootDock::new();
        let dock = ToolMetadata::new();
        dock.set_id("dock".into());
        root.set_visible_dockables(list(vec![dock.clone()]));
        state.restore(root);
        dock.tool_template().is_none()
    }
}
#[test]
fn state_uses_live_versioned_enumeration_and_propagates_restore_observer_failure() {
    let root = RootDock::new();
    let a = Content::new("a", true, false, Some(Rc::new(1_i32)));
    let b = Content::new("b", true, false, Some(Rc::new(2_i32)));
    let items = ListHandle::from_plain(vec![Some(a.clone() as Rc<dyn IDockable>), Some(b)]);
    root.set_visible_dockables(Some(items.clone()));
    let state = DockState::new();
    state.save(root.clone());
    *a.on_set.borrow_mut() = Some(Rc::new(move || items.clear()));
    assert!(catch_unwind(AssertUnwindSafe(|| state.restore(root))).is_err());
}
#[test]
fn state_reset_releases_retained_values_outside_mutable_cache_borrows() {
    struct OnDrop(Weak<DockState>, Rc<Cell<usize>>);
    impl Drop for OnDrop {
        fn drop(&mut self) {
            self.1.set(self.1.get() + 1);
            self.0.upgrade().unwrap().reset();
        }
    }
    let state = DockState::new();
    let calls = Rc::new(Cell::new(0));
    let root = RootDock::new();
    let content = Content::new(
        "item",
        true,
        false,
        Some(Rc::new(OnDrop(Rc::downgrade(&state), calls.clone()))),
    );
    root.set_visible_dockables(list(vec![content.clone()]));
    state.save(root);
    content.set(None);
    assert_eq!(calls.get(), 0);
    state.reset();
    assert_eq!(calls.get(), 1);
}

#[test]
fn content_state_interfaces_are_available_through_typed_locators() {
    let factory = Factory::new();
    let tool = Content::new("tool", true, false, None);
    let document = Content::new("document", false, true, None);
    let registry: DockableLocatorMap = Rc::new(RefCell::new(
        ferroui_system::collections::generic::Dictionary::new(),
    ));
    let located = tool.clone();
    registry
        .borrow_mut()
        .insert("tool".into(), Some(Rc::new(move || Some(located.clone()))));
    let located = document.clone();
    registry.borrow_mut().insert(
        "document".into(),
        Some(Rc::new(move || Some(located.clone()))),
    );
    factory.set_dockable_locator(Some(registry));
    assert!(Rc::ptr_eq(
        &factory
            .get_dockable::<dyn IToolContent>("tool".into())
            .unwrap(),
        &(tool as Rc<dyn IToolContent>)
    ));
    assert!(Rc::ptr_eq(
        &factory
            .get_dockable::<dyn IDocumentContent>("document".into())
            .unwrap(),
        &(document as Rc<dyn IDocumentContent>)
    ));
}
