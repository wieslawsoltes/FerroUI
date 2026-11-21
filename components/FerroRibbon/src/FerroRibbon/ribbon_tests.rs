//! Adaptations of source LayoutTests/SelectionTests and RibbonBehaviorTests.
//! Only the explicitly implemented command/tab subset is exercised.
use crate::{
    controls::{Ribbon, RibbonButton, RibbonContextualTabGroup, RibbonGroup, RibbonTab},
    primitives::RibbonMetrics,
};
use ferroribbon_core::{
    RibbonContextualActivation, RibbonDensity, RibbonDisplayMode, RibbonGroupState,
};
use ferroui_base::{
    Rect, Ref, Size, StaticType,
    controls::ResourceDictionary,
    data::core::ValueTypes,
    metadata::{MarkupDelegate, MarkupTyped},
    styling::ControlTheme,
};
use ferroui_controls::{Control, Window};
use std::{cell::RefCell, rc::Rc};
fn tab(id: &str, header: &str) -> Ref<RibbonTab> {
    let t = RibbonTab::new();
    t.set_id(Some(id.into()));
    t.set_header(Some(header.into()));
    t
}
fn ribbon() -> Ref<Ribbon> {
    let r = Ribbon::new();
    r.set_is_application_button_visible(false);
    r.set_is_display_options_button_visible(false);
    let home = tab("home", "Home");
    for (i, label) in ["Clipboard", "Font", "Paragraph", "Styles", "Editing"]
        .iter()
        .enumerate()
    {
        let g = RibbonGroup::new();
        g.set_header(Some(label.to_string()));
        g.set_reduction_order(i as i32);
        for j in 0..3 {
            let b = RibbonButton::new();
            b.set_label(Some(format!("Tool {j}")));
            b.set_icon(Some(Rc::new("A".to_owned())));
            g.items().add(Some(b.upcast()));
        }
        home.groups().add(Some(g));
    }
    r.tabs().add(Some(home));
    r.tabs().add(Some(tab("insert", "Insert")));
    let tools = RibbonContextualTabGroup::new();
    tools.set_id(Some("tableTools".into()));
    tools.set_header(Some("Table Tools".into()));
    tools.set_activation(RibbonContextualActivation::SelectOnShow);
    r.contextual_groups().add(Some(tools));
    let design = tab("tableDesign", "Table Design");
    design.set_contextual_group_id(Some("tableTools".into()));
    r.tabs().add(Some(design));
    r
}
pub(crate) fn mount(r: &Ribbon) -> Ref<Window> {
    let value = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Ribbon.xaml")
        .unwrap()
        .unwrap();
    let d = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    let theme = ValueTypes::as_object(
        &*d.try_get_resource(&<Ribbon as StaticType>::TYPE.into(), None)
            .flatten()
            .unwrap(),
    )
    .unwrap()
    .cast::<ControlTheme>()
    .unwrap();
    r.set_resources(d);
    r.set_theme(theme);
    let w = Window::new();
    w.set_content(Some(Control::boxed(r.to_ref())));
    w.show();
    resize(&w, 1400.);
    w
}
fn resize(w: &Window, width: f64) {
    w.set_width(width);
    w.set_height(300.);
    w.measure(Size::new(width, 300.));
    w.arrange(Rect::new(0., 0., width, 300.));
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
}
#[test]
fn source_selecting_tabs_by_id_updates_selection_and_restores_contextual_previous() {
    crate::group_tests::run(|| {
        let r = ribbon();
        let w = mount(&r);
        assert_eq!(r.selected_tab_id().as_deref(), Some("home"));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let log = trace.clone();
        r.add_selected_tab_changed(Some(selection_handler(move |e| {
            log.borrow_mut().push((
                e.old_tab.as_ref().and_then(|t| t.id()),
                e.new_tab.as_ref().and_then(|t| t.id()),
            ));
        })));
        assert!(r.select_tab_id("insert"));
        assert_eq!(r.selected_tab_id().as_deref(), Some("insert"));
        r.set_selected_tab_id(Some("home".into()));
        assert_eq!(r.selected_tab().unwrap().id().as_deref(), Some("home"));
        assert!(!r.select_tab_id("tableDesign"));
        r.set_selected_tab_id(Some("tableDesign".into()));
        assert_eq!(r.selected_tab_id().as_deref(), Some("home"));
        assert!(r.select_tab_id("insert"));
        let group = r.contextual_groups().get(0).unwrap();
        group.set_is_visible(true);
        assert_eq!(r.selected_tab_id().as_deref(), Some("tableDesign"));
        assert!(r.tab_headers().last().unwrap().is_contextual());
        group.set_is_visible(false);
        assert_eq!(r.selected_tab_id().as_deref(), Some("insert"));
        assert_eq!(r.visible_tabs().len(), 2);
        assert_eq!(trace.borrow().len(), 5);
        w.close();
    });
}
#[test]
fn source_active_contextual_groups_preserves_id_equality_order_and_selection() {
    crate::group_tests::run(|| {
        let r = ribbon();
        let w = mount(&r);
        r.set_active_contextual_groups(&[Some("tableTools")]);
        assert_eq!(r.selected_tab_id().as_deref(), Some("tableDesign"));
        r.set_active_contextual_groups(&[Some("TABLETOOLS")]);
        assert_eq!(r.selected_tab_id().as_deref(), Some("home"));
        assert!(!r.contextual_groups().get(0).unwrap().is_visible());
        w.close();
        // ID boundaries are independent of tab-header parenting: duplicate group
        // IDs (or a visible unset ID matching regular tabs) can repeat the same
        // cached header in the source's OnTabsChanged sequence.
        let r = Ribbon::new();
        let first = RibbonContextualTabGroup::new();
        first.set_id(Some("tableTools".into()));
        let duplicate = RibbonContextualTabGroup::new();
        duplicate.set_id(first.id());
        let unset = RibbonContextualTabGroup::new();
        let empty = RibbonContextualTabGroup::new();
        empty.set_id(Some(String::new()));
        let trace = Rc::new(RefCell::new(Vec::new()));
        for (index, group) in [&first, &duplicate, &unset, &empty].into_iter().enumerate() {
            r.contextual_groups().add(Some(group.clone()));
            let trace = trace.clone();
            group.add_property_changed_handler(Rc::new(move |args| {
                if args.property() == RibbonContextualTabGroup::is_visible_property().as_property()
                {
                    trace.borrow_mut().push(index);
                }
            }));
        }
        r.set_active_contextual_groups(&[Some("tableTools"), Some("tableTools"), None]);
        assert!(first.is_visible() && duplicate.is_visible() && unset.is_visible());
        assert!(!empty.is_visible());
        assert_eq!(*trace.borrow(), [0, 1, 2]);
        trace.borrow_mut().clear();
        r.set_active_contextual_groups(&[Some("tableTools"), None]);
        assert!(
            trace.borrow().is_empty(),
            "No notifications for unchanged values"
        );
        r.set_active_contextual_groups(&[Some("TABLETOOLS"), Some(""), Some("unknown")]);
        assert_eq!(*trace.borrow(), [0, 1, 2, 3]);
        assert!(!first.is_visible() && !duplicate.is_visible() && !unset.is_visible());
        assert!(empty.is_visible());
        r.set_active_contextual_groups(&[]);
        assert!(!empty.is_visible());
    });
}
#[test]
fn markup_contextual_group_methods_preserve_identity_and_activation_order() {
    use ferroui_base::metadata::{from_markup_value, into_markup_value};

    crate::group_tests::run(|| {
        let metadata = Ribbon::__MARKUP.unwrap();
        let find = metadata
            .find_methods("FindContextualGroup")
            .next()
            .unwrap()
            .invoke;
        let set = metadata
            .find_methods("SetActiveContextualGroups")
            .next()
            .unwrap()
            .invoke;
        let find_group = |ribbon: &Ref<Ribbon>, id: Option<&str>| {
            let found = find(&[
                Some(Control::boxed(ribbon.clone())),
                into_markup_value(id.map(str::to_owned)),
            ])
            .unwrap();
            from_markup_value::<Option<Ref<RibbonContextualTabGroup>>>(&found).unwrap()
        };
        let set_active = |ribbon: &Ref<Ribbon>, ids: Option<Vec<Option<String>>>| {
            set(&[Some(Control::boxed(ribbon.clone())), into_markup_value(ids)]).unwrap();
        };

        let r = ribbon();
        let w = mount(&r);
        let group = r.contextual_groups().get(0).unwrap();
        assert!(find_group(&r, Some("tableTools")).unwrap().ptr_eq(&group));
        assert!(find_group(&r, Some("TABLETOOLS")).is_none());
        set_active(&r, Some(vec![Some("tableTools".into())]));
        assert_eq!(r.selected_tab_id().as_deref(), Some("tableDesign"));
        set_active(&r, Some(vec![Some("TABLETOOLS".into())]));
        assert_eq!(r.selected_tab_id().as_deref(), Some("home"));
        w.close();

        let r = Ribbon::new();
        let first = RibbonContextualTabGroup::new();
        first.set_id(Some("same".into()));
        let duplicate = RibbonContextualTabGroup::new();
        duplicate.set_id(first.id());
        let unset = RibbonContextualTabGroup::new();
        let empty = RibbonContextualTabGroup::new();
        empty.set_id(Some(String::new()));
        for group in [&first, &duplicate, &unset, &empty] {
            r.contextual_groups().add(Some(group.clone()));
        }
        assert!(find_group(&r, Some("same")).unwrap().ptr_eq(&first));
        assert!(find_group(&r, None).unwrap().ptr_eq(&unset));
        assert!(find_group(&r, Some("")).unwrap().ptr_eq(&empty));
        set_active(&r, Some(vec![Some("same".into()), None]));
        assert!(first.is_visible() && duplicate.is_visible() && unset.is_visible());
        assert!(!empty.is_visible());
        set_active(&r, Some(Vec::new()));
        assert!(!first.is_visible() && !duplicate.is_visible() && !unset.is_visible());
        set_active(&r, Some(vec![Some(String::new())]));
        assert!(empty.is_visible());
        set_active(&r, None);
        assert!(
            [first, duplicate, unset, empty]
                .iter()
                .all(|g| !g.is_visible())
        );
    });
}
#[test]
fn contextual_group_replacement_and_reset_keep_only_live_subscriptions() {
    use ferroui_base::media::Color;

    crate::group_tests::run(|| {
        let r = ribbon();
        let w = mount(&r);
        let old = r.contextual_groups().get(0).unwrap();
        let contextual = r.find_tab("tableDesign").unwrap();
        let trace = |step, valid: bool| {
            assert!(valid, "{step}");
            println!("[contextual] {step}");
        };
        assert!(r.select_tab_id("insert"));
        old.set_is_visible(true);
        let header = contextual.header_element().unwrap();
        trace(
            "initial-select",
            r.selected_tab() == Some(contextual.clone()),
        );

        let replacement = RibbonContextualTabGroup::new();
        replacement.set_id(old.id());
        replacement.set_header(Some("Replacement".into()));
        replacement.set_activation(RibbonContextualActivation::SelectOnShow);
        r.contextual_groups().set(0, Some(replacement.clone()));
        trace(
            "replacement-hidden",
            r.selected_tab_id().as_deref() == Some("insert") && !contextual.is_available(),
        );
        replacement.set_is_visible(true);
        trace(
            "replacement-shown",
            r.selected_tab() == Some(contextual.clone())
                && contextual.header_element() == Some(header.clone())
                && header.contextual_header().as_deref() == Some("Replacement"),
        );
        replacement.set_header(Some("Renamed".into()));
        old.set_color(Color::from_argb(255, 1, 2, 3));
        trace(
            "detached-ignored",
            header.contextual_header().as_deref() == Some("Replacement"),
        );
        replacement.set_color(Color::from_argb(255, 9, 8, 7));
        trace(
            "replacement-refresh",
            header.contextual_header().as_deref() == Some("Renamed"),
        );
        replacement.set_is_enabled(false);
        trace(
            "disabled-header",
            !header.is_enabled() && r.selected_tab() == Some(contextual.clone()),
        );
        r.contextual_groups().clear();
        trace(
            "clear-restores",
            r.selected_tab_id().as_deref() == Some("insert") && !contextual.is_available(),
        );

        let baseline = replacement.property_changed_subscriber_count();
        replacement.set_is_visible(false);
        r.contextual_groups().add(Some(replacement.clone()));
        r.contextual_groups().add(Some(replacement.clone()));
        trace(
            "one-subscription",
            replacement.property_changed_subscriber_count() == baseline + 1,
        );
        r.contextual_groups().remove(&Some(replacement.clone()));
        trace(
            "one-occurrence-remains",
            replacement.property_changed_subscriber_count() == baseline + 1,
        );
        r.contextual_groups().clear();
        trace(
            "reset-detaches",
            replacement.property_changed_subscriber_count() == baseline,
        );
        w.close();
    });
}

