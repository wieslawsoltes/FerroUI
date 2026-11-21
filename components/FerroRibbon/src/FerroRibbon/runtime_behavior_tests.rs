//! Nine unchanged-source RibbonBehaviorTests scenarios with complete SampleRibbon. Copyright
//! Wiesław Šoltés, MIT.
use crate::{
    controls::*,
    runtime_sample::{self as sample, Mounted},
};
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{RibbonString, state::*};
use ferroui_base::{
    data::{core::Value, model::ModelTypes},
    input::KeyboardNavigation,
};
use ferroui_controls::{
    Control, ItemsSource,
    automation::{
        AutomationProperties,
        peers::ControlAutomationPeer,
        provider::{ISelectionItemProvider, ISelectionProvider},
    },
};
use std::{cell::Cell, rc::Rc};

fn type_keys(ribbon: &Ribbon, keys: &RibbonString) {
    for ch in keys.as_utf16() {
        ribbon.process_key_tip_input(*ch);
    }
}
#[test]
fn hidden_tabs_cannot_be_selected_by_id() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon.set_selected_tab_id(Some("tableDesign".into()));
        m.settle();
        assert_eq!(
            m.ribbon.selected_tab().unwrap().id().as_deref(),
            Some("home")
        );
        assert_eq!(m.ribbon.selected_tab_id().as_deref(), Some("home"));
    });
}
#[test]
fn split_button_keytip_continues_into_its_menu() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon.show_key_tips();
        m.settle();
        let home = m
            .ribbon
            .current_key_tips()
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
        type_keys(&m.ribbon, &home);
        m.settle();
        let paste = m
            .ribbon
            .current_key_tips()
            .iter()
            .find(|k| {
                k.target
                    .as_ref()
                    .and_then(|t| t.cast::<RibbonSplitButton>())
                    .is_some_and(|b| b.id().as_deref() == Some("paste"))
            })
            .unwrap()
            .tip
            .unwrap();
        type_keys(&m.ribbon, &paste);
        m.settle();
        assert!(m.ribbon.is_key_tip_mode());
        assert!(m.ribbon.current_key_tips().iter().any(|k| {
            k.target
                .as_ref()
                .and_then(|t| t.cast::<crate::compat::MenuFlyoutItem>())
                .is_some_and(|i| i.text().as_deref() == Some("Keep Source"))
        }));
        m.ribbon.cancel_key_tips();
        m.settle();
        assert!(!m.ribbon.is_key_tip_mode());
        assert!(!sample::item::<RibbonSplitButton>(&m.ribbon, "paste").is_drop_down_open());
    });
}
#[test]
fn backstage_requested_before_load_opens_once_loaded() {
    crate::group_tests::run(|| {
        let ribbon = sample::create();
        ribbon.set_is_backstage_open(true);
        let m = Mounted::with_ribbon(ribbon, 1400.);
        assert!(m.ribbon.is_backstage_open());
        m.ribbon.set_is_backstage_open(false);
        m.settle();
        let empty = sample::create();
        empty.set_backstage(None);
        let empty = Mounted::with_ribbon(empty, 1400.);
        empty.ribbon.set_is_backstage_open(true);
        empty.settle();
        assert!(!empty.ribbon.is_backstage_open());
    });
}
#[test]
fn customization_keeps_selected_custom_tab() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let customization = RibbonCustomization::new();
        let tab = RibbonCustomTab::new();
        tab.set_id(Some("custom.tab".into()));
        tab.set_label(Some("Mine".into()));
        let group = RibbonCustomGroup::new();
        group.set_id(Some("custom.group".into()));
        group.set_label(Some("G".into()));
        group.item_ids().unwrap().add(Some("cut".into()));
        tab.groups().unwrap().add(Some(group));
        customization.custom_tabs().unwrap().add(Some(tab));
        m.ribbon
            .apply_customization(Some(customization.clone()))
            .unwrap();
        m.settle();
        assert!(m.ribbon.select_tab_id("custom.tab"));
        customization
            .hidden_group_ids()
            .unwrap()
            .add(Some("editing".into()));
        m.ribbon.apply_customization(Some(customization)).unwrap();
        m.settle();
        assert_eq!(
            m.ribbon.selected_tab().unwrap().id().as_deref(),
            Some("custom.tab")
        );
    });
}
#[test]
fn tab_headers_expose_selection_patterns() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let header = m
            .ribbon
            .tab_headers()
            .iter()
            .find(|h| h.tab().id().as_deref() == Some("insert"))
            .unwrap()
            .clone();
        let peer = ControlAutomationPeer::create_peer_for_element(&header);
        let provider = peer.get_provider::<dyn ISelectionItemProvider>().unwrap();
        provider.select().unwrap();
        m.settle();
        assert_eq!(
            m.ribbon.selected_tab().unwrap().id().as_deref(),
            Some("insert")
        );
        assert!(KeyboardNavigation::get_is_tab_stop(&header));
        let home = m
            .ribbon
            .tab_headers()
            .iter()
            .find(|h| h.tab().id().as_deref() == Some("home"))
            .unwrap()
            .clone();
        assert!(!KeyboardNavigation::get_is_tab_stop(&home));
        assert!(
            ControlAutomationPeer::create_peer_for_element(&m.ribbon)
                .get_provider::<dyn ISelectionProvider>()
                .is_some()
        );
    });
}
#[test]
fn group_items_change_in_place() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let group = sample::find_group(&m.ribbon, "editing");
        let find = sample::item::<RibbonButton>(&m.ribbon, "find");
        let unloaded = Rc::new(Cell::new(0));
        let count = unloaded.clone();
        let subscription = find.unloaded(move |_, _| count.set(count.get() + 1));
        let select = RibbonButton::new();
        select.set_id(Some("select".into()));
        select.set_label(Some("Select".into()));
        select.set_icon(Some(Rc::new("".to_owned())));
        group.items().add(Some(select.upcast()));
        group.items().move_item(2, 0);
        m.settle();
        assert_eq!(unloaded.get(), 0);
        assert!(m.ribbon.find_item(Some("select".into())).is_some());
        find.remove_handler(Control::unloaded_event(), subscription);
    });
}
#[test]
fn group_order_customization_reorders_groups() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let c = RibbonCustomization::new();
        c.group_order().unwrap().insert(
            "home".into(),
            Some(RibbonItems::from_values(vec![
                Some("editing".into()),
                Some("clipboard".into()),
            ])),
        );
        m.ribbon.apply_customization(Some(c)).unwrap();
        m.settle();
        let home = m.ribbon.find_tab("home").unwrap();
        assert_eq!(
            home.display_groups()
                .iter()
                .map(|g| g.unwrap().id().unwrap())
                .collect::<Vec<_>>(),
            ["editing", "clipboard", "font", "paragraph", "styles"]
        );
        assert_eq!(
            home.groups().get(0).unwrap().id().as_deref(),
            Some("clipboard")
        );
        let json = m.ribbon.save_state_to_json().unwrap();
        m.ribbon.reset_customization().unwrap();
        m.settle();
        assert_eq!(
            home.display_groups().get(0).unwrap().id().as_deref(),
            Some("clipboard")
        );
        m.ribbon.load_state_from_json(Some(json)).unwrap();
        m.settle();
        assert_eq!(
            home.display_groups().get(0).unwrap().id().as_deref(),
            Some("editing")
        );
    });
}
#[test]
fn contextual_group_keytip_prefix_and_enabled_state() {
    crate::group_tests::run(|| {
        let ribbon = sample::create();
        let tools = ribbon.contextual_groups().get(0).unwrap();
        tools.set_key_tip(Some("J".into()));
        tools.set_is_visible(true);
        let m = Mounted::with_ribbon(ribbon, 1400.);
        m.ribbon.show_key_tips();
        m.settle();
        assert!(m.ribbon.current_key_tips().iter().any(|k| {
            k.tip == Some("JT".into())
                && k.target
                    .as_ref()
                    .and_then(|t| t.cast::<RibbonTabHeader>())
                    .is_some_and(|h| h.tab().id().as_deref() == Some("tableDesign"))
        }));
        m.ribbon.cancel_key_tips();
        tools.set_is_enabled(false);
        m.settle();
        assert!(
            !m.ribbon
                .find_tab("tableDesign")
                .unwrap()
                .header_element()
                .unwrap()
                .is_enabled()
        );
    });
}
#[derive(Clone, PartialEq, Eq)]
struct Person {
    name: String,
}
#[test]
fn tab_icon_group_screentip_and_combo_display_member() {
    crate::group_tests::run(|| {
        // The source record's only consumed contract is its readable Name.
        ModelTypes::register::<Person>(|b| {
            b.read_only::<Value<String>>("Name", |p| p.name.clone())
        });
        let ribbon = sample::create();
        ribbon
            .find_tab("insert")
            .unwrap()
            .set_icon(Some(Rc::new("\u{E710}".to_owned())));
        let editing = sample::find_group(&ribbon, "editing");
        editing.set_screen_tip(Some(Rc::new("Find and replace text".to_owned())));
        let combo = RibbonComboBox::new();
        combo.set_id(Some("people".into()));
        combo.set_display_member_path(Some("Name".into()));
        combo.set_items_source(Some(Rc::new(ItemsSource::from_values([
            Person { name: "Ada".into() },
            Person {
                name: "Linus".into(),
            },
        ]))));
        editing.items().add(Some(combo.clone().upcast()));
        let m = Mounted::with_ribbon(ribbon, 1400.);
        assert_eq!(
            AutomationProperties::get_help_text(&editing).as_deref(),
            Some("Find and replace text")
        );
        assert_eq!(
            combo
                .get_item_text(Some(Rc::new(Person { name: "Ada".into() })))
                .as_deref(),
            Some("Ada")
        );
        combo.set_selected_item(combo.effective_items().get(1));
        m.settle();
        assert_eq!(combo.text().as_deref(), Some("Linus"));
    });
}
