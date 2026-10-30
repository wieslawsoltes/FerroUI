//! Composed subclasses preserve virtual method dispatch through their base components.
use crate::controls::*;
use crate::core::DockableBaseImpl;
use ferrodock_model::controls::*;
use ferrodock_model::core::*;
use ferrodock_model::BoxedValue;
use ferroui_reactiveui::{reactive_object, ICommand};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct CustomTool {
    base: ToolDock,
    calls: RefCell<Vec<&'static str>>,
}
reactive_object!(CustomTool, base);
impl DockableBaseImpl for CustomTool {}
crate::impl_dock!(@base CustomTool, ToolDock, fn as_tool_dock(self: Rc<Self>) -> Option<Rc<dyn IToolDock>> { Some(self) }
);
impl ILocalTarget for CustomTool {}
impl IToolDock for CustomTool {
    fn alignment(&self) -> Alignment {
        self.base.alignment()
    }
    fn set_alignment(&self, value: Alignment) {
        self.base.set_alignment(value);
    }
    fn is_expanded(&self) -> bool {
        self.base.is_expanded()
    }
    fn set_is_expanded(&self, value: bool) {
        self.base.set_is_expanded(value);
    }
    fn auto_hide(&self) -> bool {
        self.base.auto_hide()
    }
    fn set_auto_hide(&self, value: bool) {
        self.base.set_auto_hide(value);
    }
    fn grip_mode(&self) -> GripMode {
        self.base.grip_mode()
    }
    fn set_grip_mode(&self, value: GripMode) {
        self.base.set_grip_mode(value);
    }
    fn add_tool(&self, tool: Option<Rc<dyn IDockable>>) {
        self.calls.borrow_mut().push(if tool.is_none() {
            "add_tool_null"
        } else {
            "add_tool"
        });
    }
}
impl CustomTool {
    fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: ToolDock::construct(weak),
            calls: RefCell::new(Vec::new()),
        });
        model.base.initialize(&model);
        model
    }
}

struct CustomDocument {
    base: DocumentDock,
    calls: RefCell<Vec<&'static str>>,
}
reactive_object!(CustomDocument, base);
impl DockableBaseImpl for CustomDocument {}
crate::impl_dock!(@base CustomDocument, DocumentDock, fn as_document_dock(self: Rc<Self>) -> Option<Rc<dyn IDocumentDock>> { Some(self) }, fn as_document_dock_factory(self: Rc<Self>) -> Option<Rc<dyn IDocumentDockFactory>> { Some(self) }
);
impl ILocalTarget for CustomDocument {}
impl IDocumentDock for CustomDocument {
    fn can_create_document(&self) -> bool {
        self.base.can_create_document()
    }
    fn set_can_create_document(&self, value: bool) {
        self.base.set_can_create_document(value);
    }
    fn create_document(&self) -> Option<Rc<dyn ICommand>> {
        self.base.create_document()
    }
    fn set_create_document(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_create_document(value);
    }
    fn enable_window_drag(&self) -> bool {
        self.base.enable_window_drag()
    }
    fn set_enable_window_drag(&self, value: bool) {
        self.base.set_enable_window_drag(value);
    }
    fn layout_mode(&self) -> DocumentLayoutMode {
        self.base.layout_mode()
    }
    fn set_layout_mode(&self, value: DocumentLayoutMode) {
        self.base.set_layout_mode(value);
    }
    fn tabs_layout(&self) -> DocumentTabLayout {
        self.base.tabs_layout()
    }
    fn set_tabs_layout(&self, value: DocumentTabLayout) {
        self.base.set_tabs_layout(value);
    }
    fn close_button_show_mode(&self) -> DocumentCloseButtonShowMode {
        self.base.close_button_show_mode()
    }
    fn set_close_button_show_mode(&self, value: DocumentCloseButtonShowMode) {
        self.base.set_close_button_show_mode(value);
    }
    fn empty_content(&self) -> Option<BoxedValue> {
        self.base.empty_content()
    }
    fn set_empty_content(&self, value: Option<BoxedValue>) {
        self.base.set_empty_content(value);
    }
    fn cascade_documents(&self) -> Option<Rc<dyn ICommand>> {
        self.base.cascade_documents()
    }
    fn set_cascade_documents(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_cascade_documents(value);
    }
    fn tile_documents_horizontal(&self) -> Option<Rc<dyn ICommand>> {
        self.base.tile_documents_horizontal()
    }
    fn set_tile_documents_horizontal(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_tile_documents_horizontal(value);
    }
    fn tile_documents_vertical(&self) -> Option<Rc<dyn ICommand>> {
        self.base.tile_documents_vertical()
    }
    fn set_tile_documents_vertical(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_tile_documents_vertical(value);
    }
    fn restore_documents(&self) -> Option<Rc<dyn ICommand>> {
        self.base.restore_documents()
    }
    fn set_restore_documents(&self, value: Option<Rc<dyn ICommand>>) {
        self.base.set_restore_documents(value);
    }
    fn add_document(&self, document: Option<Rc<dyn IDockable>>) {
        self.calls.borrow_mut().push(if document.is_none() {
            "add_document_null"
        } else {
            "add_document"
        });
    }
    fn add_tool(&self, tool: Option<Rc<dyn IDockable>>) {
        self.calls.borrow_mut().push(if tool.is_none() {
            "add_tool_null"
        } else {
            "add_tool"
        });
    }
}
impl IDocumentDockFactory for CustomDocument {
    fn document_factory(&self) -> Option<DocumentFactory> {
        self.base.document_factory()
    }
    fn set_document_factory(&self, value: Option<DocumentFactory>) {
        self.base.set_document_factory(value);
    }
}
impl CustomDocument {
    fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: DocumentDock::construct(weak),
            calls: RefCell::new(Vec::new()),
        });
        model.base.initialize(&model);
        model
    }
}

