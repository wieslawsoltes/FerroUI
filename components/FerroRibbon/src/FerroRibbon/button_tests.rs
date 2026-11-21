use crate::controls::{
    IRibbonItem, IRibbonItemOwner, RibbonButton, RibbonDropDownButton, RibbonItemHelper,
    RibbonItemLayout,
};
use crate::primitives::RibbonMetrics;
use ferroribbon_core::{
    RibbonItemSize, RibbonString,
    commands::{RibbonCommandCatalog, RibbonCommandDescriptor, RibbonRelayCommand},
};
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectImpl, Rect, Ref, Size, StaticType, StyledElementImpl,
    VisualImpl,
    controls::ResourceDictionary,
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    styling::ControlTheme,
    threading::Dispatcher,
};
use ferroui_controls::{Application, Control, ControlImpl, Panel, PanelImpl, Window};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
fn run(test: impl FnOnce() + Send + 'static) {
    crate::tests::run(move || {
        let app = Application::current().unwrap();
        let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
        app.styles().add(theme.clone());
        test();
        app.styles().remove(theme);
    })
}
type Invocation = (Ref<Control>, Option<RibbonString>, Option<i32>);
#[repr(C)]
struct Owner {
    base: Panel,
    catalog: RefCell<Option<Rc<RibbonCommandCatalog>>>,
    invoked: RefCell<Vec<Invocation>>,
    on_invoke: RefCell<Option<Rc<dyn Fn()>>>,
}
ferro_class!(Owner:Panel);
ferro_impl_classes!(Owner:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_class_info!(Owner{interfaces:[Rc<dyn IRibbonItemOwner> =>|owner|Rc::new(OwnerView(owner)) as Rc<dyn IRibbonItemOwner>]});
impl Owner {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Panel::construct(),
            catalog: RefCell::new(None),
            invoked: RefCell::new(vec![]),
            on_invoke: RefCell::new(None),
        })
    }
}
struct OwnerView(Ref<Owner>);
impl IRibbonItemOwner for OwnerView {
    fn owner_ribbon(&self) -> Option<Ref<crate::controls::Ribbon>> {
        None
    }
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn metrics(&self) -> Option<Arc<RibbonMetrics>> {
        Some(RibbonMetrics::touch())
    }
    fn command_catalog(&self) -> Option<Rc<RibbonCommandCatalog>> {
        self.0.catalog.borrow().clone()
    }
    fn on_item_invoked(
        &self,
        item: Ref<Control>,
        id: Option<RibbonString>,
        parameter: Option<BoxedValue>,
    ) {
        self.0.invoked.borrow_mut().push((
            item,
            id,
            parameter.and_then(|v| v.downcast_ref::<i32>().copied()),
        ));
        let callback = self.0.on_invoke.borrow().clone();
        if let Some(callback) = callback {
            callback();
        }
    }
}
fn item(button: &RibbonButton) -> Rc<dyn IRibbonItem> {
    let value = Control::boxed(button.to_ref());
    ValueTypes::try_convert(Some(&value), ValueType::of::<Rc<dyn IRibbonItem>>())
        .flatten()
        .unwrap()
        .downcast_ref::<Rc<dyn IRibbonItem>>()
        .unwrap()
        .clone()
}
fn mount(control: &Control) -> Ref<Window> {
    let w = Window::new();
    w.set_content(Some(Control::boxed(control.to_ref())));
    w.show();
    Dispatcher::ui_thread().run_jobs(None);
    w
}
#[test]
fn actual_push_button_linked_copy_invokes_original_click_command_and_owner_once() {
    run(|| {
        let owner = Owner::new();
        let source = RibbonButton::new();
        source.set_id(Some("cut".into()));
        source.set_label(Some("Cut".into()));
        source.set_command_parameter(Some(Rc::new(7i32)));
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        source.set_command(Some(crate::commands::adapt_core_command(
            RibbonRelayCommand::new(move || r.set(r.get() + 1)),
        )));
        let clicks = Rc::new(Cell::new(0));
        let c = clicks.clone();
        source.click(move |_, _| c.set(c.get() + 1));
        owner.children().add(source.clone().upcast::<Control>());
        let copy = item(&source)
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        assert!(copy.command().is_none());
        assert!(copy.command_parameter().is_none());
        owner.children().add(copy.clone().upcast::<Control>());
        let window = mount(&owner);
        copy.perform_click();
        assert_eq!(runs.get(), 1);
        assert_eq!(clicks.get(), 1);
        assert_eq!(owner.invoked.borrow().len(), 1);
        assert!(owner.invoked.borrow()[0].0.ptr_eq(&source));
        assert_eq!(owner.invoked.borrow()[0].2, Some(7));
        source.set_label(Some("Changed".into()));
        assert_eq!(copy.label().as_deref(), Some("Changed"));
        copy.set_label(Some("Copy".into()));
        assert_eq!(source.label().as_deref(), Some("Changed"));
        RibbonItemHelper::unlink(&copy);
        source.set_label(Some("Later".into()));
        assert_eq!(copy.label().as_deref(), Some("Copy"));
        window.close();
    })
}
#[test]
fn source_owner_explicit_precedence_and_self_exclusion_are_preserved() {
    run(|| {
        let parent = Owner::new();
        let explicit = Owner::new();
        let button = RibbonButton::new();
        parent.children().add(button.clone().upcast::<Control>());
        let w = mount(&parent);
        assert_eq!(
            RibbonItemHelper::find_owner(Some(&button))
                .unwrap()
                .as_object(),
            parent.clone().upcast::<Control>()
        );
        assert!(RibbonItemHelper::find_owner(Some(&parent)).is_none());
        RibbonItemHelper::set_owner(&button, Some(Rc::new(OwnerView(parent.clone())))).unwrap();
        assert!(
            RibbonItemHelper::get_owner(&button)
                .unwrap()
                .as_object()
                .ptr_eq(&parent)
        );
        RibbonItemHelper::set_owner(&button, Some(Rc::new(OwnerView(explicit.clone())))).unwrap();
        assert_eq!(
            RibbonItemHelper::find_owner(Some(&button))
                .unwrap()
                .as_object(),
            explicit.clone().upcast::<Control>()
        );
        button.perform_click();
        assert_eq!(explicit.invoked.borrow().len(), 1);
        assert!(parent.invoked.borrow().is_empty());
        RibbonItemHelper::set_owner(&button, None).unwrap();
        assert_eq!(
            RibbonItemHelper::find_owner(Some(&button))
                .unwrap()
                .as_object(),
            parent.clone().upcast::<Control>()
        );
        assert!(Arc::ptr_eq(
            button.metrics().as_ref().unwrap(),
            &RibbonMetrics::touch()
        ));
        w.close();
    })
}
#[test]
fn descriptor_replacement_application_command_protection_subscription_unload_and_removal() {
    run(|| {
        let owner = Owner::new();
        let catalog = RibbonCommandCatalog::new();
        owner.catalog.replace(Some(catalog.clone()));
        let button = RibbonButton::new();
        owner.children().add(button.clone().upcast::<Control>());
        let descriptor = RibbonCommandDescriptor::new(
            Some("copy".into()),
            None,
            Some(RibbonRelayCommand::new(|| {})),
        );
        catalog.register(Some(descriptor.clone()));
        button.set_command_id(Some("copy".into()));
        assert!(button.command().is_some());
        let own = RibbonRelayCommand::new(|| {});
        let own_native = crate::commands::adapt_core_command(own.clone());
        button.set_command(Some(own_native.clone()));
        descriptor.set_command(Some(RibbonRelayCommand::new(|| {})));
        assert!(button.command().as_ref() == Some(&own_native));
        button.set_command_id(None);
        assert!(button.command().as_ref() == Some(&own_native));
        button.set_command(None);
        button.set_command_id(Some("copy".into()));
        assert!(button.command().is_some());
        let before = descriptor.property_changed().handler_count();
        let window = mount(&owner);
        assert_eq!(descriptor.property_changed().handler_count(), before);
        descriptor.set_is_enabled(false);
        assert!(!button.is_enabled());
        descriptor.set_is_enabled(true);
        assert!(button.is_enabled());
        button.set_command_id(None);
        assert!(button.command().is_none());
        assert_eq!(descriptor.property_changed().handler_count(), before - 1);
        button.set_command_id(Some("copy".into()));
        window.close();
        assert_eq!(descriptor.property_changed().handler_count(), before - 1);
        let weak = button.downgrade();
        owner.children().clear();
        drop(button);
        assert!(weak.upgrade().is_none());
    })
}
#[test]
fn notify_invoked_rereads_command_after_owner_reentry_before_catalog_fallback() {
    run(|| {
        let owner = Owner::new();
        let catalog = RibbonCommandCatalog::new();
        owner.catalog.replace(Some(catalog.clone()));
        let button = RibbonButton::new();
        button.set_command_id(Some("run".into()));
        owner.children().add(button.clone().upcast::<Control>());
        let runs = Rc::new(Cell::new(0));
        let r = runs.clone();
        let command = RibbonRelayCommand::new(move || r.set(r.get() + 1));
        let descriptor = RibbonCommandDescriptor::new(Some("run".into()), None, Some(command));
        catalog.register(Some(descriptor));
        // No Resolve after catalog insertion: Notify's own no-command fallback executes.
        RibbonItemHelper::notify_invoked(&button, Some(Rc::new(11i32)));
        assert_eq!(runs.get(), 1);
        assert_eq!(owner.invoked.borrow()[0].2, Some(11));
        let weak = button.downgrade();
        owner.on_invoke.replace(Some(Rc::new(move || {
            weak.upgrade()
                .unwrap()
                .set_command(Some(crate::commands::adapt_core_command(
                    RibbonRelayCommand::new(|| {}),
                )))
        })));
        RibbonItemHelper::notify_invoked(&button, None);
        assert_eq!(runs.get(), 1);
    })
}
#[test]
fn source_chrome_and_disabled_guards_dropdown_copy_type_and_native_source_template() {
    run(|| {
        let button = RibbonButton::new();
        button.set_is_chrome_button(true);
        button.set_label(Some("Paste".into()));
        let window = mount(&button);
        let clicks = Rc::new(Cell::new(0));
        let c = clicks.clone();
        button.click(move |_, _| c.set(c.get() + 1));
        button.perform_click();
        assert_eq!(clicks.get(), 1);
        button.set_is_enabled(false);
        assert!(!item(&button).invoke());
        button.perform_click();
        assert_eq!(clicks.get(), 1);
        button.set_is_enabled(true);
        let value =
            crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Buttons.xaml")
                .unwrap()
                .unwrap();
        let resources = ValueTypes::as_object(&*value)
            .unwrap()
            .cast::<ResourceDictionary>()
            .unwrap();
        let theme = resources
            .try_get_resource(&<RibbonButton as StaticType>::TYPE.into(), None)
            .flatten()
            .unwrap();
        button.set_theme(
            ValueTypes::as_object(&*theme)
                .unwrap()
                .cast::<ControlTheme>()
                .unwrap(),
        );
        window.measure(Size::new(500., 200.));
        window.arrange(Rect::new(0., 0., 500., 200.));
        let content = button
            .get_visual_descendants()
            .find_map(|child| child.cast::<crate::primitives::RibbonItemContent>())
            .unwrap();
        assert_eq!(content.label().as_deref(), Some("Paste"));
        button.apply_layout(RibbonItemLayout::new(
            RibbonItemSize::Large,
            Some(RibbonMetrics::comfortable()),
        ));
        assert_eq!(content.item_size(), RibbonItemSize::Large);
        let dropdown = RibbonDropDownButton::new();
        assert!(dropdown.show_chevron());
        let copy = item(&dropdown).create_linked_copy().unwrap();
        assert!(copy.cast::<RibbonDropDownButton>().is_some());
        window.close();
    })
}

