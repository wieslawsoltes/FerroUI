//! Whole-source root Model corpus plus real native model/generated-control integration.
use crate::{
    controls::*,
    mvvm::{RibbonElementFactory, RibbonElementFactoryImpl},
};
use ferroribbon_core::{
    RibbonDensity, RibbonDisplayMode, RibbonMinimizeBehavior, RibbonQuickAccessPosition,
    RibbonVisibilityMode, commands::*, model::*,
};
use ferroui_base::{FerroObjectImpl, Ref, ferro_class, instantiate, threading::Dispatcher};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn check(name: &str, value: Value) {
    let expected: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/root-model-runtime.json")).unwrap();
    assert_eq!(
        &value,
        expected["corpus"]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["name"] == name)
            .unwrap(),
        "source root Model {name}"
    );
}
fn model() -> Rc<RibbonModel> {
    let m = RibbonModel::new();
    let t = RibbonTabModel::new(Some("home".into()), Some("Home".into()));
    let g = RibbonGroupModel::new(Some("tools".into()), Some("Tools".into()));
    g.items().add(Some(RibbonButtonModel::new(
        Some("save".into()),
        Some("Save".into()),
    )));
    t.groups().add(Some(g));
    m.tabs().add(Some(t));
    m
}
fn tabs(r: &Ribbon) -> Vec<Option<String>> {
    r.tabs().iter().map(|t| t.unwrap().id()).collect()
}
fn qat(r: &Ribbon) -> Vec<Option<String>> {
    r.quick_access_tool_bar()
        .map(|q| {
            q.items()
                .iter()
                .map(|i| {
                    ferroui_base::data::core::ValueTypes::try_cast(
                        &ferroui_controls::Control::boxed(i.unwrap()),
                        ferroui_base::data::core::ValueType::of::<Rc<dyn IRibbonItem>>(),
                    )
                    .unwrap()
                    .downcast_ref::<Rc<dyn IRibbonItem>>()
                    .unwrap()
                    .id()
                    .map(|s| s.to_utf8().unwrap())
                })
                .collect()
        })
        .unwrap_or_default()
}
fn settle() {
    Dispatcher::ui_thread().run_jobs(None);
}
#[test]
fn original_root_model_collections_keep_declared_tabs_move_identity_and_dispose_generated_nodes() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let declared = RibbonTab::new();
        declared.set_id(Some("declared".into()));
        declared.set_header(Some("Declared".into()));
        r.tabs().add(Some(declared));
        let m = model();
        m.tabs().add(Some(RibbonTabModel::new(
            Some("insert".into()),
            Some("Insert".into()),
        )));
        r.set_model(Some(m.clone()));
        let first = r.tabs().get(1).unwrap();
        let initial = tabs(&r);
        m.tabs().move_item(0, 1);
        let moved = tabs(&r);
        let same = r.tabs().get(2) == Some(first.clone());
        m.tabs().get(1).unwrap().set_label(Some("Renamed".into()));
        let renamed = r.tabs().get(2).unwrap().header();
        r.set_model(None);
        m.tabs().get(1).unwrap().set_label(Some("Detached".into()));
        check(
            "collections",
            json!({"name":"collections","initial":initial,"moved":moved,"same":same,"renamed":renamed,"after":tabs(&r),"keptHeader":first.header(),"releasedBackstage":r.generated_model_backstage().is_none()}),
        );
        settle();
    });
}
#[test]
fn original_root_model_selection_feedback_and_capture_during_moves() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let m = model();
        m.tabs().add(Some(RibbonTabModel::new(
            Some("two".into()),
            Some("Two".into()),
        )));
        m.set_selected_tab_id(Some("two".into()));
        r.set_model(Some(m.clone()));
        let initial = r.selected_tab().unwrap().id();
        r.set_selected_tab(r.tabs().get(0));
        let user = m.selected_tab_id();
        m.set_selected_tab_id(Some("two".into()));
        let back = r.selected_tab().unwrap().id();
        let first = r.tabs().get(1);
        m.tabs().move_item(1, 0);
        check(
            "selection",
            json!({"name":"selection","initial":initial,"user":user.map(|s|s.to_utf8().unwrap()),"back":back,"selectedAfterMove":r.selected_tab().unwrap().id(),"same":r.selected_tab()==first,"model":m.selected_tab_id().map(|s|s.to_utf8().unwrap())}),
        );
        r.set_model(None);
        settle();
    });
}
#[test]
fn original_root_model_options_notification_order_and_both_directions() {
    crate::group_tests::run(|| {
        use ferroui_system::component_model::INotifyPropertyChanged;
        let r = Ribbon::new();
        let m = model();
        r.set_model(Some(m.clone()));
        let trace = Rc::new(RefCell::new(vec![]));
        let out = trace.clone();
        let token = m
            .property_changed()
            .add(Rc::new(move |p: &str| out.borrow_mut().push(p.to_owned())));
        r.set_display_mode(RibbonDisplayMode::Simplified);
        r.set_density(RibbonDensity::Touch);
        r.set_visibility_mode(RibbonVisibilityMode::PanelTitles);
        r.set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
        r.set_is_quick_access_visible(false);
        r.set_show_quick_access_labels(true);
        r.set_minimize_behavior(RibbonMinimizeBehavior::CycleAll);
        r.set_show_group_captions(false);
        m.set_application_button_label(Some("Model File".into()));
        m.set_is_application_button_visible(false);
        check(
            "options",
            json!({"name":"options","trace":*trace.borrow(),"display":m.display_mode().to_string(),"density":m.density().to_string(),"visibility":m.visibility_mode().to_string(),"position":m.quick_access_position().to_string(),"visible":m.is_quick_access_visible(),"labels":m.show_quick_access_labels(),"minimize":m.minimize_behavior().to_string(),"captions":m.show_group_captions(),"application":r.application_button_label(),"applicationVisible":r.is_application_button_visible()}),
        );
        m.property_changed().remove(token);
        r.set_model(None);
        settle();
    });
}
#[test]
fn original_root_model_catalog_reference_restore_and_external_override() {
    crate::group_tests::run(|| {
        let before = RibbonCommandCatalog::new();
        let a = RibbonCommandCatalog::new();
        let b = RibbonCommandCatalog::new();
        let external = RibbonCommandCatalog::new();
        let r = Ribbon::new();
        r.set_command_catalog(Some(before.clone()));
        let m = model();
        m.set_command_catalog(Some(a.clone()));
        r.set_model(Some(m.clone()));
        let applied = Rc::ptr_eq(&r.command_catalog().unwrap(), &a);
        m.set_command_catalog(Some(b.clone()));
        let replaced = Rc::ptr_eq(&r.command_catalog().unwrap(), &b);
        m.set_command_catalog(None);
        let restored = Rc::ptr_eq(&r.command_catalog().unwrap(), &before);
        m.set_command_catalog(Some(a));
        r.set_command_catalog(Some(external.clone()));
        r.set_model(None);
        check(
            "catalog",
            json!({"name":"catalog","applied":applied,"replaced":replaced,"restored":restored,"externalKept":Rc::ptr_eq(&r.command_catalog().unwrap(),&external)}),
        );
        settle();
    });
}
#[test]
fn original_root_model_backstage_always_created_conditional_install_restore_and_external_override()
{
    crate::group_tests::run(|| {
        let before = RibbonBackstage::new();
        let external = RibbonBackstage::new();
        let r = Ribbon::new();
        r.set_backstage(Some(before.clone()));
        let m = model();
        r.set_model(Some(m.clone()));
        let empty_kept = r.backstage() == Some(before.clone());
        let created = r.generated_model_backstage().is_some();
        let i = RibbonBackstageItemModel::new(Some("info".into()), Some("Info".into()));
        i.set_content(Some(RibbonObject::value("data".to_owned())));
        m.backstage().items().add(Some(i));
        let generated = r.backstage().unwrap();
        let applied = generated != before;
        m.backstage().items().clear();
        let restored = r.backstage() == Some(before);
        m.backstage()
            .items()
            .add(Some(RibbonBackstageItemModel::new(
                Some("next".into()),
                Some("Next".into()),
            )));
        let same = r.backstage() == Some(generated);
        r.set_backstage(Some(external.clone()));
        r.set_model(None);
        check(
            "backstage",
            json!({"name":"backstage","emptyKept":empty_kept,"created":created,"applied":applied,"restored":restored,"same":same,"externalKept":r.backstage()==Some(external)}),
        );
        settle();
    });
}
#[test]
fn original_root_model_backstage_open_uses_reentrant_actual_value() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let m = model();
        m.backstage().set_is_open(true);
        r.set_model(Some(m.clone()));
        let failed = !r.is_backstage_open() && !m.backstage().is_open();
        let page = RibbonBackstageItemModel::new(Some("info".into()), Some("Info".into()));
        page.set_content(Some(RibbonObject::value("page".to_owned())));
        m.backstage().items().add(Some(page));
        m.backstage().set_is_open(true);
        let opened = r.is_backstage_open();
        r.set_is_backstage_open(false);
        check(
            "backstage_open",
            json!({"name":"backstage_open","failed":failed,"opened":opened,"modelClosed":!m.backstage().is_open()}),
        );
        r.set_model(None);
        settle();
    });
}
#[test]
fn original_root_model_deferred_unload_keeps_instances_and_catches_up_on_resume() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let m = model();
        r.set_model(Some(m.clone()));
        let t = r.tabs().get(0).unwrap();
        r.on_model_loaded();
        r.on_model_unloaded();
        m.tabs()
            .get(0)
            .unwrap()
            .set_label(Some("before-detach".into()));
        let before = t.header();
        settle();
        m.tabs()
            .get(0)
            .unwrap()
            .set_label(Some("while-detached".into()));
        let frozen = t.header();
        let suspended = r.model_bindings().unwrap().is_suspended();
        r.on_model_loaded();
        let caught = t.header();
        r.on_model_unloaded();
        r.on_model_loaded();
        settle();
        m.tabs()
            .get(0)
            .unwrap()
            .set_label(Some("reparented".into()));
        check(
            "lifecycle",
            json!({"name":"lifecycle","before":before,"frozen":frozen,"suspended":suspended,"caught":caught,"reparented":t.header(),"live":!r.model_bindings().unwrap().is_suspended()}),
        );
        r.set_model(None);
        settle();
    });
}
#[test]
fn original_root_model_assigned_after_detach_stays_suspended_until_load() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        r.on_model_loaded();
        r.on_model_unloaded();
        settle();
        let m = model();
        r.set_model(Some(m.clone()));
        let t = r.tabs().get(0).unwrap();
        let suspended = r.model_bindings().unwrap().is_suspended();
        m.tabs().get(0).unwrap().set_label(Some("changed".into()));
        let before = t.header();
        r.on_model_loaded();
        check(
            "assigned_detached",
            json!({"name":"assigned_detached","suspended":suspended,"before":before,"after":t.header()}),
        );
        r.set_model(None);
        settle();
    });
}
#[test]
fn original_root_model_qat_linked_and_standalone_identity_move_reverse_sync_disposal() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let m = model();
        let save = m
            .tabs()
            .get(0)
            .unwrap()
            .groups()
            .get(0)
            .unwrap()
            .items()
            .get(0)
            .unwrap();
        let direct = RibbonButtonModel::new(Some("direct".into()), Some("Direct".into()));
        m.quick_access_items().add(Some(save.clone()));
        m.quick_access_items().add(Some(direct));
        r.set_model(Some(m.clone()));
        let initial = qat(&r);
        let items = r.quick_access_tool_bar().unwrap().items();
        let copy = items.get(0).unwrap();
        let standalone = items.get(1).unwrap();
        let source = RibbonItemHelper::get_source_item(&copy).unwrap();
        let linked = Ribbon::get_model(&source)
            .is_some_and(|n| std::ptr::addr_eq(Rc::as_ptr(&n), Rc::as_ptr(&save)));
        m.quick_access_items().move_item(1, 0);
        let moved = qat(&r);
        let same = items.get(0) == Some(standalone.clone()) && items.get(1) == Some(copy);
        items.remove(&Some(standalone));
        let reverse = m
            .quick_access_items()
            .iter()
            .map(|i| i.unwrap().id().map(|s| s.to_utf8().unwrap()))
            .collect::<Vec<_>>();
        r.set_model(None);
        check(
            "qat",
            json!({"name":"qat","initial":initial,"linked":linked,"moved":moved,"same":same,"reverse":reverse,"remaining":qat(&r)}),
        );
        settle();
    });
}
#[test]
fn original_root_model_candidates_keep_collection_and_null_ids_after_disposal() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let m = model();
        m.quick_access_candidates().add(Some(RibbonButtonModel::new(
            Some("a".into()),
            Some("A".into()),
        )));
        let null = RibbonButtonModel::new(None, Some("No Id".into()));
        null.set_id(None);
        m.quick_access_candidates().add(Some(null));
        r.set_model(Some(m.clone()));
        let ids = r.quick_access_candidate_ids();
        let read = || {
            ids.iter()
                .map(|s| s.map(|s| s.to_utf8().unwrap()))
                .collect::<Vec<_>>()
        };
        let initial = read();
        m.quick_access_candidates().move_item(1, 0);
        let moved = read();
        r.set_model(None);
        m.quick_access_candidates().clear();
        check(
            "candidates",
            json!({"name":"candidates","initial":initial,"moved":moved,"after":read()}),
        );
        assert!(ids == r.quick_access_candidate_ids());
        settle();
    });
}
#[test]
fn original_root_model_same_factory_assignment_recreates_and_null_is_rejected() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let m = model();
        r.set_model(Some(m.clone()));
        let before = r.tabs().get(0);
        r.set_item_factory(Some(r.item_factory()));
        let different = before != r.tabs().get(0);
        let owner = r.item_factory().ribbon() == Some(r.clone());
        let threw =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| r.set_item_factory(None)))
                .is_err();
        check(
            "factory",
            json!({"name":"factory","different":different,"owner":owner,"threw":threw,"sameModel":Rc::ptr_eq(&r.model().unwrap(),&(m as Rc<dyn RibbonModelImpl>))}),
        );
        r.set_model(None);
        settle();
    });
}
#[repr(C)]
struct Factory {
    base: RibbonElementFactory,
    pages: Cell<i32>,
}
ferro_class!(Factory:RibbonElementFactory);
impl FerroObjectImpl for Factory {}
impl RibbonElementFactoryImpl for Factory {
    fn create_backstage(
        this: &Self,
        m: Option<Rc<dyn RibbonBackstageModelImpl>>,
        b: Option<Rc<crate::mvvm::RibbonModelBindings>>,
    ) -> Ref<RibbonBackstage> {
        this.pages.set(this.pages.get() + 1);
        <Self as crate::mvvm::RibbonElementFactoryImplExt>::parent_create_backstage(this, m, b)
    }
}
#[test]
fn native_root_model_custom_factory_dispatch_empty_backstage_linked_invocation_and_catalog_replacement()
 {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let f = instantiate(Factory {
            base: RibbonElementFactory::construct(),
            pages: Cell::new(0),
        });
        r.set_item_factory(Some(f.clone().upcast()));
        let m = model();
        let item = m
            .tabs()
            .get(0)
            .unwrap()
            .groups()
            .get(0)
            .unwrap()
            .items()
            .get(0)
            .unwrap();
        item.set_command_id(Some("save".into()));
        m.quick_access_items().add(Some(item.clone()));
        let calls = Rc::new(Cell::new(0));
        let out = calls.clone();
        let catalog = RibbonCommandCatalog::new();
        catalog.register(Some(RibbonCommandDescriptor::new(
            Some("save".into()),
            Some("Save".into()),
            Some(RibbonRelayCommand::new(move || out.set(out.get() + 1))),
        )));
        m.set_command_catalog(Some(catalog));
        r.set_model(Some(m.clone()));
        assert_eq!(f.pages.get(), 1);
        assert!(r.generated_model_backstage().is_some());
        assert!(r.backstage().is_none());
        let original = r
            .find_item(Some("save".into()))
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        let copy = r
            .quick_access_tool_bar()
            .unwrap()
            .items()
            .get(0)
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        copy.invoke_core();
        assert_eq!(calls.get(), 1);
        let next_calls = Rc::new(Cell::new(0));
        let out = next_calls.clone();
        let can = Rc::new(Cell::new(true));
        let enabled = can.clone();
        let next_command = RibbonRelayCommand::new_with_can_execute(
            move || out.set(out.get() + 1),
            Some(Rc::new(move || enabled.get())),
        );
        let next_catalog = RibbonCommandCatalog::new();
        next_catalog.register(Some(RibbonCommandDescriptor::new(
            Some("save".into()),
            Some("Replacement".into()),
            Some(next_command.clone()),
        )));
        m.set_command_catalog(Some(next_catalog));
        assert!(copy.invoke_core());
        assert_eq!(calls.get(), 1);
        assert_eq!(next_calls.get(), 1);
        can.set(false);
        next_command.notify_can_execute_changed();
        copy.invoke_core();
        assert_eq!(next_calls.get(), 1);
        can.set(true);
        next_command.notify_can_execute_changed();
        assert!(copy.invoke_core());
        assert_eq!(next_calls.get(), 2);
        item.set_label(Some("Live".into()));
        assert_eq!(copy.label().as_deref(), Some("Live"));
        copy.set_label(Some("Reverse".into()));
        assert_eq!(item.label().unwrap().to_utf8().unwrap(), "Live");
        assert_eq!(original.label().as_deref(), Some("Live")); // Source Label link is one-way.
        let old = copy.clone();
        m.tabs()
            .get(0)
            .unwrap()
            .groups()
            .get(0)
            .unwrap()
            .items()
            .clear();
        settle();
        let direct = r.quick_access_tool_bar().unwrap().items().get(0).unwrap();
        assert!(!direct.ptr_eq(&old));
        // Source Unlink stops subscriptions but deliberately retains SourceItem.
        assert_eq!(
            RibbonItemHelper::get_source_item(&old),
            Some(original.clone().upcast())
        );
        assert!(RibbonItemHelper::get_source_item(&direct).is_none());
        item.set_label(Some("new direct".into()));
        assert_eq!(
            direct.cast::<RibbonButton>().unwrap().label().as_deref(),
            Some("new direct")
        );
        assert_eq!(original.label().as_deref(), Some("Live"));
        assert_eq!(old.label().as_deref(), Some("Reverse"));
        r.set_model(None);
        settle();
    });
}

