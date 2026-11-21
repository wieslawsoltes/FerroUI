//! All eighteen pinned shared FeatureTests.cs scenarios, actual native controls. Copyright
//! Wiesław Šoltés, MIT.
use crate::{
    controls::*,
    runtime_sample::{self as sample, Mounted, RuntimeHost},
};
use ferroribbon_core::{
    RibbonContextualActivation, RibbonDisplayMode, RibbonQuickAccessPosition, RibbonString,
    RibbonVisibilityMode,
    commands::RibbonRelayCommand,
    localization::{RibbonCulture, RibbonStrings},
    model::*,
    state::*,
    theming::RibbonThemePalette,
};
use ferroui_base::{
    Ref,
    data::core::{ValueType, ValueTypes},
    layout::Orientation,
    media::Color,
    metadata::{MarkupDelegate, from_markup_value},
};
use ferroui_controls::{Border, Control, StackPanel};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn invoke(control: Ref<Control>) {
    ValueTypes::try_cast(
        &Control::boxed(control),
        ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .unwrap()
    .downcast_ref::<Rc<dyn IRibbonItem>>()
    .unwrap()
    .invoke();
}
fn text(value: Option<RibbonString>) -> Option<String> {
    value.map(|v| v.to_utf8().unwrap())
}
#[test]
fn state_round_trips_including_customization() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon.select_tab_id("insert");
        m.ribbon.set_display_mode(RibbonDisplayMode::Simplified);
        m.ribbon
            .set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
        m.ribbon.add_to_quick_access_id(Some("bold".into()));
        let c = RibbonCustomization::new();
        c.hidden_group_ids().unwrap().add(Some("editing".into()));
        c.labels()
            .unwrap()
            .insert("home".into(), Some("Start".into()));
        let tab = RibbonCustomTab::new();
        tab.set_id(Some("custom.tab".into()));
        tab.set_label(Some("My Tab".into()));
        let group = RibbonCustomGroup::new();
        group.set_id(Some("custom.group".into()));
        group.set_label(Some("Mine".into()));
        for id in ["cut", "bold"] {
            group.item_ids().unwrap().add(Some(id.into()));
        }
        tab.groups().unwrap().add(Some(group));
        c.custom_tabs().unwrap().add(Some(tab));
        m.ribbon.apply_customization(Some(c)).unwrap();
        m.settle();
        let json = m.ribbon.save_state_to_json().unwrap();
        let restored = Mounted::new(1400.);
        assert!(restored.ribbon.load_state_from_json(Some(json)).unwrap());
        restored.settle();
        assert_eq!(
            restored.ribbon.selected_tab().unwrap().id().as_deref(),
            Some("insert")
        );
        assert_eq!(
            restored.ribbon.display_mode(),
            RibbonDisplayMode::Simplified
        );
        assert_eq!(
            restored.ribbon.quick_access_position(),
            RibbonQuickAccessPosition::BelowRibbon
        );
        assert_eq!(
            restored
                .ribbon
                .get_quick_access_item_ids()
                .iter()
                .map(text)
                .collect::<Vec<_>>(),
            [Some("save".into()), Some("bold".into())]
        );
        assert_eq!(
            restored
                .ribbon
                .find_tab("home")
                .unwrap()
                .header()
                .as_deref(),
            Some("Start")
        );
        assert!(!sample::find_group(&restored.ribbon, "editing").is_visible());
        assert_eq!(
            restored
                .ribbon
                .find_tab("custom.tab")
                .unwrap()
                .groups()
                .get(0)
                .unwrap()
                .items()
                .count(),
            2
        );
        restored.ribbon.reset_customization().unwrap();
        restored.settle();
        assert!(restored.ribbon.find_tab("custom.tab").is_none());
        assert_eq!(
            restored
                .ribbon
                .find_tab("home")
                .unwrap()
                .header()
                .as_deref(),
            Some("Home")
        );
    });
}
#[test]
fn tab_order_customization_reorders_headers() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let c = RibbonCustomization::new();
        for id in ["insert", "home"] {
            c.tab_order().unwrap().add(Some(id.into()));
        }
        m.ribbon.apply_customization(Some(c)).unwrap();
        m.settle();
        assert_eq!(
            m.ribbon.tab_headers()[0].tab().id().as_deref(),
            Some("insert")
        );
    });
}
#[test]
fn keytips_navigate_levels_and_invoke() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon.show_key_tips();
        m.settle();
        assert!(m.ribbon.is_key_tip_mode());
        assert!(m.ribbon.active_key_tips().count() >= 3);
        let tips = m.ribbon.current_key_tips();
        assert!(tips.iter().any(|k| k.tip == Some("F".into())));
        assert!(tips.iter().any(|k| k.tip == Some("1".into())));
        let home = tips
            .iter()
            .find(|k| {
                k.target
                    .as_ref()
                    .and_then(|t| t.cast::<RibbonTabHeader>())
                    .is_some_and(|h| h.tab().id().as_deref() == Some("home"))
            })
            .unwrap()
            .tip
            .unwrap();
        for ch in home.as_utf16() {
            m.ribbon.process_key_tip_input(*ch);
        }
        m.settle();
        assert!(m.ribbon.current_key_tips().iter().any(|k| {
            k.tip == Some("1".into())
                && k.target
                    .as_ref()
                    .and_then(|t| t.cast::<RibbonToggleButton>())
                    .is_some_and(|b| b.id().as_deref() == Some("bold"))
        }));
        m.ribbon.process_key_tip_input(u16::from(b'1'));
        m.settle();
        assert_eq!(
            sample::item::<RibbonToggleButton>(&m.ribbon, "bold").is_checked(),
            Some(true)
        );
        assert!(!m.ribbon.is_key_tip_mode());
    });
}
#[test]
fn escape_pops_levels() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon.show_key_tips();
        m.settle();
        m.ribbon.process_key_tip_input(u16::from(b'H'));
        m.settle();
        assert!(m.ribbon.pop_key_tip_level());
        assert!(!m.ribbon.pop_key_tip_level());
        assert!(!m.ribbon.is_key_tip_mode());
    });
}
#[test]
fn search_finds_and_executes_items() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let results = m.ribbon.search_default(Some(&"bold".into()));
        let entry = results.get(0).unwrap().entry().unwrap();
        assert_eq!(text(entry.label()).as_deref(), Some("Bold"));
        assert_eq!(text(entry.path()).as_deref(), Some("Home › Font"));
        assert!(m.ribbon.execute_search_entry(Some(entry)));
        assert_eq!(
            sample::item::<RibbonToggleButton>(&m.ribbon, "bold").is_checked(),
            Some(true)
        );
        assert!(
            m.ribbon
                .search_default(Some(&"".into()))
                .iter()
                .flatten()
                .any(|r| text(r.entry().unwrap().label()).as_deref() == Some("Bold"))
        );
        assert!(
            m.ribbon
                .search_default(Some(&"info".into()))
                .iter()
                .flatten()
                .any(|r| r
                    .entry()
                    .unwrap()
                    .target()
                    .is_some_and(|o| o.downcast_ref::<Ref<RibbonBackstageItem>>().is_some()))
        );
    });
}
#[test]
fn backstage_opens_selects_first_page_and_closes() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let opened = Rc::new(Cell::new(false));
        let value = opened.clone();
        let handler = MarkupDelegate::new(move |_| {
            value.set(true);
            None
        });
        m.ribbon.add_backstage_opened(Some(handler.clone()));
        m.ribbon.invoke_application_button();
        m.settle();
        assert!(m.ribbon.is_backstage_open() && opened.get());
        let backstage = m.ribbon.backstage().unwrap();
        assert_eq!(
            backstage.selected_item().unwrap().id().as_deref(),
            Some("info")
        );
        backstage.invoke(backstage.items().get(1));
        m.settle();
        assert!(!m.ribbon.is_backstage_open());
        m.ribbon.remove_backstage_opened(Some(handler));
    });
}
#[test]
fn combo_box_commit_text_selects_matching_item() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let size = sample::item::<RibbonFontSizeComboBox>(&m.ribbon, "fontSize");
        let committed = Rc::new(RefCell::new(None));
        let value = committed.clone();
        let handler = MarkupDelegate::new(move |args| {
            value.replace(Some(
                from_markup_value::<Rc<RibbonComboBoxCommittedEventArgs>>(&args[1]).unwrap(),
            ));
            None
        });
        size.add_committed(Some(handler.clone()));
        size.commit_text(Some("14".into()));
        assert_eq!(
            size.selected_item().unwrap().downcast_ref::<f64>(),
            Some(&14.)
        );
        size.commit_text(Some("13.5".into()));
        assert!(size.selected_item().is_none());
        assert_eq!(
            committed.borrow().as_ref().unwrap().text().as_deref(),
            Some("13.5")
        );
        size.remove_committed(Some(handler));
    });
}
#[test]
fn spinner_clamps_and_formats_units() {
    crate::group_tests::run(|| {
        let spinner = RibbonSpinner::new();
        spinner.set_label(Some("Left:".into()));
        spinner.set_minimum(0.);
        spinner.set_maximum(10.);
        spinner.set_unit(Some("cm".into()));
        spinner.set_format(Some("0.0".into()));
        spinner.set_numeric_value(2.);
        let _host = RuntimeHost::new(spinner.clone().upcast(), None);
        spinner.set_numeric_value(42.);
        assert_eq!(spinner.value(), 10.);
        assert_eq!(
            spinner.format_value(spinner.value()).replace(',', "."),
            "10.0 cm"
        );
    });
}
#[test]
fn gallery_pick_executes_command_and_adapts_columns() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let gallery = sample::item::<RibbonGallery>(&m.ribbon, "stylesGallery");
        let executed = Rc::new(RefCell::new(None));
        let value = executed.clone();
        gallery.set_command(Some(crate::commands::adapt_core_command(
            RibbonRelayCommand::new_with_parameter(
                move |p| {
                    value.replace(p.and_then(|p| p.downcast_ref::<String>()).cloned());
                },
                None,
            ),
        )));
        assert_eq!(gallery.columns(), 5);
        gallery.pick(crate::metadata::model_objects::to_native(gallery.items().get(2)).unwrap());
        assert_eq!(executed.borrow().as_deref(), Some("Heading 1"));
        assert_eq!(
            gallery
                .selected_item()
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("Heading 1")
        );
        let width = m
            .ribbon
            .selected_tab()
            .unwrap()
            .groups_panel()
            .extent_width();
        m.resize(width - 30.);
        assert_eq!(gallery.columns(), 2);
    });
}
#[test]
fn color_picker_reports_color() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let picker = sample::item::<RibbonColorPicker>(&m.ribbon, "fontColor");
        let picked = Rc::new(Cell::new(None));
        let value = picked.clone();
        let handler = MarkupDelegate::new(move |args| {
            value.set(from_markup_value::<Option<Color>>(&args[1]).unwrap());
            None
        });
        picker.add_color_selected(Some(handler.clone()));
        let green = Color::from_rgb(0, 128, 0);
        picker.palette().select(Some(green));
        assert_eq!(picked.get(), Some(green));
        assert_eq!(picker.selected_color(), Some(green));
        assert!(picker.palette().recent_colors().iter().any(|c| c == green));
        picker.remove_color_selected(Some(handler));
    });
}
fn create_model() -> (
    Rc<RibbonModel>,
    Rc<RibbonToggleButtonModel>,
    Rc<RibbonContextualGroupModel>,
) {
    let model = RibbonModel::new();
    let bold = RibbonToggleButtonModel::new(Some("bold".into()), Some("Bold".into()));
    bold.set_icon(Some(RibbonIcon::glyph(Some(RibbonIcons::Bold.into()))));
    bold.set_key_tip(Some("1".into()));
    let home = RibbonTabModel::new(Some("home".into()), Some("Home".into()));
    let font = RibbonGroupModel::new(Some("font".into()), Some("Font".into()));
    font.items().add(Some(bold.clone()));
    let cut = RibbonButtonModel::new(Some("cut".into()), Some("Cut".into()));
    cut.set_icon(Some(RibbonIcon::glyph(Some(RibbonIcons::Cut.into()))));
    font.items().add(Some(cut));
    home.groups().add(Some(font));
    let colors = RibbonGroupModel::new(Some("colors".into()), Some("Colors".into()));
    colors.items().add(Some(RibbonColorPickerModel::new(
        Some("fill".into()),
        Some("Fill".into()),
    )));
    let size = RibbonComboBoxModel::with_items(
        Some("size".into()),
        Some("Size".into()),
        ["8", "10", "12"].map(|v| Some(RibbonObject::value(v.to_owned()))),
    );
    size.set_text(Some("10".into()));
    size.set_is_editable(true);
    colors.items().add(Some(size));
    home.groups().add(Some(colors));
    model.tabs().add(Some(home));
    let view = RibbonTabModel::new(Some("view".into()), Some("View".into()));
    let show = RibbonGroupModel::new(Some("show".into()), Some("Show".into()));
    show.items().add(Some(RibbonCheckBoxModel::new(
        Some("ruler".into()),
        Some("Ruler".into()),
    )));
    view.groups().add(Some(show));
    model.tabs().add(Some(view));
    let chart = RibbonContextualGroupModel::new(Some("chart".into()), Some("Chart Tools".into()));
    chart.set_activation(RibbonContextualActivation::SelectOnShow);
    model.contextual_groups().add(Some(chart.clone()));
    let design = RibbonTabModel::new(Some("chartDesign".into()), Some("Chart Design".into()));
    design.set_contextual_group_id(Some("chart".into()));
    let group = RibbonGroupModel::new(Some("type".into()), Some("Type".into()));
    group.items().add(Some(RibbonButtonModel::new(
        Some("changeType".into()),
        Some("Change Type".into()),
    )));
    design.groups().add(Some(group));
    model.tabs().add(Some(design));
    (model, bold, chart)
}
fn mount_model(model: Rc<RibbonModel>) -> Mounted {
    let ribbon = Ribbon::new();
    ribbon.set_model(Some(model));
    Mounted::with_ribbon(ribbon, 1400.)
}
#[test]
fn model_generates_elements_and_syncs_two_way() {
    crate::group_tests::run(|| {
        let (model, bold, _) = create_model();
        let m = mount_model(model);
        assert_eq!(m.ribbon.visible_tabs().len(), 2);
        let element = sample::item::<RibbonToggleButton>(&m.ribbon, "bold");
        bold.set_is_checked(true);
        assert_eq!(element.is_checked(), Some(true));
        element.set_is_checked(Some(false));
        assert!(!bold.is_checked());
        bold.set_label(Some("Strong".into()));
        assert_eq!(element.label().as_deref(), Some("Strong"));
        bold.set_is_visible(false);
        assert!(!element.is_visible());
    });
}
#[test]
fn model_selection_display_mode_and_contextual_groups_sync() {
    crate::group_tests::run(|| {
        let (model, _, chart) = create_model();
        let m = mount_model(model.clone());
        model.set_selected_tab_id(Some("view".into()));
        assert_eq!(
            m.ribbon.selected_tab().unwrap().id().as_deref(),
            Some("view")
        );
        m.ribbon.select_tab_id("home");
        assert_eq!(text(model.selected_tab_id()).as_deref(), Some("home"));
        model.set_display_mode(RibbonDisplayMode::Simplified);
        assert_eq!(m.ribbon.display_mode(), RibbonDisplayMode::Simplified);
        m.ribbon.set_visibility_mode(RibbonVisibilityMode::TabsOnly);
        assert_eq!(model.visibility_mode(), RibbonVisibilityMode::TabsOnly);
        chart.set_is_visible(true);
        m.settle();
        assert_eq!(
            m.ribbon.selected_tab().unwrap().id().as_deref(),
            Some("chartDesign")
        );
    });
}
#[test]
fn model_collections_regenerate_and_qat_syncs_back() {
    crate::group_tests::run(|| {
        let (model, bold, _) = create_model();
        let m = mount_model(model.clone());
        model.tabs().add(Some(RibbonTabModel::new(
            Some("review".into()),
            Some("Review".into()),
        )));
        m.settle();
        assert!(m.ribbon.find_tab("review").is_some());
        let italic = RibbonButtonModel::new(Some("italic".into()), Some("Italic".into()));
        italic.set_icon(Some(RibbonIcon::glyph(Some(RibbonIcons::Italic.into()))));
        model
            .find_group(Some("font".into()))
            .unwrap()
            .items()
            .add(Some(italic));
        m.settle();
        assert!(m.ribbon.find_item(Some("italic".into())).is_some());
        m.ribbon.add_to_quick_access_id(Some("bold".into()));
        m.settle();
        let members = model
            .quick_access_items()
            .iter()
            .flatten()
            .collect::<Vec<_>>();
        assert!(
            members
                .iter()
                .any(|i| std::ptr::addr_eq(i.as_ref(), bold.as_ref())),
            "QAT must retain the original allocation: expected {:p}, actual {:?}",
            Rc::as_ptr(&bold),
            members
                .iter()
                .map(|i| (text(i.id()), Rc::as_ptr(i) as *const ()))
                .collect::<Vec<_>>()
        );
        model.quick_access_items().clear();
        m.settle();
        assert_eq!(m.ribbon.get_quick_access_item_ids().count(), 0);
    });
}
#[test]
fn model_inputs_sync() {
    crate::group_tests::run(|| {
        let (model, _, _) = create_model();
        let m = mount_model(model.clone());
        let size = model.find_item(Some("size".into())).unwrap();
        let size = (size as Rc<dyn std::any::Any>)
            .downcast::<RibbonComboBoxModel>()
            .ok()
            .unwrap();
        sample::item::<RibbonComboBox>(&m.ribbon, "size").commit_text(Some("12".into()));
        assert_eq!(
            size.selected_item()
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("12")
        );
        let fill = model.find_item(Some("fill".into())).unwrap();
        let fill = (fill as Rc<dyn std::any::Any>)
            .downcast::<RibbonColorPickerModel>()
            .ok()
            .unwrap();
        sample::item::<RibbonColorPicker>(&m.ribbon, "fill")
            .palette()
            .select(Some(Color::from_rgb(0, 0, 255)));
        assert_eq!(
            fill.selected_color(),
            Some(ferroribbon_core::theming::RibbonColor::parse(Some(&"#0000FF".into())).unwrap())
        );
        let ruler = model.find_item(Some("ruler".into())).unwrap();
        let ruler = (ruler as Rc<dyn std::any::Any>)
            .downcast::<RibbonCheckBoxModel>()
            .ok()
            .unwrap();
        m.ribbon.select_tab_id("view");
        m.settle();
        invoke(sample::item::<RibbonCheckBox>(&m.ribbon, "ruler").upcast());
        assert_eq!(ruler.is_checked(), Some(true));
    });
}
#[test]
fn palette_changes_accent_brush_in_place() {
    crate::group_tests::run(|| {
        let _m = Mounted::new(1400.);
        crate::theming::RibbonTheme::reset_test_cache();
        let brush = crate::theming::RibbonTheme::resources()
            .get_brush("RibbonAccentBrush", None)
            .unwrap();
        for palette in [RibbonThemePalette::excel(), RibbonThemePalette::word()] {
            crate::theming::RibbonTheme::apply_palette(Some(palette.clone())).unwrap();
            assert_eq!(brush.color(), crate::theming::to_color(palette.accent));
        }
        crate::theming::RibbonTheme::reset_test_cache();
    });
}
#[test]
fn localized_strings_are_used_by_menus() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        struct Restore(std::sync::Arc<RibbonStrings>);
        impl Drop for Restore {
            fn drop(&mut self) {
                RibbonStrings::set_current(Some(self.0.clone())).unwrap();
            }
        }
        let _restore = Restore(RibbonStrings::current());
        RibbonStrings::set_current(Some(
            RibbonStrings::for_culture(Some(RibbonCulture::new("de-DE".into(), "de".into())))
                .unwrap(),
        ))
        .unwrap();
        let menu = m.ribbon.build_display_options_menu();
        assert!(menu.items().iter().flatten().any(|v| {
            ValueTypes::as_object(&*v)
                .and_then(|o| o.cast::<crate::compat::MenuFlyoutItem>())
                .is_some_and(|i| i.text().as_deref() == Some("Vollbildmodus"))
        }));
    });
}
#[test]
fn toolbar_overflows_and_vertical_columns() {
    crate::group_tests::run(|| {
        let toolbar = RibbonToolBar::new();
        for i in 0..20 {
            let b = RibbonButton::new();
            b.set_id(Some(format!("b{i}")));
            b.set_label(Some(format!("Button {i}")));
            b.set_icon(Some(Rc::new("".to_owned())));
            toolbar.items().add(Some(b.upcast()));
        }
        let _host = RuntimeHost::new(toolbar.clone().upcast(), Some(240.));
        assert!(toolbar.overflow_items().iter().next().is_some());
        let vertical = RibbonToolBar::new();
        vertical.set_orientation(Orientation::Vertical.into());
        vertical.set_columns(2);
        for i in 0..6 {
            let b = RibbonToggleButton::new();
            b.set_label(Some(format!("Tool {i}")));
            b.set_icon(Some(Rc::new("".to_owned())));
            b.set_group_name(Some("tools".into()));
            vertical.items().add(Some(b.upcast()));
        }
        let _vertical_host = RuntimeHost::new(vertical.clone().upcast(), None);
        assert!(vertical.overflow_items().iter().next().is_none());
        assert!(vertical.bounds().width > vertical.bounds().height / 3.);
    });
}
#[test]
fn title_bar_hosts_quick_access_toolbar() {
    crate::group_tests::run(|| {
        let ribbon = sample::create();
        let title = RibbonTitleBar::new();
        title.set_title(Some("Doc".into()));
        let panel = StackPanel::new();
        panel.children().add(title.clone());
        panel.children().add(ribbon.clone());
        let host = RuntimeHost::new(panel.upcast(), Some(1400.));
        title.set_ribbon(Some(ribbon.clone()));
        host.settle();
        assert!(ribbon.is_quick_access_hosted_externally());
        let parent = ribbon
            .quick_access_tool_bar()
            .unwrap()
            .visual_parent()
            .unwrap()
            .cast::<Border>()
            .unwrap();
        assert_eq!(parent.name().as_deref(), Some("PART_QuickAccessHost"));
    });
}
