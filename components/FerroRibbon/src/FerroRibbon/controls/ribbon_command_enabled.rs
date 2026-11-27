//! Button-family command coercion through native properties and subscription handles.
use ferroui_base::{
    FerroObject, ObjectType, StyledElement, StyledPropertyMetadata, Visual,
    input::{ICommand, InputElement},
    logging::{LogArea, LogEventLevel, Logger},
    reactive::{IDisposable, SerialDisposable},
    styling::Setter,
};
use ferroui_controls::Button;
use std::{
    cell::{Cell, RefCell},
    panic::{AssertUnwindSafe, catch_unwind},
    rc::Rc,
};

pub(super) trait CommandEnabledOwner: ObjectType {
    fn command_enablement(&self) -> &CommandEnablement;
    fn native_button(&self) -> &Button;
}

struct CommandSubscription {
    // Preserve command ownership until its subscription is removed. The native
    // handler holds only a weak owner, so this cannot retain the control.
    _command: Rc<dyn ICommand>,
    token: Rc<SerialDisposable>,
}
impl Drop for CommandSubscription {
    fn drop(&mut self) {
        self.token.dispose();
    }
}

pub(super) struct CommandEnablement {
    suppressed: Cell<bool>,
    update_generation: Cell<u64>,
    inherited_enabled: Cell<bool>,
    command: RefCell<Option<CommandSubscription>>,
    ancestors: RefCell<Vec<Rc<dyn IDisposable>>>,
    left_in_core: Cell<bool>,
    leaving: Cell<bool>,
    theme_contributes_enabled: Cell<bool>,
}
impl Default for CommandEnablement {
    fn default() -> Self {
        Self {
            suppressed: Cell::new(false),
            update_generation: Cell::new(0),
            inherited_enabled: Cell::new(true),
            command: RefCell::default(),
            ancestors: RefCell::default(),
            left_in_core: Cell::new(false),
            leaving: Cell::new(false),
            theme_contributes_enabled: Cell::new(false),
        }
    }
}
impl Drop for CommandEnablement {
    fn drop(&mut self) {
        let old = self.command.get_mut().take();
        drop(old);
        for token in self.ancestors.get_mut().drain(..) {
            token.dispose();
        }
    }
}
impl CommandEnablement {
    pub fn register<T: CommandEnabledOwner>() {
        Self::register_coercion::<T>();
        Self::register_callbacks::<T>();
    }

    fn register_coercion<T: CommandEnabledOwner>() {
        InputElement::is_enabled_property().override_metadata::<T>(
            StyledPropertyMetadata::new(None)
                .with_coerce(|owner, requested| {
                    let owner = owner.downcast_ref::<T>().unwrap();
                    owner.command_enablement().coerce(owner, requested)
                })
                .with_coerce_on_reevaluation(false),
        );
    }

    fn coerce<T: CommandEnabledOwner>(&self, owner: &T, requested: bool) -> bool {
        let button = owner.native_button();
        // The source queries even when the requested value is false.
        if button
            .command()
            .is_some_and(|command| !command.can_execute(button.command_parameter().as_ref()))
        {
            return false;
        }
        requested && !self.suppressed.get() && self.inherited_enabled.get()
    }

