use crate::DockProperties as P;
use ferrodock_model::core::DockOperation;
use ferroui_base::data::BindingMode;
use ferroui_base::threading::CancellationToken;
use ferroui_base::{FerroObject, FerroObjectImpl, Ref};
use ferroui_controls::{AppBuilder, Border, Control, StackPanel};
use ferroui_headless::{FerroTestApplicationAttribute, FerroTestAssembly, HeadlessUnitTestSession};
fn build() -> AppBuilder {
    crate::test_app_builder::TestAppBuilder::build_ferro_app()
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-settings",
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
#[test]
fn original_default_values_are_correct() {
    run(|| {
        let c = Control::new();
        assert!(!P::get_is_dock_target(&c));
        assert!(!P::get_is_drag_area(&c));
        assert!(!P::get_is_drop_area(&c));
        assert!(P::get_is_drag_enabled(&c));
        assert!(P::get_is_drop_enabled(&c));
        assert!(!P::get_show_dock_indicator_only(&c));
        assert_eq!(P::get_indicator_dock_operation(&c), DockOperation::Fill);
        assert!(P::get_dock_adorner_host(&c).is_none());
        assert!(P::get_dock_group(&c).is_none());
    });
}
#[test]
fn original_setters_update_property_values() {
    run(|| {
        let c = Control::new();
        let host = Control::new();
        P::set_is_dock_target(&c, true);
        P::set_is_drag_area(&c, true);
        P::set_is_drop_area(&c, true);
        P::set_is_drag_enabled(&c, false);
        P::set_is_drop_enabled(&c, false);
        P::set_show_dock_indicator_only(&c, true);
        P::set_indicator_dock_operation(&c, DockOperation::Left);
        P::set_dock_adorner_host(&c, Some(host.clone()));
        assert!(P::get_is_dock_target(&c));
        assert!(P::get_is_drag_area(&c));
        assert!(P::get_is_drop_area(&c));
        assert!(!P::get_is_drag_enabled(&c));
        assert!(!P::get_is_drop_enabled(&c));
        assert!(P::get_show_dock_indicator_only(&c));
        assert_eq!(P::get_indicator_dock_operation(&c), DockOperation::Left);
        assert_eq!(P::get_dock_adorner_host(&c), Some(host));
    });
}
#[test]
fn group_default_null() {
    run(|| assert_eq!(P::get_dock_group(&Border::new()), None));
}
#[test]
fn group_set_and_get() {
    run(|| {
        let c = Border::new();
        P::set_dock_group(&c, Some("TestGroup".into()));
        assert_eq!(P::get_dock_group(&c).as_deref(), Some("TestGroup"));
    });
}
#[test]
fn group_accepts_null() {
    run(|| {
        let c = Border::new();
        P::set_dock_group(&c, Some("TestGroup".into()));
        P::set_dock_group(&c, None);
        assert_eq!(P::get_dock_group(&c), None);
    });
}
#[test]
fn group_accepts_empty() {
    run(|| {
        let c = Border::new();
        P::set_dock_group(&c, Some(String::new()));
        assert_eq!(P::get_dock_group(&c).as_deref(), Some(""));
    });
}
#[test]
fn group_inherits_parent() {
    run(|| {
        let p = StackPanel::new();
        let c = Border::new();
        p.children().add(c.clone());
        P::set_dock_group(&p, Some("InheritedGroup".into()));
        assert_eq!(P::get_dock_group(&c).as_deref(), Some("InheritedGroup"));
    });
}
#[test]
fn group_local_overrides_parent() {
    run(|| {
        let p = StackPanel::new();
        let c = Border::new();
        p.children().add(c.clone());
        P::set_dock_group(&p, Some("ParentGroup".into()));
        P::set_dock_group(&c, Some("ChildGroup".into()));
        assert_eq!(P::get_dock_group(&c).as_deref(), Some("ChildGroup"));
    });
}
#[test]
fn group_inherits_multiple_levels() {
    run(|| {
        let g = StackPanel::new();
        let p = StackPanel::new();
        let c = Border::new();
        g.children().add(p.clone());
        p.children().add(c.clone());
        P::set_dock_group(&g, Some("MultiLevelGroup".into()));
        assert_eq!(P::get_dock_group(&c).as_deref(), Some("MultiLevelGroup"));
    });
}
#[test]
fn group_uses_closest_ancestor() {
    run(|| {
        let g = StackPanel::new();
        let p = StackPanel::new();
        let c = Border::new();
        g.children().add(p.clone());
        p.children().add(c.clone());
        P::set_dock_group(&g, Some("GrandparentGroup".into()));
        P::set_dock_group(&p, Some("ParentGroup".into()));
        assert_eq!(P::get_dock_group(&c).as_deref(), Some("ParentGroup"));
    });
}
#[test]
fn group_is_case_sensitive() {
    run(|| {
        let c = Border::new();
        P::set_dock_group(&c, Some("TestGroup".into()));
        assert_eq!(P::get_dock_group(&c).as_deref(), Some("TestGroup"));
        assert_ne!(P::get_dock_group(&c).as_deref(), Some("testgroup"));
    });
}
#[test]
fn group_preserves_whitespace() {
    run(|| {
        let c = Border::new();
        P::set_dock_group(&c, Some("  Test Group  ".into()));
        assert_eq!(P::get_dock_group(&c).as_deref(), Some("  Test Group  "));
    });
}
#[test]
fn original_metadata_has_two_way_defaults_and_exact_inheritance_flags() {
    run(|| {
        let c = Control::new();
        let metadata = [
            (P::is_dock_target_property().as_property(), false),
            (P::is_drag_area_property().as_property(), false),
            (P::is_drop_area_property().as_property(), false),
            (P::is_drag_enabled_property().as_property(), true),
            (P::is_drop_enabled_property().as_property(), true),
            (P::show_dock_indicator_only_property().as_property(), false),
            (P::indicator_dock_operation_property().as_property(), false),
            (P::dock_adorner_host_property().as_property(), true),
            (P::dock_group_property().as_property(), true),
        ];
        for (property, inherits) in metadata {
            assert!(property.is_attached());
            assert_eq!(property.inherits(), inherits);
            assert_eq!(
                property.get_metadata(c.get_type()).default_binding_mode(),
                BindingMode::TwoWay
            );
        }
    });
}
#[test]
fn inherited_values_update_reparent_clear_and_explicit_null_without_inheriting_targets() {
    run(|| {
        let a = StackPanel::new();
        let b = StackPanel::new();
        let c = Border::new();
        a.children().add(c.clone());
        P::set_dock_group(&a, Some("A".into()));
        P::set_dock_group(&b, Some("B".into()));
        P::set_is_drag_enabled(&a, false);
        P::set_is_drop_enabled(&a, false);
        P::set_is_dock_target(&a, true);
        P::set_is_drag_area(&a, true);
        P::set_is_drop_area(&a, true);
        P::set_show_dock_indicator_only(&a, true);
        P::set_indicator_dock_operation(&a, DockOperation::OuterTop);
        let host = Control::new();
        P::set_dock_adorner_host(&a, Some(host.clone()));
        assert!(!P::get_is_drag_enabled(&c));
        assert!(!P::get_is_drop_enabled(&c));
        assert!(!P::get_is_dock_target(&c));
        assert!(!P::get_is_drag_area(&c));
        assert!(!P::get_is_drop_area(&c));
        assert!(!P::get_show_dock_indicator_only(&c));
        assert_eq!(P::get_indicator_dock_operation(&c), DockOperation::Fill);
        assert_eq!(P::get_dock_adorner_host(&c), Some(host));
        P::set_dock_group(&c, None);
        assert!(P::get_dock_group(&c).is_none());
        c.clear_value(P::dock_group_property());
        assert_eq!(P::get_dock_group(&c).as_deref(), Some("A"));
        a.children().remove(&c);
        b.children().add(c.clone());
        assert_eq!(P::get_dock_group(&c).as_deref(), Some("B"));
        assert!(P::get_is_drag_enabled(&c));
        assert!(P::get_dock_adorner_host(&c).is_none());
    });
}
#[repr(C)]
struct DerivedProperties {
    base: P,
}
ferroui_base::ferro_class!(DerivedProperties:P);
ferroui_base::ferro_class_info!(DerivedProperties {});
ferroui_base::ferro_impl_classes!(DerivedProperties:FerroObjectImpl);
#[test]
fn original_property_owner_can_be_constructed_and_extended() {
    run(|| {
        let owner = P::new();
        P::set_dock_group(&owner, Some("Owner".into()));
        assert_eq!(P::get_dock_group(&owner).as_deref(), Some("Owner"));
        let derived: Ref<DerivedProperties> = ferroui_base::instantiate(DerivedProperties {
            base: P::construct(),
        });
        assert!(derived.cast::<P>().is_some());
        let object = FerroObject::new();
        P::set_is_drop_enabled(&object, false);
        assert!(!P::get_is_drop_enabled(&object));
    });
}
