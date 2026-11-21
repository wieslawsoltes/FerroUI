use crate::controls::{
    IRibbonItem, RibbonButton, RibbonButtonGroup, RibbonControlBase, RibbonItemHelper,
    RibbonItemLayout, RibbonLabel, RibbonSeparator, RibbonStackPanel,
};
use crate::primitives::{RibbonGroupItemsPanel, RibbonMetrics};
use ferroribbon_core::{RibbonGroupState, RibbonItemSize};
use ferroui_base::{
    Rect, Ref, Size, StaticType,
    controls::ResourceDictionary,
    data::core::{ValueType, ValueTypes},
    input::KeyboardNavigation,
    styling::ControlTheme,
};
use ferroui_controls::{Application, Control, Panel, StackPanel, Window};
use std::{cell::RefCell, rc::Rc, sync::Arc};
fn run(test: impl FnOnce() + Send + 'static) {
    crate::tests::run(move || {
        let app = Application::current().unwrap();
        let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
        app.styles().add(theme.clone());
        test();
        app.styles().remove(theme);
    })
}
fn resources() -> Ref<ResourceDictionary> {
    let value =
        crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/SimpleItems.xaml")
            .unwrap()
            .unwrap();
    ValueTypes::as_object(&*value).unwrap().cast().unwrap()
}
fn theme<T: StaticType>(r: &ResourceDictionary) -> Ref<ControlTheme> {
    ValueTypes::as_object(&*r.try_get_resource(&T::TYPE.into(), None).flatten().unwrap())
        .unwrap()
        .cast()
        .unwrap()
}
fn item(control: &Control) -> Rc<dyn IRibbonItem> {
    let value = Control::boxed(control.to_ref());
    ValueTypes::try_convert(Some(&value), ValueType::of::<Rc<dyn IRibbonItem>>())
        .flatten()
        .unwrap()
        .downcast_ref::<Rc<dyn IRibbonItem>>()
        .unwrap()
        .clone()
}
fn mount(control: &Control) -> Ref<Window> {
    let w = Window::new();
    w.set_content(Some(Control::boxed(control.to_ref())));
    w.show();
    w.measure(Size::new(800., 300.));
    w.arrange(Rect::new(0., 0., 800., 300.));
    w
}
#[test]
fn source_container_defaults_collection_identity_and_live_child_layouts() {
    run(|| {
        let group = RibbonButtonGroup::new();
        assert_eq!(group.size(), RibbonItemSize::Small);
        assert!(!group.can_add_to_quick_access());
        assert!(!KeyboardNavigation::get_is_tab_stop(&group));
        assert!(group.items() == group.items());
        assert!(group.host().is_none());
        assert!(!item(&group).invoke());
        let button = RibbonButton::new();
        button.set_size(RibbonItemSize::Large);
        group.items().add(Some(button.clone().upcast()));
        assert_eq!(button.current_size(), RibbonItemSize::Small);
        assert!(!button.actual_show_label());
        let metrics = RibbonMetrics::touch();
        group.apply_layout(RibbonItemLayout {
            size: RibbonItemSize::Large,
            metrics: Some(metrics.clone()),
            is_simplified: true,
            show_label: Some(true),
            group_state: RibbonGroupState::Collapsed,
        });
        assert_eq!(button.current_size(), RibbonItemSize::Small);
        assert!(!button.actual_show_label());
        assert!(button.is_simplified());
        assert!(Arc::ptr_eq(button.metrics().as_ref().unwrap(), &metrics));
        let stack = RibbonStackPanel::new();
        let large = RibbonButton::new();
        large.set_size_definition(Some("Large, Medium, Small".into()));
        stack.items().add(Some(large.clone().upcast()));
        for (state, expected) in [
            (RibbonGroupState::Large, RibbonItemSize::Large),
            (RibbonGroupState::Medium, RibbonItemSize::Medium),
            (RibbonGroupState::Small, RibbonItemSize::Small),
            (RibbonGroupState::Collapsed, RibbonItemSize::Large),
        ] {
            stack.apply_layout(RibbonItemLayout {
                size: RibbonItemSize::Large,
                metrics: Some(metrics.clone()),
                is_simplified: false,
                show_label: Some(false),
                group_state: state,
            });
            assert_eq!(large.current_size(), expected);
            assert!(large.actual_show_label());
        }
        stack.apply_layout(RibbonItemLayout {
            is_simplified: true,
            ..RibbonItemLayout::new(RibbonItemSize::Large, Some(metrics))
        });
        assert_eq!(large.current_size(), RibbonItemSize::Small);
    })
}
#[test]
fn source_template_hosts_reparent_actual_children_preserve_state_and_sync_mutations() {
    run(|| {
        let stack = RibbonStackPanel::new();
        stack.set_orientation(ferroui_base::layout::Orientation::Vertical);
        stack.set_spacing(7.5);
        let r = resources();
        stack.set_theme(theme::<RibbonStackPanel>(&r));
        let previous = Panel::new();
        let a = RibbonButton::new();
        a.set_label(Some("A".into()));
        previous.children().add(a.clone());
        stack.items().add(Some(a.clone().upcast()));
        let w = mount(&stack);
        assert_eq!(previous.children().count(), 0);
        let host = stack.host().unwrap();
        assert!(host.children().get(0).ptr_eq(&a));
        let panel = host.cast::<StackPanel>().unwrap();
        assert_eq!(panel.spacing(), 7.5);
        assert_eq!(
            panel.orientation(),
            ferroui_base::layout::Orientation::Vertical
        );
        let b = RibbonButton::new();
        b.set_label(Some("B".into()));
        stack.items().add(Some(b.clone().upcast()));
        assert_eq!(host.children().count(), 2);
        stack.items().move_item(1, 0);
        assert!(host.children().get(0).ptr_eq(&b));
        assert_eq!(a.label().as_deref(), Some("A"));
        stack.items().remove(&Some(a.clone().upcast()));
        assert!(a.visual_parent().is_none());
        assert!(host.children().get(0).ptr_eq(&b));
        stack.set_theme(theme::<RibbonStackPanel>(&resources()));
        stack.apply_styling();
        stack.apply_template();
        let replacement = stack.host().unwrap();
        assert!(!host.ptr_eq(&replacement));
        assert_eq!(host.children().count(), 0);
        assert!(replacement.children().get(0).ptr_eq(&b));
        stack.set_template(None);
        stack.apply_template();
        assert!(stack.host().is_none());
        assert_eq!(replacement.children().count(), 0);
        w.close();
    })
}
#[test]
fn source_container_copies_only_copyable_children_and_stack_settings_not_common_state() {
    run(|| {
        let stack = RibbonStackPanel::new();
        stack.set_id(Some("source".into()));
        stack.set_label(Some("Do not copy container common state".into()));
        stack.set_spacing(-2.25);
        stack.set_orientation(ferroui_base::layout::Orientation::Vertical);
        let source = RibbonButton::new();
        source.set_label(Some("Copyable".into()));
        stack.items().add(Some(source.clone().upcast()));
        stack.items().add(Some(RibbonLabel::new().upcast()));
        stack.items().add(Some(RibbonSeparator::new().upcast()));
        stack.items().add(None);
        let copy = item(&stack)
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonStackPanel>()
            .unwrap();
        assert_eq!(copy.items().count(), 1);
        assert_eq!(copy.spacing(), -2.25);
        assert_eq!(copy.orientation(), stack.orientation());
        assert_eq!(copy.id(), None);
        assert_eq!(copy.label(), None);
        assert!(RibbonItemHelper::get_source_item(&copy).is_none());
        let child = copy.items().get(0).unwrap().cast::<RibbonButton>().unwrap();
        assert!(
            RibbonItemHelper::get_source_item(&child)
                .unwrap()
                .ptr_eq(&source)
        );
        source.set_label(Some("Changed".into()));
        assert_eq!(child.label().as_deref(), Some("Changed"));
        RibbonItemHelper::unlink_tree(Some(copy.clone().upcast()));
        source.set_label(Some("After unlink".into()));
        assert_eq!(child.label().as_deref(), Some("Changed"));
        let empty = RibbonButtonGroup::new();
        empty.items().add(Some(RibbonLabel::new().upcast()));
        assert!(item(&empty).create_linked_copy().is_none());
    })
}
#[test]
fn source_container_branch_uses_height_even_when_its_current_size_is_large() {
    run(|| {
        let group = RibbonButtonGroup::new();
        group.set_current_size(RibbonItemSize::Large);
        let panel = RibbonGroupItemsPanel::new();
        group.set_height(10.);
        group.measure(Size::INFINITY);
        assert!(!panel.is_full_height(&group));
        group.set_height(100.);
        group.invalidate_measure();
        group.measure(Size::INFINITY);
        assert!(panel.is_full_height(&group));
    })
}
#[test]
fn source_collection_notifications_reentry_null_error_and_weak_native_lifetime_are_explicit() {
    run(|| {
        use ferroui_system::component_model::INotifyPropertyChanged;
        let group = RibbonButtonGroup::new();
        let items = group.items();
        let trace = Rc::new(RefCell::new(vec![]));
        let t = trace.clone();
        items
            .property_changed()
            .add(Rc::new(move |name| t.borrow_mut().push(name.to_string())));
        let t = trace.clone();
        items.add_collection_changed(Rc::new(move |_| {
            t.borrow_mut().push("CollectionChanged".into())
        }));
        items.add(None);
        assert_eq!(&*trace.borrow(), &["Count", "Item[]", "CollectionChanged"]);
        assert_eq!(items.count(), 1);
        // Null is retained in Items and skipped by OfType before a template host exists.
        group.set_theme(theme::<RibbonButtonGroup>(&resources()));
        let w = Window::new();
        w.set_content(Some(Control::boxed(group.clone().upcast::<Control>())));
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            w.show();
            w.measure(Size::new(800., 300.));
        }));
        assert!(failed.is_err());
        assert_eq!(items.count(), 1);
        items.clear();
        w.close();
        let independent = RibbonStackPanel::new();
        let external = independent.items();
        let weak = independent.downgrade();
        drop(independent);
        assert!(weak.upgrade().is_none());
        external.add(Some(RibbonLabel::new().upcast()));
        // This proves native weak-owner cleanup only; source external-collection
        // retention through its strong method delegate is a separately recorded gap.
    })
}

