//! Native property-service regressions needed by the command-enabled policy.
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, StyledPropertyMetadata, VisualImpl, ferro_class,
    ferro_impl_classes,
    input::{InputElement, InputElementImpl},
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    Button, ButtonImpl, ContentControlImpl, ControlImpl, primitives::TemplatedControlImpl,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

type Query = dyn Fn(usize, Option<&ferroui_base::BoxedValue>) -> bool;
type Handlers = RefCell<BTreeMap<u64, Rc<dyn Fn()>>>;
pub(crate) struct ProbeCommand {
    query: Rc<Query>,
    pub(crate) queries: Cell<usize>,
    pub(crate) parameters: RefCell<Vec<Option<String>>>,
    handlers: Rc<Handlers>,
    next: Cell<u64>,
}
impl ProbeCommand {
    pub(crate) fn new(
        query: impl Fn(usize, Option<&ferroui_base::BoxedValue>) -> bool + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            query: Rc::new(query),
            queries: Cell::new(0),
            parameters: RefCell::default(),
            handlers: Rc::default(),
            next: Cell::new(0),
        })
    }
    pub(crate) fn notify(&self) {
        let snapshot: Vec<_> = self.handlers.borrow().values().cloned().collect();
        for handler in snapshot {
            handler();
        }
    }
    pub(crate) fn subscriptions(&self) -> usize {
        self.handlers.borrow().len()
    }
}
impl ferroui_base::input::ICommand for ProbeCommand {
    fn can_execute(&self, parameter: Option<&ferroui_base::BoxedValue>) -> bool {
        let count = self.queries.get() + 1;
        self.queries.set(count);
        self.parameters.borrow_mut().push(parameter.map(|value| {
            value
                .downcast_ref::<String>()
                .expect("probe accepts string parameters")
                .clone()
        }));
        (self.query)(count, parameter)
    }
    fn execute(&self, _parameter: Option<&ferroui_base::BoxedValue>) {
        panic!("invocation is outside this command-state probe");
    }
    fn can_execute_changed(
        &self,
        handler: Rc<dyn Fn()>,
    ) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        let token = self.next.get();
        self.next.set(token + 1);
        self.handlers.borrow_mut().insert(token, handler);
        let weak = Rc::downgrade(&self.handlers);
        ferroui_base::reactive::Disposable::create(move || {
            if let Some(handlers) = weak.upgrade() {
                handlers.borrow_mut().remove(&token);
            }
        })
    }
}

fn families() -> Vec<Ref<Button>> {
    use crate::controls::{RibbonButton, RibbonCheckBox, RibbonDropDownButton, RibbonToggleButton};
    vec![
        RibbonButton::new().upcast(),
        RibbonDropDownButton::new().upcast(),
        RibbonToggleButton::new().upcast(),
        RibbonCheckBox::new().upcast(),
    ]
}

#[test]
fn ribbon_command_enablement_coerces_all_families_before_loading_and_retains_requested_values() {
    crate::tests::run(|| {
        for button in families() {
            let allowed = Rc::new(Cell::new(false));
            let value = allowed.clone();
            let command = ProbeCommand::new(move |_, _| value.get());
            button.set_command(Some(command.clone()));
            assert_eq!(command.subscriptions(), 1);
            assert_eq!(command.queries.get(), 2);
            assert!(!button.is_enabled());
            assert!(!button.is_effectively_enabled());
            allowed.set(true);
            command.notify();
            assert_eq!(command.queries.get(), 4);
            assert!(button.is_enabled());
            button.set_is_enabled(false);
            assert_eq!(command.queries.get(), 5);
            allowed.set(false);
            command.notify();
            assert_eq!(command.queries.get(), 7);
            allowed.set(true);
            command.notify();
            assert_eq!(command.queries.get(), 9);
            assert!(!button.is_enabled());
            button.set_is_enabled(true);
            assert_eq!(command.queries.get(), 10);
            assert!(button.is_enabled());
            button.set_command(None);
            assert_eq!(command.subscriptions(), 0);
            assert_eq!(command.queries.get(), 10);
            assert!(button.is_enabled());
        }
    });
}