struct CustomSplit {
    base: SplitViewDock,
    calls: RefCell<Vec<&'static str>>,
}
reactive_object!(CustomSplit, base);
impl DockableBaseImpl for CustomSplit {}
crate::impl_dock!(@base CustomSplit, SplitViewDock, fn as_split_view_dock(self: Rc<Self>) -> Option<Rc<dyn ISplitViewDock>> { Some(self) }
);
impl ISplitViewDock for CustomSplit {
    fn compact_pane_length(&self) -> f64 {
        self.base.compact_pane_length()
    }
    fn set_compact_pane_length(&self, value: f64) {
        self.base.set_compact_pane_length(value);
    }
    fn display_mode(&self) -> SplitViewDisplayMode {
        self.base.display_mode()
    }
    fn set_display_mode(&self, value: SplitViewDisplayMode) {
        self.base.set_display_mode(value);
    }
    fn is_pane_open(&self) -> bool {
        self.base.is_pane_open()
    }
    fn set_is_pane_open(&self, value: bool) {
        self.base.set_is_pane_open(value);
    }
    fn open_pane_length(&self) -> f64 {
        self.base.open_pane_length()
    }
    fn set_open_pane_length(&self, value: f64) {
        self.base.set_open_pane_length(value);
    }
    fn pane_placement(&self) -> SplitViewPanePlacement {
        self.base.pane_placement()
    }
    fn set_pane_placement(&self, value: SplitViewPanePlacement) {
        self.base.set_pane_placement(value);
    }
    fn use_light_dismiss_overlay_mode(&self) -> bool {
        self.base.use_light_dismiss_overlay_mode()
    }
    fn set_use_light_dismiss_overlay_mode(&self, value: bool) {
        self.base.set_use_light_dismiss_overlay_mode(value);
    }
    fn pane_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.base.pane_dockable()
    }
    fn set_pane_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        self.base.set_pane_dockable(value);
    }
    fn content_dockable(&self) -> Option<Rc<dyn IDockable>> {
        self.base.content_dockable()
    }
    fn set_content_dockable(&self, value: Option<Rc<dyn IDockable>>) {
        self.base.set_content_dockable(value);
    }
    fn open_pane(&self) {
        self.calls.borrow_mut().push("open_pane");
        self.base.open_pane_core();
    }
    fn close_pane(&self) {
        self.calls.borrow_mut().push("close_pane");
        self.base.close_pane_core();
    }
    fn toggle_pane(&self) {
        self.calls.borrow_mut().push("toggle_pane");
        self.base.toggle_pane_core();
    }
}
impl CustomSplit {
    fn new() -> Rc<Self> {
        let model = Rc::new_cyclic(|weak: &Weak<Self>| Self {
            base: SplitViewDock::construct(weak),
            calls: RefCell::new(Vec::new()),
        });
        model.base.initialize(&model);
        model
    }
}