#[test]
fn original_root_model_duplicate_qat_keeps_first_copy_and_source_identity() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let m = model();
        let item = m
            .tabs()
            .get(0)
            .unwrap()
            .groups()
            .get(0)
            .unwrap()
            .items()
            .get(0)
            .unwrap();
        m.quick_access_items().add(Some(item.clone()));
        m.quick_access_items().add(Some(item));
        r.set_model(Some(m));
        let items = r.quick_access_tool_bar().unwrap().items();
        let first = items.get(0).unwrap();
        let second = items.get(1).unwrap();
        let distinct = first != second;
        r.model().unwrap().quick_access_items().remove_at(0);
        check(
            "duplicate_qat",
            json!({"name":"duplicate_qat","distinct":distinct,"firstSurvives":items.get(0)==Some(first.clone()),"count":items.count(),"sourceKept":RibbonItemHelper::get_source_item(&second)==RibbonItemHelper::get_source_item(&first)}),
        );
        r.set_model(None);
        settle();
    });
}

#[test]
fn original_root_model_invalid_selection_and_null_qat_item() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let m = model();
        m.set_selected_tab_id(Some("missing".into()));
        m.quick_access_items().add(None);
        r.set_model(Some(m.clone()));
        check(
            "invalid_selection_null_qat",
            json!({"name":"invalid_selection_null_qat","model":m.selected_tab_id().map(|s|s.to_utf8().unwrap()),"selected":r.selected_tab().and_then(|t|t.id()),"qat":r.quick_access_tool_bar().unwrap().items().count()}),
        );
        r.set_model(None);
        settle();
    });
}

