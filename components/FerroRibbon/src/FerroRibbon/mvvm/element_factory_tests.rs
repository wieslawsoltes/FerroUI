use super::*;
use crate::controls::*;
use ferroribbon_core::{commands::*, model::*};
use ferroui_base::{
    FerroObject, FerroObjectImpl, Ref,
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, instantiate,
};
use ferroui_controls::{Control, automation::AutomationProperties};
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub(super) fn check(name: &str, mut actual: Value) {
    let v: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/element-factory.json")).unwrap();
    let expected = v["corpus"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap();
    actual
        .as_object_mut()
        .unwrap()
        .insert("name".into(), json!(name));
    // These corpus numbers are source double fields. JSON writes integral doubles
    // without a decimal; compare their f64 values, not serializer number tags.
    fn doubles(value: &mut Value) {
        match value {
            Value::Number(n) => *value = json!(n.as_f64().unwrap()),
            Value::Array(v) => v.iter_mut().for_each(doubles),
            Value::Object(v) => v.values_mut().for_each(doubles),
            _ => {}
        }
    }
    let mut expected = expected.clone();
    doubles(&mut expected);
    doubles(&mut actual);
    assert_eq!(actual, expected);
}
#[test]
fn original_factory_associations_preserve_identity_and_do_not_raise_properties() {
    crate::group_tests::run(|| {
        let a = FerroObject::new();
        let b = FerroObject::new();
        let changes = Rc::new(Cell::new(0));
        let c = changes.clone();
        a.add_property_changed_handler(Rc::new(move |_| c.set(c.get() + 1)));
        let first = RibbonSliderModel::new(Some("same".into()), Some("first".into()));
        let second = RibbonSliderModel::new(Some("same".into()), Some("second".into()));
        let before = RibbonElementFactory::get_model(&a).is_none();
        RibbonElementFactory::set_model(&a, first.clone());
        let after = Rc::ptr_eq(
            &RibbonElementFactory::get_model(&a).unwrap(),
            &(first.clone() as Rc<dyn RibbonNodeModelImpl>),
        );
        RibbonElementFactory::set_model(&a, second.clone());
        check(
            "association",
            json!({"before":before,"after":after,"replaced":Rc::ptr_eq(&RibbonElementFactory::get_model(&a).unwrap(),&(second.clone() as Rc<dyn RibbonNodeModelImpl>)),"separate":RibbonElementFactory::get_model(&b).is_none()}),
        );
        assert_eq!(changes.get(), 0);
        let weak = Rc::downgrade(&second);
        drop(second);
        assert!(weak.upgrade().is_some());
        drop(a);
        assert!(weak.upgrade().is_none());
    });
}
#[test]
fn original_slider_factory_two_way_suspend_resume_and_disposal() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonSliderModel::new(Some("s".into()), Some("Slider".into()));
        m.set_minimum(-4.);
        m.set_maximum(20.);
        m.set_step_frequency(2.5);
        m.set_slider_width(139.);
        m.set_value(7.25);
        let e = f
            .create_slider(m.clone(), b.clone())
            .cast::<RibbonSlider>()
            .unwrap();
        let initial = [
            e.minimum(),
            e.maximum(),
            e.step_frequency(),
            e.input_width(),
            e.value(),
        ];
        e.set_numeric_value(8.5);
        let from_element = m.value();
        m.set_value(12.);
        let from_model = e.value();
        b.suspend();
        e.set_numeric_value(2.);
        m.set_value(16.);
        let suspended = [m.value(), e.value()];
        b.resume();
        let resumed = e.value();
        b.dispose();
        e.set_numeric_value(1.);
        m.set_value(19.);
        check(
            "slider",
            json!({"initial":initial,"fromElement":from_element,"fromModel":from_model,"suspended":suspended,"resumed":resumed,"disposed":[m.value(),e.value()]}),
        );
    });
}
#[test]
fn original_toggle_and_text_factory_null_and_feedback_rules() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonToggleButtonModel::new(Some("t".into()), Some("Toggle".into()));
        m.set_is_checked(true);
        m.set_group_name(Some("g".into()));
        let e = f
            .create_toggle_button(m.clone(), b.clone())
            .cast::<RibbonToggleButton>()
            .unwrap();
        e.set_is_checked(None);
        let nil = m.is_checked();
        e.set_is_checked(Some(true));
        let yes = m.is_checked();
        m.set_group_name(Some("new".into()));
        check(
            "toggle",
            json!({"nil":nil,"yes":yes,"group":e.group_name()}),
        );
        b.dispose();
        let b = RibbonModelBindings::new();
        let m = RibbonTextBoxModel::new(Some("t".into()), Some("Text".into()));
        m.set_text(None);
        m.set_placeholder(Some("hint".into()));
        m.set_input_width(154.);
        let e = f
            .create_text_box(m.clone(), b.clone())
            .cast::<RibbonTextBox>()
            .unwrap();
        let initial = e.text();
        e.set_text(Some("new".into()));
        let from_element = m.text().map(|v| v.to_utf8().unwrap());
        m.set_text(None);
        check(
            "text",
            json!({"initial":initial,"fromElement":from_element,"fromModel":e.text(),"placeholder":e.placeholder_text(),"width":e.input_width()}),
        );
        b.dispose();
    });
}
#[test]
fn original_watch_inner_does_not_apply_initially_and_detaches_replaced_tips() {
    crate::group_tests::run(|| {
        let b = RibbonModelBindings::new();
        let m = RibbonSliderModel::new(Some("s".into()), Some("Slider".into()));
        let first = ferroribbon_core::model::RibbonScreenTip::with_title(Some("first".into()));
        let second = ferroribbon_core::model::RibbonScreenTip::with_title(Some("second".into()));
        m.set_screen_tip(Some(first.clone()));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let out = trace.clone();
        let mm = m.clone();
        let getter = m.clone();
        RibbonElementFactory::watch_inner(
            &b,
            m.clone(),
            Rc::new(move || {
                getter
                    .screen_tip()
                    .map(|t| t as Rc<dyn INotifyPropertyChanged>)
            }),
            Some("ScreenTip".into()),
            Rc::new(move || {
                out.borrow_mut().push(
                    mm.screen_tip()
                        .and_then(|t| t.title())
                        .map(|v| v.to_utf8().unwrap())
                        .unwrap_or("null".into()),
                )
            }),
        );
        first.set_title(Some("a".into()));
        m.set_screen_tip(Some(second.clone()));
        first.set_title(Some("old".into()));
        second.set_title(Some("b".into()));
        b.suspend();
        second.set_title(Some("suspended".into()));
        m.set_screen_tip(Some(first.clone()));
        b.resume();
        second.set_title(Some("stale".into()));
        first.set_title(Some("final".into()));
        b.dispose();
        first.set_title(Some("disposed".into()));
        check("watch_inner", json!({"trace":*trace.borrow()}));
        assert_eq!(first.property_changed().handler_count(), 0);
        assert_eq!(second.property_changed().handler_count(), 0);
    });
}
#[test]
fn factory_common_properties_keywords_automation_and_linked_slider_are_live() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonSliderModel::new(Some("opacity".into()), Some("Opacity".into()));
        m.set_automation_id(Some("explicit".into()));
        m.keywords().add(None);
        m.keywords().add(Some("\u{1f600},x".into()));
        m.set_value(40.);
        let e = f
            .create_item(Some(m.clone()), b.clone())
            .unwrap()
            .cast::<RibbonSlider>()
            .unwrap();
        assert_eq!(e.label().as_deref(), Some("Opacity"));
        assert_eq!(RibbonSearch::get_keywords(&e).as_deref(), Some(",😀,x"));
        assert_eq!(
            AutomationProperties::get_automation_id(&e).as_deref(),
            Some("explicit")
        );
        m.set_automation_id(None);
        assert_eq!(
            AutomationProperties::get_automation_id(&e).as_deref(),
            Some("opacity")
        );
        m.set_is_enabled(false);
        assert!(!e.is_enabled());
        m.set_is_enabled(true);
        assert!(e.is_enabled());
        m.set_is_visible(false);
        assert!(!e.is_visible());
        m.set_is_visible(true);
        assert!(e.is_visible());
        m.keywords().clear();
        assert!(
            e.get_local_value(RibbonSearch::keywords_property())
                .is_none()
        );
        let item = ValueTypes::try_cast(
            &Control::boxed(e.clone()),
            ValueType::of::<Rc<dyn IRibbonItem>>(),
        )
        .unwrap();
        let copy = item
            .downcast_ref::<Rc<dyn IRibbonItem>>()
            .unwrap()
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonSlider>()
            .unwrap();
        copy.set_numeric_value(62.);
        assert_eq!(m.value(), 62.);
        m.set_value(18.);
        assert_eq!(copy.value(), 18.);
        assert!(RibbonElementFactory::get_model(&copy).is_none());
        RibbonItemHelper::unlink(&copy);
        copy.set_numeric_value(7.);
        assert_eq!(m.value(), 18.);
        b.dispose();
        m.set_value(9.);
        assert_eq!(e.value(), 18.);
        assert!(RibbonElementFactory::get_model(&e).is_some());
    });
}
#[test]
fn factory_commands_resolve_replace_invalidate_and_preserve_exact_parameter_payload() {
    crate::group_tests::run(|| {
        struct SaveRequest(&'static str);

        let calls = Rc::new(RefCell::new(Vec::new()));
        let param = Rc::new(SaveRequest("first"));
        let capture = param.clone();
        let out = calls.clone();
        let cmd: Rc<dyn ICommand> = Rc::new(RibbonRelayCommand::construct(
            Rc::new(move |p| {
                let request = p.unwrap().downcast_ref::<SaveRequest>().unwrap();
                assert!(std::ptr::eq(request, capture.as_ref()));
                out.borrow_mut().push(request.0);
            }),
            None,
        ));
        let cat = RibbonCommandCatalog::new();
        let d = cat.register_command(
            Some("save".into()),
            Some("Save".into()),
            Some(cmd),
            RibbonCommandMetadata::default(),
        );
        let r = Ribbon::new();
        r.set_command_catalog(Some(cat));
        let f = RibbonElementFactory::new();
        f.set_ribbon(Some(r));
        let b = RibbonModelBindings::new();
        let m = RibbonButtonModel::new(Some("b".into()), Some("Button".into()));
        m.set_command_id(Some("save".into()));
        m.set_command_parameter(Some(RibbonObject::reference(param.clone())));
        let e = f
            .create_item(Some(m.clone()), b.clone())
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        e.perform_click();
        assert_eq!(*calls.borrow(), vec!["first"]);
        let out = calls.clone();
        let replacement = Rc::new(SaveRequest("next"));
        let capture = replacement.clone();
        let cmd: Rc<dyn ICommand> = RibbonRelayCommand::new_with_parameter(
            move |p| {
                let request = p.unwrap().downcast_ref::<SaveRequest>().unwrap();
                if request.0 == "first" {
                    assert!(std::ptr::eq(request, param.as_ref()));
                    out.borrow_mut().push("replacement with first argument");
                } else {
                    assert!(std::ptr::eq(request, capture.as_ref()));
                    out.borrow_mut().push(request.0);
                }
            },
            None,
        );
        d.set_command(Some(cmd));
        f.invalidate_commands();
        e.perform_click();
        m.set_command_parameter(Some(RibbonObject::reference(replacement)));
        e.perform_click();
        assert_eq!(
            *calls.borrow(),
            ["first", "replacement with first argument", "next"]
        );
        b.suspend();
        m.set_command_id(None);
        f.invalidate_commands();
        assert!(e.command().is_some());
        b.resume();
        assert!(e.command().is_none());
        b.dispose();
    });
}
#[repr(C)]
struct DerivedFactory {
    base: RibbonElementFactory,
    creates: Cell<usize>,
    binds: Cell<usize>,
}
ferro_class!(DerivedFactory:RibbonElementFactory);
ferro_class_info!(DerivedFactory {});
impl FerroObjectImpl for DerivedFactory {}
impl RibbonElementFactoryImpl for DerivedFactory {
    fn create_button(
        this: &Self,
        _m: Rc<dyn RibbonButtonModelImpl>,
        _b: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        this.creates.set(this.creates.get() + 1);
        let e = RibbonButton::new();
        e.set_name(Some("custom".into()));
        e.upcast()
    }
    fn bind_common(
        this: &Self,
        e: Ref<Control>,
        m: RibbonItemModelReference,
        b: Rc<RibbonModelBindings>,
    ) {
        this.binds.set(this.binds.get() + 1);
        Self::parent_bind_common(this, e, m, b)
    }
}
#[test]
fn factory_virtual_creation_and_common_dispatch_use_same_native_controls() {
    crate::group_tests::run(|| {
        let f = instantiate(DerivedFactory {
            base: RibbonElementFactory::construct(),
            creates: Cell::new(0),
            binds: Cell::new(0),
        });
        let b = RibbonModelBindings::new();
        let m = RibbonButtonModel::new(Some("b".into()), Some("Button".into()));
        let base = f.clone().upcast::<RibbonElementFactory>();
        let e = base.create_item(Some(m.clone()), b.clone()).unwrap();
        assert_eq!(e.name().as_deref(), Some("custom"));
        assert_eq!(
            e.cast::<RibbonButton>().unwrap().label().as_deref(),
            Some("Button")
        );
        assert_eq!(f.creates.get(), 1);
        assert_eq!(f.binds.get(), 1);
        assert!(Rc::ptr_eq(
            &RibbonElementFactory::get_model(&e).unwrap(),
            &(m as Rc<dyn RibbonNodeModelImpl>)
        ));
        assert!(base.create_item(None, b.clone()).is_none());
        b.dispose();
    });
}

struct UnsupportedModel {
    base: RibbonItemModel,
}
impl INotifyPropertyChanged for UnsupportedModel {
    fn property_changed(&self) -> &ferroui_system::events::Event<str> {
        self.base.property_changed()
    }
}
impl ferroribbon_core::ObservableObjectImpl for UnsupportedModel {}
impl RibbonNodeModelImpl for UnsupportedModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for UnsupportedModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
#[test]
fn unsupported_models_return_none_and_custom_content_uses_the_native_presenter() {
    crate::group_tests::run(|| {
        register_ribbon_item_model::<UnsupportedModel>().unwrap();
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        assert!(
            f.create_item(
                Some(Rc::new(UnsupportedModel {
                    base: RibbonItemModel::construct(Some("unknown".into()), None)
                })),
                b.clone()
            )
            .is_none()
        );
        let m = RibbonCustomItemModel::new(Some("custom".into()), None);
        let custom = f.create_item(Some(m), b.clone()).unwrap();
        assert!(custom.is::<ferroui_controls::presenters::ContentPresenter>());
        b.dispose();
    });
}

#[test]
fn original_factory_command_token_filters_and_refreshes_both_entries_on_resume() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonButtonModel::new(Some("b".into()), Some("Button".into()));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let out = trace.clone();
        let model = m.clone();
        f.bind_commands(
            &b,
            m.clone(),
            Rc::new(move || {
                out.borrow_mut().push(
                    model
                        .command_id()
                        .map(|v| v.to_utf8().unwrap())
                        .unwrap_or("null".into()),
                )
            }),
            vec![Some("CommandId".into())],
        );
        m.set_label(Some("Other".into()));
        m.set_command_id(Some("first".into()));
        f.invalidate_commands();
        b.suspend();
        m.set_command_id(Some("second".into()));
        f.invalidate_commands();
        b.resume();
        b.dispose();
        f.invalidate_commands();
        check("commands", json!({"trace":*trace.borrow()}));
    });
}