    fn register_callbacks<T: CommandEnabledOwner>() {
        Button::command_property()
            .changed()
            .add_class_handler::<T>(|owner, change| {
                if change.is_effective_value_change() {
                    let (_, command) = change.get_old_and_new_value::<Option<Rc<dyn ICommand>>>();
                    let state = owner.command_enablement();
                    if !state.leaving.get() {
                        state.left_in_core.set(false);
                    }
                    state.leave();
                    state.subscribe(owner, command);
                    state.update_safe(owner);
                }
            })
            .detach();
        Button::command_parameter_property()
            .changed()
            .add_class_handler::<T>(|owner, change| {
                if change.is_effective_value_change() {
                    // The source metadata callback coerces before the separate
                    // ButtonBase property-change update (and propagates errors).
                    owner
                        .native_button()
                        .coerce_value(InputElement::is_enabled_property().as_property());
                    owner.command_enablement().update_safe(owner);
                }
            })
            .detach();
        StyledElement::parent_property()
            .changed()
            .add_class_handler::<T>(|owner, _| owner.command_enablement().refresh_ancestors(owner))
            .detach();
        Visual::visual_parent_property()
            .changed()
            .add_class_handler::<T>(|owner, change| {
                owner.command_enablement().refresh_ancestors(owner);
                owner.native_button().update_is_effectively_enabled();
                let (old, new) =
                    change.get_old_and_new_value::<Option<ferroui_base::Ref<Visual>>>();
                if old.is_some() && new.is_none() {
                    owner.command_enablement().parent_removed(owner);
                }
            })
            .detach();
    }

    pub fn install<T: CommandEnabledOwner>(owner: &T) {
        owner.command_enablement().refresh_ancestors(owner);
    }

    pub fn theme_changed<T: CommandEnabledOwner>(&self, owner: &T, parent: impl FnOnce()) {
        let button = owner.native_button();
        let mut theme = button.get_effective_theme();
        let mut seen = std::collections::HashSet::new();
        let mut contributes = false;
        while let Some(current) = theme {
            assert!(
                seen.insert(std::ptr::from_ref(&*current)),
                "Cyclic control theme ancestry"
            );
            contributes |= current.setters().snapshot().iter().any(|setter| {
                setter
                    .as_any()
                    .and_then(|value| value.downcast_ref::<Setter>())
                    .is_some_and(|setter| {
                        setter.property() == Some(InputElement::is_enabled_property().as_property())
                    })
            });
            theme = current.based_on();
        }
        let previous = self.theme_contributes_enabled.replace(contributes);
        if !previous && !contributes {
            parent();
            return;
        }
        // Native theme detachment and application normally occur in separate
        // passes. Keep this command property's replacement atomic, including
        // its notifications, using the framework's own nested styling scope.
        button.with_styling_update(|| {
            parent();
            button.apply_styling();
        });
        if contributes
            && button
                .get_local_value(InputElement::is_enabled_property())
                .is_some()
        {
            // A weaker explicit style still invalidates the source command
            // coercion, even when a local value or local binding wins.
            button.coerce_value(InputElement::is_enabled_property().as_property());
        } else if previous && !contributes {
            button.reset_coercion(InputElement::is_enabled_property().as_property());
        }
    }

    pub fn visual_enter<T: CommandEnabledOwner>(&self, owner: &T) {
        self.left_in_core.set(false);
        self.refresh_ancestors(owner);
        self.enter(owner);
    }

    pub fn visual_leave(&self) {
        self.left_in_core.set(true);
        self.leave();
    }

    fn parent_removed<T: CommandEnabledOwner>(&self, owner: &T) {
        // Clearing the source's inherited contribution also clears its
        // coercion output, retaining any local value or binding underneath.
        owner
            .native_button()
            .reset_coercion(InputElement::is_enabled_property().as_property());
        if !self.left_in_core.replace(false) {
            self.leave();
        }
    }

    fn subscribe<T: CommandEnabledOwner>(&self, owner: &T, command: Option<Rc<dyn ICommand>>) {
        let Some(command) = command else { return };
        let button = owner.native_button();
        if self.command.borrow().is_some()
            || !button
                .command()
                .is_some_and(|current| Rc::ptr_eq(&current, &command))
        {
            // Removal can reenter the command property setter. Keep the nested
            // setter's subscription and ignore the abandoned replacement.
            return;
        }
        let target = button.to_ref().cast::<T>().unwrap().downgrade();
        let token = Rc::new(SerialDisposable::new());
        let current = Rc::downgrade(&token);
        self.command.replace(Some(CommandSubscription {
            _command: command.clone(),
            token: token.clone(),
        }));
        // Publish a slot before calling user code. A nested change can dispose
        // the slot even while the command is still installing the handler.
        command.register_can_execute_changed(
            Rc::new(move || {
                let (Some(owner), Some(token)) = (target.upgrade(), current.upgrade()) else {
                    return;
                };
                let state = owner.command_enablement();
                let active = state
                    .command
                    .borrow()
                    .as_ref()
                    .is_some_and(|command| Rc::ptr_eq(&command.token, &token));
                if active {
                    state.update(&*owner);
                }
            }),
            &mut |registration| token.set(Some(registration)),
        );
    }