#[test]
fn original_root_model_null_candidate_throws_after_model_and_tabs_assigned() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let m = model();
        m.quick_access_candidates().add(None);
        let threw = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            r.set_model(Some(m.clone()))
        }))
        .is_err();
        check(
            "null_candidate",
            json!({"name":"null_candidate","threw":threw,"assigned":Rc::ptr_eq(&r.model().unwrap(),&(m as Rc<dyn RibbonModelImpl>)),"tabs":r.tabs().count()}),
        );
        r.set_model(None);
        settle();
    });
}

#[test]
fn original_root_model_reentrant_model_replacement_during_options_assignment() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let m = model();
        m.set_density(RibbonDensity::Touch);
        let next = RibbonModel::new();
        next.tabs().add(Some(RibbonTabModel::new(
            Some("next".into()),
            Some("Next".into()),
        )));
        next.set_density(RibbonDensity::Compact);
        let calls = Rc::new(Cell::new(0));
        let out = calls.clone();
        let weak = r.downgrade();
        let replacement = next.clone();
        let token = r.add_property_changed_handler(Rc::new(move |e| {
            if e.property() == Ribbon::density_property().as_property()
                && e.is_effective_value_change()
            {
                let count = out.get();
                out.set(count + 1);
                if count == 0 {
                    weak.upgrade().unwrap().set_model(Some(replacement.clone()));
                }
            }
        }));
        r.set_model(Some(m.clone()));
        r.remove_property_changed_handler(token);
        m.tabs().get(0).unwrap().set_label(Some("Old model".into()));
        m.set_application_button_label(Some("Old File".into()));
        check(
            "reentrant_options",
            json!({"name":"reentrant_options","calls":calls.get(),"assigned":Rc::ptr_eq(&r.model().unwrap(),&(next as Rc<dyn RibbonModelImpl>)),"tabs":tabs(&r),"selected":r.selected_tab().and_then(|t|t.id()),"density":r.density().to_string(),"application":r.application_button_label()}),
        );
        r.set_model(None);
        settle();
    });
}

