//! Source-scenario adaptations for the actual group. This is not the upstream
//! Uno/WinUI runtime and does not verify the missing owning Ribbon/CAD paths.
use crate::{
    controls::{RibbonButton, RibbonGroup},
    primitives::RibbonMetrics,
};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{RibbonGroupState, RibbonItemSize, collections::RibbonReadOnlyList};
use ferroui_base::{
    Rect, Ref, Size, StaticType, controls::ResourceDictionary, data::core::ValueTypes,
    styling::ControlTheme,
};
use ferroui_controls::{Application, Control, Panel, Window};
use std::{cell::RefCell, rc::Rc, sync::Arc};
pub(crate) fn run(test: impl FnOnce() + Send + 'static) {
    crate::tests::run(move || {
        let app = Application::current().unwrap();
        let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
        app.styles().add(theme.clone());
        test();
        app.styles().remove(theme);
    })
}
fn set_group_theme(group: &RibbonGroup) {
    let value = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Groups.xaml")
        .unwrap()
        .unwrap();
    let dictionary = ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    let theme = ValueTypes::as_object(
        &*dictionary
            .try_get_resource(&<RibbonGroup as StaticType>::TYPE.into(), None)
            .flatten()
            .unwrap(),
    )
    .unwrap()
    .cast::<ControlTheme>()
    .unwrap();
    group.set_resources(dictionary);
    group.set_theme(theme);
}
fn mount(group: &RibbonGroup) -> Ref<Window> {
    set_group_theme(group);
    let w = Window::new();
    w.set_content(Some(Control::boxed(group.to_ref())));
    w.show();
    w.measure(Size::new(800., 300.));
    w.arrange(Rect::new(0., 0., 800., 300.));
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    w
}
#[test]
fn source_group_incremental_children_visibility_subscriptions_and_invalidation() {
    run(|| {
        let group = RibbonGroup::new();
        assert_eq!(group.state(), RibbonGroupState::Large);
        assert!(!group.is_simplified());
        assert_eq!(group.row_count(), 3);
        assert!(group.can_collapse());
        assert_eq!(group.reduction_order(), 0);
        assert!(group.is_group_visible());
        assert!(group.items() == group.items());
        let previous = Panel::new();
        let a = RibbonButton::new();
        a.set_label(Some("A".into()));
        previous.children().add(a.clone());
        group.items().add(Some(a.clone().upcast()));
        assert_eq!(previous.children().count(), 0);
        assert!(group.items_panel().children().get(0).ptr_eq(&a));
        let w = mount(&group);
        assert!(a.is_loaded());
        let b = RibbonButton::new();
        group.items().add(Some(b.clone().upcast()));
        assert!(group.items_panel().children().get(0).ptr_eq(&a));
        assert!(a.is_loaded());
        group.items().move_item(1, 0);
        assert!(group.items_panel().children().get(0).ptr_eq(&b));
        assert!(group.items_panel().children().get(1).ptr_eq(&a));
        let before = group.layout_version();
        a.set_is_visible(false);
        assert_eq!(group.layout_version(), before.wrapping_add(1));
        group.items().remove(&Some(a.clone().upcast()));
        let before = group.layout_version();
        a.set_is_visible(true);
        assert_eq!(group.layout_version(), before);
        assert!(a.visual_parent().is_none());
        group.set_is_group_visible(false);
        assert!(!group.is_visible());
        group.set_is_group_visible(true);
        assert!(group.is_visible());
        w.close();
    })
}
#[test]
fn source_group_state_cache_metric_identity_mask_equivalence_and_item_sizes() {
    run(|| {
        let group = RibbonGroup::new();
        let button = RibbonButton::new();
        button.set_size_definition(Some("Large,Medium,Small".into()));
        group.items().add(Some(button.clone().upcast()));
        let metrics = RibbonMetrics::touch();
        group.apply_state(RibbonGroupState::Medium, false, metrics.clone(), None);
        assert_eq!(button.current_size(), RibbonItemSize::Medium);
        assert!(Arc::ptr_eq(&group.metrics(), &metrics));
        assert_eq!(group.items_panel().row_count(), 3);
        // Source cache deliberately omits RowCount: invalidation does not reset _state.
        group.set_row_count(2);
        group.apply_state(RibbonGroupState::Medium, false, metrics.clone(), None);
        assert_eq!(group.items_panel().row_count(), 3);
        group.apply_state(RibbonGroupState::Small, false, metrics.clone(), None);
        assert_eq!(group.items_panel().row_count(), 2);
        assert_eq!(button.current_size(), RibbonItemSize::Small);
        group.apply_state(RibbonGroupState::Collapsed, false, metrics.clone(), None);
        assert_eq!(button.current_size(), RibbonItemSize::Large);
        let values = RibbonFixedItems::new(vec![true]);
        let mask = values.clone() as RibbonReadOnlyList<_>;
        group.apply_state(
            RibbonGroupState::Large,
            true,
            metrics.clone(),
            Some(mask.clone()),
        );
        assert_eq!(button.current_size(), RibbonItemSize::Small);
        assert!(button.is_simplified());
        let equivalent = RibbonFixedItems::new(vec![true]) as RibbonReadOnlyList<_>;
        group.apply_state(RibbonGroupState::Large, true, metrics, Some(equivalent));
        assert!(Rc::ptr_eq(
            &group.items_panel().in_line_mask().unwrap(),
            &mask
        ));
        values.set(0, false);
        assert!(!group.items_panel().is_in_line(0));
    })
}
#[test]
fn source_group_templates_and_collapsed_popup_move_same_panel_and_restore_latest_state() {
    run(|| {
        let group = RibbonGroup::new();
        group.set_header(Some("Clipboard".into()));
        let button = RibbonButton::new();
        button.set_size_definition(Some("Large,Medium,Small".into()));
        group.items().add(Some(button.clone().upcast()));
        group.open_popup();
        assert!(!group.is_popup_open());
        let w = mount(&group);
        let panel = group.items_panel();
        let original = panel.visual_parent().unwrap();
        let metrics = RibbonMetrics::comfortable();
        group.apply_state(RibbonGroupState::Collapsed, false, metrics.clone(), None);
        assert!(group.collapsed_button().unwrap().is_visible());
        assert_eq!(
            group.collapsed_button().unwrap().size(),
            RibbonItemSize::Large
        );
        group.open_popup();
        assert!(group.is_popup_open());
        assert!(!panel.visual_parent().unwrap().ptr_eq(&original));
        assert!(panel.children().get(0).ptr_eq(&button));
        assert_eq!(button.current_size(), RibbonItemSize::Large);
        group.apply_state(RibbonGroupState::Medium, false, metrics.clone(), None);
        assert_eq!(button.current_size(), RibbonItemSize::Large);
        group.close_popup();
        assert!(!group.is_popup_open());
        assert_eq!(button.current_size(), RibbonItemSize::Medium);
        group.open_popup();
        assert_eq!(button.current_size(), RibbonItemSize::Large);
        set_group_theme(&group);
        group.apply_styling();
        group.apply_template();
        assert!(group.is_popup_open());
        assert!(panel.children().get(0).ptr_eq(&button));
        group.close_popup();
        assert!(!group.is_popup_open());
        assert_eq!(group.state(), RibbonGroupState::Medium); // Source SyncItems in OnApplyTemplate resets _state: closing now keeps the popup item sizes.
        assert_eq!(button.current_size(), RibbonItemSize::Large);
        group.apply_state(RibbonGroupState::Medium, false, metrics, None);
        assert_eq!(button.current_size(), RibbonItemSize::Medium);
        assert!(!panel.visual_parent().unwrap().ptr_eq(&original));
        assert!(panel.children().get(0).ptr_eq(&button));
        group.set_template(None);
        group.apply_template();
        assert!(panel.visual_parent().is_none());
        w.close();
    })
}
#[test]
fn source_group_overflow_query_is_lazy_live_and_stops_after_end() {
    run(|| {
        let group = RibbonGroup::new();
        let first = RibbonButton::new();
        let second = RibbonButton::new();
        group.items().add(Some(first.clone().upcast()));
        group.items().add(Some(second.clone().upcast()));
        let values = RibbonFixedItems::new(vec![true, false]);
        group.apply_state(
            RibbonGroupState::Large,
            true,
            RibbonMetrics::comfortable(),
            Some(values.clone() as RibbonReadOnlyList<_>),
        );
        let query = group.get_overflow_items();
        let mut cursor = query.iter();
        values.set(0, false);
        assert!(cursor.next().unwrap().ptr_eq(&first));
        second.set_is_visible(false);
        assert!(cursor.next().is_none());
        second.set_is_visible(true);
        assert!(cursor.next().is_none());
        assert_eq!(query.iter().count(), 2);
        first.set_simplified_visibility(ferroribbon_core::RibbonSimplifiedVisibility::Overflow);
        values.set(0, true);
        assert!(query.iter().next().unwrap().ptr_eq(&first));
    })
}
type HandlerFn =
    dyn Fn(Option<ferroui_base::BoxedValue>, ferroui_base::interactivity::RoutedEventArgs);