#[repr(C)]
struct OverflowItem {
    base: crate::controls::RibbonControlBase,
    calls: std::cell::Cell<usize>,
    fail: bool,
    id: &'static str,
}
ferroui_base::ferro_class!(OverflowItem:RibbonControlBase);
ferroui_base::ferro_class_info!(OverflowItem {});
ferroui_base::ferro_impl_classes!(OverflowItem:ferroui_base::FerroObjectImpl,ferroui_base::StyledElementImpl,ferroui_base::VisualImpl,ferroui_base::layout::LayoutableImpl,ferroui_base::interactivity::InteractiveImpl,ferroui_base::input::InputElementImpl,ferroui_controls::ControlImpl,ferroui_controls::primitives::TemplatedControlImpl);
impl crate::controls::RibbonControlBaseImpl for OverflowItem {
    fn create_overflow_menu_items_core(this: &Self) -> crate::controls::RibbonOverflowMenuItems {
        this.calls.set(this.calls.get() + 1);
        assert!(!this.fail, "selector");
        let entry = ferroui_controls::Separator::new();
        entry.set_tag(Some(Rc::new(this.id.to_string())));
        ferroribbon_core::model::RibbonItemsSource::from_values(vec![Some(entry.upcast())])
    }
}
#[test]
fn source_container_overflow_of_type_select_many_is_lazy_and_repeats_version_failure() {
    run(|| {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/container-query.json"))
                .unwrap();
        let expected = &corpus["corpus"];
        let group = RibbonButtonGroup::new();
        let child = ferroui_base::instantiate(OverflowItem {
            base: crate::controls::RibbonControlBase::construct(),
            calls: std::cell::Cell::new(0),
            fail: false,
            id: "first",
        });
        group.items().add(None);
        group.items().add(Some(Panel::new().upcast()));
        group.items().add(Some(RibbonLabel::new().upcast()));
        group.items().add(Some(child.clone().upcast()));
        let query = item(&group).create_overflow_menu_items();
        let mut cursor = query.iter();
        assert_eq!(
            child.calls.get(),
            expected["callsBefore"].as_u64().unwrap() as usize
        );
        assert!(
            cursor
                .next()
                .unwrap()
                .unwrap()
                .cast::<ferroui_controls::Separator>()
                .is_some()
        );
        assert_eq!(child.calls.get(), 1);
        group.items().add(Some(RibbonSeparator::new().upcast()));
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cursor.next()));
        assert!(failed.is_err());
        let repeated = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cursor.next()));
        assert!(repeated.is_err());
        assert_eq!(
            expected["versionErrors"],
            serde_json::json!(["InvalidOperationException", "InvalidOperationException"])
        );
        assert_eq!(
            query.iter().count(),
            expected["count"].as_u64().unwrap() as usize
        );
        assert_eq!(
            child.calls.get(),
            expected["callsAfter"].as_u64().unwrap() as usize
        );
    })
}
#[test]
fn source_container_select_many_resumes_after_selector_failure() {
    run(|| {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/container-query.json"))
                .unwrap();
        let expected = &corpus["corpus"];
        let group = RibbonButtonGroup::new();
        let probe = |id, fail| {
            ferroui_base::instantiate(OverflowItem {
                base: RibbonControlBase::construct(),
                calls: std::cell::Cell::new(0),
                id,
                fail,
            })
        };
        let bad = probe("bad", true);
        let good = probe("good", false);
        group.items().add(Some(bad.clone().upcast()));
        group.items().add(Some(good.clone().upcast()));
        let mut cursor = item(&group).create_overflow_menu_items().iter();
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cursor.next()));
        assert!(failed.is_err());
        assert_eq!(expected["selectorError"].as_str(), Some("selector"));
        let entry = cursor.next().unwrap().unwrap();
        let tag = entry.tag().unwrap();
        assert_eq!(
            tag.downcast_ref::<String>().map(String::as_str),
            expected["afterSelectorError"].as_str()
        );
        assert_eq!(
            bad.calls.get(),
            expected["badCalls"].as_u64().unwrap() as usize
        );
        assert_eq!(
            good.calls.get(),
            expected["goodCalls"].as_u64().unwrap() as usize
        );
    });
}
