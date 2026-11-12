use super::IValueEntry;
use crate::data::core::{IBindingExpressionSink, SinkRef};
use crate::data::{BindingBase, BindingError, BindingExpressionBase, BindingPriority, BindingValueType};
use crate::{BoxedValue, FerroObject, FerroProperty, WeakRef};
use std::any::Any;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The value entry of a style frame for a setter whose value is a binding.
///
/// The managed original instances the binding when the style is attached: the
/// frame gets the binding expression as its entry, and the expression is
/// started when the value of the entry is first asked for. Most of these
/// expressions are never asked: the style is not active, or a value above it
/// (a local value, a style later in the order) is the effective one. The
/// expression is the largest object a style attach creates, so the entry is
/// this record instead, and it creates the expression the moment the store
/// asks it for a value or for the expression itself. What the store does with
/// an expression it never started is ask for its property and stop it, which
/// the record answers without one.
///
/// The record stays the entry of the frame after the expression exists (the
/// store compares entries by identity), and it is the sink of its expression:
/// a change of the expression reaches the store as a change of this entry.
///
/// It is 64 bytes, a block of 80 with the counts of its `Rc`
/// (`setter_binding_entry_is_a_small_record`): the binding and the expression
/// made from it share one field.
pub(crate) struct SetterBindingEntry {
    this: Weak<SetterBindingEntry>,
    property: &'static FerroProperty,
    owner: WeakRef<FerroObject>,
    state: Cell<State>,
    priority: BindingPriority,
}

/// What an entry has: the binding, until the expression is asked for, and the
/// expression from then on.
enum State {
    Binding(Rc<dyn BindingBase>),
    Expression(Rc<dyn BindingExpressionBase>),
    /// The state is being read, or the expression is being made.
    Vacant,
}

impl SetterBindingEntry {
    /// Creates the entry of `binding` for `property` of `owner`, in a frame
    /// of `priority`. Panics for a direct property, which has no value
    /// entries.
    pub(crate) fn new(
        owner: &FerroObject,
        property: &'static FerroProperty,
        binding: Rc<dyn BindingBase>,
        priority: BindingPriority,
    ) -> Rc<Self> {
        assert!(!property.is_direct(), "direct properties have no value entries");
        crate::perf_count!(SetterBindingEntriesCreated);
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            property,
            owner: owner.to_weak(),
            state: Cell::new(State::Binding(binding)),
            priority,
        })
    }

    /// The expression of the entry, instanced from the binding the first
    /// time it is asked for. `None` once the owner is gone.
    fn expression(&self) -> Option<Rc<dyn BindingExpressionBase>> {
        match self.state.replace(State::Vacant) {
            State::Expression(expression) => {
                self.state.set(State::Expression(expression.clone()));
                Some(expression)
            }
            State::Binding(binding) => {
                let Some(owner) = self.owner.upgrade() else {
                    self.state.set(State::Binding(binding));
                    return None;
                };
                // Instancing a binding and attaching the expression ask
                // nothing of the store, so the entry is not asked for a value
                // while it has neither.
                let expression = binding.create_instance(&owner, Some(self.property), None);
                let sink: Weak<dyn IBindingExpressionSink> = self.this.clone();
                expression.attach(SinkRef::Other(sink), None, &owner, self.property, self.priority);
                self.state.set(State::Expression(expression.clone()));
                Some(expression)
            }
            State::Vacant => None,
        }
    }

    /// The expression of the entry when it was instanced.
    fn instanced(&self) -> Option<Rc<dyn BindingExpressionBase>> {
        let state = self.state.replace(State::Vacant);
        let expression = match &state {
            State::Expression(expression) => Some(expression.clone()),
            State::Binding(_) | State::Vacant => None,
        };
        self.state.set(state);
        expression
    }
}

impl IValueEntry for SetterBindingEntry {
    fn property(&self) -> &'static FerroProperty {
        self.property
    }

    fn has_value(&self) -> bool {
        self.expression().is_some_and(|expression| expression.has_value())
    }

    fn try_get_value(&self, out: &mut dyn Any) -> bool {
        self.expression().is_some_and(|expression| expression.try_get_value(out))
    }

    fn get_value_boxed(&self) -> Option<BoxedValue> {
        self.expression().and_then(|expression| expression.get_value_boxed())
    }

    fn get_data_validation_state(&self) -> Option<(BindingValueType, Option<BindingError>)> {
        // Asked of an entry that had a value or that changed: an expression
        // that was never instanced has neither.
        self.instanced().and_then(|expression| expression.get_data_validation_state())
    }

    fn unsubscribe(&self) {
        if let Some(expression) = self.instanced() {
            expression.unsubscribe();
        }
    }

    fn as_binding_expression(self: Rc<Self>) -> Option<Rc<dyn BindingExpressionBase>> {
        self.expression()
    }

    fn templated_parent_changed(&self) {
        // An expression that was not instanced reads the templated parent
        // when it starts.
        if let Some(untyped) = self.instanced().as_deref().and_then(BindingExpressionBase::as_untyped) {
            untyped.templated_parent_changed();
        }
    }
}

impl IBindingExpressionSink for SetterBindingEntry {
    fn on_changed(&self, instance: &Rc<dyn BindingExpressionBase>, has_value_changed: bool, has_error_changed: bool) {
        let (Some(owner), Some(this)) = (self.owner.upgrade(), self.this.upgrade()) else { return };
        let entry: Rc<dyn IValueEntry> = this;
        owner.values().on_entry_changed(
            &owner,
            self.property,
            &entry,
            instance.priority(),
            has_value_changed,
            has_error_changed,
        );
    }

    fn on_completed(&self, instance: &Rc<dyn BindingExpressionBase>) {
        if let Some(owner) = self.owner.upgrade() {
            owner.values().on_expression_completed(&owner, instance);
        }
    }
}