#[test]
fn source_click_handled_flag_does_not_suppress_its_command() {
    run(|| {
        let button = RibbonButton::new();
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        button.set_command(Some(crate::commands::adapt_core_command(
            RibbonRelayCommand::new(move || c.set(c.get() + 1)),
        )));
        button.click(|_, e| e.set_handled(true));
        let w = mount(&button);
        button.perform_click();
        assert_eq!(calls.get(), 1);
        w.close();
    })
}

#[test]
fn source_button_isenabled_property_must_include_command_coercion() {
    run(|| {
        let button = RibbonButton::new();
        let command = RibbonRelayCommand::new_with_parameter(|_| {}, Some(Rc::new(|_| false)));
        button.set_command(Some(crate::commands::adapt_core_command(command)));
        let w = mount(&button);
        assert!(
            !button.is_enabled(),
            "command denial must be observable through the IsEnabled property"
        );
        w.close();
    })
}
#[test]
fn source_click_multicast_must_continue_after_an_earlier_handler_marks_handled() {
    run(|| {
        let button = RibbonButton::new();
        let calls = Rc::new(Cell::new(0));
        let c = calls.clone();
        button.click(|_, e| e.set_handled(true));
        button.click(move |_, _| c.set(c.get() + 1));
        let w = mount(&button);
        button.perform_click();
        assert_eq!(
            calls.get(),
            1,
            "source Click?.Invoke invokes the full multicast snapshot"
        );
        w.close();
    })
}