#[test]
fn markup_tab_lookup_and_selection_distinguish_unset_and_empty_values() {
    use ferroui_base::{
        data::core::ValueType,
        metadata::{from_markup_value, into_markup_value},
    };

    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let hidden = RibbonTab::new();
        hidden.set_id(Some("hidden".into()));
        hidden.set_is_tab_visible(false);
        let unnamed = RibbonTab::new();
        unnamed.set_id(Some("unnamed".into()));
        let empty = tab("empty", "");
        for item in [&hidden, &unnamed, &empty] {
            r.tabs().add(Some(item.clone()));
        }
        let metadata = Ribbon::__MARKUP.unwrap();
        let find = metadata.find_methods("FindTab").next().unwrap().invoke;
        let select = metadata
            .find_methods("SelectTab")
            .find(|m| (m.parameters[0])() != ValueType::of::<Ref<RibbonTab>>())
            .unwrap()
            .invoke;
        let find_tab = |value: Option<&str>| {
            let found = find(&[
                Some(Control::boxed(r.clone())),
                into_markup_value(value.map(str::to_owned)),
            ])
            .unwrap();
            from_markup_value::<Option<Ref<RibbonTab>>>(&found).unwrap()
        };
        let select_tab = |value: Option<&str>| {
            let selected = select(&[
                Some(Control::boxed(r.clone())),
                into_markup_value(value.map(str::to_owned)),
            ])
            .unwrap();
            from_markup_value::<bool>(&selected).unwrap()
        };
        assert_eq!(find_tab(None), Some(hidden.clone()));
        assert!(!select_tab(None));
        println!("[contextual] lookup-null-hidden-first");
        r.tabs().remove(&Some(hidden));
        assert_eq!(find_tab(None), Some(unnamed.clone()));
        assert!(select_tab(None));
        assert_eq!(r.selected_tab(), Some(unnamed));
        println!("[contextual] lookup-null-available");
        assert_eq!(find_tab(Some("")), Some(empty.clone()));
        assert!(select_tab(Some("")));
        assert_eq!(r.selected_tab(), Some(empty));
        println!("[contextual] lookup-empty");
    });
}

