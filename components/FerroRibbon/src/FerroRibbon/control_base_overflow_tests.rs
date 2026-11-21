//! Native default-base overflow versus its exact source method and menu helper.
use crate::{
    compat::{FontIcon, MenuFlyoutItem},
    controls::*,
};
use ferroui_base::{Ref, data::core::ValueTypes};
use ferroui_controls::{Control, MenuFlyout, StackPanel, TextBlock, ToolTip};
use serde_json::{Value, json};
use std::cell::{Cell, RefCell};
#[repr(C)]
struct DefaultOverflow {
    base: RibbonControlBase,
    calls: Cell<u32>,
    trace: RefCell<Vec<Option<String>>>,
}
ferroui_base::ferro_class!(DefaultOverflow:RibbonControlBase);
ferroui_base::ferro_class_info!(DefaultOverflow {});
ferroui_base::ferro_impl_classes!(DefaultOverflow:ferroui_base::FerroObjectImpl,ferroui_base::StyledElementImpl,ferroui_base::VisualImpl,ferroui_base::layout::LayoutableImpl,ferroui_base::interactivity::InteractiveImpl,ferroui_base::input::InputElementImpl,ferroui_controls::ControlImpl,ferroui_controls::primitives::TemplatedControlImpl);
impl RibbonControlBaseImpl for DefaultOverflow {
    fn invoke_core(this: &Self) -> bool {
        this.calls.set(this.calls.get() + 1);
        this.trace.borrow_mut().push(this.label());
        false
    }
}
fn glyph(menu: &MenuFlyoutItem) -> Ref<FontIcon> {
    ValueTypes::as_object(&*menu.icon().unwrap())
        .unwrap()
        .cast::<FontIcon>()
        .unwrap()
}
#[test]
fn default_base_overflow_matches_source_snapshots_identity_and_virtual_invocation() {
    crate::group_tests::run(|| {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/control-base-overflow.json"
        ))
        .unwrap();
        for expected in fixture["corpus"]["cases"].as_array().unwrap() {
            let source = ferroui_base::instantiate(DefaultOverflow {
                base: RibbonControlBase::construct(),
                calls: Cell::new(0),
                trace: RefCell::new(vec![]),
            });
            source.set_label(expected["label"].as_str().map(str::to_owned));
            source.set_is_enabled(expected["enabled"].as_bool().unwrap());
            source.set_shortcut(Some("  ".into()));
            let icon = FontIcon::new();
            icon.set_glyph(Some("g".into()));
            source.set_icon(Some(Control::boxed(icon.clone())));
            let sequence = source.create_overflow_menu_items_core();
            assert_eq!(sequence.iter().count(), 1);
            let menu = sequence
                .iter()
                .next()
                .unwrap()
                .unwrap()
                .cast::<MenuFlyoutItem>()
                .unwrap();
            let initial = json!({"text":menu.text(),"icon":glyph(&menu).glyph(),"newIcon":!glyph(&menu).ptr_eq(&icon),"shortcut":menu.keyboard_accelerator_text_override(),"IsEnabled":menu.is_enabled(),"Calls":source.calls.get()});
            assert_eq!(initial, expected["initial"]);
            assert_eq!(
                json!(sequence.iter().next().unwrap().unwrap().ptr_eq(&menu)),
                expected["same"]
            );
            source.set_label(Some("Changed".into()));
            source.set_shortcut(Some("other".into()));
            let next_icon = FontIcon::new();
            next_icon.set_glyph(Some("h".into()));
            source.set_icon(Some(Control::boxed(next_icon)));
            source.set_is_enabled(!expected["enabled"].as_bool().unwrap());
            menu.invoke_menu_core();
            let after = json!({"text":menu.text(),"icon":glyph(&menu).glyph(),"shortcut":menu.keyboard_accelerator_text_override(),"IsEnabled":menu.is_enabled(),"Calls":source.calls.get(),"Trace":*source.trace.borrow()});
            assert_eq!(after, expected["after"]);
            let next = source
                .create_overflow_menu_items_core()
                .iter()
                .next()
                .unwrap()
                .unwrap()
                .cast::<MenuFlyoutItem>()
                .unwrap();
            assert_eq!(
                json!({"text":next.text(),"icon":glyph(&next).glyph(),"shortcut":next.keyboard_accelerator_text_override(),"IsEnabled":next.is_enabled(),"different":!next.ptr_eq(&menu)}),
                expected["second"]
            );
        }
    });
}
#[test]
fn default_base_overflow_retains_source_only_for_the_entry_lifetime() {
    crate::group_tests::run(|| {
        let source = ferroui_base::instantiate(DefaultOverflow {
            base: RibbonControlBase::construct(),
            calls: Cell::new(0),
            trace: RefCell::new(vec![]),
        });
        let weak = source.downgrade();
        let sequence = source.create_overflow_menu_items_core();
        let entry = sequence
            .iter()
            .next()
            .unwrap()
            .unwrap()
            .cast::<MenuFlyoutItem>()
            .unwrap();
        drop(source);
        drop(sequence);
        assert!(weak.upgrade().is_some());
        entry.invoke_menu_core();
        assert_eq!(weak.upgrade().unwrap().calls.get(), 1);
        drop(entry);
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn overflow_and_screen_tips_follow_effective_ribbon_command_enablement() {
    crate::group_tests::run(|| {
        let ribbon = Ribbon::new();
        let tab = RibbonTab::new();
        let group = RibbonGroup::new();
        let button = RibbonSplitButton::new();
        button.set_label(Some("Overflow enablement probe".into()));
        let tip = RibbonScreenTip::new();
        tip.set_disabled_reason(Some("Group unavailable".into()));
        button.set_screen_tip(Some(Control::boxed(tip)));
        group.items().add(Some(button.clone().upcast()));
        tab.groups().add(Some(group.clone()));
        ribbon.tabs().add(Some(tab));
        let window = crate::ribbon_tests::mount(&ribbon);
        assert!(button.visual_parent().is_some());

        let create = |source: &Control| {
            RibbonItemHelper::create_menu_item(source, Some("Probe".into()), None, None, None)
                .unwrap()
        };
        let reason = || {
            let tooltip = ToolTip::get_tip(&button).unwrap();
            let wrapper = ValueTypes::as_object(&*tooltip)
                .unwrap()
                .cast::<ToolTip>()
                .unwrap();
            let content = wrapper.content().unwrap();
            ValueTypes::as_object(&*content)
                .unwrap()
                .cast::<RibbonScreenTip>()
                .unwrap()
                .disabled_reason()
        };
        let state = |enabled, expected_reason| {
            assert_eq!(button.is_effectively_enabled(), enabled);
            assert_eq!(create(&button).is_enabled(), enabled, "overflow snapshot");
            let overflow = button
                .create_overflow_menu_items_core()
                .iter()
                .next()
                .unwrap()
                .unwrap();
            assert_eq!(overflow.is_enabled(), enabled, "split-button overflow");
            assert_eq!(button.invoke_core(), enabled, "split-button invocation");
            assert_eq!(reason().as_deref(), expected_reason, "automatic update");
            RibbonItemHelper::update_tool_tip(button.clone()).unwrap();
            assert_eq!(reason().as_deref(), expected_reason, "explicit update");
        };

        let initially_enabled = create(&button);
        state(true, None);
        group.set_is_enabled(false);
        let initially_disabled = create(&button);
        state(false, Some("Group unavailable"));
        button.set_is_enabled(true);
        state(false, Some("Group unavailable"));

        let panel = StackPanel::new();
        let text = TextBlock::new();
        panel.children().add(text.clone());
        panel.set_is_enabled(false);
        assert!(!text.is_effectively_enabled());
        assert!(create(&text).is_enabled());

        group.set_is_enabled(true);
        state(true, None);
        assert!(initially_enabled.is_enabled());
        assert!(!initially_disabled.is_enabled());
        button.set_is_enabled(false);
        state(false, Some("Group unavailable"));

        button.set_is_enabled(true);
        let flyout = MenuFlyout::new();
        let entry = MenuFlyoutItem::new();
        entry.set_text(Some("Choice".into()));
        flyout.items().add(Some(Control::boxed(entry)));
        button.set_flyout(Some(flyout.upcast()));
        group.set_is_enabled(false);
        button.open_drop_down();
        assert!(!button.is_drop_down_open());
        group.set_is_enabled(true);
        button.open_drop_down();
        assert!(button.is_drop_down_open());
        button.close_drop_down();
        window.close();
    });
}
