//! Actual native dialog scenarios: source workflows, not mocked view actions.
use super::*;
use crate::controls::{
    IRibbonItem, RibbonButton, RibbonButtonGroup, RibbonButtonImpl, RibbonColorDialogError,
    RibbonGroup, RibbonQuickAccessToolBar, RibbonTab,
};
use ferroui_base::{
    data::core::ValueTypes, interactivity::RoutedEventArgs, media::Color, threading::Dispatcher,
};
use ferroui_controls::{ButtonImpl, ScrollViewer, Window, primitives::PopupOverlayLayer};
use futures_util::FutureExt;
fn mount() -> (Ref<Ribbon>, Ref<Window>) {
    let ribbon = Ribbon::new();
    let tab = RibbonTab::new();
    tab.set_id(Some("home".into()));
    tab.set_header(Some("Home".into()));
    let group = RibbonGroup::new();
    group.set_id(Some("clip".into()));
    group.set_header(Some("Clip\nboard".into()));
    let save = RibbonButton::new();
    save.set_id(Some("save".into()));
    save.set_label(Some("Save\nAll".into()));
    group.items().add(Some(save.clone().upcast()));
    let container = RibbonButtonGroup::new();
    let open = RibbonButton::new();
    open.set_id(Some("open".into()));
    open.set_label(Some("Alpha".into()));
    container.items().add(Some(open.upcast()));
    group.items().add(Some(container.upcast()));
    let duplicate = RibbonButton::new();
    duplicate.set_id(Some("save".into()));
    duplicate.set_label(Some("Duplicate".into()));
    group.items().add(Some(duplicate.upcast()));
    let linked = RibbonButton::new();
    linked.set_id(Some("copy".into()));
    linked.set_label(Some("Linked".into()));
    RibbonItemHelper::set_source_item(&linked, save.clone().upcast::<Control>());
    group.items().add(Some(linked.upcast()));
    let excluded = RibbonButton::new();
    excluded.set_id(Some("excluded".into()));
    excluded.set_label(Some("Excluded".into()));
    excluded.set_can_add_to_quick_access(false);
    group.items().add(Some(excluded.upcast()));
    tab.groups().add(Some(group));
    ribbon.tabs().add(Some(tab));
    ribbon.set_quick_access_tool_bar(Some(RibbonQuickAccessToolBar::new()));
    assert!(ribbon.add_to_quick_access(&save));
    let panel = StackPanel::new();
    let outside = Button::new();
    outside.set_content(Some(Rc::new("Outside".to_owned())));
    panel.children().add(outside.clone());
    panel.children().add(ribbon.clone());
    let window = Window::new();
    window.set_width(1000.);
    window.set_height(800.);
    window.set_content(Some(Control::boxed(panel)));
    window.show();
    window.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(outside.focus());
    (ribbon, window)
}
fn row_buttons(list: &RibbonCustomizationList) -> Vec<Ref<Button>> {
    let scroll = list.root().child().unwrap().cast::<ScrollViewer>().unwrap();
    let panel = ValueTypes::as_object(&*scroll.content().unwrap())
        .unwrap()
        .cast::<StackPanel>()
        .unwrap();
    panel
        .children()
        .snapshot()
        .iter()
        .map(|c| c.cast::<Button>().unwrap())
        .collect()
}
// Settle native UI work like upstream runtime Click/WaitForIdle scenarios.
fn invoke(button: &Button) {
    button.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    Dispatcher::ui_thread().run_jobs(None);
}
fn text_buttons(root: &Control, label: &str) -> Vec<Ref<Button>> {
    root.get_self_and_visual_descendants()
        .filter_map(|c| c.cast::<Button>())
        .filter(|c| {
            c.content()
                .as_ref()
                .and_then(|v| v.downcast_ref::<String>())
                .is_some_and(|s| s == label)
        })
        .collect()
}
fn label(value: Option<RibbonString>) -> String {
    value.unwrap().to_utf8().unwrap()
}
fn active_host(window: &Window) -> Ref<RibbonDialogHost> {
    PopupOverlayLayer::get_popup_overlay_layer(window)
        .unwrap()
        .children()
        .snapshot()
        .iter()
        .filter_map(|c| c.cast::<RibbonDialogHost>())
        .next_back()
        .unwrap()
}
fn edit_box(host: &RibbonDialogHost) -> Ref<TextBox> {
    ValueTypes::as_object(&*host.content().unwrap())
        .unwrap()
        .cast::<TextBox>()
        .unwrap()
}
#[test]
fn customize_dialog_native_pages_catalog_working_state_cancel_and_primary() {
    crate::group_tests::run(|| {
        let (ribbon, window) = mount();
        let original = ribbon.customization();
        let dialog =
            RibbonCustomizeDialog::new(ribbon.clone(), RibbonCustomizePage::Ribbon).unwrap();
        assert!(!Rc::ptr_eq(
            &original,
            &dialog.working_state().customization().unwrap()
        ));
        assert_eq!(
            dialog
                .editor
                .borrow()
                .commands()
                .iter()
                .map(|c| label(c.label.clone()))
                .collect::<Vec<_>>(),
            ["Alpha", "Save All"]
        );
        assert_eq!(
            label(dialog.editor.borrow().commands()[1].path.clone()),
            "Home › Clip\nboard"
        );
        let calls = Rc::new(RefCell::new(Vec::new()));
        let observed = calls.clone();
        let subscription = dialog.closed(move |primary| observed.borrow_mut().push(primary));
        dialog.show(window.clone().upcast()).unwrap();
        assert!(dialog.is_open());
        assert_eq!(dialog.body.width(), 860.);
        assert!(dialog.ribbon_toggle.is_focused());
        invoke(&dialog.qat_toggle);
        assert_eq!(dialog.page(), RibbonCustomizePage::QuickAccessToolbar);
        assert!(!dialog.ribbon_page.is_visible());
        assert!(dialog.qat_page.is_visible());
        invoke(&dialog.ribbon_toggle);
        dialog.structure_list.select(Some(&"home".into()));
        let button = text_buttons(
            &dialog,
            label(RibbonStrings::current().new_group()).as_str(),
        )
        .pop()
        .unwrap();
        invoke(&button);
        let row = dialog.structure_list.selected().unwrap();
        let id = row.id.clone().unwrap();
        let spelling = id.to_utf8().unwrap();
        let suffix = spelling.strip_prefix("custom.group.").unwrap();
        assert_eq!(suffix.len(), 8);
        assert!(
            suffix
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
        dialog.command_list.select(Some(&"save".into()));
        dialog.action(Action::Add).unwrap();
        let old = row_buttons(&dialog.structure_list);
        dialog.action(Action::Add).unwrap();
        assert!(
            old.iter()
                .zip(row_buttons(&dialog.structure_list))
                .all(|(a, b)| a.ptr_eq(&b)),
            "duplicate command addition must preserve actual native row controls"
        );
        assert!(original.custom_groups().unwrap().is_empty());
        dialog.close(false).unwrap();
        assert!(!dialog.is_open());
        assert!(original.custom_groups().unwrap().is_empty());
        dialog.show(window.clone().upcast()).unwrap();
        dialog.close(true).unwrap();
        assert!(
            ribbon
                .customization()
                .custom_groups()
                .unwrap()
                .iter()
                .any(|g| g.unwrap().id() == Some(id.clone()))
        );
        assert_eq!(*calls.borrow(), [false, true]);
        subscription.dispose();
        window.close();
    });
}
#[test]
fn customize_dialog_native_qat_filter_size_shared_modal_and_drop() {
    crate::group_tests::run(|| {
        let (ribbon, window) = mount();
        assert_eq!(
            ribbon
                .default_quick_access_item_ids()
                .unwrap()
                .iter()
                .map(label)
                .collect::<Vec<_>>(),
            ["save"]
        );
        assert!(ribbon.add_to_quick_access_id(Some("open".into())));
        let dialog =
            RibbonCustomizeDialog::new(ribbon.clone(), RibbonCustomizePage::QuickAccessToolbar)
                .unwrap();
        dialog.show(window.clone().upcast()).unwrap();
        let rejected = crate::dialog_future_test_support::show_color(
            Some(window.clone().upcast()),
            Color::from_rgb(1, 2, 3),
            None,
        );
        assert_eq!(
            rejected.clone().now_or_never(),
            Some(Err(RibbonColorDialogError::AlreadyOpen))
        );
        dialog.search.set_text(Some("SAVE"));
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(row_buttons(&dialog.command_list).len(), 1);
        assert_eq!(
            row_buttons(&dialog.qat_commands).len(),
            2,
            "QAT command page has its source independent unfiltered list"
        );
        window.set_width(700.);
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(dialog.body.width(), 604.);
        window.set_width(300.);
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(dialog.body.width(), 320.);
        dialog.qat_commands.select(Some(&"open".into()));
        dialog.action(Action::QatAdd).unwrap();
        dialog.qat_list.select(Some(&"open".into()));
        dialog.action(Action::QatUp).unwrap();
        assert_eq!(
            dialog
                .working_state()
                .quick_access_item_ids()
                .unwrap()
                .iter()
                .map(label)
                .collect::<Vec<_>>(),
            ["open", "save"]
        );
        dialog.qat_below.set_is_checked(Some(true));
        assert_eq!(
            dialog.working_state().quick_access_position(),
            RibbonQuickAccessPosition::BelowRibbon
        );
        dialog.action(Action::QatReset).unwrap();
        assert_eq!(
            dialog
                .working_state()
                .quick_access_item_ids()
                .unwrap()
                .iter()
                .map(label)
                .collect::<Vec<_>>(),
            ["save"]
        );
        assert!(dialog.working_state().is_quick_access_visible());
        dialog.close(false).unwrap();
        let color = crate::dialog_future_test_support::show_color(
            Some(window.clone().upcast()),
            Color::from_rgb(1, 2, 3),
            None,
        );
        assert!(color.clone().now_or_never().is_none());
        assert_eq!(
            dialog.show(window.clone().upcast()),
            Err(RibbonCustomizeDialogError::AlreadyOpen)
        );
        window.close();
        assert!(
            color.clone().now_or_never().is_none(),
            "owner dismissal queues the source result after Closed"
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(color.clone().now_or_never(), Some(Ok(None)));
        let retained = dialog.body.clone();
        let weak = dialog.downgrade();
        drop(dialog);
        assert!(
            weak.upgrade().is_none(),
            "retained native body controls do not retain the dialog controller"
        );
        drop(retained);
    });
}
#[test]
fn customize_dialog_native_nested_rename_keeps_parent_controls_and_working_state() {
    crate::group_tests::run(|| {
        let (ribbon, window) = mount();
        let dialog = RibbonCustomizeDialog::new(ribbon, RibbonCustomizePage::Ribbon).unwrap();
        dialog.show(window.clone().upcast()).unwrap();
        dialog.structure_list.select(Some(&"clip".into()));
        let body = dialog.body.clone();
        let state = dialog.working_state();
        dialog.rename().unwrap();
        assert!(!dialog.is_open());
        let host = active_host(&window);
        let text = edit_box(&host);
        assert_eq!(text.text().as_deref(), Some("Clip\nboard"));
        dialog.structure_list.select(Some(&"home".into()));
        text.set_text(Some("Renamed"));
        invoke(&host.primary_button().unwrap());
        assert!(dialog.is_open());
        assert!(dialog.body.ptr_eq(&body));
        assert!(Rc::ptr_eq(&dialog.working_state(), &state));
        assert_eq!(
            state
                .customization()
                .unwrap()
                .labels()
                .unwrap()
                .get(&"clip".into()),
            Some(Some("Renamed".into()))
        );
        assert_eq!(
            dialog.structure_list.selected().unwrap().id,
            Some("home".into())
        );
        dialog.close(false).unwrap();
        window.close();
    });
}
#[test]
fn customize_dialog_native_nested_import_invalid_message_and_partial_import_fields() {
    crate::group_tests::run(|| {
        let (ribbon, window) = mount();
        let dialog = RibbonCustomizeDialog::new(ribbon, RibbonCustomizePage::Ribbon).unwrap();
        dialog.show(window.clone().upcast()).unwrap();
        let state = dialog.working_state();
        state.set_is_quick_access_visible(false);
        let previous = RibbonCustomizeDialog::set_invalid_import_message(Some(
            "Invalid customization data".into(),
        ));
        dialog.import_export().unwrap();
        let host = active_host(&window);
        assert_eq!(
            host.default_button(),
            crate::compat::ContentDialogButton::None
        );
        edit_box(&host).set_text(Some("{"));
        invoke(&host.primary_button().unwrap());
        let error = active_host(&window);
        let error_text = ValueTypes::as_object(&*error.content().unwrap())
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        assert_eq!(
            error_text.text().as_deref(),
            Some("Invalid customization data")
        );
        assert!(!dialog.is_open());
        invoke(&error.close_button().unwrap());
        assert!(dialog.is_open());
        dialog.import_export().unwrap();
        let host = active_host(&window);
        edit_box(&host).set_text(Some(r#"{"quickAccessItemIds":["open"],"quickAccessPosition":"BelowRibbon","isQuickAccessVisible":true,"customization":{"labels":{"home":"Imported"}}}"#));
        invoke(&host.primary_button().unwrap());
        assert!(dialog.is_open());
        assert!(Rc::ptr_eq(&dialog.working_state(), &state));
        assert!(
            !state.is_quick_access_visible(),
            "source import does not replace this field"
        );
        assert_eq!(
            state.quick_access_position(),
            RibbonQuickAccessPosition::BelowRibbon
        );
        assert_eq!(
            state
                .quick_access_item_ids()
                .unwrap()
                .iter()
                .map(label)
                .collect::<Vec<_>>(),
            ["open"]
        );
        assert_eq!(
            state
                .customization()
                .unwrap()
                .labels()
                .unwrap()
                .get(&"home".into()),
            Some(Some("Imported".into()))
        );
        RibbonCustomizeDialog::set_invalid_import_message(previous);
        dialog.close(false).unwrap();
        window.close();
    });
}

#[test]
fn customize_dialog_nested_title_uses_locale_after_parent_closed() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::customize_dialog_nested_title_uses_locale_after_parent_closed"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        let original = RibbonStrings::current();
        struct Restore(std::sync::Arc<RibbonStrings>);
        impl Drop for Restore {
            fn drop(&mut self) {
                RibbonStrings::set_current(Some(self.0.clone())).unwrap();
            }
        }
        let _restore = Restore(original.clone());
        let changed = RibbonStrings::for_culture(Some(original.culture())).unwrap();
        changed
            .override_string(Some("Rename".into()), Some("After close...".into()))
            .unwrap();
        changed
            .override_string(
                Some("ImportExport".into()),
                Some("Import after close".into()),
            )
            .unwrap();
        let (ribbon, window) = mount();
        let dialog = RibbonCustomizeDialog::new(ribbon, RibbonCustomizePage::Ribbon).unwrap();
        dialog.show(window.clone().upcast()).unwrap();
        let subscription = dialog.closed(move |_| {
            RibbonStrings::set_current(Some(changed.clone())).unwrap();
        });
        dialog.structure_list.select(Some(&"clip".into()));
        dialog.rename().unwrap();
        let host = active_host(&window);
        assert_eq!(
            host.title().unwrap().downcast_ref::<String>().unwrap(),
            "After close"
        );
        invoke(&host.close_button().unwrap());
        RibbonStrings::set_current(Some(original)).unwrap();
        dialog.import_export().unwrap();
        let host = active_host(&window);
        assert_eq!(
            host.title().unwrap().downcast_ref::<String>().unwrap(),
            "Import after close"
        );
        invoke(&host.close_button().unwrap());
        subscription.dispose();
        dialog.close(false).unwrap();
        window.close();
    });
}

// A real native subclass reimplements the read-only item Id, matching the
// reference's raw interface case. Normal RibbonButton intentionally resolves an
// empty Id via Name/CommandId, so it cannot represent this extension consumer.
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[repr(C)]
struct CatalogRawIdButton {
    base: RibbonButton,
}
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
ferro_class!(CatalogRawIdButton:RibbonButton);
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
ferro_class_info!(CatalogRawIdButton{interfaces:[Rc<dyn IRibbonItem> =>|value|Rc::new(CatalogRawIdView(value)) as Rc<dyn IRibbonItem>]});
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
ferro_impl_classes!(CatalogRawIdButton:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl,RibbonButtonImpl);
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
struct CatalogRawIdView(Ref<CatalogRawIdButton>);
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
impl crate::controls::IRibbonItem for CatalogRawIdView {
    fn as_object(&self) -> Ref<ferroui_base::FerroObject> {
        self.0.clone().upcast()
    }
    fn id(&self) -> Option<RibbonString> {
        Some("".into())
    }
    fn label(&self) -> Option<RibbonString> {
        self.0.label().map(Into::into)
    }
    fn icon(&self) -> Option<BoxedValue> {
        self.0.icon()
    }
    fn key_tip(&self) -> Option<RibbonString> {
        self.0.key_tip().map(Into::into)
    }
    fn command_id(&self) -> Option<RibbonString> {
        self.0.command_id().map(Into::into)
    }
    fn size(&self) -> ferroribbon_core::RibbonItemSize {
        self.0.size()
    }
    fn effective_size_definition(&self) -> ferroribbon_core::layout::RibbonSizeDefinition {
        self.0.effective_size_definition()
    }
    fn current_size(&self) -> ferroribbon_core::RibbonItemSize {
        self.0.current_size()
    }
    fn simplified_visibility(&self) -> ferroribbon_core::RibbonSimplifiedVisibility {
        self.0.simplified_visibility()
    }
    fn show_label_in_simplified_mode(&self) -> bool {
        self.0.show_label_in_simplified_mode()
    }
    fn can_add_to_quick_access(&self) -> bool {
        self.0.can_add_to_quick_access()
    }
    fn apply_layout(&self, layout: crate::controls::RibbonItemLayout) {
        self.0.apply_layout(layout)
    }
    fn create_linked_copy(&self) -> Option<Ref<Control>> {
        self.0.create_linked_copy_core()
    }
    fn create_overflow_menu_items(&self) -> crate::controls::RibbonOverflowMenuItems {
        self.0.create_overflow_menu_items_core()
    }
    fn on_key_tip(&self) -> Rc<crate::controls::RibbonKeyTipResult> {
        self.0.on_key_tip_core()
    }
    fn invoke(&self) -> bool {
        self.0.invoke_core()
    }
}
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
#[test]
fn customize_dialog_native_catalog_audit_against_unchanged_source_cultures() {
    crate::group_tests::run(|| {
        use ferroui_base::utilities::CultureInfo;
        use serde_json::{Value, json};
        let source: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/customization-catalog.json"
        ))
        .unwrap();
        let previous = CultureInfo::current_formatting();
        let mut mismatches = Vec::new();
        for case in source["cases"].as_array().unwrap() {
            let culture = case["culture"].as_str().unwrap();
            CultureInfo::set_current_formatting(CultureInfo::get_culture_info(culture));
            let ribbon = Ribbon::new();
            let tab = RibbonTab::new();
            tab.set_header(Some("Home".into()));
            let group = RibbonGroup::new();
            group.set_header(Some("Clip\nboard".into()));
            for (i, value) in source["labels"].as_array().unwrap().iter().enumerate() {
                let item = RibbonButton::new();
                item.set_id(Some(format!("id{i}")));
                item.set_label(value.as_str().map(str::to_owned));
                group.items().add(Some(item.upcast()));
            }
            let duplicate = RibbonButton::new();
            duplicate.set_id(Some("id1".into()));
            duplicate.set_label(Some("Duplicate label".into()));
            group.items().add(Some(duplicate.upcast()));
            let linked = RibbonButton::new();
            linked.set_id(Some("linked".into()));
            linked.set_label(Some("Linked".into()));
            RibbonItemHelper::set_source_item(&linked, RibbonButton::new().upcast::<Control>());
            group.items().add(Some(linked.upcast()));
            let excluded = RibbonButton::new();
            excluded.set_id(Some("noqat".into()));
            excluded.set_label(Some("Excluded".into()));
            excluded.set_can_add_to_quick_access(false);
            group.items().add(Some(excluded.upcast()));
            let no_id = RibbonButton::new();
            no_id.set_label(Some("No ID".into()));
            group.items().add(Some(no_id.upcast()));
            let empty_id = instantiate(CatalogRawIdButton {
                base: RibbonButton::construct(),
            });
            empty_id.set_id(Some("".into()));
            empty_id.set_label(Some("Empty ID".into()));
            group.items().add(Some(empty_id.upcast()));
            tab.groups().add(Some(group));
            ribbon.tabs().add(Some(tab));
            let custom = RibbonTab::new();
            custom.set_header(Some("Custom".into()));
            custom.is_custom.set(true);
            let group = RibbonGroup::new();
            let item = RibbonButton::new();
            item.set_id(Some("custom".into()));
            item.set_label(Some("Custom".into()));
            group.items().add(Some(item.upcast()));
            custom.groups().add(Some(group));
            ribbon.tabs().add(Some(custom));
            let no_header = RibbonTab::new();
            let group = RibbonGroup::new();
            let item = RibbonButton::new();
            item.set_id(Some("nullpath".into()));
            item.set_label(Some("No parent label".into()));
            group.items().add(Some(item.upcast()));
            no_header.groups().add(Some(group));
            ribbon.tabs().add(Some(no_header));
            let commands = collect_commands(&ribbon).unwrap();
            let rows:Vec<_>=commands.iter().map(|c|json!({"Id":c.id.to_utf8().unwrap(),"Label":label(c.label.clone()),"Path":label(c.path.clone())})).collect();
            if json!(rows) != case["commands"] {
                mismatches.push(json!({"culture":culture,"kind":"ordering","source":case["commands"],"native":rows}));
            }
            let comparer = CommandText::current().unwrap();
            for filter in case["filters"].as_array().unwrap() {
                let query = filter["query"].as_str().unwrap_or("");
                let blank = ferroribbon_core::text::trim(&query.encode_utf16().collect::<Vec<_>>())
                    .is_empty();
                let ids: Vec<_> = commands
                    .iter()
                    .filter(|c| {
                        blank
                            || comparer.contains(&label(c.label.clone()), query)
                            || comparer.contains(&label(c.path.clone()), query)
                    })
                    .map(|c| c.id.to_utf8().unwrap())
                    .collect();
                if json!(ids) != filter["ids"] {
                    mismatches.push(json!({"culture":culture,"kind":"filter","query":filter["query"],"source":filter["ids"],"native":ids}));
                }
            }
        }
        CultureInfo::set_current_formatting(previous);
        let output_dir = std::env::var_os("FERRORIBBON_TEMP")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        std::fs::create_dir_all(&output_dir).unwrap();
        let output = output_dir.join("customization-catalog-native-audit-second.json");
        std::fs::write(&output, serde_json::to_vec_pretty(&mismatches).unwrap()).unwrap();
        assert!(
            mismatches.is_empty(),
            "{} source/native catalog differences; see {}",
            mismatches.len(),
            output.display()
        );
    });
}