#[test]
fn original_tab_group_factory_preserves_moves_duplicate_occurrences_and_detaches_removed_nodes() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let model = RibbonTabModel::new(Some("tab".into()), Some("Home".into()));
        let first = RibbonGroupModel::new(Some("same".into()), Some("First".into()));
        let second = RibbonGroupModel::new(Some("same".into()), Some("Second".into()));
        let item = RibbonButtonModel::new(Some("item".into()), Some("Before".into()));
        first.items().add(Some(item.clone()));
        model.groups().add(Some(first.clone()));
        model.groups().add(Some(second.clone()));
        let tab = f.create_tab(model.clone(), b.clone());
        let first_element = tab.groups().get(0).unwrap();
        let second_element = tab.groups().get(1).unwrap();
        let item_element = first_element
            .items()
            .get(0)
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        model.groups().move_item(1, 0);
        let moved = tab.groups().get(0).unwrap().ptr_eq(&second_element)
            && tab.groups().get(1).unwrap().ptr_eq(&first_element);
        model.groups().add(Some(first.clone()));
        let distinct = !tab.groups().get(2).unwrap().ptr_eq(&first_element);
        model.groups().remove_at(1);
        let retained_first = tab.groups().get(1).unwrap().ptr_eq(&first_element);
        item.set_label(Some("After".into()));
        let renamed = item_element.label();
        first.items().clear();
        item.set_label(Some("Removed".into()));
        let stopped = item_element.label();
        let associated = Rc::ptr_eq(
            &Ribbon::get_model(&item_element).unwrap(),
            &(item.clone() as Rc<dyn RibbonNodeModelImpl>),
        );
        b.suspend();
        model.set_label(Some("Paused".into()));
        second.set_label(Some("Changed".into()));
        let during = tab.header();
        b.resume();
        let after = tab.header();
        let group = tab.groups().get(0).unwrap().header();
        b.dispose();
        model.set_label(Some("Disposed".into()));
        check(
            "structure_tabs",
            json!({"moved":moved,"distinct":distinct,"retainedFirst":retained_first,"renamed":renamed,"stopped":stopped,"associated":associated,"during":during,"after":after,"group":group,"disposed":tab.header(),"groups":tab.groups().iter().map(|g|g.unwrap().header().unwrap()).collect::<Vec<_>>()}),
        );
    });
}
fn model_tip(v: Option<ferroui_base::BoxedValue>) -> String {
    match v {
        None => "null".into(),
        Some(v) => {
            if let Some(t) = v.downcast_ref::<Rc<ferroribbon_core::model::RibbonScreenTip>>() {
                format!("model:{}", t.title().unwrap().to_utf8().unwrap())
            } else if let Some(s) = v.downcast_ref::<String>() {
                format!("string:{s}")
            } else {
                "other".into()
            }
        }
    }
}
#[test]
fn original_group_factory_resets_same_screen_tips_and_preserves_missing_launcher_visibility() {
    crate::group_tests::run(|| {
        use ferroribbon_core::{RibbonGroupItemsLayout, RibbonSimplifiedVisibility};
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let tip = ferroribbon_core::model::RibbonScreenTip::with_title(Some("old".into()));
        let launcher = ferroribbon_core::model::RibbonScreenTip::with_title(Some("launch".into()));
        let m = RibbonGroupModel::new(Some("g".into()), Some("Group".into()));
        m.set_screen_tip(Some(tip.clone()));
        m.set_dialog_launcher_screen_tip(Some(launcher.clone()));
        m.set_dialog_launcher_command_id(Some("missing".into()));
        m.set_row_count(2);
        m.set_reduction_order(7);
        m.set_can_collapse(false);
        m.set_items_layout(RibbonGroupItemsLayout::Rows);
        m.set_simplified_visibility(RibbonSimplifiedVisibility::Overflow);
        let g = f.create_group(m.clone(), b.clone());
        let tips = Rc::new(RefCell::new(Vec::new()));
        let launchers = Rc::new(RefCell::new(Vec::new()));
        let out = tips.clone();
        let out2 = launchers.clone();
        g.add_property_changed_handler(Rc::new(move |e| {
            if !e.is_effective_value_change() {
                return;
            }
            if e.property() == RibbonGroup::screen_tip_property().as_property() {
                out.borrow_mut().push(model_tip(
                    e.get_new_value::<Option<ferroui_base::BoxedValue>>(),
                ));
            }
            if e.property() == RibbonGroup::dialog_launcher_screen_tip_property().as_property() {
                out2.borrow_mut().push(model_tip(
                    e.get_new_value::<Option<ferroui_base::BoxedValue>>(),
                ));
            }
        }));
        tip.set_title(Some("new".into()));
        launcher.set_title(Some("changed".into()));
        m.set_label(Some("Renamed".into()));
        let tip_trace = tips.borrow().clone();
        let launcher_trace = launchers.borrow().clone();
        let missing = g.dialog_launcher_command().is_none();
        let visible = g.is_dialog_launcher_visible();
        m.set_dialog_launcher_command_id(None);
        let hidden = !g.is_dialog_launcher_visible();
        b.dispose();
        tip.set_title(Some("disposed".into()));
        launcher.set_title(Some("disposed".into()));
        check(
            "structure_group",
            json!({"tipTrace":tip_trace,"launcherTrace":launcher_trace,"missing":missing,"visible":visible,"hidden":hidden,"row":g.row_count(),"reduction":g.reduction_order(),"collapse":g.can_collapse(),"layout":g.items_layout().raw(),"simplified":g.simplified_visibility().raw(),"header":g.header(),"tipSame":Rc::ptr_eq(g.screen_tip().unwrap().downcast_ref::<Rc<ferroribbon_core::model::RibbonScreenTip>>().unwrap(),&tip),"launcherSame":Rc::ptr_eq(g.dialog_launcher_screen_tip().unwrap().downcast_ref::<Rc<ferroribbon_core::model::RibbonScreenTip>>().unwrap(),&launcher)}),
        );
        assert_eq!(*tips.borrow(), tip_trace);
        assert_eq!(*launchers.borrow(), launcher_trace);
    });
}
#[test]
fn original_nested_container_factory_keeps_rows_and_joined_groups_as_same_native_types() {
    crate::group_tests::run(|| {
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let row = RibbonRowModel::new(Some("row".into()));
        let joined = RibbonButtonGroupModel::new(Some("joined".into()));
        let a = RibbonButtonModel::new(Some("a".into()), Some("A".into()));
        let z = RibbonSliderModel::new(Some("z".into()), Some("Z".into()));
        joined.items().add(Some(a.clone()));
        row.items().add(Some(joined.clone()));
        row.items().add(Some(z));
        let e = f
            .create_item(Some(row.clone()), b.clone())
            .unwrap()
            .cast::<RibbonStackPanel>()
            .unwrap();
        let j = e
            .items()
            .get(0)
            .unwrap()
            .cast::<RibbonButtonGroup>()
            .unwrap();
        let child = j.items().get(0).unwrap();
        row.items().move_item(0, 1);
        joined.items().add(Some(RibbonLabelModel::new(
            Some("label".into()),
            Some("Label".into()),
        )));
        let preserved =
            e.items().get(1).unwrap().ptr_eq(&j) && j.items().get(0).unwrap().ptr_eq(&child);
        let a_element = child.cast::<RibbonButton>().unwrap();
        a.set_label(Some("Renamed".into()));
        let before = a_element.label();
        b.dispose();
        a.set_label(Some("Detached".into()));
        joined.items().clear();
        check(
            "structure_container",
            json!({"preserved":preserved,"before":before,"after":a_element.label(),"outer":e.items().count(),"inner":j.items().count(),"associated":Rc::ptr_eq(&Ribbon::get_model(&j).unwrap(),&(joined as Rc<dyn RibbonNodeModelImpl>)),"rowKind":"RibbonStackPanel","groupKind":"RibbonButtonGroup"}),
        );
    });
}