#[test]
fn source_button_click_and_command_precede_flyout_opening_without_toggle_close() {
    run(|| {
        use ferroui_controls::{Flyout, TextBlock};
        let button = RibbonButton::new();
        let flyout = Flyout::new();
        flyout.set_content(Some(Control::boxed(TextBlock::new())));
        let trace = Rc::new(RefCell::new(vec![]));
        let t = trace.clone();
        let weak = flyout.downgrade();
        button.click(move |_, _| {
            assert!(!weak.upgrade().unwrap().is_open());
            t.borrow_mut().push("click");
        });
        let t = trace.clone();
        button.set_command(Some(crate::commands::adapt_core_command(
            RibbonRelayCommand::new(move || t.borrow_mut().push("command")),
        )));
        let t = trace.clone();
        let token = flyout.opening(move |_| t.borrow_mut().push("opening"));
        button.set_flyout(flyout.clone());
        let w = mount(&button);
        button.perform_click();
        assert_eq!(&*trace.borrow(), &["click", "command", "opening"]);
        assert!(flyout.is_open());
        flyout.hide();
        token.dispose();
        w.close();
        // Repeated clicks must request ShowAt, not native Button's default toggle.
        let other = RibbonButton::new();
        other.set_flyout(flyout.clone());
        let w = mount(&other);
        other.perform_click();
        assert!(flyout.is_open());
        other.perform_click();
        assert!(flyout.is_open());
        flyout.hide();
        w.close();
    })
}