#[test]
fn customize_dialog_root_request_order_cancellation_qat_entry_and_busy() {
    crate::group_tests::run(|| {
        use crate::compat::MenuFlyoutItem;
        let detached = Ribbon::new();
        let seen = Rc::new(Cell::new(0));
        let count = seen.clone();
        let subscription = detached.customize_requested(move |_| count.set(count.get() + 1));
        assert!(
            detached
                .show_customize_dialog(RibbonCustomizePage::Ribbon)
                .unwrap()
                .is_none()
        );
        assert_eq!(seen.get(), 1, "request event runs before attachment check");
        subscription.dispose();
        detached
            .show_customize_dialog(RibbonCustomizePage::Ribbon)
            .unwrap();
        assert_eq!(seen.get(), 1);
        let (ribbon, window) = mount();
        let trace = Rc::new(RefCell::new(Vec::new()));
        let first = trace.clone();
        let a = ribbon.customize_requested(move |args| {
            first.borrow_mut().push((args.page, args.handled.get()));
            args.handled.set(true);
        });
        let second = trace.clone();
        let b = ribbon.customize_requested(move |args| {
            second.borrow_mut().push((args.page, args.handled.get()))
        });
        assert!(
            ribbon
                .show_customize_dialog(RibbonCustomizePage::Ribbon)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            *trace.borrow(),
            [
                (RibbonCustomizePage::Ribbon, false),
                (RibbonCustomizePage::Ribbon, true)
            ]
        );
        a.dispose();
        b.dispose();
        let menu = ribbon.build_quick_access_customize_menu();
        let more = menu
            .items()
            .iter()
            .filter_map(|item| ValueTypes::as_object(&*item?))
            .filter_map(|item| item.cast::<MenuFlyoutItem>())
            .find(|item| {
                item.text()
                    == RibbonStrings::current()
                        .more_commands()
                        .map(|s| s.to_utf8().unwrap())
            })
            .unwrap();
        more.invoke_menu_core();
        let dialog = active_host(&window)
            .cast::<RibbonCustomizeDialog>()
            .unwrap();
        assert_eq!(dialog.page(), RibbonCustomizePage::QuickAccessToolbar);
        assert!(dialog.is_open());
        more.invoke_menu_core();
        assert_eq!(
            ribbon.last_customize_dialog_error(),
            Some(RibbonCustomizeDialogError::AlreadyOpen)
        );
        assert!(dialog.is_open());
        dialog.close(false).unwrap();
        window.close();
        window.set_content(None);
        let weak = ribbon.downgrade();
        drop(dialog);
        drop(ribbon);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(
            weak.upgrade().is_none(),
            "QAT menu entries retain only a weak ribbon after native unload jobs"
        );
    });
}

