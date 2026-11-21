use crate::controls::{
    IRibbonItem, IRibbonItemEditable, RibbonControlBase, RibbonControlBaseImpl, RibbonItemLayout,
    RibbonSimplifiedLabel,
};
use crate::primitives::RibbonMetrics;
fn run(test: impl FnOnce() + Send + 'static) {
    crate::tests::run(move || {
        let app = ferroui_controls::Application::current().unwrap();
        let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
        app.styles().add(theme.clone());
        test();
        app.styles().remove(theme);
    })
}
use ferroribbon_core::{RibbonGroupState, RibbonItemSize, RibbonSimplifiedVisibility};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, VisualImpl,
    ferro_class, ferro_impl_classes, instantiate,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ControlImpl, ToolTip, automation::AutomationProperties};
use std::{cell::RefCell, rc::Rc, sync::Arc};

#[test]
fn composite_ribbon_items_expose_live_named_group_peers() {
    run(|| {
        use crate::controls::{RibbonComboBox, RibbonGallery, RibbonSplitButton};
        use ferroui_controls::{
            TextBlock,
            automation::peers::{AutomationControlType, ControlAutomationPeer},
        };

        let editor = RibbonComboBox::new();
        let peer = ControlAutomationPeer::create_peer_for_element(&editor);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::None
        );
        assert!(!peer.is_control_element());
        assert!(!peer.is_content_element());

        editor.set_id(Some("editor".into()));
        editor.set_label(Some("Font\nFamily".into()));
        editor.set_key_tip(Some("FF".into()));
        editor.set_shortcut(Some("Ctrl+D".into()));
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Group
        );
        assert_eq!(peer.get_class_name(), "NamedContainerAutomationPeer");
        assert_eq!(peer.get_automation_id().as_deref(), Some("editor"));
        assert_eq!(peer.get_name(), "Font Family");
        assert_eq!(peer.get_access_key().as_deref(), Some("FF"));
        assert_eq!(peer.get_accelerator_key().as_deref(), Some("Ctrl+D"));
        assert!(peer.is_control_element());
        assert!(peer.is_content_element());
        assert!(!peer.is_keyboard_focusable());

        editor.set_label(Some("Typeface".into()));
        assert_eq!(peer.get_name(), "Typeface");
        editor.set_label(None);
        assert!(!peer.is_control_element());
        let label = TextBlock::new();
        label.set_text(Some("Settings"));
        AutomationProperties::set_labeled_by(&editor, Some(label.clone().upcast()));
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Group
        );
        assert_eq!(peer.get_name(), "Settings");
        assert!(peer.is_control_element());
        AutomationProperties::set_labeled_by(&editor, None);
        assert!(!peer.is_control_element());

        let gallery = RibbonGallery::new();
        gallery.set_label(Some("Styles".into()));
        let gallery_peer = ControlAutomationPeer::create_peer_for_element(&gallery);
        assert_eq!(
            gallery_peer.get_automation_control_type(),
            AutomationControlType::Group
        );
        assert_eq!(gallery_peer.get_name(), "Styles");

        let split = RibbonSplitButton::new();
        split.set_label(Some("Paste".into()));
        let split_peer = ControlAutomationPeer::create_peer_for_element(&split);
        assert_eq!(
            split_peer.get_automation_control_type(),
            AutomationControlType::SplitButton
        );
        assert_eq!(split_peer.get_name(), "Paste");
    });
}

