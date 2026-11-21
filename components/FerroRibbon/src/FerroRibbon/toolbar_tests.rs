use super::*;
use crate::controls::RibbonToggleButton;
use ferroribbon_core::commands::{RibbonCommandDescriptor, RibbonRelayCommand};
use ferroui_base::{data::core::ValueType, input::InputElement, metadata::MarkupTyped};
use ferroui_controls::{
    Window,
    automation::peers::{AutomationControlType, ControlAutomationPeer},
};
use serde_json::{Value, json};
fn run(test: impl FnOnce() + Send + 'static) {
    use ferroui_headless::FerroHeadlessPlatformExtensions;
    ferroui_headless::HeadlessUnitTestSession::start_new(|| {
        ferroui_controls::AppBuilder::configure::<Application>().use_headless(Default::default())
    })
    .dispatch(
        move || {
            crate::register_types();
            let app = Application::current().unwrap();
            app.styles()
                .add(ferroui_themes_simple::SimpleTheme::new().as_style());
            for name in ["Buttons", "Checked", "SimpleItems", "Toolbars"] {
                let v = crate::compiled_shared::try_load(
                    None,
                    &format!("ferres://FerroRibbon/Themes/{name}.xaml"),
                )
                .unwrap()
                .unwrap();
                let dictionary = ValueTypes::as_object(&*v)
                    .unwrap()
                    .cast::<ferroui_base::controls::ResourceDictionary>()
                    .unwrap();
                app.resources().merged_dictionaries().add(dictionary.into());
            }
            test();
        },
        ferroui_base::threading::CancellationToken::none(),
    )
    .wait()
    .unwrap();
}
fn corpus(name: &str) -> Value {
    serde_json::from_str::<Value>(include_str!("../../tests/fixtures/toolbars.json")).unwrap()["corpus"]["cases"].as_array().unwrap().iter().find(|v|v["name"]==name).unwrap().clone()
}
fn button(name: &str) -> Ref<RibbonButton> {
    let b = RibbonButton::new();
    b.set_name(Some(name.into()));
    b.set_label(Some(name.into()));
    b.set_icon(Some(Rc::new("A".to_string())));
    b
}
fn mount(control: Ref<Control>, width: f64, height: f64) -> Ref<Window> {
    let w = Window::new();
    w.set_content(Some(Control::boxed(control)));
    w.set_width(width);
    w.set_height(height);
    w.show();
    w.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    w
}
fn visibility(items: &RibbonControlItems) -> Vec<&'static str> {
    items
        .iter()
        .map(|i| {
            if i.unwrap().is_visible() {
                "Visible"
            } else {
                "Collapsed"
            }
        })
        .collect()
}
fn send(c: &Control, key: Key) -> bool {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_routed_event(Some(InputElement::key_down_event()));
    c.raise_event(&e);
    e.handled()
}
#[test]
fn original_toolbar_defaults_and_layout_property_callbacks() {
    run(|| {
        let t = RibbonToolBar::new();
        let b = &t.overflow_button;
        assert_eq!(
            json!({"name":"defaults","orientation":t.orientation().raw(),"Columns":t.columns(),"ShowLabels":t.show_labels(),"IsOverflowEnabled":t.is_overflow_enabled(),"density":t.density().to_string(),"IsTabStop":KeyboardNavigation::get_is_tab_stop(&t),"automation":AutomationProperties::get_name(&t),"button":{"Id":b.id(),"Label":b.label(),"ShowLabel":b.show_label(),"CanAddToQuickAccess":b.can_add_to_quick_access(),"IsChromeButton":b.is_chrome_button()},"overflow":t.overflow_items().count()}),
            corpus("defaults")
        );
        let a = button("a");
        let sep = RibbonSeparator::new();
        t.items().add(Some(a.clone().upcast()));
        t.items().add(Some(sep.clone().upcast()));
        let expected = corpus("layouts");
        assert_eq!(a.current_size().to_string(), expected["initial"]["size"]);
        assert!(a.is_simplified());
        assert!(!a.actual_show_label());
        assert_eq!(
            sep.orientation(),
            ferroui_base::layout::Orientation::Vertical
        );
        t.set_orientation(Orientation::Vertical);
        t.set_columns(-4);
        t.set_show_labels(true);
        t.set_density(RibbonDensity::Touch);
        t.set_is_overflow_enabled(false);
        assert_eq!(
            t.panel.orientation().raw(),
            expected["changed"]["orientation"]
        );
        assert_eq!(t.panel.columns(), expected["changed"]["columns"]);
        assert_eq!(
            sep.orientation(),
            ferroui_base::layout::Orientation::Horizontal
        );
        assert_eq!(a.current_size().to_string(), expected["changed"]["size"]);
        assert!(a.is_simplified());
        assert!(
            !a.actual_show_label(),
            "source null override delegates to Auto/Medium simplified label rule"
        );
        assert!(Arc::ptr_eq(&a.metrics().unwrap(), &t.metrics()));
        assert!(!t.panel.is_overflow_enabled());
    });
}
#[test]
fn original_invocation_order_rereads_ribbon_after_handler_and_preserves_ids() {
    run(|| {
        let t = RibbonToolBar::new();
        let first = Ribbon::new();
        let next = Ribbon::new();
        t.set_ribbon(first.clone());
        let trace = Rc::new(RefCell::new(Vec::new()));
        let log = trace.clone();
        first.add_item_invoked(Some(crate::root_command_tests::invocation(move |_| {
            log.borrow_mut().push("wrong".to_string())
        })));
        let log = trace.clone();
        next.add_item_invoked(Some(crate::root_command_tests::invocation(move |_| {
            log.borrow_mut().push("ribbon".into())
        })));
        let log = trace.clone();
        let weak = t.downgrade();
        let r = next.clone();
        t.add_item_invoked(Some(crate::root_command_tests::invocation(move |e| {
            assert_eq!(e.item_id, Some("id".into()));
            assert_eq!(
                e.parameter.as_ref().unwrap().downcast_ref::<i32>(),
                Some(&7)
            );
            log.borrow_mut().push(format!(
                "toolbar:{}:7",
                e.command_id.as_ref().unwrap().to_utf8().unwrap()
            ));
            weak.upgrade().unwrap().set_ribbon(r.clone());
        })));
        let item = button("a");
        item.set_id(Some("id".into()));
        t.on_item_invoked(item.upcast(), Some("cmd".into()), Some(Rc::new(7i32)));
        assert_eq!(
            json!(*trace.borrow()),
            corpus("invoked_replacement")["trace"]
        );
        assert!(t.ribbon().unwrap() == next);
    });
}
#[test]
fn toolbar_interfaces_catalog_resolution_nested_items_and_weak_owner_lifetime() {
    run(|| {
        let t = RibbonToolBar::new();
        let catalog = RibbonCommandCatalog::new();
        let count = Rc::new(Cell::new(0));
        let n = count.clone();
        let command = RibbonRelayCommand::new(move || n.set(n.get() + 1));
        let descriptor =
            RibbonCommandDescriptor::new(Some("draw".into()), Some("Draw".into()), Some(command));
        catalog.register(Some(descriptor.clone()));
        let container = crate::controls::RibbonStackPanel::new();
        let b = button("a");
        b.set_command_id(Some("DRAW".into()));
        container.items().add(Some(b.clone().upcast()));
        t.items().add(Some(container.upcast()));
        let w = mount(t.clone().upcast(), 500., 100.);
        t.set_command_catalog(Some(catalog));
        assert!(b.command().is_some());
        b.perform_click();
        assert_eq!(count.get(), 1);
        descriptor.set_is_enabled(false);
        assert!(!b.is_enabled());
        let boxed = Control::boxed(t.clone());
        let host =
            ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn IRibbonLayoutHost>>()).unwrap();
        host.downcast_ref::<Rc<dyn IRibbonLayoutHost>>()
            .unwrap()
            .invalidate_items_layout();
        let owner =
            ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn IRibbonItemOwner>>()).unwrap();
        assert!(
            owner
                .downcast_ref::<Rc<dyn IRibbonItemOwner>>()
                .unwrap()
                .as_object()
                .ptr_eq(&t)
        );
        drop(owner);
        drop(host);
        drop(boxed);
        w.close();
        w.set_content(None);
        drop(w);
        let held = t.items();
        let weak = t.downgrade();
        drop(t);
        assert!(weak.upgrade().is_none());
        assert_eq!(held.count(), 1);
        assert!(RibbonItemHelper::find_owner(Some(&b)).is_none());
    });
}
#[test]
fn source_sync_reparents_same_items_reorders_and_retemplates_same_panel() {
    run(|| {
        let t = RibbonToolBar::new();
        let a = button("a");
        let b = button("b");
        let prior = StackPanel::new();
        prior.children().add(a.clone());
        t.items().add(Some(a.clone().upcast()));
        t.items().add(Some(b.clone().upcast()));
        let w = mount(t.clone().upcast(), 500., 80.);
        let old = t.host.borrow().clone().unwrap();
        let panel = t.panel.clone();
        t.set_template(Some(ferroui_controls::templates::FuncControlTemplate::new(
            |_, scope| {
                let host = Border::new();
                scope.register("PART_ItemsHost", host.clone().upcast());
                host.upcast()
            },
        )));
        t.apply_template();
        assert!(old.child().is_none());
        t.items().move_item(1, 0);
        assert_eq!(prior.children().count(), 0);
        assert!(
            t.host
                .borrow()
                .clone()
                .unwrap()
                .child()
                .unwrap()
                .ptr_eq(&panel)
        );
        let names: Vec<_> = panel
            .children()
            .to_vec()
            .iter()
            .map(|c| c.name().unwrap_or_default())
            .collect();
        assert_eq!(json!(names), corpus("sync_identity")["children"]);
        assert!(
            RibbonItemHelper::get_owner(&a)
                .unwrap()
                .as_object()
                .ptr_eq(&t)
        );
        w.close();
    });
}
#[test]
fn upstream_toolbar_overflows_and_vertical_two_column_palette_fits() {
    run(|| {
        let t = RibbonToolBar::new();
        for i in 0..20 {
            t.items().add(Some(button(&format!("Button {i}")).upcast()));
        }
        let w = mount(t.clone().upcast(), 240., 100.);
        assert!(t.overflow_items().count() > 0);
        let held = t.overflow_items();
        assert!(held == t.overflow_items());
        let vertical = RibbonToolBar::new();
        vertical.set_orientation(Orientation::Vertical);
        vertical.set_columns(2);
        for i in 0..6 {
            let b = RibbonToggleButton::new();
            b.set_label(Some(format!("Tool {i}")));
            b.set_icon(Some(Rc::new("A".to_string())));
            b.set_group_name(Some("tools".into()));
            vertical.items().add(Some(b.upcast()));
        }
        let vw = mount(vertical.clone().upcast(), 400., 300.);
        assert_eq!(vertical.overflow_items().count(), 0);
        assert!(vertical.bounds().width > vertical.bounds().height / 3.);
        w.set_width(2000.);
        w.update_layout();
        assert_eq!(held.count(), 0);
        w.close();
        vw.close();
    });
}
#[test]
fn toolbar_overflow_accessibility_restores_removal_and_plain_false_tabstop() {
    run(|| {
        let t = RibbonToolBar::new();
        let first = button("first");
        t.items().add(Some(first.upcast()));
        let a = button("a");
        let b = button("b");
        KeyboardNavigation::set_is_tab_stop(&b, false);
        a.set_width(180.);
        b.set_width(180.);
        t.items().add(Some(a.clone().upcast()));
        t.items().add(Some(b.clone().upcast()));
        let w = mount(t.clone().upcast(), 120., 100.);
        assert!(
            t.overflow_items()
                .iter()
                .any(|i| i == Some(a.clone().upcast()))
        );
        t.update_overflow_accessibility();
        assert!(!KeyboardNavigation::get_is_tab_stop(&a));
        assert_eq!(
            AutomationProperties::get_accessibility_view(&a),
            AccessibilityView::Raw
        );
        t.items().remove(&Some(a.clone().upcast()));
        assert_eq!(
            KeyboardNavigation::get_is_tab_stop(&a),
            corpus("arrange_before_accessibility")["afterRemoval"]
                .as_bool()
                .unwrap()
        );
        crate::primitives::ribbon_hidden_focus::set(&a.clone().upcast(), false);
        assert_eq!(
            KeyboardNavigation::get_is_tab_stop(&a),
            corpus("arrange_before_accessibility")["afterShown"]
                .as_bool()
                .unwrap()
        );
        assert_eq!(
            a.get_local_value(AutomationProperties::accessibility_view_property()),
            None
        );
        w.set_width(2000.);
        w.update_layout();
        t.update_overflow_accessibility();
        assert!(!KeyboardNavigation::get_is_tab_stop(&b));
        assert!(!KeyboardNavigation::get_is_tab_stop(&t.overflow_button));
        w.close();
    });
}
#[test]
fn toolbar_overflow_retains_linked_copy_invokes_once_and_unlinks_on_close() {
    run(|| {
        let t = RibbonToolBar::new();
        t.items().add(Some(button("first").upcast()));
        let b = button("source");
        b.set_width(180.);
        b.set_id(Some("source-id".into()));
        t.items().add(Some(b.clone().upcast()));
        let executions = Rc::new(Cell::new(0));
        let n = executions.clone();
        b.set_command(Some(crate::commands::adapt_core_command(
            RibbonRelayCommand::new(move || n.set(n.get() + 1)),
        )));
        let invoked = Rc::new(Cell::new(0));
        let n = invoked.clone();
        t.add_item_invoked(Some(crate::root_command_tests::invocation(move |e| {
            assert_eq!(e.item_id, Some("source-id".into()));
            n.set(n.get() + 1);
        })));
        let w = mount(t.clone().upcast(), 120., 100.);
        assert_eq!(t.open_flyouts.borrow().len(), 0);
        t.show_overflow();
        let (flyout, panel) = {
            let leases = t.open_flyouts.borrow();
            assert_eq!(leases.len(), 1);
            (leases[0].flyout.clone(), leases[0].cleanup.panel.clone())
        };
        assert!(flyout.is_open());
        assert!(flyout.popup().host().is_some());
        let copy = panel.children().get(0).cast::<RibbonButton>().unwrap();
        assert!(!copy.ptr_eq(&b));
        assert_eq!(copy.current_size(), RibbonItemSize::Medium);
        assert!(!copy.is_simplified());
        assert!(copy.actual_show_label());
        copy.set_label(Some("copy-origin".into()));
        assert_eq!(b.label().as_deref(), Some("source"));
        b.set_label(Some("source-updated".into()));
        assert_eq!(copy.label().as_deref(), Some("source-updated"));
        copy.perform_click();
        assert_eq!(executions.get(), 1);
        assert_eq!(invoked.get(), 1);
        flyout.hide();
        assert_eq!(t.open_flyouts.borrow().len(), 0);
        b.set_label(Some("after-close".into()));
        assert_eq!(copy.label().as_deref(), Some("source-updated"));
        assert!(t.items().contains(&Some(b.upcast())));
        w.close();
    });
}
#[test]
fn toolbar_roving_focus_clamps_and_keeps_composite_arrow_keys() {
    run(|| {
        let t = RibbonToolBar::new();
        let a = button("a");
        let b = button("b");
        t.items().add(Some(a.clone().upcast()));
        t.items().add(Some(b.clone().upcast()));
        let w = mount(t.clone().upcast(), 500., 100.);
        assert!(a.focus());
        assert!(send(&a, Key::Left));
        assert!(a.is_focused());
        assert!(send(&a, Key::Right));
        assert!(b.is_focused());
        assert!(send(&b, Key::Right));
        assert!(b.is_focused());
        let input = crate::controls::RibbonTextBox::new();
        t.items().add(Some(input.clone().upcast()));
        w.update_layout();
        let mut e = KeyEventArgs::new();
        e.key = Key::Right;
        e.set_source(input.to_ref());
        t.on_key_down(&e);
        assert!(!e.handled());
        w.close();
    });
}
struct ContextHandler(Box<dyn Fn(Option<String>)>);
impl PartialEq for ContextHandler {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class ContextHandler{this:Rc<ContextHandler>,handles:[Rc<ContextHandler>],methods:[fn Invoke(Option<BoxedValue>,Option<String>)=>|t:&Rc<ContextHandler>,_,v|(t.0)(v)]});
fn context_handler(f: impl Fn(Option<String>) + 'static) -> MarkupDelegate {
    ValueTypes::register_reference::<ContextHandler>();
    MarkupDelegate::for_method(
        Some(Rc::new(ContextHandler(Box::new(f)))),
        ContextHandler::MARKUP,
        &ContextHandler::MARKUP.methods[0],
    )
}
#[test]
fn original_context_attached_property_is_passive_until_update_and_preserves_identity() {
    run(|| {
        let t = RibbonContextualToolBar::new();
        let a = button("a");
        let b = button("b");
        RibbonContextualToolBar::set_context(&b, Some("text".into()));
        t.items().add(Some(a.clone().upcast()));
        t.items().add(Some(b.clone().upcast()));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let log = trace.clone();
        t.add_context_changed(Some(context_handler(move |v| log.borrow_mut().push(v))));
        let expected = corpus("context");
        assert_eq!(json!(visibility(&t.items())), expected["before"]);
        let w = mount(t.clone().upcast(), 500., 100.);
        t.set_active_context(Some("text".into()));
        RibbonContextualToolBar::set_context(&a, Some("text".into()));
        assert_eq!(
            if a.is_visible() {
                "Visible"
            } else {
                "Collapsed"
            },
            expected["unchanged"]
        );
        t.items().move_item(1, 0);
        t.set_active_context(Some("Text".into()));
        t.set_active_context(None);
        assert_eq!(json!(*trace.borrow()), expected["trace"]);
        assert_eq!(json!(visibility(&t.items())), expected["final"]);
        let host = t.host.borrow().clone().unwrap();
        assert_eq!(
            json!(
                host.children()
                    .to_vec()
                    .iter()
                    .map(|c| c.name().unwrap())
                    .collect::<Vec<_>>()
            ),
            expected["order"]
        );
        assert!(host.children().get(1).ptr_eq(&a));
        // Context is attached to any DependencyObject, not just a control.
        let item = RibbonMenuBarItem::new();
        RibbonContextualToolBar::set_context(&item, Some("é\u{301}𐐨".into()));
        assert_eq!(
            RibbonContextualToolBar::get_context(&item).as_deref(),
            Some("é\u{301}𐐨")
        );
        w.close();
    });
}
#[test]
fn upstream_menu_rebuild_keeps_entries_headers_live_and_menu_identity() {
    run(|| {
        let file = RibbonMenuBarItem::new();
        file.set_header(Some("File".into()));
        let open = crate::compat::MenuFlyoutItem::new();
        open.set_text(Some("Open".into()));
        file.items().add(Some(open.clone().upcast()));
        let bar = RibbonMenuBar::new();
        bar.items().add(Some(file.clone()));
        let w = mount(bar.clone().upcast(), 600., 100.);
        let menu = file.menu();
        let old_header = bar.menus.borrow()[0].header.clone();
        let edit = RibbonMenuBarItem::new();
        edit.set_header(Some("Edit".into()));
        bar.items().add(Some(edit));
        let save = crate::compat::MenuFlyoutItem::new();
        save.set_text(Some("Save".into()));
        file.items().add(Some(save.upcast()));
        assert!(file.menu() == menu);
        assert_eq!(file.items().count(), 2);
        assert_eq!(menu.items().count(), 2);
        assert!(
            Control::from_boxed(&menu.items().view().get_at(0).unwrap())
                .unwrap()
                .ptr_eq(&open)
        );
        file.set_header(Some("FILE".into()));
        assert_eq!(
            bar.menus.borrow()[0]
                .header
                .content()
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("FILE")
        );
        assert_eq!(
            old_header
                .content()
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("File")
        );
        assert!(bar.open_menu(Some("file")));
        assert!(bar.is_menu_open());
        assert!(menu.is_open());
        menu.hide();
        assert!(!bar.is_menu_open());
        assert_eq!(file.header_changed.len(), 1);
        bar.items().clear();
        assert_eq!(file.header_changed.len(), 0);
        w.close();
    });
}
#[test]
fn original_menu_header_lookup_uses_pinned_utf16_ordinal_ignore_case() {
    run(|| {
        let bar = RibbonMenuBar::new();
        for label in [None, Some("é"), Some("e\u{301}"), Some("𐐨"), Some("İ")] {
            let item = RibbonMenuBarItem::new();
            item.set_header(label.map(Into::into));
            bar.items().add(Some(item));
        }
        let w = mount(bar.clone().upcast(), 600., 100.);
        for case in corpus("menu_lookup")["results"].as_array().unwrap() {
            assert_eq!(
                bar.open_menu(case["label"].as_str()),
                case["found"].as_bool().unwrap(),
                "{case}"
            );
        }
        let open = bar.open.borrow().clone();
        if let Some(menu) = open {
            menu.hide();
        }
        w.close();
    });
}
#[test]
fn toolbar_automation_peer_reports_native_tool_bar_role() {
    run(|| {
        let t = RibbonToolBar::new();
        let peer = ControlAutomationPeer::create_peer_for_element(&t);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::ToolBar
        );
        assert_eq!(peer.get_class_name(), "RibbonToolBar");
    });
}