#[test]
fn original_contextual_factory_preserves_color_and_visibility_two_way_guards() {
    crate::group_tests::run(|| {
        use ferroribbon_core::{RibbonContextualActivation, theming::RibbonColor};
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let m = RibbonContextualGroupModel::new(Some("tools".into()), Some("Tools".into()));
        m.set_color(RibbonColor::new(73, 12, 24, 48));
        m.set_activation(RibbonContextualActivation::SelectOnShow);
        m.set_key_tip(Some("KT".into()));
        m.set_is_enabled(false);
        let e = f.create_contextual_group(Some(m.clone()), Some(b.clone()));
        let initial = e.is_visible();
        m.set_is_visible(true);
        let from_model = e.is_visible();
        e.set_is_visible(false);
        let from_element = m.is_visible();
        b.suspend();
        e.set_is_visible(true);
        let suspended = m.is_visible();
        b.resume();
        let resumed = e.is_visible();
        b.dispose();
        e.set_is_visible(true);
        let color = e.color();
        check(
            "structure_contextual",
            json!({"initial":initial,"fromModel":from_model,"fromElement":from_element,"suspended":suspended,"resumed":resumed,"disposed":m.is_visible(),"header":e.header(),"keytip":e.key_tip(),"enabled":e.is_enabled(),"activation":e.activation().raw(),"color":[color.a,color.r,color.g,color.b],"associated":Rc::ptr_eq(&Ribbon::get_model(&e).unwrap(),&(m as Rc<dyn RibbonNodeModelImpl>))}),
        );
    });
}
#[test]
fn generated_contextual_tabs_use_actual_root_selection_and_previous_tab_restoration() {
    crate::group_tests::run(|| {
        use ferroribbon_core::RibbonContextualActivation;
        let f = RibbonElementFactory::new();
        let b = RibbonModelBindings::new();
        let ribbon = Ribbon::new();
        f.set_ribbon(Some(ribbon.clone()));
        let regular = RibbonTabModel::new(Some("regular".into()), Some("Home".into()));
        let contextual = RibbonTabModel::new(Some("picture".into()), Some("Picture".into()));
        contextual.set_contextual_group_id(Some("tools".into()));
        let m = RibbonContextualGroupModel::new(Some("tools".into()), Some("Tools".into()));
        m.set_activation(RibbonContextualActivation::SelectOnShow);
        let group = f.create_contextual_group(Some(m.clone()), Some(b.clone()));
        ribbon.contextual_groups().add(Some(group.clone()));
        let home = f.create_tab(regular, b.clone());
        let picture = f.create_tab(contextual, b.clone());
        ribbon.tabs().add(Some(home.clone()));
        ribbon.tabs().add(Some(picture.clone()));
        let window = crate::ribbon_tests::mount(&ribbon);
        assert!(ribbon.selected_tab().unwrap().ptr_eq(&home));
        assert!(!picture.is_available());
        m.set_is_visible(true);
        assert!(picture.is_available());
        assert!(ribbon.selected_tab().unwrap().ptr_eq(&picture));
        group.set_is_visible(false);
        assert!(!m.is_visible());
        assert!(!picture.is_available());
        assert!(ribbon.selected_tab().unwrap().ptr_eq(&home));
        b.dispose();
        let disposed_m = RibbonContextualGroupModel::new(Some("disposed".into()), None);
        let unbound = f.create_contextual_group(Some(disposed_m.clone()), Some(b.clone()));
        assert!(Ribbon::get_model(&unbound).is_some());
        assert!(unbound.header().is_none());
        for (model, bindings, expected) in [
            (None, None, "ArgumentNullException: model"),
            (
                Some(m as Rc<dyn RibbonContextualGroupModelImpl>),
                None,
                "ArgumentNullException: bindings",
            ),
        ] {
            let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                f.create_contextual_group(model, bindings)
            }))
            .unwrap_err();
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap();
            assert_eq!(message, expected);
        }
        f.set_ribbon(None);
        window.close();
        window.set_content(None);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    });
}

#[test]
fn native_weak_tag_backreference_preserves_association_without_a_cycle() {
    crate::group_tests::run(|| {
        let element = FerroObject::new();
        let weak_element = element.to_weak();
        let model = RibbonSliderModel::new(Some("weak-link".into()), Some("Weak link".into()));
        let weak_model = Rc::downgrade(&model);
        model.set_tag(Some(RibbonObject::reference(Rc::new(element.to_weak()))));
        RibbonElementFactory::set_model(&element, model.clone());
        assert!(
            model
                .tag()
                .unwrap()
                .downcast_ref::<ferroui_base::WeakRef<FerroObject>>()
                .unwrap()
                .upgrade()
                .unwrap()
                .ptr_eq(&element)
        );
        drop(model);
        assert!(
            weak_model.upgrade().is_some(),
            "the element owns its associated model"
        );
        assert!(Rc::ptr_eq(
            &RibbonElementFactory::get_model(&element).unwrap(),
            &(weak_model.upgrade().unwrap() as Rc<dyn RibbonNodeModelImpl>)
        ));
        drop(element);
        assert!(weak_element.upgrade().is_none());
        assert!(weak_model.upgrade().is_none());
    });
}
