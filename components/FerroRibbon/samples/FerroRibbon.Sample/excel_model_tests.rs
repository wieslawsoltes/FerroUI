//! Independent unchanged C# graph and command traces, not a second model builder.
use crate::excel_view_model::{ExcelParameterFormatter, ExcelViewModel};
use ferroribbon_core::{
    RibbonString, layout::RibbonSizeDefinition, model::*, theming::RibbonColor,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use serde_json::{Value, json};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};
fn fixture() -> Value {
    serde_json::from_str::<Value>(include_str!("../../tests/fixtures/excel-model.json")).unwrap()["data"]
        .clone()
}
fn txt(v: Option<RibbonString>) -> Option<String> {
    v.map(|s| s.to_utf8().unwrap())
}
fn clr(c: RibbonColor) -> u32 {
    ((c.a as u32) << 24) | ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32
}
fn size(v: RibbonSizeDefinition) -> Value {
    json!({"Large":v.large.raw(),"Medium":v.medium.raw(),"Small":v.small.raw()})
}
fn ico(v: Option<Rc<RibbonIcon>>) -> Value {
    v.map(|v|json!({"Kind":v.kind.raw(),"Value":txt(v.value.clone()),"FontFamily":txt(v.font_family.clone()),"Foreground":txt(v.foreground.clone()),"ViewBoxSize":v.view_box_size})).unwrap_or(Value::Null)
}
fn obj(v: Option<RibbonObject>) -> Value {
    v.map(|v| {
        if let Some(s) = v.downcast_ref::<RibbonString>() {
            json!(s.to_utf8().unwrap())
        } else if let Some(n) = v.downcast_ref::<f64>() {
            json!(n)
        } else {
            panic!("unexpected Excel graph payload")
        }
    })
    .unwrap_or(Value::Null)
}
fn cast<T: ?Sized + ferroribbon_core::model::RibbonModelView>(
    value: &Rc<dyn Any>,
) -> Option<Rc<T>> {
    try_cast_model::<T>(Some(value.clone())).unwrap()
}
macro_rules! nodes {
    ($values:expr) => {
        json!(
            ($values)
                .into_iter()
                .map(|n| n.map(|v| node(v)).unwrap_or(Value::Null))
                .collect::<Vec<_>>()
        )
    };
}
fn node(v: Rc<dyn RibbonNodeModelImpl>) -> Value {
    let id = txt(v.id());
    let id = id.map(|s| {
        if s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
            "<generated>".into()
        } else {
            s
        }
    });
    assert!(v.screen_tip().is_none());
    let mut out = json!({"$type":v.model_type_name(),"Id":id,"Label":txt(v.label()),"Icon":ico(v.icon()),"LargeIcon":ico(v.large_icon()),"IsEnabled":v.is_enabled(),"IsVisible":v.is_visible(),"KeyTip":txt(v.key_tip()),"ScreenTip":null,"Description":txt(v.description()),"Order":v.order(),"Tag":obj(v.tag()),"AutomationId":txt(v.automation_id()),"MergeAction":v.merge_action().raw(),"Keywords":v.keywords().iter().map(txt).collect::<Vec<_>>()});
    // Core's declared interface dispatch is exercised for both ancestors and leaves.
    let any: Rc<dyn Any> = v;
    macro_rules! extend {
        ($e:expr) => {
            out.as_object_mut()
                .unwrap()
                .extend($e.as_object().unwrap().clone());
        };
    }
    if let Some(v) = cast::<dyn RibbonItemModelImpl>(&any) {
        extend!(
            json!({"Size":v.size().raw(),"SizeDefinition":v.size_definition().map(size),"EffectiveSizeDefinition":size(v.effective_size_definition()),"SimplifiedVisibility":v.simplified_visibility().raw(),"ShowLabelInSimplified":v.show_label_in_simplified(),"CommandId":txt(v.command_id()),"Command":v.command().map(|_|true),"CommandParameter":obj(v.command_parameter()),"Shortcut":txt(v.shortcut()),"CanAddToQuickAccess":v.can_add_to_quick_access(),"ShowLabel":v.show_label()})
        );
    }
    if let Some(v) = cast::<dyn RibbonDropDownButtonModelImpl>(&any) {
        extend!(
            json!({"MenuItems":nodes!(v.menu_items().iter()),"DropDownContent":obj(v.drop_down_content())})
        );
    }
    if let Some(v) = cast::<dyn RibbonButtonGroupModelImpl>(&any) {
        extend!(json!({"Items":nodes!(v.items().iter())}));
    }
    if let Some(v) = any.downcast_ref::<RibbonBackstageItemModel>() {
        extend!(
            json!({"Command":v.command().map(|_|true),"CommandId":txt(v.command_id()),"CommandParameter":obj(v.command_parameter()),"Content":obj(v.content()),"Placement":v.placement().raw(),"ClosesBackstage":v.closes_backstage(),"HasSeparatorBefore":v.has_separator_before()})
        );
    }
    if let Some(v) = any.downcast_ref::<RibbonContextualGroupModel>() {
        extend!(json!({"Color":clr(v.color()),"Activation":v.activation().raw()}));
    }
    if let Some(v) = any.downcast_ref::<RibbonSplitButtonModel>() {
        extend!(
            json!({"IsCheckable":v.is_checkable(),"IsChecked":v.is_checked(),"FollowLastChoice":v.follow_last_choice()})
        );
    }
    if let Some(v) = any.downcast_ref::<RibbonToggleButtonModel>() {
        extend!(json!({"IsChecked":v.is_checked(),"GroupName":txt(v.group_name())}));
    }
    if let Some(v) = any.downcast_ref::<RibbonMenuItemModel>() {
        extend!(
            json!({"Command":v.command().map(|_|true),"CommandId":txt(v.command_id()),"CommandParameter":obj(v.command_parameter()),"IsCheckable":v.is_checkable(),"IsChecked":v.is_checked(),"GroupName":txt(v.group_name()),"Shortcut":txt(v.shortcut()),"Items":nodes!(v.items().iter())})
        );
    }
    if let Some(v) = any.downcast_ref::<RibbonTabModel>() {
        extend!(
            json!({"ContextualGroupId":txt(v.contextual_group_id()),"IsContextual":v.is_contextual(),"Groups":nodes!(v.groups().iter())})
        );
    }
    if let Some(v) = any.downcast_ref::<RibbonGroupModel>() {
        assert!(v.dialog_launcher_screen_tip().is_none());
        extend!(
            json!({"ItemsLayout":v.items_layout().raw(),"RowCount":v.row_count(),"DialogLauncherCommandId":txt(v.dialog_launcher_command_id()),"DialogLauncherCommand":v.dialog_launcher_command().map(|_|true),"DialogLauncherScreenTip":null,"ReductionOrder":v.reduction_order(),"CanCollapse":v.can_collapse(),"SimplifiedVisibility":v.simplified_visibility().raw(),"Items":nodes!(v.items().iter()),"SlideOutItems":nodes!(v.slide_out_items().iter())})
        );
    }
    if let Some(v) = any.downcast_ref::<RibbonComboBoxModel>() {
        extend!(
            json!({"Text":txt(v.text()),"Items":v.items().iter().map(obj).collect::<Vec<_>>(),"SelectedItem":obj(v.selected_item()),"IsEditable":v.is_editable(),"InputWidth":v.input_width(),"Placeholder":txt(v.placeholder()),"DisplayMemberPath":txt(v.display_member_path()),"PreviewFontFamily":v.preview_font_family(),"MaxDropDownHeight":v.max_drop_down_height()})
        );
    }
    if let Some(v) = any.downcast_ref::<RibbonColorPickerModel>() {
        assert_eq!(v.theme_colors().count(), 0);
        assert_eq!(v.standard_colors().count(), 0);
        assert_eq!(v.recent_colors().count(), 0);
        extend!(
            json!({"SelectedColor":v.selected_color().map(clr),"AutomaticColor":clr(v.automatic_color()),"ShowAutomatic":v.show_automatic(),"ShowNoColor":v.show_no_color(),"ShowMoreColors":v.show_more_colors(),"IsSplit":v.is_split(),"ThemeColors":[],"StandardColors":[],"RecentColors":[]})
        );
    }
    if let Some(v) = any.downcast_ref::<RibbonGalleryModel>() {
        extend!(
            json!({"SelectedItem":v.selected_item().map(|n|node(n)),"MinColumns":v.min_columns(),"MaxColumns":v.max_columns(),"DropDownColumns":v.drop_down_columns(),"ItemWidth":v.item_width(),"ItemHeight":v.item_height(),"IsFilterEnabled":v.is_filter_enabled(),"ShowLabels":v.show_labels(),"Rows":v.rows(),"Items":nodes!(v.items().iter()),"MenuItems":nodes!(v.menu_items().iter()),"PreviewCommand":v.preview_command().map(|_|true)})
        );
    }
    if let Some(v) = any.downcast_ref::<RibbonGalleryItemModel>() {
        extend!(
            json!({"Category":txt(v.category()),"Content":obj(v.content()),"Value":obj(v.value())})
        );
    }
    if let Some(v) = any.downcast_ref::<RibbonGridPickerModel>() {
        extend!(json!({"Rows":v.rows(),"Columns":v.columns()}));
    }
    if let Some(v) = any.downcast_ref::<RibbonCheckBoxModel>() {
        extend!(json!({"IsChecked":v.is_checked(),"IsThreeState":v.is_three_state()}));
    }
    if let Some(v) = any.downcast_ref::<RibbonSpinnerModel>() {
        extend!(
            json!({"Value":v.value(),"Minimum":v.minimum(),"Maximum":v.maximum(),"Increment":v.increment(),"InputWidth":v.input_width(),"Format":txt(v.format()),"Unit":txt(v.unit())})
        );
    }
    out
}
fn graph(m: Rc<RibbonModel>) -> Value {
    let b = m.backstage();
    json!({"ApplicationButtonLabel":txt(m.application_button_label()),"Backstage":{"Title":txt(b.title()),"IsOpen":b.is_open(),"SelectedItem":b.selected_item().map(|n|node(n)),"Items":nodes!(b.items().iter())},"CommandCatalog":m.command_catalog().map(|_|"catalog"),"ContextualGroups":nodes!(m.contextual_groups().iter()),"Density":m.density().raw(),"DisplayMode":m.display_mode().raw(),"IsApplicationButtonVisible":m.is_application_button_visible(),"IsQuickAccessVisible":m.is_quick_access_visible(),"MinimizeBehavior":m.minimize_behavior().raw(),"QuickAccessCandidates":nodes!(m.quick_access_candidates().iter()),"QuickAccessItems":nodes!(m.quick_access_items().iter()),"QuickAccessPosition":m.quick_access_position().raw(),"SelectedTab":m.selected_tab().map(|n|node(n)),"SelectedTabId":txt(m.selected_tab_id()),"ShowGroupCaptions":m.show_group_captions(),"ShowQuickAccessLabels":m.show_quick_access_labels(),"Tabs":nodes!(m.tabs().iter()),"TabStripItems":nodes!(m.tab_strip_items().iter()),"Title":txt(m.title()),"VisibilityMode":m.visibility_mode().raw(),"VisibleTabs":nodes!(m.clone().visible_tabs().iter())})
}
fn formatter() -> ExcelParameterFormatter {
    Rc::new(|p| {
        if let Some(s) = p.downcast_ref::<String>() {
            Ok(s.clone())
        } else if let Some(b) = p.downcast_ref::<bool>() {
            Ok(if *b { "True" } else { "False" }.into())
        } else if let Some(n) = p.downcast_ref::<f64>() {
            Ok(n.to_string())
        } else if let Some(s) = p.downcast_ref::<RibbonGridSize>() {
            Ok(s.to_string())
        } else {
            Err("unexpected reference case parameter".into())
        }
    })
}
fn normalize(v: &mut Value) {
    match v {
        Value::Number(n) => *v = json!(n.as_f64().unwrap()),
        Value::Array(a) => a.iter_mut().for_each(normalize),
        Value::Object(m) => m.values_mut().for_each(normalize),
        _ => {}
    }
}
#[test]
fn excel_entire_source_model_graph_catalog_and_reference_aliases_match() {
    let f = fixture();
    let vm = ExcelViewModel::new(formatter());
    let mut actual = graph(vm.ribbon());
    let mut expected = f["graph"].clone();
    normalize(&mut actual);
    normalize(&mut expected);
    assert_eq!(actual, expected);
    let catalog=json!(vm.commands().commands().iter().map(|d|json!({"id":d.id().to_utf8().unwrap(),"label":txt(d.label()),"shortcut":txt(d.shortcut()),"category":txt(d.category()),"enabled":d.is_enabled(),"check":d.is_checked(),"command":d.command().is_some()})).collect::<Vec<_>>());
    assert_eq!(catalog, f["catalog"]);
    let m = vm.ribbon();
    let alias = json!({"catalog":Rc::ptr_eq(&vm.commands(),&m.command_catalog().unwrap()),"bold":vm.bold().equals_node(m.find_item(Some("bold".into())).unwrap().as_ref()),"font":vm.font_family().equals_node(m.find_item(Some("fontFamily".into())).unwrap().as_ref()),"pasteCandidate":m.find_item(Some("paste".into())).unwrap().equals_node(m.quick_access_candidates().get(0).unwrap().as_ref()),"boldCandidate":vm.bold().equals_node(m.quick_access_candidates().get(1).unwrap().as_ref())});
    assert_eq!(alias, f["aliases"]);
}
#[test]
fn excel_whole_source_commands_notifications_and_reentrancy_match() {
    for c in fixture()["cases"].as_array().unwrap() {
        let vm = ExcelViewModel::new(formatter());
        let trace = Rc::new(RefCell::new(Vec::new()));
        let out = trace.clone();
        let weak = Rc::downgrade(&vm);
        let token = vm.property_changed().add(Rc::new(move |p| {
            let vm = weak.upgrade().unwrap();
            out.borrow_mut()
                .push(json!({"property":p,"status":vm.status(),"selected":vm.selected_cell()}));
        }));
        let kind = c["kind"].as_str().unwrap();
        let mut result = Value::Null;
        let error = Value::Null;
        let mut extra = None;
        let mut chart_extra = None;
        match kind {
            "catalog" => {
                let p: Option<Rc<dyn Any>> = match c["parameter"].as_str().unwrap() {
                    "null" => None,
                    "empty" => Some(Rc::new(String::new())),
                    "text" => Some(Rc::new("value".to_owned())),
                    "unicode" => Some(Rc::new("字e\u{301}😀".to_owned())),
                    "bool" => Some(Rc::new(true)),
                    "number" => Some(Rc::new(12.5_f64)),
                    "grid" => Some(Rc::new(RibbonGridSize::new(2, 3))),
                    _ => panic!("{c}"),
                };
                let id = RibbonString::from(c["id"].as_str().unwrap());
                if c["direct"] == true {
                    vm.commands()
                        .try_resolve(Some(&id))
                        .unwrap()
                        .execute(p.as_deref());
                    result = json!(true);
                } else {
                    result = json!(vm.commands().execute(Some(&id), p));
                }
            }
            "lookup" => {
                let id = c["id"].as_str().map(RibbonString::from);
                result = json!(vm.commands().execute(id.as_ref(), None));
            }
            "chart" => {
                vm.toggle_chart_command()
                    .execute(Some(&"ignored".to_owned()));
                vm.toggle_chart_command().execute(None);
            }
            "save" | "undo" | "redo" => vm
                .ribbon()
                .quick_access_items()
                .iter()
                .flatten()
                .find(|n| txt(n.id()).as_deref() == Some(kind))
                .unwrap()
                .command()
                .unwrap()
                .execute(Some(&"ignored".to_owned())),
            "backstage" => vm
                .ribbon()
                .backstage()
                .items()
                .iter()
                .flatten()
                .find(|n| txt(n.id()).as_deref() == Some("save"))
                .unwrap()
                .command()
                .unwrap()
                .execute(Some(&"ignored".to_owned())),
            "table" | "tableNull" | "tableWrong" => {
                let item = vm.ribbon().find_item(Some("table".into())).unwrap();
                let any: Rc<dyn Any> = item;
                let table = cast::<dyn RibbonDropDownButtonModelImpl>(&any).unwrap();
                let any: Rc<dyn Any> = table.menu_items().get(0).unwrap();
                let picker = cast::<dyn RibbonGridPickerModelImpl>(&any).unwrap();
                let grid = RibbonGridSize::new(2, 3);
                let wrong = "wrong".to_owned();
                picker.command().unwrap().execute(if kind == "table" {
                    Some(&grid)
                } else if kind == "tableWrong" {
                    Some(&wrong)
                } else {
                    None
                });
            }
            "status" => {
                vm.set_status(Some("Ready".into()));
                vm.set_status(Some(String::new()));
                vm.set_status(None);
                vm.set_status(Some("new".into()));
            }
            "selected" => {
                vm.set_selected_cell(Some("A1".into()));
                vm.set_selected_cell(Some("L24".into()));
                vm.set_selected_cell(None);
            }
            "statusReentry" => {
                let weak = Rc::downgrade(&vm);
                let entered = Cell::new(false);
                extra = Some(vm.property_changed().add(Rc::new(move |p| {
                    if p == "Status" && !entered.replace(true) {
                        weak.upgrade()
                            .unwrap()
                            .commands()
                            .execute(Some(&"copy".into()), Some(Rc::new("nested".to_owned())));
                    }
                })));
                vm.commands()
                    .execute(Some(&"paste".into()), Some(Rc::new("outer".to_owned())));
            }
            "chartReentry" => {
                let weak = Rc::downgrade(&vm);
                let entered = Cell::new(false);
                chart_extra = Some(vm.chart_tools().property_changed().add(Rc::new(move |p| {
                    if p == "IsVisible" && !entered.replace(true) {
                        weak.upgrade().unwrap().toggle_chart_command().execute(None);
                    }
                })));
                vm.toggle_chart_command().execute(None);
            }
            _ => panic!("unhandled {c}"),
        }
        vm.property_changed().remove(token);
        if let Some(t) = extra {
            vm.property_changed().remove(t);
        }
        if let Some(t) = chart_extra {
            vm.chart_tools().property_changed().remove(t);
        }
        let state = json!({"status":vm.status(),"selected":vm.selected_cell(),"chart":vm.chart_tools().is_visible(),"trace":trace.borrow().clone()});
        assert_eq!(state, c["state"], "{c}");
        if kind == "catalog" || kind == "lookup" {
            assert_eq!(result, c["result"], "{c}");
            if kind == "lookup" {
                assert_eq!(error, c["error"], "{c}");
            }
        }
    }
}
#[test]
fn excel_alignment_array_identity_and_replacement_preserve_group_snapshot() {
    let vm = ExcelViewModel::new(formatter());
    let a = vm.align();
    assert!(Rc::ptr_eq(&a, &vm.align()));
    let original = a.borrow()[1].clone();
    let replacement = RibbonToggleButtonModel::new(Some("replacement".into()), None);
    a.borrow_mut()[1] = replacement.clone();
    assert!(Rc::ptr_eq(&vm.align().borrow()[1], &replacement));
    assert!(
        original.equals_node(
            vm.ribbon()
                .find_item(Some("alignCenter".into()))
                .unwrap()
                .as_ref()
        )
    );
    assert!(
        !replacement.equals_node(
            vm.ribbon()
                .find_item(Some("alignCenter".into()))
                .unwrap()
                .as_ref()
        )
    );
}