#[test]
fn tool_add_dispatches_to_the_composed_subclass_from_the_base_component() {
    let model = CustomTool::new();
    model.base.add_tool(Tool::new());
    let erased: Rc<dyn IToolDock> = model.clone();
    erased.add_tool(Some(Tool::new()));
    assert_eq!(*model.calls.borrow(), ["add_tool", "add_tool"]);
}

#[test]
fn document_creation_and_base_add_methods_dispatch_to_the_composed_subclass() {
    let model = CustomDocument::new();
    model.base.add_document(Document::new());
    model.base.add_tool(Tool::new());
    model.set_document_factory(Some(Rc::new(|| Some(Document::new()))));
    model.create_document().unwrap().execute(None);
    assert_eq!(
        *model.calls.borrow(),
        ["add_document", "add_tool", "add_document"]
    );
}

#[test]
fn split_view_actions_dispatch_to_the_composed_subclass_from_the_base_component() {
    let model = CustomSplit::new();
    model.base.open_pane();
    assert!(model.is_pane_open());
    model.base.close_pane();
    assert!(!model.is_pane_open());
    model.base.toggle_pane();
    assert!(model.is_pane_open());
    assert_eq!(
        *model.calls.borrow(),
        ["open_pane", "close_pane", "toggle_pane"]
    );
}

struct CustomLeaf {
    base: Document,
}
reactive_object!(CustomLeaf, base);
impl DockableBaseImpl for CustomLeaf {
    fn on_close(_this: &Self) -> bool {
        false
    }
    fn on_selected(this: &Self) {
        this.set_title("selected".into());
    }
    fn on_visible_bounds_changed(this: &Self, x: f64, y: f64, width: f64, height: f64) {
        assert_eq!(this.get_visible_bounds(), (x, y, width, height));
        this.set_title("bounds".into());
    }
}
crate::impl_dockable!(@base CustomLeaf, Document,
    fn as_document(self: Rc<Self>) -> Option<Rc<dyn IDocument>> { Some(self) },
    fn as_mdi_document(self: Rc<Self>) -> Option<Rc<dyn IMdiDocument>> { Some(self) },
    fn as_docking_window_state(&self) -> Option<&dyn IDockingWindowState> { Some(self) }
);
impl ILocalTarget for CustomLeaf {}
impl IDocument for CustomLeaf {}
impl IMdiDocument for CustomLeaf {
    fn mdi_bounds(&self) -> DockRect {
        self.base.mdi_bounds()
    }
    fn set_mdi_bounds(&self, value: DockRect) {
        self.base.set_mdi_bounds(value);
    }
    fn mdi_state(&self) -> MdiWindowState {
        self.base.mdi_state()
    }
    fn set_mdi_state(&self, value: MdiWindowState) {
        self.base.set_mdi_state(value);
    }
    fn mdi_z_index(&self) -> i32 {
        self.base.mdi_z_index()
    }
    fn set_mdi_z_index(&self, value: i32) {
        self.base.set_mdi_z_index(value);
    }
}
impl IDockingWindowState for CustomLeaf {
    fn is_open(&self) -> bool {
        self.base.is_open()
    }
    fn set_is_open(&self, value: bool) {
        self.base.set_is_open(value);
    }
    fn is_active(&self) -> bool {
        self.base.is_active()
    }
    fn set_is_active(&self, value: bool) {
        self.base.set_is_active(value);
    }
    fn is_selected(&self) -> bool {
        self.base.is_selected()
    }
    fn set_is_selected(&self, value: bool) {
        self.base.set_is_selected(value);
    }
    fn docking_state(&self) -> DockingWindowState {
        self.base.docking_state()
    }
    fn set_docking_state(&self, value: DockingWindowState) {
        self.base.set_docking_state(value);
    }
}