#[test]
fn source_native_ribbon_resizes_reduces_restores_and_switches_presentation() {
    crate::group_tests::run(|| {
        let r = ribbon();
        let w = mount(&r);
        let t = r.selected_tab().unwrap();
        let groups = t.groups();
        assert!(
            groups
                .iter()
                .all(|g| g.unwrap().state() == RibbonGroupState::Large)
        );
        resize(&w, 300.);
        assert!(
            groups
                .iter()
                .any(|g| g.unwrap().state() != RibbonGroupState::Large)
        );
        resize(&w, 1400.);
        assert!(
            groups
                .iter()
                .all(|g| g.unwrap().state() == RibbonGroupState::Large)
        );
        r.set_display_mode(RibbonDisplayMode::Simplified);
        resize(&w, 300.);
        assert!(t.groups_panel().has_overflow());
        assert!(groups.iter().all(|g| g.unwrap().is_simplified()));
        r.set_display_mode(RibbonDisplayMode::Classic);
        resize(&w, 1400.);
        assert!(groups.iter().all(|g| !g.unwrap().is_simplified()));
        let full_height = r.bounds().height;
        r.set_show_group_captions(false);
        resize(&w, 1400.);
        assert!(r.bounds().height < full_height);
        r.set_show_group_captions(true);
        resize(&w, 1400.);
        let original = r.metrics();
        r.set_density(RibbonDensity::Touch);
        assert!(!std::sync::Arc::ptr_eq(&original, &r.metrics()));
        assert_eq!(r.metrics(), RibbonMetrics::touch());
        w.close();
    });
}
#[test]
fn source_tab_strip_compaction_and_host_reparenting_follow_ribbon_width() {
    use crate::controls::{RibbonItemHelper, RibbonSimplifiedLabel};
    use ferroribbon_core::RibbonItemSize;
    use ferroui_controls::{Grid, Panel, StackPanel, templates::FuncControlTemplate};

    crate::group_tests::run(|| {
        let ribbon = ribbon();
        let comments = RibbonButton::new();
        comments.set_label(Some("Comments".into()));
        comments.set_simplified_label(RibbonSimplifiedLabel::Show);
        ribbon
            .tab_strip_items()
            .add(Some(comments.clone().upcast()));
        let window = mount(&ribbon);
        window.set_content(None);
        let layout = Grid::new();
        let old_host = StackPanel::new();
        old_host.set_width(12.);
        old_host.set_height(12.);
        layout.children().add(ribbon.clone());
        layout.children().add(old_host.clone());
        window.set_content(Some(Control::boxed(layout.clone())));
        assert_eq!(ribbon.compact_tab_strip_width(), 900.);

        let resize_ribbon = |width| {
            ribbon.set_width(width);
            window.update_layout();
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(ribbon.bounds().width, width);
        };
        let assert_layout = |step, item: &RibbonButton, size, label| {
            let trace = serde_json::json!({
                "step": step,
                "width": ribbon.bounds().width,
                "size": item.current_size().to_string(),
                "label": item.actual_show_label(),
                "simplified": item.is_simplified(),
            });
            println!("[tab-strip] {trace}");
            assert_eq!(item.current_size(), size);
            assert_eq!(item.actual_show_label(), label);
            assert!(item.is_simplified());
        };
        let medium = RibbonItemSize::Medium;
        let small = RibbonItemSize::Small;
        resize_ribbon(901.);
        assert_layout("wide", &comments, medium, true);
        resize_ribbon(900.);
        assert_layout("boundary", &comments, medium, true);
        resize_ribbon(899.);
        assert_layout("compact", &comments, small, false);

        let extra = RibbonButton::new();
        extra.set_id(Some("reviewAction".into()));
        extra.set_label(Some("Review".into()));
        extra.set_simplified_label(RibbonSimplifiedLabel::Show);
        old_host.children().add(extra.clone());
        window.update_layout();
        assert!(extra.visual_parent().unwrap().ptr_eq(&old_host));
        ribbon.tab_strip_items().add(Some(extra.clone().upcast()));
        assert_eq!(old_host.children().count(), 0);
        assert_eq!(
            extra.visual_parent().unwrap().name().as_deref(),
            Some("PART_TabStripItemsHost")
        );
        assert!(
            RibbonItemHelper::get_owner(&extra)
                .and_then(|owner| owner.owner_ribbon())
                .is_some_and(|owner| owner.ptr_eq(&ribbon))
        );
        assert!(
            ribbon
                .find_item(Some("reviewAction".into()))
                .unwrap()
                .ptr_eq(&extra)
        );
        layout.children().remove(old_host);
        assert_layout("added-existing", &comments, medium, true);
        assert_layout("added-new", &extra, medium, true);
        resize_ribbon(898.);
        assert_layout("still-compact", &extra, medium, true);
        resize_ribbon(900.);
        assert_layout("wide-again", &extra, medium, true);
        resize_ribbon(899.);
        assert_layout("compact-again", &extra, small, false);

        ribbon
            .tab_strip_items()
            .remove(&Some(extra.clone().upcast()));
        assert_layout("after-remove", &comments, medium, true);
        assert!(extra.visual_parent().is_none());
        assert!(ribbon.find_item(Some("reviewAction".into())).is_none());

        resize_ribbon(900.);
        ribbon.set_compact_tab_strip_width(920.);
        assert_layout("threshold-changed", &comments, medium, true);
        resize_ribbon(901.);
        assert_layout("custom-compact", &comments, small, false);
        resize_ribbon(920.);
        assert_layout("custom-boundary", &comments, medium, true);
        resize_ribbon(919.);
        assert_layout("custom-compact-again", &comments, small, false);

        let previous = comments.visual_parent().unwrap().cast::<Panel>().unwrap();
        let replacement = StackPanel::new();
        let host = replacement.clone();
        ribbon.set_template(Some(FuncControlTemplate::for_type::<Ribbon>(
            move |_, scope| {
                scope.register("PART_TabStripItemsHost", host.clone().upcast());
                host.clone().upcast()
            },
        )));
        ribbon.apply_template();
        window.update_layout();
        assert!(comments.visual_parent().unwrap().ptr_eq(&replacement));
        assert_eq!(previous.children().count(), 0);
        assert_layout("retemplated", &comments, small, false);
        window.close();
    });
}