#[test]
fn linked_actual_buttons_get_distinct_rich_screentips_and_refresh_application_disabled_reason() {
    run(|| {
        use crate::controls::RibbonScreenTip;
        use ferroui_controls::ToolTip;
        fn content(button: &RibbonButton) -> Ref<RibbonScreenTip> {
            let tip = button.get_value(ToolTip::tip_property()).unwrap();
            let tip = ValueTypes::as_object(&*tip)
                .unwrap()
                .cast::<ToolTip>()
                .unwrap();
            ValueTypes::as_object(&*tip.content().unwrap())
                .unwrap()
                .cast::<RibbonScreenTip>()
                .unwrap()
        }
        let source = RibbonButton::new();
        source.set_label(Some("Cut".into()));
        let description = RibbonScreenTip::new();
        description.set_description(Some("Cut it".into()));
        description.set_disabled_reason(Some("Nothing selected".into()));
        source.set_screen_tip(Some(Control::boxed(&description)));
        let copy = item(&source)
            .create_linked_copy()
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        let source_tip = content(&source);
        let copy_tip = content(&copy);
        assert!(!source_tip.ptr_eq(&copy_tip));
        assert!(!source_tip.ptr_eq(&description));
        assert!(source_tip.disabled_reason().is_none());
        assert!(copy_tip.disabled_reason().is_none());
        source.set_is_enabled(false);
        assert_eq!(
            content(&source).disabled_reason().as_deref(),
            Some("Nothing selected")
        );
        assert_eq!(
            content(&copy).disabled_reason().as_deref(),
            Some("Nothing selected")
        );
        assert!(!content(&source).ptr_eq(&content(&copy)));
    })
}

