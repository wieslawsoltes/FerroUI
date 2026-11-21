use super::*;
use crate::controls::RibbonToggleButton;
use ferroui_base::{
    controls::ResourceDictionary, data::core::ValueTypes, styling::ControlTheme,
    threading::Dispatcher,
};
use ferroui_controls::Window;
fn mount(c: &RibbonStatusBar) -> Ref<Window> {
    let value = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Status.xaml")
        .unwrap()
        .unwrap();
    let d = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    d.add_merged_dictionary(
        crate::theming::RibbonThemeResources::new().upcast::<ResourceDictionary>(),
    );
    let theme = d
        .try_get_resource(&RibbonStatusBar::TYPE.into(), None)
        .flatten()
        .unwrap();
    c.set_resources(d);
    c.set_theme(
        ValueTypes::as_object(&*theme)
            .unwrap()
            .cast::<ControlTheme>()
            .unwrap(),
    );
    let w = Window::new();
    w.set_width(600.);
    w.set_content(Some(Control::boxed(c.to_ref())));
    w.show();
    w.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    w
}
#[test]
fn original_cad_status_icon_only_and_status_metrics_use_actual_native_controls() {
    crate::group_tests::run(|| {
        let c = RibbonStatusBar::new();
        let grid = RibbonToggleButton::new();
        grid.set_id(Some("grid".into()));
        grid.set_label(Some("Grid".into()));
        c.items().add(Some(grid.clone().upcast()));
        assert!(
            RibbonItemHelper::find_owner(Some(&grid))
                .unwrap()
                .as_object()
                .ptr_eq(&c)
        );
        let w = mount(&c);
        assert!(grid.actual_show_label());
        assert_eq!(grid.current_size(), RibbonItemSize::Small);
        assert!(grid.is_simplified());
        assert_eq!(grid.metrics().unwrap().simplified_item_height(), 22.);
        assert_eq!(c.metrics().font_size(), 11.);
        c.set_show_labels(false);
        assert!(!grid.actual_show_label());
        c.set_show_labels(true);
        assert!(grid.actual_show_label());
        grid.set_show_label(false);
        c.set_show_labels(false);
        c.set_show_labels(true);
        assert!(!grid.actual_show_label());
        assert!(Arc::ptr_eq(&c.metrics(), &RibbonStatusBar::new().metrics()));
        w.close();
    });
}
#[test]
fn duplicate_items_shared_hosts_and_retemplate_keep_original_control_identity() {
    crate::group_tests::run(|| {
        let c = RibbonStatusBar::new();
        let grid = RibbonToggleButton::new();
        c.items().add(Some(grid.clone().upcast()));
        c.items().add(Some(grid.clone().upcast()));
        let w = mount(&c);
        let old_start = c.start.borrow().clone().unwrap();
        let old_end = c.end.borrow().clone().unwrap();
        assert_eq!(c.items().count(), 2);
        assert_eq!(old_start.children().count(), 1);
        assert!(old_start.children().get(0).ptr_eq(&grid));
        c.end_items().add(Some(grid.clone().upcast()));
        assert_eq!(old_start.children().count(), 0);
        assert_eq!(old_end.children().count(), 1);
        assert!(old_end.children().get(0).ptr_eq(&grid));
        let original = c.template();
        c.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
            |_, _| ferroui_controls::Border::new().upcast(),
        )));
        c.apply_template();
        assert_eq!(old_start.children().count(), 0);
        assert_eq!(old_end.children().count(), 0);
        c.set_template(original);
        c.apply_template();
        assert!(
            c.end
                .borrow()
                .as_ref()
                .unwrap()
                .children()
                .get(0)
                .ptr_eq(&grid)
        );
        w.close();
    });
}
#[test]
fn native_collection_subscription_is_weak_and_removed_when_status_bar_drops() {
    crate::group_tests::run(|| {
        let c = RibbonStatusBar::new();
        let items = c.items();
        let grid = RibbonToggleButton::new();
        items.add(Some(grid.clone().upcast()));
        let weak = c.downgrade();
        drop(c);
        assert!(weak.upgrade().is_none());
        items.add(Some(RibbonToggleButton::new().upcast()));
        assert_eq!(items.count(), 2);
        assert!(RibbonItemHelper::find_owner(Some(&grid)).is_none());
    });
}