#[repr(C)]
struct Item {
    base: RibbonControlBase,
    trace: RefCell<Vec<String>>,
}
ferro_class!(Item:RibbonControlBase);
ferro_impl_classes!(Item:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl Item {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonControlBase::construct(),
            trace: RefCell::new(vec![]),
        })
    }
}
impl RibbonControlBaseImpl for Item {
    fn on_ribbon_property_changed(this: &Self, e: &FerroPropertyChangedEventArgs<'_>) {
        this.trace.borrow_mut().push(format!(
            "{}:{}:{}",
            e.property().name(),
            this.current_size().raw(),
            this.actual_show_label()
        ));
        if e.property() == RibbonControlBase::label_property().as_property()
            && this.label().as_deref() == Some("Paste")
        {
            assert_eq!(
                this.get_value(AutomationProperties::name_property())
                    .as_deref(),
                this.label().as_deref()
            );
        }
    }
    fn on_layout_applied_core(this: &Self, layout: RibbonItemLayout) {
        assert_eq!(this.current_size(), layout.size);
        assert_eq!(this.metrics(), layout.metrics);
        assert_eq!(this.is_simplified(), layout.is_simplified);
        this.trace.borrow_mut().push("layout".into());
    }
}
fn item(control: &Item) -> Rc<dyn IRibbonItem> {
    let boxed: BoxedValue = Rc::new(control.to_ref());
    ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItem>>())
        .flatten()
        .unwrap()
        .downcast_ref::<Rc<dyn IRibbonItem>>()
        .unwrap()
        .clone()
}
fn editable(control: &Item) -> Rc<dyn IRibbonItemEditable> {
    let boxed: BoxedValue = Rc::new(control.to_ref());
    ValueTypes::try_convert(Some(&boxed), ValueType::of::<Rc<dyn IRibbonItemEditable>>())
        .flatten()
        .unwrap()
        .downcast_ref::<Rc<dyn IRibbonItemEditable>>()
        .unwrap()
        .clone()
}
#[test]
fn generated_common_defaults_and_raw_versus_effective_ids_use_actual_native_base() {
    run(|| {
        let p = Item::new();
        let i = item(&p);
        let e = editable(&p);
        assert_eq!(p.id(), None);
        assert_eq!(p.label(), None);
        assert_eq!(p.icon(), None);
        assert_eq!(p.large_icon(), None);
        assert_eq!(p.size(), RibbonItemSize::Medium);
        assert_eq!(p.current_size(), RibbonItemSize::Medium);
        assert!(!p.is_simplified());
        assert!(p.actual_show_label());
        assert!(p.show_label());
        assert!(p.can_add_to_quick_access());
        assert_eq!(p.simplified_visibility(), RibbonSimplifiedVisibility::Auto);
        assert_eq!(p.simplified_label(), RibbonSimplifiedLabel::Auto);
        assert!(Arc::ptr_eq(
            p.metrics().as_ref().unwrap(),
            &RibbonMetrics::comfortable()
        ));
        assert!(!p.show_label_in_simplified_mode());
        assert!(i.create_linked_copy().is_none());
        p.set_name(Some("named".into()));
        assert_eq!(i.id().unwrap().to_utf8().unwrap(), "named");
        assert_eq!(IRibbonItemEditable::id(&*e), None);
        e.set_id(Some("".into()));
        assert_eq!(i.id().unwrap().to_utf8().unwrap(), "named");
        e.set_id(Some("explicit".into()));
        assert_eq!(i.id(), Some("explicit".into()));
        assert_eq!(
            p.get_value(AutomationProperties::automation_id_property())
                .as_deref(),
            Some("named")
        );
        p.set_id(Some("later".into()));
        assert_eq!(
            p.get_value(AutomationProperties::automation_id_property())
                .as_deref(),
            Some("named")
        );
    })
}
#[test]
fn apply_layout_source_order_locks_preferred_size_and_dispatches_real_virtual_hook() {
    run(|| {
        let p = Item::new();
        p.set_size(RibbonItemSize::Large);
        assert_eq!(p.current_size(), RibbonItemSize::Large);
        assert_eq!(&*p.trace.borrow(), &["Size:0:true"]);
        p.trace.borrow_mut().clear();
        let trace = Rc::new(RefCell::new(vec![]));
        let events = trace.clone();
        p.add_property_changed_handler(Rc::new(move |e| {
            events.borrow_mut().push(e.property().name().to_owned())
        }));
        let metrics = RibbonMetrics::compact();
        item(&p).apply_layout(RibbonItemLayout {
            size: RibbonItemSize::Small,
            metrics: Some(metrics.clone()),
            is_simplified: true,
            show_label: Some(false),
            group_state: RibbonGroupState::Collapsed,
        });
        assert_eq!(
            &*trace.borrow(),
            &["Metrics", "IsSimplified", "CurrentSize", "ActualShowLabel"]
        );
        assert_eq!(&*p.trace.borrow(), &["layout"]);
        assert!(Arc::ptr_eq(p.metrics().as_ref().unwrap(), &metrics));
        p.set_size(RibbonItemSize::Medium);
        assert_eq!(p.current_size(), RibbonItemSize::Small);
        p.set_current_size(RibbonItemSize::Large);
        p.set_size(RibbonItemSize::Small);
        assert_eq!(p.current_size(), RibbonItemSize::Large);
        p.apply_layout(RibbonItemLayout::default());
        assert_eq!(p.metrics(), None);
        assert_eq!(p.current_size(), RibbonItemSize::Large);
    })
}
#[test]
fn common_label_modes_size_definitions_unknown_enums_and_direct_presentation_setters() {
    run(|| {
        let p = Item::new();
        for size in [
            RibbonItemSize::Large,
            RibbonItemSize::Medium,
            RibbonItemSize::Small,
            RibbonItemSize::from_raw(42),
        ] {
            p.set_size(size);
            for mode in [
                RibbonSimplifiedLabel::Auto,
                RibbonSimplifiedLabel::Show,
                RibbonSimplifiedLabel::Hide,
                RibbonSimplifiedLabel::from_raw(42),
            ] {
                p.set_simplified_label(mode);
                let expected = mode == RibbonSimplifiedLabel::Show
                    || (mode != RibbonSimplifiedLabel::Hide
                        && mode != RibbonSimplifiedLabel::Show
                        && size == RibbonItemSize::Large);
                assert_eq!(p.show_label_in_simplified_mode(), expected);
                p.apply_layout(RibbonItemLayout {
                    size: RibbonItemSize::Medium,
                    metrics: None,
                    is_simplified: true,
                    show_label: None,
                    group_state: RibbonGroupState::Large,
                });
                assert_eq!(p.actual_show_label(), expected);
            }
            for text in [
                None,
                Some(""),
                Some("bad"),
                Some("Large, Medium, Small"),
                Some("Small,Small,Small"),
            ] {
                p.set_size_definition(text.map(Into::into));
                let expected = ferroribbon_core::layout::RibbonSizeDefinition::try_parse(text)
                    .unwrap_or_else(|| {
                        ferroribbon_core::layout::RibbonSizeDefinition::for_preferred_size(size)
                    });
                assert_eq!(item(&p).effective_size_definition(), expected);
            }
        }
        p.trace.borrow_mut().clear();
        p.set_current_size(RibbonItemSize::from_raw(71));
        p.set_metrics(None);
        p.set_is_simplified(false);
        p.set_actual_show_label(false);
        p.set_can_add_to_quick_access(false);
        assert!(p.trace.borrow().is_empty());
    })
}
#[test]
fn native_common_tooltip_refresh_order_ownership_and_incomplete_services_are_explicit() {
    run(|| {
        let p = Item::new();
        p.set_label(Some("Paste".into()));
        assert_eq!(&*p.trace.borrow(), &["Label:1:true"]);
        assert!(p.get_value(ToolTip::tip_property()).is_some());
        p.set_key_tip(Some("V".into()));
        p.set_shortcut(Some("Ctrl+V".into()));
        assert_eq!(
            p.get_value(AutomationProperties::access_key_property())
                .as_deref(),
            Some("V")
        );
        assert_eq!(
            p.get_value(AutomationProperties::accelerator_key_property())
                .as_deref(),
            Some("Ctrl+V")
        );
        p.set_value(
            AutomationProperties::name_property(),
            Some("Application".to_owned()),
        );
        p.set_label(Some("Changed".into()));
        assert_eq!(
            p.get_value(AutomationProperties::name_property())
                .as_deref(),
            Some("Application")
        );
        assert!(!p.execute_command(None).unwrap());
    })
}