#[test]
fn ribbon_command_enablement_uses_visual_tree_hooks_and_subscribes_after_off_tree_replacement() {
    crate::tests::run(|| {
        use ferroui_base::threading::Dispatcher;
        use ferroui_controls::{Control, Window};
        for button in families() {
            let allowed = Rc::new(Cell::new(false));
            let value = allowed.clone();
            let command = ProbeCommand::new(move |_, _| value.get());
            button.set_command(Some(command.clone()));
            assert_eq!(command.queries.get(), 2);
            let window = Window::new();
            window.set_content(Some(Control::boxed(button.to_ref())));
            window.show();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(command.subscriptions(), 1);
            assert_eq!(command.queries.get(), 2);
            window.set_content(None);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(command.subscriptions(), 0);
            allowed.set(true);
            command.notify();
            assert_eq!(command.queries.get(), 2);
            // Removing the inherited contribution clears coercion without a
            // query, as the actual reference-runtime removal corpus records.
            assert!(button.is_enabled());
            window.set_content(Some(Control::boxed(button.to_ref())));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(command.subscriptions(), 1);
            assert_eq!(command.queries.get(), 4);
            assert!(button.is_enabled());
            window.set_content(None);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(command.subscriptions(), 0);
            let replacement = ProbeCommand::new(|_, _| false);
            button.set_command(Some(replacement.clone()));
            assert_eq!(replacement.subscriptions(), 1);
            assert_eq!(replacement.queries.get(), 2);
            window.set_content(Some(Control::boxed(button.to_ref())));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(replacement.subscriptions(), 1);
            assert_eq!(replacement.queries.get(), 2);
            window.set_content(None);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(replacement.subscriptions(), 0);
            window.close();
        }
    });
}

#[test]
fn ribbon_enabled_property_follows_disabled_ancestors_and_recovers_after_unrooted_reparenting() {
    crate::tests::run(|| {
        use ferroui_controls::StackPanel;
        for button in families() {
            let grandparent = StackPanel::new();
            let parent = StackPanel::new();
            grandparent.children().add(parent.to_ref());
            parent.children().add(button.to_ref());
            assert!(button.is_enabled());
            grandparent.set_is_enabled(false);
            assert!(!button.is_enabled());
            assert!(!button.is_effectively_enabled());
            grandparent.set_is_enabled(true);
            assert!(button.is_enabled());
            button.set_is_enabled(false);
            grandparent.set_is_enabled(false);
            grandparent.set_is_enabled(true);
            assert!(!button.is_enabled());
            button.set_is_enabled(true);
            grandparent.set_is_enabled(false);
            assert!(!button.is_enabled());
            assert!(parent.children().remove(button.to_ref()));
            assert!(button.is_enabled());
            assert!(button.is_effectively_enabled());
            assert_eq!(
                button.get_local_value(InputElement::is_enabled_property()),
                Some(true)
            );
            let enabled_parent = StackPanel::new();
            enabled_parent.children().add(button.to_ref());
            assert!(button.is_enabled());
        }
    });
}

#[test]
fn ribbon_command_subscription_does_not_retain_its_control_and_unsubscribes_on_drop() {
    crate::tests::run(|| {
        for button in families() {
            let command = ProbeCommand::new(|_, _| true);
            button.set_command(Some(command.clone()));
            assert_eq!(command.subscriptions(), 1);
            let weak = button.downgrade();
            drop(button);
            assert!(weak.upgrade().is_none());
            assert_eq!(command.subscriptions(), 0);
            command.notify();
        }
    });
}

#[test]
fn ribbon_command_enabled_linked_copies_refresh_screen_tips_and_release_subscriptions() {
    crate::tests::run(|| {
        use crate::controls::{
            IRibbonItem, RibbonButton, RibbonCheckBox, RibbonScreenTip, RibbonToggleButton,
        };
        use ferroui_base::data::core::{ValueType, ValueTypes};
        use ferroui_controls::{Control, ToolTip};
        struct HostTheme(
            Ref<ferroui_controls::Application>,
            Rc<dyn ferroui_base::styling::IStyle>,
        );
        impl Drop for HostTheme {
            fn drop(&mut self) {
                self.0.styles().remove(self.1.clone());
            }
        }
        let app = ferroui_controls::Application::current().unwrap();
        let style = ferroui_themes_simple::SimpleTheme::new().as_style();
        app.styles().add(style.clone());
        let _theme = HostTheme(app, style);
        fn disabled_reason(button: &Button) -> Option<String> {
            let value = ToolTip::get_tip(button).unwrap();
            let wrapper = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ToolTip>()
                .unwrap();
            let content = wrapper.content().unwrap();
            ValueTypes::as_object(&*content)
                .unwrap()
                .cast::<RibbonScreenTip>()
                .unwrap()
                .disabled_reason()
        }
        for button in families() {
            let tip = RibbonScreenTip::new();
            tip.set_title(Some("Command".into()));
            tip.set_disabled_reason(Some("Unavailable".into()));
            let content = Some(Control::boxed(tip.clone()));
            if let Some(owner) = button.cast::<RibbonButton>() {
                owner.set_screen_tip(content);
            } else if let Some(owner) = button.cast::<RibbonToggleButton>() {
                owner.set_screen_tip(content);
            } else {
                button
                    .cast::<RibbonCheckBox>()
                    .unwrap()
                    .set_screen_tip(content);
            }
            let allowed = Rc::new(Cell::new(false));
            let value = allowed.clone();
            let command = ProbeCommand::new(move |_, _| value.get());
            button.set_command(Some(command.clone()));
            let boxed = Control::boxed(button.clone());
            let interface =
                ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn IRibbonItem>>()).unwrap();
            let item = interface.downcast_ref::<Rc<dyn IRibbonItem>>().unwrap();
            let copy = item.create_linked_copy().unwrap().cast::<Button>().unwrap();
            assert_eq!(
                command.subscriptions(),
                1,
                "source linked properties exclude Command; the copy forwards invocation"
            );
            assert!(copy.command().is_none());
            for value in [&*button, &*copy] {
                assert!(!value.is_enabled());
                assert_eq!(disabled_reason(value).as_deref(), Some("Unavailable"));
            }
            allowed.set(true);
            command.notify();
            for value in [&*button, &*copy] {
                assert!(value.is_enabled());
                assert_eq!(disabled_reason(value), None);
            }
            copy.set_is_enabled(false);
            assert!(
                button.is_enabled(),
                "LinkCommon enabled synchronization is one-way"
            );
            assert_eq!(disabled_reason(&copy).as_deref(), Some("Unavailable"));
            let replacement = ProbeCommand::new(|_, _| false);
            button.set_command(Some(replacement.clone()));
            assert!(!button.is_enabled());
            assert!(!copy.is_enabled());
            assert_eq!(command.subscriptions(), 0);
            assert_eq!(replacement.subscriptions(), 1);
            let weak = copy.downgrade();
            drop(copy);
            assert!(weak.upgrade().is_none());
            assert_eq!(replacement.subscriptions(), 1);
            button.set_command(None);
            assert_eq!(replacement.subscriptions(), 0);
            assert_eq!(tip.disabled_reason().as_deref(), Some("Unavailable"));
        }
    });
}