    fn enter<T: CommandEnabledOwner>(&self, owner: &T) {
        let missing = self.command.borrow().is_none();
        if missing {
            self.subscribe(owner, owner.native_button().command());
            self.update(owner);
        }
    }

    fn leave(&self) {
        struct Restore<'a>(&'a Cell<bool>, bool);
        impl Drop for Restore<'_> {
            fn drop(&mut self) {
                self.0.set(self.1);
            }
        }
        let _restore = Restore(&self.leaving, self.leaving.replace(true));
        let old = self.command.take();
        drop(old);
    }

    fn update<T: CommandEnabledOwner>(&self, owner: &T) {
        let generation = self.update_generation.get().wrapping_add(1);
        self.update_generation.set(generation);
        let button = owner.native_button();
        let can_execute = button
            .command()
            .is_none_or(|command| command.can_execute(button.command_parameter().as_ref()));
        if self.update_generation.get() != generation {
            return;
        }
        if self.suppressed.replace(!can_execute) != !can_execute {
            button.coerce_value(InputElement::is_enabled_property().as_property());
        }
    }

    fn update_safe<T: CommandEnabledOwner>(&self, owner: &T) {
        // Source UpdateCanExecuteSafe catches/logs query failure only at command
        // and parameter changes. Loaded and CanExecuteChanged propagate failure.
        if let Err(error) = catch_unwind(AssertUnwindSafe(|| self.update(owner)))
            && let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::CONTROL)
        {
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap_or("command predicate panicked");
            logger.log_with_values(None, "Failed to update CanExecute: {Error}", &[&message]);
        }
    }

    fn refresh_ancestors<T: CommandEnabledOwner>(&self, owner: &T) {
        let mut current = owner.native_button().inheritance_parent();
        let detached = current.is_none();
        let mut enabled = true;
        let mut subscriptions = Vec::new();
        let target = owner
            .native_button()
            .to_ref()
            .cast::<T>()
            .unwrap()
            .downgrade();
        while let Some(parent) = current {
            if let Some(input) = parent.downcast_ref::<InputElement>() {
                enabled &= input.is_enabled_core();
            }
            let weak = target.clone();
            subscriptions.push(parent.property_changed(move |change| {
                if change.is_effective_value_change()
                    && (change.property() == InputElement::is_enabled_property().as_property()
                        || change.property()
                            == InputElement::is_effectively_enabled_property().as_property()
                        || change.property() == StyledElement::parent_property().as_property()
                        || change.property() == Visual::visual_parent_property().as_property())
                    && let Some(owner) = weak.upgrade()
                {
                    let removed =
                        if change.property() == Visual::visual_parent_property().as_property() {
                            let (old, new) =
                                change.get_old_and_new_value::<Option<ferroui_base::Ref<Visual>>>();
                            old.is_some() && new.is_none()
                        } else {
                            false
                        };
                    if removed && let Some(input) = change.sender().downcast_ref::<InputElement>() {
                        input.update_is_effectively_enabled();
                    }
                    owner.command_enablement().refresh_ancestors(&*owner);
                    if removed {
                        owner.command_enablement().parent_removed(&*owner);
                    }
                }
            }));
            current = FerroObject::inheritance_parent(&parent);
        }
        let old = self.ancestors.replace(subscriptions);
        for token in old {
            token.dispose();
        }
        if self.inherited_enabled.replace(enabled) != enabled {
            let button = owner.native_button();
            if detached {
                button.reset_coercion(InputElement::is_enabled_property().as_property());
            } else {
                button.coerce_value(InputElement::is_enabled_property().as_property());
            }
        }
    }
}