#[test]
fn reentrant_layout_keeps_source_outer_write_order_and_locks_before_metrics_notification() {
    run(|| {
        let p = instantiate(RibbonControlBase::construct());
        p.set_size(RibbonItemSize::Large);
        let inner = RibbonMetrics::touch();
        let inner_for_handler = inner.clone();
        let weak = p.downgrade();
        let entered = Rc::new(std::cell::Cell::new(false));
        let entered_for_handler = entered.clone();
        p.add_property_changed_handler(Rc::new(move |e| {
            if e.property() == RibbonControlBase::metrics_property().as_property()
                && !entered_for_handler.replace(true)
            {
                let p = weak.upgrade().unwrap();
                p.set_size(RibbonItemSize::Medium);
                assert_eq!(p.current_size(), RibbonItemSize::Large);
                p.apply_layout(RibbonItemLayout {
                    size: RibbonItemSize::Medium,
                    metrics: Some(inner_for_handler.clone()),
                    is_simplified: false,
                    show_label: Some(true),
                    group_state: RibbonGroupState::Medium,
                });
            }
        }));
        p.apply_layout(RibbonItemLayout {
            size: RibbonItemSize::Small,
            metrics: Some(RibbonMetrics::compact()),
            is_simplified: true,
            show_label: Some(false),
            group_state: RibbonGroupState::Small,
        });
        assert!(entered.get());
        assert!(Arc::ptr_eq(p.metrics().as_ref().unwrap(), &inner));
        assert!(p.is_simplified());
        assert_eq!(p.current_size(), RibbonItemSize::Small);
        assert!(!p.actual_show_label());
    })
}