use crate::controls::{RibbonButtonImpl, RibbonButtonImplExt};
use ferroui_controls::{ButtonImpl, ContentControlImpl};
#[repr(C)]
struct TrackedButton {
    base: RibbonButton,
    unlinked: Rc<Cell<i32>>,
}
ferro_class!(TrackedButton:RibbonButton);
ferro_impl_classes!(TrackedButton:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,ButtonImpl);
impl TrackedButton {
    fn new(unlinked: Rc<Cell<i32>>) -> Ref<Self> {
        instantiate(Self {
            base: RibbonButton::construct(),
            unlinked,
        })
    }
}
impl RibbonButtonImpl for TrackedButton {
    fn create_copy_instance(t: &Self) -> Ref<RibbonButton> {
        Self::new(t.unlinked.clone()).upcast()
    }
    fn on_unlinked_core(t: &Self) {
        t.unlinked.set(t.unlinked.get() + 1);
        Self::parent_on_unlinked_core(t);
    }
}
#[test]
fn overflow_cleanup_calls_virtual_unlinked_exactly_once_on_close_and_owner_detach() {
    run(|| {
        for detach in [false, true] {
            let t = RibbonToolBar::new();
            t.items().add(Some(button("first").upcast()));
            let count = Rc::new(Cell::new(0));
            let b = TrackedButton::new(count.clone());
            b.set_label(Some("tracked".into()));
            b.set_width(200.);
            t.items().add(Some(b.upcast()));
            let w = mount(t.clone().upcast(), 100., 100.);
            t.show_overflow();
            let (flyout, copy) = {
                let leases = t.open_flyouts.borrow();
                (
                    leases[0].flyout.clone(),
                    leases[0].cleanup.panel.children().get(0),
                )
            };
            assert_eq!(count.get(), 0);
            if detach {
                w.set_content(None);
            } else {
                flyout.hide();
            }
            assert_eq!(count.get(), 1);
            assert_eq!(t.open_flyouts.borrow().len(), 0);
            let weak = t.downgrade();
            w.close();
            w.set_content(None);
            drop(t);
            assert!(weak.upgrade().is_none());
            drop(flyout);
            drop(copy);
            assert_eq!(count.get(), 1);
        }
    });
}
#[test]
fn context_changed_supports_reentrant_context_updates_and_retained_collection_cleanup() {
    run(|| {
        let t = RibbonContextualToolBar::new();
        let a = button("a");
        RibbonContextualToolBar::set_context(&a, Some("a".into()));
        t.items().add(Some(a.clone().upcast()));
        let b = button("b");
        RibbonContextualToolBar::set_context(&b, Some("b".into()));
        t.items().add(Some(b.clone().upcast()));
        let held = t.items();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let log = calls.clone();
        let weak = t.downgrade();
        let handler = context_handler(move |value| {
            log.borrow_mut().push(value.clone());
            if value.as_deref() == Some("a") {
                weak.upgrade().unwrap().set_active_context(Some("b".into()));
            }
        });
        t.add_context_changed(Some(handler.clone()));
        t.set_active_context(Some("a".into()));
        assert_eq!(*calls.borrow(), [Some("a".into()), Some("b".into())]);
        assert!(!a.is_visible());
        assert!(b.is_visible());
        t.remove_context_changed(Some(handler));
        t.set_active_context(None);
        assert_eq!(calls.borrow().len(), 2);
        let weak = t.downgrade();
        drop(t);
        assert!(weak.upgrade().is_none());
        held.add(Some(button("later").upcast()));
        assert_eq!(held.count(), 3);
    });
}