#[test]
fn concrete_leaf_components_use_the_enclosing_identity_for_notifications_and_lifecycle_sync() {
    use crate::factory_parameter_tests::RecordingFactory;
    use ferroui_reactive::ObservableExt;
    use ferroui_reactiveui::IReactiveNotifyPropertyChanged;
    let model = Rc::new_cyclic(|weak: &Weak<CustomLeaf>| CustomLeaf {
        base: Document::construct(weak.clone()),
    });
    let factory = Rc::new(RecordingFactory::default());
    factory.supports_state_sync.set(true);
    model.set_factory(Some(factory.clone()));
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let weak = Rc::downgrade(&model);
    let subscription = model.changed().subscribe_next(move |change| {
        assert!(Rc::ptr_eq(&change.sender, &weak.upgrade().unwrap()));
        log.borrow_mut().push(change.property_name);
    });
    model.base.set_title("derived".into());
    model.base.set_mdi_state(MdiWindowState::Maximized);
    model.base.set_is_open(true);
    assert_eq!(*events.borrow(), ["Title", "MdiState", "IsOpen"]);
    assert!(!model.base.on_close());
    model.base.on_selected();
    assert_eq!(model.title(), "selected");
    model.base.set_visible_bounds(1.0, 2.0, 3.0, 4.0);
    assert_eq!(model.title(), "bounds");
    let model: Rc<dyn IDockable> = model;
    assert!(Rc::ptr_eq(&factory.state_requests.borrow()[0].0, &model));
    subscription.dispose();
}

#[test]
fn fluent_append_helpers_invoke_the_enclosing_model_overrides() {
    use ferrodock_model::fluent_extensions::{
        DocumentDockAppendFluentExt, ToolDockAppendFluentExt,
    };
    let document = CustomDocument::new();
    let tool = CustomTool::new();
    let item: Rc<dyn IDockable> = Tool::new();
    let returned = document
        .clone()
        .append_document(item.clone())
        .append_tool(item.clone());
    assert!(Rc::ptr_eq(&returned, &document));
    assert_eq!(&*document.calls.borrow(), &["add_document", "add_tool"]);
    let returned = tool.clone().append_tool(item);
    assert!(Rc::ptr_eq(&returned, &tool));
    assert_eq!(&*tool.calls.borrow(), &["add_tool"]);
    assert!(document.base.visible_dockables().is_none());
    assert!(tool.base.visible_dockables().is_none());
}

#[test]
fn original_inherited_marker_queries_retain_enclosing_custom_model_identity() {
    // The fallback uses actual inherited contracts, not a type-name/kind guess.
    let tool = CustomTool::new();
    let document = CustomDocument::new();
    let leaf = Rc::new_cyclic(|runtime: &Weak<CustomLeaf>| CustomLeaf {
        base: Document::construct(runtime.clone()),
    });
    for model in [
        tool as Rc<dyn IDockable>,
        document as Rc<dyn IDockable>,
        leaf as Rc<dyn IDockable>,
    ] {
        let target = model
            .clone()
            .as_local_target()
            .expect("inherited source local target");
        assert_eq!(
            Rc::as_ptr(&model) as *const (),
            Rc::as_ptr(&target) as *const ()
        );
        assert!(model.clone().as_global_target().is_none());
        assert!(!model.runtime_type_name().contains("::"));
    }
}

#[test]
fn null_factory_results_reach_the_same_enclosing_add_document_virtual_slot() {
    let model = CustomDocument::new();
    model.set_document_factory(Some(Rc::new(|| None)));
    model.create_document().unwrap().execute(None);
    model.base.add_document_nullable(None);
    model.base.add_tool_nullable(None);
    assert_eq!(
        &*model.calls.borrow(),
        &["add_document_null", "add_document_null", "add_tool_null"]
    );
    assert!(model.visible_dockables().is_none());
}

#[test]
fn null_tool_reaches_original_virtual_slot_and_base_without_factory_is_noop() {
    let base = ToolDock::new();
    base.add_tool_nullable(None);
    assert!(base.visible_dockables().is_none());
    let custom = CustomTool::new();
    custom.base.add_tool_nullable(None);
    assert_eq!(&*custom.calls.borrow(), &["add_tool_null"]);
}
