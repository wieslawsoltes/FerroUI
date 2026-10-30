use super::*;
use crate::{controls::MdiLayoutPanel, model_projection};
use ferrodock_model::{
    collections::ListHandle,
    controls::{IDocument, IMdiDocument},
    core::{
        DockRect, DockingWindowState, IDockable, IDockingWindowState, ILocalTarget, MdiWindowState,
    },
};
use ferrodock_model_reactiveui::{controls::Document, core::DockableBaseImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Size, StyledElementImpl,
    Vector, VisualImpl,
};
use ferroui_controls::{AppBuilder, Application, Border, Control, ControlImpl, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-mdi-layout",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn entry(document: Rc<dyn IMdiDocument>) -> MdiLayoutEntry {
    let control = Border::new();
    control.set_use_layout_rounding(false);
    control.measure(Size::new(1000.0, 800.0));
    MdiLayoutEntry::new(control.upcast(), document)
}
fn entries(items: Vec<MdiLayoutEntry>) -> ListHandle<MdiLayoutEntry> {
    ListHandle::from_plain(items)
}
#[test]
fn original_minimized_order_reservation_defaults_and_maximized_bounds_are_preserved() {
    run(|| {
        let normal = Document::new();
        let maximized = Document::new();
        maximized.set_mdi_state(MdiWindowState::Maximized);
        let first = Document::new();
        first.set_mdi_state(MdiWindowState::Minimized);
        first.set_mdi_z_index(5);
        let second = Document::new();
        second.set_mdi_state(MdiWindowState::Minimized);
        second.set_mdi_z_index(1);
        let list = entries(vec![
            entry(normal.clone()),
            entry(first.clone()),
            entry(maximized.clone()),
            entry(second.clone()),
        ]);
        ClassicMdiLayoutManager::instance().arrange(&list, Size::new(800.0, 600.0));
        assert_eq!(normal.mdi_bounds(), DockRect::new(0.0, 0.0, 480.0, 340.8));
        assert_eq!(
            maximized.mdi_bounds(),
            DockRect::new(24.0, 24.0, 480.0, 340.8)
        );
        assert_eq!(
            list.get(0).control().bounds(),
            Rect::new(0.0, 0.0, 480.0, 340.8)
        );
        assert_eq!(
            list.get(2).control().bounds(),
            Rect::new(0.0, 0.0, 800.0, 568.0)
        );
        assert_eq!(
            list.get(3).control().bounds(),
            Rect::new(0.0, 568.0, 220.0, 28.0)
        );
        assert_eq!(
            list.get(1).control().bounds(),
            Rect::new(224.0, 568.0, 220.0, 28.0)
        );
        assert_eq!(first.mdi_bounds(), DockRect::default());
        assert_eq!(second.mdi_bounds(), DockRect::default());
    });
}
#[test]
fn valid_bounds_are_clamped_without_writing_back_and_invalid_bounds_get_cascaded() {
    run(|| {
        let a = Document::new();
        a.set_mdi_bounds(DockRect::new(-10.0, 500.0, 1000.0, 300.0));
        let b = Document::new();
        b.set_mdi_bounds(DockRect::new(f64::NAN, 1.0, 0.0, -2.0));
        let list = entries(vec![entry(a.clone()), entry(b.clone())]);
        ClassicMdiLayoutManager::instance().arrange(&list, Size::new(400.0, 300.0));
        assert_eq!(a.mdi_bounds(), DockRect::new(-10.0, 500.0, 1000.0, 300.0));
        assert_eq!(
            list.get(0).control().bounds(),
            Rect::new(0.0, 0.0, 400.0, 300.0)
        );
        assert_eq!(b.mdi_bounds(), DockRect::new(24.0, 24.0, 240.0, 180.0));
    });
}
#[test]
fn minimized_width_compresses_stably_and_zero_width_does_not_arrange_minimized_controls() {
    run(|| {
        let mut items = Vec::new();
        for _ in 0..3 {
            let d = Document::new();
            d.set_mdi_state(MdiWindowState::Minimized);
            items.push(entry(d));
        }
        let list = entries(items);
        ClassicMdiLayoutManager::instance().arrange(&list, Size::new(308.0, 200.0));
        for i in 0..3 {
            assert_eq!(
                list.get(i).control().bounds(),
                Rect::new(104.0 * i as f64, 168.0, 100.0, 28.0)
            );
        }
        ClassicMdiLayoutManager::instance().arrange(&list, Size::new(0.0, 200.0));
        assert_eq!(
            list.get(1).control().bounds(),
            Rect::new(104.0, 168.0, 100.0, 28.0)
        );
    });
}
#[test]
fn drag_clamps_with_minimized_reservation_and_empty_entries_keep_final_size() {
    run(|| {
        let document = Document::new();
        let manager = ClassicMdiLayoutManager::new();
        let empty = entries(vec![]);
        let start = Rect::new(5.0, 10.0, 100.0, 80.0);
        assert_eq!(
            manager.get_drag_bounds(
                document.clone(),
                start,
                Vector::new(-30.0, 500.0),
                Size::new(400.0, 300.0),
                &empty
            ),
            Rect::new(0.0, 220.0, 100.0, 80.0)
        );
        let minimized = Document::new();
        minimized.set_mdi_state(MdiWindowState::Minimized);
        let list = entries(vec![entry(minimized)]);
        assert_eq!(
            manager.get_drag_bounds(
                document,
                start,
                Vector::new(1000.0, 1000.0),
                Size::new(400.0, 300.0),
                &list
            ),
            Rect::new(300.0, 188.0, 100.0, 80.0)
        );
    });
}
#[test]
fn resize_evaluates_all_edges_in_source_order_and_preserves_unknown_bits() {
    run(|| {
        let doc = Document::new();
        doc.set_min_width(0.0);
        doc.set_min_height(0.0);
        let manager = ClassicMdiLayoutManager::new();
        let empty = entries(vec![]);
        for bits in 0..16 {
            let direction = MdiResizeDirection::from_bits_retain(bits | 128);
            let mut expected = Rect::new(100.0, 100.0, 300.0, 200.0);
            if bits & 1 != 0 {
                expected.x += 10.0;
                expected.width -= 10.0;
            }
            if bits & 4 != 0 {
                expected.width += 10.0;
            }
            if bits & 2 != 0 {
                expected.y += 20.0;
                expected.height -= 20.0;
            }
            if bits & 8 != 0 {
                expected.height += 20.0;
            }
            assert_eq!(
                manager.get_resize_bounds(
                    doc.clone(),
                    Rect::new(100.0, 100.0, 300.0, 200.0),
                    Vector::new(10.0, 20.0),
                    direction,
                    Size::new(1000.0, 800.0),
                    &empty
                ),
                expected
            );
        }
        assert_eq!(MdiResizeDirection::default(), MdiResizeDirection::NONE);
    });
}
#[test]
fn resize_constraints_anchor_left_top_edges_and_available_area_clamping_runs_last() {
    run(|| {
        let doc = Document::new();
        let manager = ClassicMdiLayoutManager::new();
        let empty = entries(vec![]);
        let size = Size::new(1000.0, 800.0);
        let start = Rect::new(100.0, 100.0, 300.0, 200.0);
        assert_eq!(
            manager.get_resize_bounds(
                doc.clone(),
                start,
                Vector::new(250.0, 170.0),
                MdiResizeDirection::LEFT | MdiResizeDirection::TOP,
                size,
                &empty
            ),
            Rect::new(200.0, 180.0, 200.0, 120.0)
        );
        doc.set_max_width(250.0);
        doc.set_max_height(150.0);
        assert_eq!(
            manager.get_resize_bounds(
                doc.clone(),
                start,
                Vector::new(-100.0, -100.0),
                MdiResizeDirection::LEFT | MdiResizeDirection::TOP,
                size,
                &empty
            ),
            Rect::new(150.0, 150.0, 250.0, 150.0)
        );
        assert_eq!(
            manager.get_resize_bounds(
                doc,
                start,
                Vector::default(),
                MdiResizeDirection::NONE,
                Size::new(100.0, 80.0),
                &empty
            ),
            Rect::new(0.0, 0.0, 100.0, 80.0)
        );
    });
}
#[test]
fn contradictory_constraints_and_nan_infinite_negative_geometry_keep_source_branches() {
    run(|| {
        let doc = Document::new();
        doc.set_min_width(300.0);
        doc.set_max_width(100.0);
        doc.set_min_height(200.0);
        doc.set_max_height(60.0);
        let manager = ClassicMdiLayoutManager::new();
        let empty = entries(vec![]);
        let r = manager.get_resize_bounds(
            doc.clone(),
            Rect::new(0.0, 0.0, 150.0, 70.0),
            Vector::default(),
            MdiResizeDirection::NONE,
            Size::new(1000.0, 800.0),
            &empty,
        );
        assert_eq!(r, Rect::new(0.0, 0.0, 300.0, 200.0));
        let r = manager.get_drag_bounds(
            doc.clone(),
            Rect::new(f64::NAN, -0.0, f64::INFINITY, 30.0),
            Vector::default(),
            Size::new(500.0, 400.0),
            &empty,
        );
        assert!(r.x.is_nan());
        assert_eq!(r.width, 500.0);
        let r = manager.get_drag_bounds(
            doc.clone(),
            Rect::new(1.0, 2.0, 3.0, 4.0),
            Vector::default(),
            Size::new(f64::NAN, 400.0),
            &empty,
        );
        assert!(r.width.is_nan());
        assert_eq!(
            manager.get_drag_bounds(
                doc,
                Rect::new(0.0, 0.0, -1.0, 3.0),
                Vector::default(),
                Size::new(100.0, 80.0),
                &empty
            ),
            Rect::default()
        );
    });
}
#[test]
fn z_order_uses_a_snapshot_moves_only_first_active_member_and_retains_duplicates() {
    run(|| {
        let a = Document::new();
        let b = Document::new();
        let absent = Document::new();
        let manager = ClassicMdiLayoutManager::new();
        let list: ListHandle<Rc<dyn IMdiDocument>> =
            ListHandle::from_plain(vec![a.clone(), b.clone(), a.clone()]);
        manager.update_z_order(&list, Some(a.clone()));
        assert_eq!(b.mdi_z_index(), 0);
        assert_eq!(a.mdi_z_index(), 2);
        assert_eq!(list.count(), 3);
        manager.update_z_order(&list, Some(absent.clone()));
        assert_eq!(b.mdi_z_index(), 1);
        assert_eq!(absent.mdi_z_index(), 0);
        manager.update_z_order(&ListHandle::from_plain(vec![]), None);
    });
}
struct RecordingManager {
    calls: Cell<usize>,
    seen: RefCell<Vec<Rc<dyn IMdiDocument>>>,
}
impl RecordingManager {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            calls: Cell::new(0),
            seen: RefCell::new(Vec::new()),
        })
    }
}
impl IMdiLayoutManager for RecordingManager {
    fn arrange(&self, list: &ListHandle<MdiLayoutEntry>, size: Size) {
        self.calls.set(self.calls.get() + 1);
        self.seen
            .replace(list.iter().map(|e| e.document()).collect());
        ClassicMdiLayoutManager::instance().arrange(list, size);
    }
    fn get_drag_bounds(
        &self,
        d: Rc<dyn IMdiDocument>,
        r: Rect,
        v: Vector,
        s: Size,
        e: &ListHandle<MdiLayoutEntry>,
    ) -> Rect {
        ClassicMdiLayoutManager::instance().get_drag_bounds(d, r, v, s, e)
    }
    fn get_resize_bounds(
        &self,
        d: Rc<dyn IMdiDocument>,
        r: Rect,
        v: Vector,
        x: MdiResizeDirection,
        s: Size,
        e: &ListHandle<MdiLayoutEntry>,
    ) -> Rect {
        ClassicMdiLayoutManager::instance().get_resize_bounds(d, r, v, x, s, e)
    }
    fn update_z_order(
        &self,
        d: &ListHandle<Rc<dyn IMdiDocument>>,
        a: Option<Rc<dyn IMdiDocument>>,
    ) {
        ClassicMdiLayoutManager::instance().update_z_order(d, a);
    }
}
#[repr(C)]
struct Measured {
    base: Control,
    measures: Rc<RefCell<Vec<Size>>>,
}
ferro_class!(Measured:Control);
ferroui_base::ferro_class_info!(Measured {});
ferro_impl_classes!(Measured:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl LayoutableImpl for Measured {
    fn measure_override(this: &Self, size: Size) -> Size {
        this.measures.borrow_mut().push(size);
        Size::default()
    }
}
#[test]
fn native_panel_measures_every_child_filters_original_context_handles_and_custom_manager_invalidates_arrange(
) {
    run(|| {
        let panel = MdiLayoutPanel::new();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let child = instantiate(Measured {
            base: Control::construct(),
            measures: calls.clone(),
        });
        let document = Document::new();
        let other = Border::new();
        other.set_data_context(Some(Rc::new("unrelated".to_owned())));
        child.set_data_context(Some(model_projection::to_mdi_document_value(
            document.clone(),
        )));
        panel.children().add(child.clone());
        panel.children().add(other.clone());
        let window = Window::new();
        window.set_width(600.0);
        window.set_height(400.0);
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
        assert!(calls.borrow().contains(&Size::new(600.0, 400.0)));
        assert_eq!(child.bounds(), Rect::new(0.0, 0.0, 360.0, 240.0));
        assert_eq!(other.bounds(), Rect::default());
        let manager = RecordingManager::new();
        panel.set_layout_manager(Some(manager.clone()));
        assert!(!panel.is_arrange_valid());
        window.update_layout();
        assert_eq!(manager.calls.get(), 1);
        assert_eq!(manager.seen.borrow().len(), 1);
        assert!(Rc::ptr_eq(
            &manager.seen.borrow()[0],
            &(document.clone() as Rc<dyn IMdiDocument>)
        ));
        document.set_mdi_bounds(DockRect::new(10.0, 20.0, 220.0, 130.0));
        panel.set_layout_manager(None);
        window.update_layout();
        assert_eq!(child.bounds(), Rect::new(10.0, 20.0, 220.0, 130.0));
        assert_eq!(manager.calls.get(), 1);
        assert!(panel.layout_manager().is_none());
        panel.clear_value(MdiLayoutPanel::layout_manager_property());
        assert!(panel.layout_manager().is_some());
        window.close();
    });
}
#[test]
fn mdi_entry_and_default_manager_keep_original_control_document_and_default_instance_identity() {
    run(|| {
        let doc = Document::new();
        let e = entry(doc.clone());
        let clone = e.clone();
        assert_eq!(e.control(), clone.control());
        assert!(Rc::ptr_eq(&e.document(), &clone.document()));
        assert!(e == clone);
        let manager = ClassicMdiLayoutManager::instance();
        assert!(Rc::ptr_eq(&manager, &ClassicMdiLayoutManager::instance()));
        assert!(!Rc::ptr_eq(&manager, &ClassicMdiLayoutManager::new()));
        let panel = MdiLayoutPanel::new();
        assert!(Rc::ptr_eq(
            &panel.layout_manager().unwrap(),
            &(manager as Rc<dyn IMdiLayoutManager>)
        ));
        let boxed = model_projection::to_mdi_document_value(doc.clone());
        assert!(Rc::ptr_eq(
            &model_projection::dockable(Some(&boxed)).unwrap(),
            &(doc.clone() as Rc<dyn IDockable>)
        ));
        let other = model_projection::to_dockable_value(doc.clone());
        assert!(Rc::ptr_eq(
            &model_projection::mdi_document(Some(&other)).unwrap(),
            &(doc as Rc<dyn IMdiDocument>)
        ));
    });
}
struct Probe {
    base: Document,
    name: &'static str,
    calls: Rc<RefCell<Vec<String>>>,
    state_hook: RefCell<Option<Rc<dyn Fn()>>>,
    key_hook: RefCell<Option<Rc<dyn Fn()>>>,
}
ferroui_reactiveui::reactive_object!(Probe, base);
impl DockableBaseImpl for Probe {}
ferrodock_model_reactiveui::impl_dockable!(@base Probe,Document,fn as_document(self:Rc<Self>)->Option<Rc<dyn IDocument>>{Some(self)},fn as_mdi_document(self:Rc<Self>)->Option<Rc<dyn IMdiDocument>>{Some(self)});
impl ILocalTarget for Probe {}
impl IDocument for Probe {}
impl IMdiDocument for Probe {
    fn mdi_bounds(&self) -> DockRect {
        self.log("bounds");
        self.base.mdi_bounds()
    }
    fn set_mdi_bounds(&self, v: DockRect) {
        self.log("set-bounds");
        self.base.set_mdi_bounds(v);
    }
    fn mdi_state(&self) -> MdiWindowState {
        self.log("state");
        let hook = self.state_hook.borrow_mut().take();
        if let Some(hook) = hook {
            hook();
        }
        self.base.mdi_state()
    }
    fn set_mdi_state(&self, v: MdiWindowState) {
        self.base.set_mdi_state(v);
    }
    fn mdi_z_index(&self) -> i32 {
        self.log("key");
        let hook = self.key_hook.borrow_mut().take();
        if let Some(hook) = hook {
            hook();
        }
        self.base.mdi_z_index()
    }
    fn set_mdi_z_index(&self, v: i32) {
        self.log("set-z");
        self.base.set_mdi_z_index(v);
    }
}
impl IDockingWindowState for Probe {
    fn is_open(&self) -> bool {
        self.base.is_open()
    }
    fn set_is_open(&self, v: bool) {
        self.base.set_is_open(v);
    }
    fn is_active(&self) -> bool {
        self.base.is_active()
    }
    fn set_is_active(&self, v: bool) {
        self.base.set_is_active(v);
    }
    fn is_selected(&self) -> bool {
        self.base.is_selected()
    }
    fn set_is_selected(&self, v: bool) {
        self.base.set_is_selected(v);
    }
    fn docking_state(&self) -> DockingWindowState {
        self.base.docking_state()
    }
    fn set_docking_state(&self, v: DockingWindowState) {
        self.base.set_docking_state(v);
    }
}
impl Probe {
    fn new(name: &'static str, calls: Rc<RefCell<Vec<String>>>) -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            base: Document::construct(weak.clone()),
            name,
            calls,
            state_hook: RefCell::new(None),
            key_hook: RefCell::new(None),
        })
    }
    fn log(&self, kind: &str) {
        self.calls
            .borrow_mut()
            .push(format!("{}:{kind}", self.name));
    }
}
#[test]
fn source_partition_passes_and_cached_stable_sort_key_evaluation_remain_observable() {
    run(|| {
        for count in 1..=3 {
            let calls = Rc::new(RefCell::new(Vec::new()));
            let mut all = Vec::new();
            for name in ["a", "b", "c"].into_iter().take(count) {
                let p = Probe::new(name, calls.clone());
                p.set_mdi_state(MdiWindowState::Minimized);
                all.push(entry(p));
            }
            let list = entries(all);
            ClassicMdiLayoutManager::new().arrange(&list, Size::new(800.0, 600.0));
            let calls = calls.borrow();
            assert_eq!(
                calls.iter().filter(|s| s.ends_with(":state")).count(),
                count * 2
            );
            assert_eq!(
                calls.iter().filter(|s| s.ends_with(":key")).count(),
                if count == 1 { 0 } else { count }
            );
            if count == 3 {
                assert_eq!(&calls[0..3], &["a:state", "b:state", "c:state"]);
                assert_eq!(&calls[3..6], &["a:key", "b:key", "c:key"]);
                assert_eq!(&calls[6..9], &["a:state", "b:state", "c:state"]);
            }
        }
    });
}
#[test]
fn sort_key_model_changes_affect_second_partition_and_drag_argument_stays_unused() {
    run(|| {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let a = Probe::new("a", calls.clone());
        let b = Probe::new("b", calls.clone());
        let c = Probe::new("c", calls.clone());
        a.set_mdi_state(MdiWindowState::Minimized);
        c.set_mdi_state(MdiWindowState::Minimized);
        let changed = b.clone();
        a.key_hook.replace(Some(Rc::new(move || {
            changed.set_mdi_state(MdiWindowState::Minimized)
        })));
        let list = entries(vec![entry(a.clone()), entry(b.clone()), entry(c)]);
        ClassicMdiLayoutManager::new().arrange(&list, Size::new(800.0, 600.0));
        assert_eq!(list.get(1).control().bounds(), Rect::default());
        assert_eq!(b.base.mdi_bounds(), DockRect::default());
        calls.borrow_mut().clear();
        ClassicMdiLayoutManager::new().get_drag_bounds(
            a,
            Rect::new(0.0, 0.0, 10.0, 10.0),
            Vector::default(),
            Size::new(100.0, 100.0),
            &entries(vec![]),
        );
        assert!(calls.borrow().is_empty());
    });
}
#[test]
fn source_plain_list_mutation_during_model_state_read_invalidates_enumeration() {
    run(|| {
        let calls = Rc::new(RefCell::new(Vec::new()));
        let a = Probe::new("a", calls);
        let list = entries(vec![entry(a.clone())]);
        let changed = list.clone();
        a.state_hook.replace(Some(Rc::new(move || changed.clear())));
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ClassicMdiLayoutManager::new().arrange(&list, Size::new(800.0, 600.0))
        }))
        .is_err());
        assert!(list.is_empty());
    });
}