#[test]
fn overflow_padding_preserves_source_minimum_outer_width() {
    run(|| {
        let t = RibbonToolBar::new();
        let w = mount(t.clone().upcast(), 500., 100.);
        t.show_overflow();
        let flyout = t.open_flyouts.borrow()[0].flyout.clone();
        let viewer = Control::from_boxed(&flyout.content().unwrap())
            .unwrap()
            .cast::<ScrollViewer>()
            .unwrap();
        let padded = Control::from_boxed(&viewer.content().unwrap())
            .unwrap()
            .cast::<Border>()
            .unwrap();
        padded.measure(ferroui_base::Size::INFINITY);
        assert_eq!(
            padded.desired_size().width,
            corpus("overflow")["minimumOuterWidth"].as_f64().unwrap(),
            "source StackPanel MinWidth includes its own Padding"
        );
        flyout.hide();
        w.close();
    });
}

struct OpeningHandler(Box<dyn Fn(Option<BoxedValue>)>);
impl PartialEq for OpeningHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class OpeningHandler{this:Rc<OpeningHandler>,handles:[Rc<OpeningHandler>],methods:[fn Invoke(Option<BoxedValue>,ferroui_base::utilities::EventArgs)=>|t:&Rc<OpeningHandler>,sender,_|(t.0)(sender)]});
#[test]
fn menu_header_keyboard_wrap_opening_sender_hover_switch_and_click_toggle() {
    run(|| {
        let bar = RibbonMenuBar::new();
        let file = RibbonMenuBarItem::new();
        file.set_header(Some("File".into()));
        let entry = crate::compat::MenuFlyoutItem::new();
        entry.set_text(Some("Open".into()));
        file.items().add(Some(entry.clone().upcast()));
        let edit = RibbonMenuBarItem::new();
        edit.set_header(Some("Edit".into()));
        bar.items().add(Some(file.clone()));
        bar.items().add(Some(edit.clone()));
        ValueTypes::register_reference::<OpeningHandler>();
        let count = Rc::new(Cell::new(0));
        let n = count.clone();
        let weak_file = file.downgrade();
        file.add_opening(Some(MarkupDelegate::for_method(
            Some(Rc::new(OpeningHandler(Box::new(move |sender| {
                assert!(
                    ValueTypes::as_object(&*sender.unwrap())
                        .unwrap()
                        .ptr_eq(&weak_file.upgrade().unwrap())
                );
                n.set(n.get() + 1);
                entry.set_is_enabled(false);
            })))),
            OpeningHandler::MARKUP,
            &OpeningHandler::MARKUP.methods[0],
        )));
        let w = mount(bar.clone().upcast(), 500., 100.);
        let (first, second) = {
            let menus = bar.menus.borrow();
            (menus[0].header.clone(), menus[1].header.clone())
        };
        bar.focus_first();
        assert!(first.is_focused());
        assert!(send(&first, Key::Left));
        assert!(second.is_focused());
        assert!(send(&second, Key::Right));
        assert!(first.is_focused());
        assert!(send(&first, Key::Down));
        assert!(bar.is_menu_open());
        assert_eq!(count.get(), 1);
        let menu = file.menu();
        assert!(
            menu.overlay_input_pass_through_element()
                .unwrap()
                .ptr_eq(&bar)
        );
        let e = ferroui_base::input::PointerEventArgs::new(
            Some(InputElement::pointer_entered_event()),
            second.clone().upcast::<Control>(),
            ferroui_base::input::Pointer::new(1, ferroui_base::input::PointerType::Mouse, true),
            None,
            ferroui_base::Point::default(),
            0,
            ferroui_base::input::PointerPointProperties::default(),
            KeyModifiers::NONE,
        );
        second.raise_event(&e);
        assert!(!menu.is_open());
        assert!(edit.menu().is_open());
        assert!(bar.is_menu_open());
        second.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(
            Button::click_event(),
        ));
        assert!(!bar.is_menu_open());
        assert_eq!(count.get(), 1);
        w.close();
    });
}