#[test]
fn source_rebuild_preserves_header_and_tab_identity_and_removes_contextual_subscription() {
    crate::group_tests::run(|| {
        let r = ribbon();
        let w = mount(&r);
        let h = r.tab_headers()[0].clone();
        let first = r.tabs().get(0).unwrap();
        first.set_header(Some("Renamed".into()));
        assert!(r.tab_headers()[0].ptr_eq(&h));
        assert!(h.tab().ptr_eq(&first));
        let group = r.contextual_groups().get(0).unwrap();
        let before = group.property_changed_subscriber_count();
        r.contextual_groups().clear();
        assert_eq!(group.property_changed_subscriber_count() + 1, before);
        w.close();
    });
}

type SelectionFn = dyn Fn(Rc<crate::controls::RibbonTabChangedEventArgs>);
struct SelectionHandler(Box<SelectionFn>);
impl PartialEq for SelectionHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class SelectionHandler{this:Rc<SelectionHandler>,handles:[Rc<SelectionHandler>],methods:[fn Invoke(Option<ferroui_base::BoxedValue>,Rc<crate::controls::RibbonTabChangedEventArgs>) => |h:&Rc<SelectionHandler>,_sender,args|(h.0)(args)]});
fn selection_handler(
    f: impl Fn(Rc<crate::controls::RibbonTabChangedEventArgs>) + 'static,
) -> MarkupDelegate {
    ValueTypes::register_reference::<SelectionHandler>();
    MarkupDelegate::for_method(
        Some(Rc::new(SelectionHandler(Box::new(f)))),
        SelectionHandler::MARKUP,
        &SelectionHandler::MARKUP.methods[0],
    )
}
