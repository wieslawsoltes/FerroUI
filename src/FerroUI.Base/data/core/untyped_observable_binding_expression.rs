use super::{
    impl_untyped_binding_expression, ExpressionError, Publish, UntypedBindingExpression,
    UntypedBindingExpressionBase,
};
use crate::data::{BindingNotification, BindingPriority};
use crate::reactive::{IDisposable, IObservable, IObserver};
use crate::BoxedValue;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A binding expression whose values come from an observable of untyped
/// values.
pub struct UntypedObservableBindingExpression {
    this: Weak<UntypedObservableBindingExpression>,
    base: UntypedBindingExpressionBase,
    observable: Rc<dyn IObservable<Option<BoxedValue>>>,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl UntypedObservableBindingExpression {
    pub fn new(observable: Rc<dyn IObservable<Option<BoxedValue>>>, priority: BindingPriority) -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<UntypedObservableBindingExpression>| Self {
            this: this.clone(),
            base: UntypedBindingExpressionBase::new(this.clone(), priority, None, false),
            observable,
            subscription: RefCell::new(None),
        })
    }
}

struct WeakObserver(Weak<UntypedObservableBindingExpression>);

impl IObserver<Option<BoxedValue>> for WeakObserver {
    fn on_next(&self, value: Option<BoxedValue>) {
        let Some(this) = self.0.upgrade() else { return };
        match value.as_ref().and_then(|v| v.downcast_ref::<BindingNotification>()) {
            Some(n) => {
                let error = n.error().map(|e| ExpressionError::new(e, n.error_type()));
                this.base.publish_value(Publish::Value(n.value()), error, false);
            }
            None => this.base.publish_value(Publish::Value(value), None, false),
        }
    }
}

impl UntypedBindingExpression for UntypedObservableBindingExpression {
    fn base(&self) -> &UntypedBindingExpressionBase {
        &self.base
    }

    fn description(&self) -> String {
        "Observable".to_string()
    }

    fn start_core(&self) {
        let subscription = self.observable.subscribe(Rc::new(WeakObserver(self.this.clone())));
        self.subscription.replace(Some(subscription));
    }

    fn stop_core(&self) {
        let subscription = self.subscription.borrow_mut().take();
        if let Some(s) = subscription {
            s.dispose();
        }
    }
}

impl_untyped_binding_expression!(UntypedObservableBindingExpression);