#[test]
fn customize_dialog_three_outcomes_preserve_source_results_and_acceptance_only_apply() {
    crate::group_tests::run(|| {
        use crate::{compat::ContentDialogButton, controls::RibbonDialogOutcome};
        use ferroui_base::input::{InputElement, Key, KeyEventArgs};
        let (ribbon, window) = mount();
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/dialog-outcomes.json"))
                .unwrap();
        let cases = fixture["cases"].as_array().unwrap();
        let ordinary = cases
            .iter()
            .filter(|c| c["mode"] == "none" && c["hiding"] == false && c["command"] == false)
            .collect::<Vec<_>>();
        assert_eq!(ordinary.len(), 3);
        for keyboard in [false, true] {
            for case in &ordinary {
                ribbon.set_quick_access_position(RibbonQuickAccessPosition::AboveRibbon);
                let dialog =
                    RibbonCustomizeDialog::new(ribbon.clone(), RibbonCustomizePage::Ribbon)
                        .unwrap();
                dialog.set_primary_button_text(Some("Accept".into()));
                dialog.set_secondary_button_text(Some("Other".into()));
                dialog.set_close_button_text(Some("Dismiss".into()));
                dialog
                    .working_state()
                    .set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
                let (button, expected) = match case["button"].as_str().unwrap() {
                    "Primary" => (ContentDialogButton::Primary, RibbonDialogOutcome::Primary),
                    "Secondary" => (
                        ContentDialogButton::Secondary,
                        RibbonDialogOutcome::Secondary,
                    ),
                    _ => (ContentDialogButton::Close, RibbonDialogOutcome::Dismissed),
                };
                let expected_raw = match expected {
                    RibbonDialogOutcome::Dismissed => 0,
                    RibbonDialogOutcome::Primary => 1,
                    RibbonDialogOutcome::Secondary => 2,
                };
                assert_eq!(case["after"], serde_json::json!([expected_raw]));
                let trace = Rc::new(RefCell::new(Vec::<String>::new()));
                let observed = trace.clone();
                let legacy = dialog.closed(move |accepted| {
                    observed.borrow_mut().push(format!("legacy:{accepted}"))
                });
                let observed = trace.clone();
                let weak = dialog.downgrade();
                let typed = dialog.closed_with_outcome(move |outcome| {
                    assert!(!weak.upgrade().unwrap().is_open());
                    observed.borrow_mut().push(format!("typed:{outcome:?}"));
                });
                dialog.set_default_button(button);
                dialog.show(window.clone().upcast()).unwrap();
                assert_eq!(dialog.last_outcome(), None);
                if keyboard {
                    let mut event = KeyEventArgs::default();
                    event.key = Key::Enter;
                    event.set_routed_event(Some(InputElement::key_down_event()));
                    dialog.raise_event(&event);
                    assert!(!event.handled());
                } else {
                    let control = match button {
                        ContentDialogButton::Primary => dialog.primary_button().unwrap(),
                        ContentDialogButton::Secondary => dialog.secondary_button().unwrap(),
                        _ => dialog.close_button().unwrap(),
                    };
                    invoke(&control);
                }
                assert!(!dialog.is_open());
                assert_eq!(dialog.last_outcome(), Some(expected));
                assert_eq!(
                    *trace.borrow(),
                    [
                        format!("legacy:{}", expected == RibbonDialogOutcome::Primary),
                        format!("typed:{expected:?}")
                    ]
                );
                assert_eq!(
                    ribbon.quick_access_position(),
                    if expected == RibbonDialogOutcome::Primary {
                        RibbonQuickAccessPosition::BelowRibbon
                    } else {
                        RibbonQuickAccessPosition::AboveRibbon
                    }
                );
                dialog
                    .close_with_outcome(RibbonDialogOutcome::Primary)
                    .unwrap();
                assert_eq!(
                    trace.borrow().len(),
                    2,
                    "closed dialog is not completed twice"
                );
                legacy.dispose();
                typed.dispose();
            }
        }
        let dialog = RibbonCustomizeDialog::new(ribbon, RibbonCustomizePage::Ribbon).unwrap();
        let count = Rc::new(Cell::new(0));
        let seen = count.clone();
        let weak = dialog.downgrade();
        let owner = window.downgrade();
        let token = dialog.closed_with_outcome(move |_| {
            let value = seen.get() + 1;
            seen.set(value);
            if value == 1 {
                weak.upgrade()
                    .unwrap()
                    .show(owner.upgrade().unwrap().upcast())
                    .unwrap();
            }
        });
        dialog.show(window.clone().upcast()).unwrap();
        dialog
            .close_with_outcome(RibbonDialogOutcome::Dismissed)
            .unwrap();
        assert!(dialog.is_open());
        assert_eq!(
            dialog.last_outcome(),
            None,
            "reopened session has its own result"
        );
        dialog
            .close_with_outcome(RibbonDialogOutcome::Secondary)
            .unwrap();
        assert_eq!(dialog.last_outcome(), Some(RibbonDialogOutcome::Secondary));
        assert_eq!(count.get(), 2);
        token.dispose();
        window.close();
        window.set_content(None);
        Dispatcher::ui_thread().run_jobs(None);
    });
}