struct Handler(Box<HandlerFn>);
impl PartialEq for Handler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class Handler {this:Rc<Handler>,handles:[Rc<Handler>],methods:[fn Invoke(Option<ferroui_base::BoxedValue>,ferroui_base::interactivity::RoutedEventArgs)=>|handler:&Rc<Handler>,sender,args|(handler.0)(sender,args)]});
fn handler(
    action: impl Fn(Option<ferroui_base::BoxedValue>, ferroui_base::interactivity::RoutedEventArgs)
    + 'static,
) -> ferroui_base::metadata::MarkupDelegate {
    use ferroui_base::metadata::MarkupTyped;
    ValueTypes::register_reference::<Handler>();
    ferroui_base::metadata::MarkupDelegate::for_method(
        Some(Rc::new(Handler(Box::new(action)))),
        Handler::MARKUP,
        &Handler::MARKUP.methods[0],
    )
}
#[test]
fn source_group_launcher_raw_multicast_order_command_replacement_and_parameter_reread() {
    run(|| {
        use ferroribbon_core::commands::RibbonRelayCommand;
        let group = RibbonGroup::new();
        group.set_header(Some("Font".into()));
        group.set_is_dialog_launcher_visible(true);
        let w = mount(&group);
        assert!(group.dialog_launcher().unwrap().is_visible());
        let trace = Rc::new(RefCell::new(vec![]));
        let t = trace.clone();
        let weak = group.downgrade();
        let can_trace = trace.clone();
        let command = RibbonRelayCommand::new_with_parameter(
            move |parameter| {
                t.borrow_mut().push(format!(
                    "execute:{}",
                    parameter.unwrap().downcast_ref::<i32>().unwrap()
                ))
            },
            Some(Rc::new(move |parameter| {
                let value = *parameter.unwrap().downcast_ref::<i32>().unwrap();
                assert!(value == 9 || value == 11);
                can_trace.borrow_mut().push(format!("can:{value}"));
                weak.upgrade()
                    .unwrap()
                    .set_dialog_launcher_command_parameter(Some(Rc::new(11i32)));
                true
            })),
        );
        let command = crate::commands::adapt_core_command(command);
        let weak = group.downgrade();
        let t = trace.clone();
        let b = handler(move |_, _| t.borrow_mut().push("B".into()));
        let remove = b.clone();
        let t = trace.clone();
        let a = handler(move |sender, args| {
            let group = weak.upgrade().unwrap();
            assert!(
                ValueTypes::as_object(&*sender.unwrap())
                    .unwrap()
                    .ptr_eq(&group)
            );
            t.borrow_mut().push("A".into());
            args.set_handled(true);
            group.remove_dialog_launcher_click(Some(remove.clone()));
            group.set_dialog_launcher_command(Some(command.clone()));
            group.set_dialog_launcher_command_parameter(Some(Rc::new(9i32)));
        });
        group.add_dialog_launcher_click(Some(a.clone()));
        group.add_dialog_launcher_click(Some(b));
        group.open_dialog_launcher().unwrap();
        assert_eq!(
            &*trace.borrow(),
            &["A", "B", "can:9", "can:11", "execute:11"]
        );
        group.remove_dialog_launcher_click(Some(a));
        w.close();
    })
}
#[test]
fn source_collapsed_popup_escape_closes_and_restores_keyboard_focus() {
    run(|| {
        use ferroui_base::input::{InputElement, Key, KeyEventArgs};
        let group = RibbonGroup::new();
        group.items().add(Some(RibbonButton::new().upcast()));
        let w = mount(&group);
        group.apply_state(
            RibbonGroupState::Collapsed,
            false,
            RibbonMetrics::comfortable(),
            None,
        );
        group.open_popup();
        assert!(group.is_popup_open());
        let mut e = KeyEventArgs::new();
        e.key = Key::Escape;
        e.set_routed_event(Some(InputElement::key_down_event()));
        group.items_panel().raise_event(&e);
        assert!(e.handled());
        assert!(!group.is_popup_open());
        assert!(group.collapsed_button().unwrap().is_focused());
        w.close();
    })
}
#[test]
fn popup_placement_metadata_null_order_and_default_arguments_use_native_helper() {
    run(|| {
        use crate::primitives::RibbonPopupPlacement;
        use ferroui_base::metadata::{MarkupInvokeError, MarkupTyped};
        use ferroui_controls::primitives::Popup;
        let methods = <RibbonPopupPlacement as MarkupTyped>::MARKUP.methods;
        assert_eq!(methods.len(), 3);
        let error = (methods[0].invoke)(&[None, None]).unwrap_err();
        assert!(
            matches!(error, MarkupInvokeError::Failed(ref message) if message.ends_with("(Parameter 'popup')"))
        );
        let popup = Popup::new();
        let error = (methods[0].invoke)(&[Some(Control::boxed(popup.clone())), None]).unwrap_err();
        assert!(
            matches!(error, MarkupInvokeError::Failed(ref message) if message.ends_with("(Parameter 'anchor')"))
        );
        let anchor = Panel::new();
        let child = Panel::new();
        child.set_width(30.);
        child.set_height(20.);
        popup.set_child(child.clone());
        (methods[0].invoke)(&[
            Some(Control::boxed(popup.clone())),
            Some(Control::boxed(anchor)),
        ])
        .unwrap();
        assert_eq!(popup.horizontal_offset(), 0.);
        assert_eq!(popup.vertical_offset(), 2.);
        assert_eq!(child.desired_size(), Size::new(30., 20.));
    });
}

#[test]
fn synchronous_source_state_width_probes_refresh_nested_template_parts_without_dispatch() {
    run(|| {
        let group = RibbonGroup::new();
        group.set_header(Some("Tools".into()));
        let b = RibbonButton::new();
        b.set_width(400.);
        b.set_label(Some("Wide".into()));
        group.items().add(Some(b.upcast()));
        let w = mount(&group);
        let metrics = RibbonMetrics::comfortable();
        let large = group.measure_width(RibbonGroupState::Large, metrics.clone(), 120.);
        let collapsed = group.measure_width(RibbonGroupState::Collapsed, metrics.clone(), 120.);
        assert!(large >= 400., "visible wide item: {large}");
        assert!(
            collapsed < 150.,
            "collapsed header must exclude item: {collapsed}"
        );
        for _ in 0..4 {
            assert_eq!(
                group.measure_width(RibbonGroupState::Large, metrics.clone(), 120.),
                large
            );
            assert_eq!(
                group.measure_width(RibbonGroupState::Collapsed, metrics.clone(), 120.),
                collapsed
            );
        }
        w.close();
    });
}
