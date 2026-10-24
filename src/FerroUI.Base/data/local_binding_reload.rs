//! Reversible replacement of an explicitly owned local binding.
use super::{BindingBase, BindingExpressionBase, BindingPriority, BindingValueType};
use crate::diagnostics::FerroObjectDiagnosticExtensions;
use crate::{FerroObject, PropertyValue, Ref, StyledProperty};
use std::rc::Rc;

/// A prepared replacement (or removal) of a supported local binding on a
/// preserved object. Preparation constructs an inactive expression for the
/// actual target; it does not start subscriptions or change the visible value.
///
/// This primitive is for explicit reload adapters. The adapter must establish
/// XAML ownership and supply the correct source, namescope and service context.
/// Synchronous one/two-way bindings are supported; other expression kinds fail
/// preparation. Direct properties, animations and arbitrary observer side
/// effects are outside this contract.
///
/// Commit retains the original subscriptions until `finish`. `rollback` restores
/// the original expression and user input without sending that input to either
/// model. Dropping an unfinished change attempts rollback.
pub struct LocalBindingReload<T: PropertyValue> {
    target: Ref<FerroObject>,
    property: &'static StyledProperty<T>,
    original: Option<Rc<dyn BindingExpressionBase>>,
    replacement: Option<Rc<dyn BindingExpressionBase>>,
    replacement_value: Option<T>,
    original_value: Option<T>,
    original_current_value: bool,
    prepared_version: u64,
    applied_version: u64,
    state: State,
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Prepared,
    Applying,
    Applied,
    Finished,
}