#[repr(C)]
struct NativeEnabledCoercion {
    base: Button,
    blocked: Cell<bool>,
}
ferro_class!(NativeEnabledCoercion: Button);
ferro_impl_classes!(NativeEnabledCoercion:
    FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl,
    InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl,
    ContentControlImpl, ButtonImpl
);
impl NativeEnabledCoercion {
    fn static_constructor() {
        InputElement::is_enabled_property().override_metadata::<Self>(
            StyledPropertyMetadata::new(None).with_coerce(|owner, requested| {
                let owner = owner.downcast_ref::<Self>().unwrap();
                requested && !owner.blocked.get()
            }),
        );
    }
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: Button::construct(),
            blocked: Cell::new(false),
        })
    }
    fn block(&self, value: bool) {
        self.blocked.set(value);
        self.coerce_value(InputElement::is_enabled_property().as_property());
    }
}

#[test]
fn native_enabled_coercion_recovers_the_unset_default() {
    crate::tests::run(|| {
        let control = NativeEnabledCoercion::new();
        assert!(control.is_enabled());
        control.block(true);
        assert!(!control.is_enabled());
        control.block(false);
        assert!(control.is_enabled());
        assert_eq!(
            control.get_local_value(InputElement::is_enabled_property()),
            None
        );
    });
}

#[test]
fn native_enabled_coercion_retains_local_values_through_repeated_updates() {
    crate::tests::run(|| {
        let control = NativeEnabledCoercion::new();
        for requested in [false, true, false, true] {
            control.set_is_enabled(requested);
            for blocked in [true, false, true, false] {
                control.block(blocked);
                assert_eq!(control.is_enabled(), requested && !blocked);
            }
        }
        control.block(true);
        control.clear_value(InputElement::is_enabled_property());
        assert!(!control.is_enabled());
        control.block(false);
        assert!(control.is_enabled());
    });
}

#[test]
fn native_enabled_coercion_preserves_style_animation_and_hidden_local_inputs() {
    crate::tests::run(|| {
        use ferroui_base::data::BindingPriority;
        let control = NativeEnabledCoercion::new();
        let property = InputElement::is_enabled_property();
        let style = control
            .set_value_with_priority(property, false, BindingPriority::Style)
            .unwrap();
        assert!(!control.is_enabled());
        control.block(true);
        control.set_is_enabled(true);
        assert!(!control.is_enabled());
        assert_eq!(control.get_local_value(property), Some(true));
        control.block(false);
        assert!(control.is_enabled());
        let animation = control
            .set_value_with_priority(property, false, BindingPriority::Animation)
            .unwrap();
        control.block(true);
        control.block(false);
        assert!(!control.is_enabled());
        assert_eq!(control.get_local_value(property), Some(true));
        animation.dispose();
        assert!(control.is_enabled());
        control.clear_local_value(property);
        assert!(!control.is_enabled());
        style.dispose();
        assert!(control.is_enabled());
        assert_eq!(control.get_local_value(property), None);
    });
}