#[test]
fn native_root_model_toggle_qat_two_way_state_exact_command_event_and_unlink() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let m = RibbonModel::new();
        let tab = RibbonTabModel::new(Some("home".into()), Some("Home".into()));
        let group = RibbonGroupModel::new(Some("tools".into()), Some("Tools".into()));
        let toggle = RibbonToggleButtonModel::new(Some("toggle".into()), Some("Toggle".into()));
        let commands = Rc::new(Cell::new(0));
        let out = commands.clone();
        toggle.set_command(Some(RibbonRelayCommand::new(move || {
            out.set(out.get() + 1)
        })));
        group.items().add(Some(toggle.clone()));
        tab.groups().add(Some(group));
        m.tabs().add(Some(tab));
        m.quick_access_items().add(Some(toggle.clone()));
        let invoked = Rc::new(Cell::new(0));
        let out = invoked.clone();
        r.add_item_invoked(Some(ferroui_base::metadata::MarkupDelegate::new(
            move |_| {
                out.set(out.get() + 1);
                None
            },
        )));
        r.set_model(Some(m.clone()));
        let original = r
            .find_item(Some("toggle".into()))
            .unwrap()
            .cast::<RibbonToggleButton>()
            .unwrap();
        let copy = r
            .quick_access_tool_bar()
            .unwrap()
            .items()
            .get(0)
            .unwrap()
            .cast::<RibbonToggleButton>()
            .unwrap();
        copy.set_is_checked(Some(true));
        assert_eq!(original.is_checked(), Some(true));
        assert!(toggle.is_checked());
        toggle.set_is_checked(false);
        assert_eq!(copy.is_checked(), Some(false));
        copy.on_click();
        assert_eq!(commands.get(), 1);
        assert_eq!(invoked.get(), 1);
        assert_eq!(original.is_checked(), Some(true));
        assert!(toggle.is_checked());
        toggle.set_is_checked(false);
        assert!(copy.invoke_core());
        assert_eq!(commands.get(), 2);
        // Source peer Toggle raises Click; InvokeCore then reports invocation again.
        // Pinned checked-controls.json records one command and two notifications.
        assert_eq!(invoked.get(), 3);
        r.quick_access_tool_bar().unwrap().items().clear();
        assert!(m.quick_access_items().is_empty());
        toggle.set_is_checked(false);
        assert_eq!(original.is_checked(), Some(false));
        assert_eq!(copy.is_checked(), Some(true));
        copy.set_is_checked(Some(false));
        copy.set_is_checked(Some(true));
        assert!(!toggle.is_checked());
        r.set_model(None);
        settle();
    });
}