#[test]
fn customization_primary_applies_before_cancelled_command_and_closing_for_click_and_enter() {
    crate::group_tests::run(|| {
        use crate::compat::ContentDialogButton;
        use ferroui_base::input::{InputElement, Key, KeyEventArgs};
        for keyboard in [false, true] {
            let (ribbon, window) = mount();
            ribbon.set_quick_access_position(RibbonQuickAccessPosition::AboveRibbon);
            let dialog =
                RibbonCustomizeDialog::new(ribbon.clone(), RibbonCustomizePage::Ribbon).unwrap();
            dialog
                .working_state()
                .set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
            let weak = ribbon.downgrade();
            let calls = Rc::new(Cell::new(0));
            let seen = calls.clone();
            let cancel = dialog.button_click(ContentDialogButton::Primary, move |args| {
                assert_eq!(
                    weak.upgrade().unwrap().quick_access_position(),
                    RibbonQuickAccessPosition::BelowRibbon
                );
                seen.set(seen.get() + 1);
                args.set_cancel(true);
            });
            dialog.set_default_button(ContentDialogButton::Primary);
            dialog.show(window.clone().upcast()).unwrap();
            if keyboard {
                let mut e = KeyEventArgs::default();
                e.key = Key::Enter;
                e.set_routed_event(Some(InputElement::key_down_event()));
                dialog.raise_event(&e);
            } else {
                invoke(&dialog.primary_button().unwrap());
            }
            assert_eq!(calls.get(), 1);
            assert!(dialog.is_open());
            assert_eq!(dialog.last_outcome(), None);
            cancel.dispose();
            dialog.close(false).unwrap();
            window.close();
        }
    });
}

