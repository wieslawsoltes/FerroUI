use super::{impl_untyped_binding_expression, Publish, UntypedBindingExpression, UntypedBindingExpressionBase};
use crate::data::{BindingMode, BindingPriority};
use crate::reactive::IDisposable;
use crate::{BoxedValue, FerroObject, FerroProperty, WeakRef};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The binding expression of an
/// [`IndexerBinding`](crate::data::IndexerBinding): binds a property of the
/// target to a registered property of another object.
///
/// The source and the target are held weakly: unlike the managed
/// implementation there is no collector for the reference cycle that a
/// binding between two objects (or of an object to itself) would otherwise
/// create.
pub struct IndexerBindingExpression {
    this: Weak<IndexerBindingExpression>,
    base: UntypedBindingExpressionBase,
    source: WeakRef<FerroObject>,
    source_property: &'static FerroProperty,
    target: WeakRef<FerroObject>,
    target_property: Option<&'static FerroProperty>,
    mode: BindingMode,
    source_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    target_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl IndexerBindingExpression {
    pub fn new(
        source: &FerroObject,
        source_property: &'static FerroProperty,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        mode: BindingMode,
    ) -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<IndexerBindingExpression>| Self {
            this: this.clone(),
            base: UntypedBindingExpressionBase::new(this.clone(), BindingPriority::LocalValue, None, false),
            source: source.to_weak(),
            source_property,
            target: target.to_weak(),
            target_property,
            mode,
            source_subscription: RefCell::new(None),
            target_subscription: RefCell::new(None),
        })
    }

    fn publish_source_value(&self) {
        if let Some(source) = self.source.upgrade() {
            let value = source.get_value_untyped(self.source_property);
            self.base.publish_value(Publish::Value(Some(value)), None, false);
        }
    }
}

impl UntypedBindingExpression for IndexerBindingExpression {
    fn base(&self) -> &UntypedBindingExpressionBase {
        &self.base
    }

    fn description(&self) -> String {
        format!("IndexerBinding {})", self.source_property.name())
    }

    fn write_value_to_source(&self, value: Option<BoxedValue>) -> bool {
        if let (Some(source), Some(value)) = (self.source.upgrade(), value) {
            source.set_value_untyped(self.source_property, (*value).as_any(), BindingPriority::LocalValue);
        }
        true
    }

    fn start_core(&self) {
        if matches!(self.mode, BindingMode::TwoWay | BindingMode::OneWayToSource) {
            if let (Some(target), Some(target_property)) = (self.target.upgrade(), self.target_property) {
                let weak = self.this.clone();
                let id = target_property.id();
                let subscription = target.property_changed(move |e| {
                    if e.property().id() != id {
                        return;
                    }
                    let Some(this) = weak.upgrade() else { return };
                    if let Some(source) = this.source.upgrade() {
                        source.set_value_untyped(this.source_property, e.new_value(), BindingPriority::LocalValue);
                    }
                });
                self.target_subscription.replace(Some(subscription));
            }
        }

        if self.mode != BindingMode::OneWayToSource {
            if let Some(source) = self.source.upgrade() {
                let weak = self.this.clone();
                let id = self.source_property.id();
                let subscription = source.property_changed(move |e| {
                    if e.property().id() == id {
                        if let Some(this) = weak.upgrade() {
                            this.publish_source_value();
                        }
                    }
                });
                self.source_subscription.replace(Some(subscription));
            }
            self.publish_source_value();
        }

        if self.mode == BindingMode::OneTime {
            self.stop();
        }
    }

    fn stop_core(&self) {
        let subscription = self.source_subscription.borrow_mut().take();
        if let Some(s) = subscription {
            s.dispose();
        }
        let subscription = self.target_subscription.borrow_mut().take();
        if let Some(s) = subscription {
            s.dispose();
        }
    }
}

impl_untyped_binding_expression!(IndexerBindingExpression);

impl Drop for IndexerBindingExpression {
    fn drop(&mut self) {
        if let Some(s) = self.source_subscription.get_mut().take() {
            s.dispose();
        }
        if let Some(s) = self.target_subscription.get_mut().take() {
            s.dispose();
        }
    }
}
