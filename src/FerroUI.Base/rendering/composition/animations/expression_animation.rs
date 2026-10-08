use super::{
    CompositionAnimation, ExpressionAnimationInstance, ICompositionAnimation,
    ICompositionAnimationBase,
};
use crate::rendering::composition::expressions::{Expression, ExpressionParser, ExpressionVariant};
use crate::rendering::composition::server::ServerObjectId;
use crate::rendering::composition::{AsCompositionObject, CompositionObject, Compositor};
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

/// A Composition Animation that uses a mathematical equation to calculate the value for an animating property every frame.
///
/// The core of ExpressionAnimations allows a developer to define a mathematical equation that can be used to calculate the value
/// of a targeted animating property each frame.
/// This contrasts [`KeyFrameAnimation`](super::KeyFrameAnimation)s, which use an interpolator to define how the animating
/// property changes over time. The mathematical equation can be defined using references to properties
/// of Composition objects, mathematical functions and operators and Input.
/// Use the `start_animation` method of a composition object to start the animation.
pub struct ExpressionAnimation {
    base: CompositionAnimation,
    expression: RefCell<Option<String>>,
    parsed_expression: RefCell<Option<std::sync::Arc<Expression>>>,
}

impl Deref for ExpressionAnimation {
    type Target = CompositionAnimation;

    fn deref(&self) -> &CompositionAnimation {
        &self.base
    }
}

impl ExpressionAnimation {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> Rc<ExpressionAnimation> {
        Rc::new(ExpressionAnimation {
            base: CompositionAnimation::new(compositor),
            expression: RefCell::new(None),
            parsed_expression: RefCell::new(None),
        })
    }

    /// The mathematical equation specifying how the animated value is calculated each frame.
    /// The Expression is the core of an [`ExpressionAnimation`] and represents the equation
    /// the system will use to calculate the value of the animation property each frame.
    /// The equation is set on this property in the form of a string.
    /// Although expressions can be defined by simple mathematical equations such as "2+2",
    /// the real power lies in creating mathematical relationships where the input values can change frame over frame.
    pub fn expression(&self) -> Option<String> {
        self.expression.borrow().clone()
    }

    pub fn set_expression(&self, value: Option<String>) {
        *self.expression.borrow_mut() = value;
        *self.parsed_expression.borrow_mut() = None;
    }

    /// The parsed expression. Panics with the parse error when the
    /// expression is not valid (upstream throws the parse exception).
    fn parsed_expression(&self) -> std::sync::Arc<Expression> {
        if let Some(parsed) = self.parsed_expression.borrow().clone() {
            return parsed;
        }
        let source = self.expression.borrow().clone().unwrap_or_default();
        let parsed = match ExpressionParser::parse(&source) {
            Ok(parsed) => std::sync::Arc::new(parsed),
            Err(error) => panic!("{error}"),
        };
        *self.parsed_expression.borrow_mut() = Some(parsed.clone());
        parsed
    }
}

impl ICompositionAnimationBase for ExpressionAnimation {
    fn as_composition_animation(&self) -> Option<&dyn ICompositionAnimation> {
        Some(self)
    }
}

impl ICompositionAnimation for ExpressionAnimation {
    fn target(&self) -> Option<String> {
        self.base.target()
    }

    fn create_instance(
        &self,
        target_object: ServerObjectId,
        final_value: Option<ExpressionVariant>,
    ) -> super::AnimationInstanceFactory {
        let expression = self.parsed_expression();
        let parameters = self.create_snapshot_source();
        super::AnimationInstanceFactory::new(move || {
            ExpressionAnimationInstance::new(expression, target_object, final_value, Rc::new(parameters.build()))
        })
    }
}

impl AsCompositionObject for ExpressionAnimation {
    fn as_composition_object(&self) -> &CompositionObject {
        self.base.object()
    }

    fn composition_type_name(&self) -> &'static str {
        "ExpressionAnimation"
    }
}