#[test]
fn source_entry_constructor_and_readonly_properties_have_native_value_metadata() {
    run(|| {
        use ferroui_base::data::core::ValueType;
        use ferroui_base::metadata::{MarkupTypeKind, MarkupTyped};
        use ferroui_base::{BoxedValue, Ref};
        let descriptor = MdiLayoutEntry::MARKUP;
        assert_eq!(descriptor.kind, MarkupTypeKind::Struct);
        assert_eq!(descriptor.namespace(), "FerroDock.Ferro.Mdi");
        assert_eq!(
            (descriptor.nullable.unwrap())(),
            ValueType::of::<Option<MdiLayoutEntry>>()
        );
        let control = Border::new().upcast::<Control>();
        let document = Document::new() as Rc<dyn IMdiDocument>;
        let value = (descriptor.constructors[0].invoke)(&[
            Some(Rc::new(control.clone()) as BoxedValue),
            Some(Rc::new(document.clone()) as BoxedValue),
        ])
        .unwrap()
        .unwrap();
        let entry = value.downcast_ref::<MdiLayoutEntry>().unwrap();
        assert_eq!(entry.control(), control);
        assert!(Rc::ptr_eq(&entry.document(), &document));
        let control_property = descriptor.find_property("Control").unwrap();
        let document_property = descriptor.find_property("Document").unwrap();
        assert!(control_property.set.is_none());
        assert!(document_property.set.is_none());
        let actual = (control_property.get.unwrap())(&[Some(value.clone())])
            .unwrap()
            .unwrap();
        assert_eq!(actual.downcast_ref::<Ref<Control>>().unwrap(), &control);
        let actual = (document_property.get.unwrap())(&[Some(value)])
            .unwrap()
            .unwrap();
        assert!(Rc::ptr_eq(
            actual.downcast_ref::<Rc<dyn IMdiDocument>>().unwrap(),
            &document
        ));
    });
}