#[cfg(test)]
#[path = "ribbon_command_enabled_runtime_tests.rs"]
mod runtime_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_enabled_tests::ProbeCommand;
    use ferroui_base::{
        FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_impl_classes,
        input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
        layout::LayoutableImpl,
    };
    use ferroui_controls::{
        ButtonImpl, ContentControlImpl, ControlImpl, primitives::TemplatedControlImpl,
    };

    #[repr(C)]
    struct DecisionProbe {
        base: Button,
        state: CommandEnablement,
        coercions: Cell<usize>,
        last: Cell<Option<bool>>,
    }
    ferro_class!(DecisionProbe: Button);
    ferro_impl_classes!(DecisionProbe:
        FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl,
        InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl
    );
    impl ButtonImpl for DecisionProbe {
        fn uses_default_command_enablement(_this: &Self) -> bool {
            false
        }
    }
    impl DecisionProbe {
        fn static_constructor() {
            InputElement::is_enabled_property().override_metadata::<Self>(
                StyledPropertyMetadata::new(None).with_coerce(|object, requested| {
                    let owner = object.downcast_ref::<Self>().unwrap();
                    let result = owner.state.coerce(owner, requested);
                    owner.coercions.set(owner.coercions.get() + 1);
                    owner.last.set(Some(result));
                    result
                }),
            );
        }
        fn new(suppressed: bool) -> Ref<Self> {
            let state = CommandEnablement::default();
            state.suppressed.set(suppressed);
            instantiate(Self {
                base: Button::construct(),
                state,
                coercions: Cell::new(0),
                last: Cell::new(None),
            })
        }
    }
    impl CommandEnabledOwner for DecisionProbe {
        fn command_enablement(&self) -> &CommandEnablement {
            &self.state
        }
        fn native_button(&self) -> &Button {
            self
        }
    }

    #[test]
    fn source_update_sequences_match_real_native_coercion_and_query_counts() {
        crate::tests::run(|| {
            let fixture: serde_json::Value =
                serde_json::from_str(include_str!("../../../tests/fixtures/button-enabled.json"))
                    .unwrap();
            for case in fixture["updates"].as_array().unwrap() {
                let pattern = case["pattern"].as_str().unwrap().to_owned();
                let command = ProbeCommand::new(move |count, _| {
                    pattern == "allow" || (pattern == "alternating" && !count.is_multiple_of(2))
                });
                // Match the explicitly fixed input seam of the C# fixture;
                // no automatic command/lifecycle callbacks are installed here.
                let owner = DecisionProbe::new(case["initialSuppression"].as_bool().unwrap());
                owner.set_command(Some(command.clone()));
                for (index, expected) in case["steps"].as_array().unwrap().iter().enumerate() {
                    if index == 4 {
                        owner.set_command(None);
                    } else {
                        owner.set_command_parameter(
                            (index != 2)
                                .then(|| Rc::new(format!("p{index}")) as ferroui_base::BoxedValue),
                        );
                    }
                    owner.state.update(&*owner);
                    assert_eq!(
                        serde_json::json!(owner.state.suppressed.get()),
                        expected["suppressed"],
                        "{case}:{index}"
                    );
                    assert_eq!(
                        serde_json::json!(owner.coercions.get()),
                        expected["coerces"],
                        "{case}:{index}"
                    );
                    assert_eq!(
                        serde_json::json!(owner.last.get()),
                        expected["result"],
                        "{case}:{index}"
                    );
                    assert_eq!(
                        serde_json::json!(command.queries.get()),
                        expected["queries"],
                        "{case}:{index}"
                    );
                    assert_eq!(
                        serde_json::json!(*command.parameters.borrow()),
                        expected["parameters"],
                        "{case}:{index}"
                    );
                }
            }
        });
    }
}
