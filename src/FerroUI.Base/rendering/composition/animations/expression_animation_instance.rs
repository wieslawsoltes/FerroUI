use super::{AnimationInstanceBase, IAnimationInstance, PropertySetSnapshot};
use crate::rendering::composition::expressions::{
    BuiltInExpressionFfi, Expression, ExpressionEvaluationContext, ExpressionVariant,
};
use crate::rendering::composition::server::{CompositionProperty, IServerClockItem, ServerCompositor, ServerObjectId};
use std::cell::Cell;
use std::collections::HashSet;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Server-side counterpart of [`ExpressionAnimation`](super::ExpressionAnimation) with values baked-in.
pub struct ExpressionAnimationInstance {
    base: AnimationInstanceBase,
    expression: Rc<Expression>,
    starting_value: Cell<ExpressionVariant>,
    final_value: Option<ExpressionVariant>,
}

impl ExpressionAnimationInstance {
    pub fn new(
        expression: Rc<Expression>,
        target: ServerObjectId,
        final_value: Option<ExpressionVariant>,
        parameters: Rc<PropertySetSnapshot>,
    ) -> Rc<ExpressionAnimationInstance> {
        Rc::new_cyclic(|this: &Weak<ExpressionAnimationInstance>| ExpressionAnimationInstance {
            base: AnimationInstanceBase::new(this.clone(), target, parameters),
            expression,
            starting_value: Cell::new(ExpressionVariant::default()),
            final_value,
        })
    }

    fn evaluate_core(&self, _now: Duration, current_value: ExpressionVariant) -> ExpressionVariant {
        let starting_value = self.starting_value.get();
        self.base.with_target_expression_object(|target| {
            let ctx = ExpressionEvaluationContext {
                parameters: Some(&**self.base.parameters()),
                target,
                foreign_function_interface: Some(BuiltInExpressionFfi::instance()),
                starting_value,
                final_value: self.final_value.unwrap_or(starting_value),
                current_value,
            };
            self.expression.evaluate(&ctx)
        })
    }
}

impl IServerClockItem for ExpressionAnimationInstance {
    fn on_tick(&self) {
        self.base.on_tick()
    }
}

impl IAnimationInstance for ExpressionAnimationInstance {
    fn target_object(&self) -> ServerObjectId {
        self.base.target_object()
    }

    fn resolve(&self, compositor: &Rc<ServerCompositor>) {
        self.base.resolve(compositor)
    }

    fn evaluate(&self, now: Duration, current_value: ExpressionVariant) -> ExpressionVariant {
        self.base.begin_evaluate();
        self.evaluate_core(now, current_value)
    }

    fn initialize(&self, _started_at: Duration, starting_value: ExpressionVariant, property: &'static CompositionProperty) {
        self.starting_value.set(starting_value);
        let mut hs = HashSet::new();
        self.expression.collect_references(&mut hs);
        self.base.initialize(property, &hs);
    }

    fn activate(&self) {
        self.base.activate()
    }

    fn deactivate(&self) {
        self.base.deactivate()
    }

    fn invalidate(&self) {
        self.base.invalidate()
    }
}