#[test]
fn native_manager_metadata_invokes_all_original_contract_methods_and_singleton_field() {
    run(|| {
        use ferroui_base::metadata::{MarkupTypeKind, MarkupTyped};
        use ferroui_base::BoxedValue;
        fn boxed<T: ferroui_base::PropertyValue>(value: T) -> Option<BoxedValue> {
            Some(Rc::new(value))
        }
        let class = ClassicMdiLayoutManager::MARKUP;
        let contract = <dyn IMdiLayoutManager>::MARKUP;
        assert_eq!(class.kind, MarkupTypeKind::Class);
        assert_eq!(contract.kind, MarkupTypeKind::Interface);
        assert_eq!(class.methods.len(), 4);
        assert_eq!(contract.methods.len(), 4);
        let shared = (class.fields[0].get)().unwrap();
        assert!(Rc::ptr_eq(
            shared
                .downcast_ref::<Rc<ClassicMdiLayoutManager>>()
                .unwrap(),
            &ClassicMdiLayoutManager::instance()
        ));
        let created = (class.constructors[0].invoke)(&[]).unwrap().unwrap();
        assert!(!Rc::ptr_eq(
            created
                .downcast_ref::<Rc<ClassicMdiLayoutManager>>()
                .unwrap(),
            &ClassicMdiLayoutManager::instance()
        ));
        for (descriptor, this) in [
            (class, Some(created)),
            (
                contract,
                boxed(ClassicMdiLayoutManager::instance() as Rc<dyn IMdiLayoutManager>),
            ),
        ] {
            let document = Document::new() as Rc<dyn IMdiDocument>;
            document.set_min_width(0.0);
            document.set_min_height(0.0);
            let list = entries(vec![entry(document.clone())]);
            let method = |name| {
                descriptor
                    .methods
                    .iter()
                    .find(|method| method.name == name)
                    .unwrap()
            };
            (method("Arrange").invoke)(&[
                this.clone(),
                boxed(list.clone()),
                boxed(Size::new(500.0, 400.0)),
            ])
            .unwrap();
            assert_eq!(document.mdi_bounds(), DockRect::new(0.0, 0.0, 300.0, 240.0));
            let start = Rect::new(20.0, 30.0, 80.0, 90.0);
            let args = [
                this.clone(),
                boxed(document.clone()),
                boxed(start),
                boxed(Vector::new(4.0, 6.0)),
                boxed(Size::new(500.0, 400.0)),
                boxed(list.clone()),
            ];
            let dragged = (method("GetDragBounds").invoke)(&args).unwrap().unwrap();
            assert_eq!(
                *dragged.downcast_ref::<Rect>().unwrap(),
                Rect::new(24.0, 36.0, 80.0, 90.0)
            );
            let args = [
                this.clone(),
                boxed(document.clone()),
                boxed(start),
                boxed(Vector::new(4.0, 6.0)),
                boxed(MdiResizeDirection::RIGHT | MdiResizeDirection::BOTTOM),
                boxed(Size::new(500.0, 400.0)),
                boxed(list),
            ];
            let resized = (method("GetResizeBounds").invoke)(&args).unwrap().unwrap();
            assert_eq!(
                *resized.downcast_ref::<Rect>().unwrap(),
                Rect::new(20.0, 30.0, 84.0, 96.0)
            );
            document.set_mdi_z_index(42);
            (method("UpdateZOrder").invoke)(&[
                this,
                boxed(ListHandle::from_plain(vec![document.clone()])),
                boxed(Some(document.clone())),
            ])
            .unwrap();
            assert_eq!(document.mdi_z_index(), 0);
        }
    });
}

#[test]
fn native_resize_flags_metadata_preserves_declared_values_and_records_unknown_conversion_boundary()
{
    use ferroui_base::metadata::MarkupTyped;
    let descriptor = MdiResizeDirection::MARKUP;
    assert!(descriptor.is_flags);
    let actual: Vec<_> = descriptor
        .enum_members
        .iter()
        .map(|member| (member.name, member.value))
        .collect();
    assert_eq!(
        actual,
        [
            ("None", 0),
            ("Left", 1),
            ("Top", 2),
            ("Right", 4),
            ("Bottom", 8)
        ]
    );
    for bits in 0..16 {
        let value = (descriptor.enum_from_value.unwrap())(bits).unwrap();
        assert_eq!(
            value.downcast_ref::<MdiResizeDirection>().unwrap().bits(),
            bits as i32
        );
        assert_eq!((descriptor.enum_to_value.unwrap())(&*value), Some(bits));
    }
    // The Rust flag value keeps unknown bits, while the published shared metadata
    // rejects unnamed numeric bits. Track that source interop difference explicitly.
    assert_eq!(MdiResizeDirection::from_bits_retain(128).bits(), 128);
    assert!((descriptor.enum_from_value.unwrap())(128).is_none());
}