#[test]
fn customization_inherited_native_future_keeps_sizing_nested_deferral_and_reopen_state() {
    crate::group_tests::run(|| {
        let (ribbon, window) = mount();
        let dialog = RibbonCustomizeDialog::new(ribbon, RibbonCustomizePage::Ribbon).unwrap();
        let future = dialog
            .show_dialog(window.clone().upcast())
            .unwrap()
            .shared();
        assert!(dialog.is_open());
        assert_eq!(dialog.last_outcome(), None);
        window.set_width(700.);
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(dialog.body.width(), 604.);
        dialog.structure_list.select(Some(&"clip".into()));
        let pending = Rc::new(RefCell::new(None));
        let handle = pending.clone();
        let token = dialog.dialog_closing(move |args| {
            handle.replace(Some(args.get_deferral()));
        });
        dialog.rename().unwrap();
        assert!(dialog.is_open());
        assert!(dialog.is_dialog_closing());
        assert!(dialog.last_nested_error().is_none());
        token.dispose();
        pending.take().unwrap().complete().unwrap();
        assert!(!dialog.is_open());
        let nested = active_host(&window);
        assert!(!nested.ptr_eq(&dialog));
        assert_eq!(future.clone().now_or_never(), None);
        invoke(&nested.close_button().unwrap());
        assert!(dialog.is_open());
        assert_eq!(dialog.last_outcome(), None);
        assert_eq!(
            future.clone().now_or_never(),
            Some(Ok(crate::controls::RibbonDialogOutcome::Dismissed))
        );
        dialog.close(false).unwrap();
        window.close();
    });
}
#[test]
fn customization_nested_cancel_waits_for_source_one_second_timeout_and_cleans_subscription() {
    crate::group_tests::run(|| {
        let (ribbon, window) = mount();
        let dialog = RibbonCustomizeDialog::new(ribbon, RibbonCustomizePage::Ribbon).unwrap();
        dialog.show(window.clone().upcast()).unwrap();
        dialog.structure_list.select(Some(&"clip".into()));
        let cancel = dialog.dialog_closing(|args| args.set_cancel(true));
        dialog.rename().unwrap();
        assert!(dialog.is_open());
        assert_eq!(dialog.pending_nested.borrow().len(), 1);
        assert!(dialog.last_nested_error().is_none());
        // Timer callbacks run through the real dispatcher loop. Sleeping this
        // UI thread and draining ready jobs alone does not service its backend.
        let pump = |duration| {
            use ferroui_base::threading::{DispatcherFrame, DispatcherPriority, DispatcherTimer};
            let frame = DispatcherFrame::new();
            let stop = frame.clone();
            let timer = DispatcherTimer::run_once(
                move || stop.set_continue(false),
                duration,
                DispatcherPriority::NORMAL,
            );
            Dispatcher::ui_thread().push_frame(&frame);
            timer.dispose();
        };
        pump(std::time::Duration::from_millis(250));
        assert!(dialog.last_nested_error().is_none());
        assert_eq!(dialog.pending_nested.borrow().len(), 1);
        pump(std::time::Duration::from_millis(850));
        assert_eq!(
            dialog.last_nested_error(),
            Some(RibbonCustomizeDialogError::AlreadyOpen)
        );
        assert!(dialog.pending_nested.borrow().is_empty());
        assert!(dialog.is_open());
        cancel.dispose();
        dialog.close(false).unwrap();
        window.close();
    });
}