impl<T: PropertyValue> LocalBindingReload<T> {
    pub fn prepare(
        target: &Ref<FerroObject>,
        property: &'static StyledProperty<T>,
        replacement: Option<&dyn BindingBase>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Result<Self, String> {
        let diagnostic = target.get_diagnostic(property);
        let original = if target.has_local_value_binding(property) {
            Some(
                target
                    .get_expression(property)
                    .ok_or("Local binding is not an expression")?,
            )
        } else {
            None
        };
        if diagnostic.priority() < BindingPriority::LocalValue
            || original.as_ref().is_some_and(|expression| {
                expression.priority() != BindingPriority::LocalValue
                    || !expression.can_pause_for_reload()
            })
            || (original.is_none() && diagnostic.is_overridden_current_value())
        {
            return Err("The original binding does not support transactional reload".into());
        }
        let prepared_version = target.local_write_version(property);
        let original_value = (diagnostic.priority() == BindingPriority::LocalValue)
            .then(|| target.get_value(property));
        let replacement =
            replacement.map(|binding| binding.create_instance(target, Some(property), anchor));
        if replacement.as_ref().is_some_and(|expression| {
            expression.default_priority() != BindingPriority::LocalValue
                || !expression.can_pause_for_reload()
        }) {
            return Err("The replacement binding does not support transactional reload".into());
        }
        Ok(Self {
            target: target.clone(),
            property,
            original,
            replacement,
            replacement_value: None,
            original_value,
            original_current_value: diagnostic.is_overridden_current_value(),
            prepared_version,
            applied_version: 0,
            state: State::Prepared,
        })
    }

    /// Replaces a binding with a converted literal, or clears its contribution.
    pub fn prepare_value(
        target: &Ref<FerroObject>,
        property: &'static StyledProperty<T>,
        value: Option<T>,
    ) -> Result<Self, String> {
        let mut change = Self::prepare(target, property, None, None)?;
        change.replacement_value = value;
        Ok(change)
    }

    fn owns(&self, expression: &Rc<dyn BindingExpressionBase>) -> bool {
        self.target
            .get_expression(self.property)
            .is_some_and(|current| Rc::ptr_eq(&current, expression))
    }

    pub fn commit(&mut self) -> Result<(), String> {
        if self.state != State::Prepared {
            return Err("Binding change has already been used".into());
        }
        if self.target.local_write_version(self.property) != self.prepared_version
            || self.original.as_ref().map_or_else(
                || self.target.has_local_value_binding(self.property),
                |expression| !self.owns(expression),
            )
        {
            return Err("The original binding changed after preparation".into());
        }
        let diagnostic = self.target.get_diagnostic(self.property);
        if diagnostic.priority() < BindingPriority::LocalValue {
            return Err("An animation took ownership after preparation".into());
        }
        // Source notifications do not count as application assignments. They may
        // have arrived since preparation; rollback must restore the current value.
        self.original_value = (diagnostic.priority() == BindingPriority::LocalValue)
            .then(|| self.target.get_value(self.property));
        self.original_current_value = diagnostic.is_overridden_current_value();
        if self
            .original
            .as_ref()
            .is_some_and(|expression| !expression.pause_for_reload())
        {
            return Err("The original binding is already paused".into());
        }
        self.state = State::Applying;
        // Detaching an old binding and installing/clearing the new contribution
        // each own exactly one write. Do not absorb reentrant application writes
        // into our ownership baseline.
        self.applied_version = self
            .prepared_version
            .checked_add(1 + u64::from(self.original.is_some()))
            .expect("Local binding reload version overflow");
        if let Some(original) = &self.original {
            self.target
                .values()
                .detach_expression_for_reload(self.property, original);
        }
        match &self.replacement {
            Some(expression) => {
                self.target.values().add_binding_expression(
                    &self.target,
                    self.property,
                    expression.clone(),
                );
            }
            None => {
                match &self.replacement_value {
                    Some(value) => self.target.set_value(self.property, value.clone()),
                    None => self.target.clear_value(self.property),
                }
                self.target.on_update_data_validation(
                    self.property,
                    BindingValueType::UNSET_VALUE,
                    None,
                );
            }
        }
        if self.target.local_write_version(self.property) != self.applied_version {
            return Err("Application wrote the property during binding replacement".into());
        }
        self.state = State::Applied;
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), String> {
        if self.state == State::Prepared || self.state == State::Finished {
            return Ok(());
        }
        let owns_slot = match &self.replacement {
            Some(expression) => self.owns(expression),
            None => !self.target.has_local_value_binding(self.property),
        };
        if !owns_slot || self.target.local_write_version(self.property) > self.applied_version {
            // An application callback took ownership. Never overwrite its state.
            self.state = State::Finished;
            if let Some(original) = &self.original {
                original.dispose();
            }
            return Err("Application changed the binding during the reload transaction".into());
        }
        if let Some(expression) = &self.replacement {
            expression.pause_for_reload();
            self.target
                .values()
                .detach_expression_for_reload(self.property, expression);
            expression.dispose();
        }
        // Restore the value while both expressions are detached and muted, so
        // it cannot be sent to either model.
        match &self.original_value {
            Some(value) => {
                self.target.set_value(self.property, value.clone());
                if self.original_current_value {
                    self.target.set_current_value(self.property, value.clone());
                }
            }
            None => self.target.clear_value(self.property),
        }
        if let Some(original) = &self.original {
            self.target
                .values()
                .restore_expression_after_reload(self.property, original.clone());
            if let Some((state, error)) = original.get_data_validation_state() {
                self.target
                    .on_update_data_validation(self.property, state, error);
            }
        } else {
            self.target.on_update_data_validation(
                self.property,
                BindingValueType::UNSET_VALUE,
                None,
            );
        }
        self.state = State::Finished;
        if let Some(original) = &self.original {
            original.resume_after_reload();
        }
        Ok(())
    }

    /// Accepts the replacement and releases the original subscriptions.
    pub fn finish(&mut self) -> Result<(), String> {
        if self.state != State::Applied {
            return Err("Binding change has not committed".into());
        }
        self.state = State::Finished;
        if let Some(original) = &self.original {
            original.dispose();
        }
        Ok(())
    }
}

impl<T: PropertyValue> Drop for LocalBindingReload<T> {
    fn drop(&mut self) {
        // Property observers are application code and may panic. Do not double
        // panic while unwinding a failed commit; the session reports that failure.
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| self.rollback()));
    }
}