#[test]
fn source_hosted_flyout_must_assign_owner_and_open_without_a_strong_cycle() {
    run(|| {
        use ferroui_controls::{Flyout, TextBlock};
        let owner = Owner::new();
        let button = RibbonButton::new();
        let flyout = Flyout::new();
        let content = TextBlock::new();
        flyout.set_content(Some(Control::boxed(&content)));
        button.set_flyout(flyout.clone());
        owner.children().add(button.clone().upcast::<Control>());
        let w = mount(&owner);
        button.perform_click();
        assert_eq!(
            RibbonItemHelper::get_owner(&content).unwrap().as_object(),
            owner.clone().upcast::<FerroObject>()
        );
        assert!(flyout.is_open());
        flyout.hide();
        let weak_owner = owner.downgrade();
        let weak_button = button.downgrade();
        w.set_content(None);
        w.close();
        drop(w);
        drop(button);
        drop(owner);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak_owner.upgrade().is_none());
        assert!(weak_button.upgrade().is_none());
        assert!(RibbonItemHelper::get_owner(&content).is_none());
    })
}

#[test]
fn source_hosted_flyout_removes_the_old_hook_and_preserves_an_explicit_owner() {
    run(|| {
        use ferroui_controls::{Flyout, TextBlock};

        let owner = Owner::new();
        let explicit = Owner::new();
        let button = RibbonButton::new();
        let old = Flyout::new();
        let current = Flyout::new();
        let old_content = TextBlock::new();
        let current_content = TextBlock::new();
        RibbonItemHelper::set_owner(&current_content, Some(Rc::new(OwnerView(explicit.clone()))))
            .unwrap();
        old.set_content(Some(Control::boxed(&old_content)));
        current.set_content(Some(Control::boxed(&current_content)));
        button.set_flyout(old.clone());
        button.set_flyout(current.clone());
        owner.children().add(button.clone().upcast::<Control>());
        let window = mount(&owner);

        old.show_at(&button);
        assert!(old.is_open());
        assert!(RibbonItemHelper::get_owner(&old_content).is_none());
        old.hide();
        button.perform_click();
        assert!(current.is_open());
        assert_eq!(
            RibbonItemHelper::get_owner(&current_content)
                .unwrap()
                .as_object(),
            explicit.upcast::<FerroObject>()
        );
        current.hide();
        window.close();
    });
}

#[test]
fn source_hosted_flyout_preserves_an_owner_binding_before_its_first_value() {
    run(|| {
        use ferroui_base::{ElementRef, data::BindingPriority, reactive::LightweightSubject};
        use ferroui_controls::{Flyout, TextBlock};
        use ferroui_system::IObserver;

        let owner = Owner::new();
        let alternate = Owner::new();
        let button = RibbonButton::new();
        let flyout = Flyout::new();
        let content = TextBlock::new();
        let property = RibbonItemHelper::owner_property();
        let values = Rc::new(LightweightSubject::new());
        let _binding = content.bind(property, values.clone(), BindingPriority::LocalValue);
        assert!(content.has_local_value_binding(property.as_property()));
        assert!(content.get_local_value(property).is_none());

        flyout.set_content(Some(Control::boxed(&content)));
        button.set_flyout(flyout.clone());
        owner.children().add(button.clone().upcast::<Control>());
        let window = mount(&owner);
        button.perform_click();
        assert!(flyout.is_open());
        assert!(content.has_local_value_binding(property.as_property()));
        assert!(
            RibbonItemHelper::get_owner(&content).is_none(),
            "the target must not replace the pending local owner binding"
        );
        let alternate = alternate.upcast::<FerroObject>();
        values.on_next(ElementRef::from_nullable(Some(alternate.clone())));
        assert_eq!(
            RibbonItemHelper::get_owner(&content).unwrap().as_object(),
            alternate
        );
        flyout.hide();
        window.close();
    });
}