#[test]
fn original_root_model_unload_retains_even_without_model_until_callback_drains() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let weak = r.downgrade();
        r.on_model_unloaded();
        drop(r);
        let before = weak.upgrade().is_some();
        settle();
        check(
            "unload_retention",
            json!({"name":"unload_retention","before":before,"after":weak.upgrade().is_some()}),
        );
    });
}

#[test]
fn native_model_subscriptions_do_not_retain_a_dropped_ribbon_view() {
    crate::group_tests::run(|| {
        let model = model();
        let ribbon = Ribbon::new();
        ribbon.set_model(Some(model.clone()));
        settle();
        let weak = ribbon.downgrade();
        drop(ribbon);
        settle();
        assert!(weak.upgrade().is_none());
        model.set_application_button_label(Some("After view drop".into()));
        settle();
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn source_factory_context_identity_survives_assignment_clearing_and_model_transitions() {
    crate::group_tests::run(|| {
        let ribbon = Ribbon::new();
        let other = Ribbon::new();
        let factory = RibbonElementFactory::new();
        let mut states = Vec::new();
        let snapshot = |phase: &str| {
            let context = factory.ribbon();
            json!({"phase":phase,"context":if context.as_ref().is_some_and(|c|c.ptr_eq(&ribbon)){"self"}else if context.as_ref().is_some_and(|c|c.ptr_eq(&other)){"other"}else{"null"},"sameFactory":ribbon.item_factory().ptr_eq(&factory),"modelAssigned":ribbon.model().is_some()})
        };
        states.push(snapshot("initial"));
        factory.set_ribbon(Some(other.clone()));
        states.push(snapshot("standalone-other"));
        ribbon.set_item_factory(Some(factory.clone()));
        states.push(snapshot("assigned-other"));
        factory.set_ribbon(Some(ribbon.clone()));
        states.push(snapshot("assigned-self"));
        ribbon.set_item_factory(Some(factory.clone()));
        states.push(snapshot("same-assignment"));
        factory.set_ribbon(None);
        states.push(snapshot("cleared"));
        ribbon.set_model(Some(RibbonModel::new()));
        states.push(snapshot("model"));
        ribbon.set_model(None);
        states.push(snapshot("model-cleared"));
        ribbon.set_item_factory(Some(RibbonElementFactory::new()));
        states.push(snapshot("replaced"));
        factory.set_ribbon(Some(other.clone()));
        states.push(snapshot("detached-other"));
        check(
            "factory_context",
            json!({"name":"factory_context","states":states}),
        );
    });
}

#[test]
fn native_factory_parent_context_does_not_retain_empty_model_ribbons() {
    crate::group_tests::run(|| {
        for phase in [
            "default",
            "before-assignment",
            "after-assignment",
            "same-assignment",
        ] {
            let ribbon = Ribbon::new();
            let factory = if phase == "default" {
                ribbon.item_factory()
            } else {
                RibbonElementFactory::new()
            };
            assert!(factory.ribbon().is_none());
            if phase == "before-assignment" {
                factory.set_ribbon(Some(ribbon.clone()));
            }
            if phase != "default" {
                ribbon.set_item_factory(Some(factory.clone()));
            }
            factory.set_ribbon(Some(ribbon.clone()));
            if phase == "same-assignment" {
                ribbon.set_item_factory(Some(factory.clone()));
            }
            assert!(ribbon.model().is_none());
            assert!(factory.ribbon().unwrap().ptr_eq(&ribbon));
            let weak = ribbon.downgrade();
            drop(ribbon);
            assert!(weak.upgrade().is_none(), "{phase}");
            assert!(factory.ribbon().is_none());
        }
    });
}

#[test]
fn native_factory_shared_parent_contexts_and_later_setters_remain_weak() {
    crate::group_tests::run(|| {
        let first = Ribbon::new();
        let second = Ribbon::new();
        let factory = RibbonElementFactory::new();
        first.set_item_factory(Some(factory.clone()));
        second.set_item_factory(Some(factory.clone()));
        for owner in [&first, &second] {
            factory.set_ribbon(Some(owner.clone()));
            assert!(factory.ribbon().unwrap().ptr_eq(owner));
            factory.set_ribbon(None);
            assert!(factory.ribbon().is_none());
            factory.set_ribbon(Some(owner.clone()));
        }
        let weak_first = first.downgrade();
        let weak_second = second.downgrade();
        drop(first);
        assert!(weak_first.upgrade().is_none());
        assert!(factory.ribbon().unwrap().ptr_eq(&second));
        drop(second);
        assert!(weak_second.upgrade().is_none());
        assert!(factory.ribbon().is_none());
    });
}

#[test]
fn native_factory_standalone_and_detached_contexts_keep_acyclic_ownership() {
    crate::group_tests::run(|| {
        let factory = RibbonElementFactory::new();
        let standalone = Ribbon::new();
        let weak_standalone = standalone.downgrade();
        factory.set_ribbon(Some(standalone));
        assert!(weak_standalone.upgrade().is_some());
        factory.set_ribbon(None);
        assert!(weak_standalone.upgrade().is_none());
        let parent = Ribbon::new();
        parent.set_item_factory(Some(factory.clone()));
        factory.set_ribbon(Some(parent.clone()));
        parent.set_item_factory(Some(RibbonElementFactory::new()));
        let weak_parent = parent.downgrade();
        drop(parent);
        assert!(
            weak_parent.upgrade().is_some(),
            "a detached standalone factory owns its context"
        );
        factory.set_ribbon(None);
        assert!(weak_parent.upgrade().is_none());
    });
}

#[test]
fn native_factory_replacement_keeps_other_shared_owner_registration() {
    crate::group_tests::run(|| {
        let first = Ribbon::new();
        let second = Ribbon::new();
        let factory = RibbonElementFactory::new();
        first.set_item_factory(Some(factory.clone()));
        second.set_item_factory(Some(factory.clone()));
        factory.set_ribbon(Some(second.clone()));
        first.set_item_factory(Some(RibbonElementFactory::new()));
        let weak_first = first.downgrade();
        let weak_second = second.downgrade();
        drop(first);
        assert!(weak_first.upgrade().is_none());
        drop(second);
        assert!(weak_second.upgrade().is_none());
        assert!(factory.ribbon().is_none());
    });
}
