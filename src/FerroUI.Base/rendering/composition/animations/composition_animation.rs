use super::{PropertySetSnapshot, PropertySetSnapshotSource};
use crate::media::Color;
use crate::numerics::{Matrix3x2, Matrix4x4, Quaternion, Vector2, Vector3, Vector4};
use crate::rendering::composition::expressions::{
    Expression, ExpressionEvaluationContext, ExpressionParseException, ExpressionParser, ExpressionVariant,
    IExpressionForeignFunctionInterface, IExpressionObject, IExpressionParameterCollection,
};
use crate::rendering::composition::{AsCompositionObject, CompositionObject, CompositionPropertySet, Compositor};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

/// This is the base class for ExpressionAnimation and KeyFrameAnimation.
///
/// Use the `start_animation` method of a composition object to start the
/// animation. Value parameters (as opposed to reference parameters which
/// are set using [`set_reference_parameter`](Self::set_reference_parameter))
/// are copied and "embedded" into an expression at the time
/// `start_animation` is called. Changing the value of the variable after
/// `start_animation` is called will not affect the value of the
/// ExpressionAnimation. See the remarks of
/// [`ExpressionAnimation`](super::ExpressionAnimation) for additional
/// information.
///
/// A class derives from it by embedding it (`base`) and dereferencing to
/// it; the class implements [`ICompositionAnimation`](super::ICompositionAnimation),
/// whose `Rc<dyn ..>` is the handle of an animation of any class.
pub struct CompositionAnimation {
    object: CompositionObject,
    property_set: Rc<CompositionPropertySet>,
    target: RefCell<Option<String>>,
    initial_value_expressions: InitialValueExpressionCollection,
}

impl CompositionAnimation {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> Self {
        Self {
            object: CompositionObject::new(compositor, None),
            property_set: CompositionPropertySet::new_parameters(compositor),
            target: RefCell::new(None),
            initial_value_expressions: InitialValueExpressionCollection::default(),
        }
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The associated compositor.
    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// Clears all of the parameters of the animation.
    pub fn clear_all_parameters(&self) {
        self.property_set.clear_all()
    }

    /// Clears a parameter from the animation.
    pub fn clear_parameter(&self, key: &str) {
        self.property_set.clear(key)
    }

    fn set_variant(&self, key: &str, value: ExpressionVariant) {
        self.property_set.set(key, value)
    }

    /// Sets a boolean value parameter for use with an expression animation
    /// or an expression key frame.
    pub fn set_boolean_parameter(&self, key: &str, value: bool) {
        self.set_variant(key, value.into())
    }

    pub fn set_color_parameter(&self, key: &str, value: Color) {
        self.set_variant(key, value.into())
    }

    /// Sets an object as a reference parameter of an expression: the
    /// expression reads its properties (`key.Offset`) every time it is
    /// evaluated, as with [`set_reference_parameter`](Self::set_reference_parameter).
    //
    // Deviation (winui-composition.md): WinUI takes an `IAnimationObject`,
    // which lets an object that is not a composition object (a XAML element)
    // stand for one through `PopulatePropertyInfo`. Here the parameter is a
    // composition object, the same as `set_reference_parameter`; an element
    // facade passes the composition object behind it (its visual).
    pub fn set_expression_reference_parameter(&self, key: &str, animation_object: Rc<dyn AsCompositionObject>) {
        self.set_reference_parameter(key, animation_object)
    }

    pub fn set_matrix3x2_parameter(&self, key: &str, value: Matrix3x2) {
        self.set_variant(key, value.into())
    }

    pub fn set_matrix4x4_parameter(&self, key: &str, value: Matrix4x4) {
        self.set_variant(key, value.into())
    }

    pub fn set_quaternion_parameter(&self, key: &str, value: Quaternion) {
        self.set_variant(key, value.into())
    }

    pub fn set_reference_parameter(&self, key: &str, composition_object: Rc<dyn AsCompositionObject>) {
        self.property_set.set_object(key, composition_object)
    }

    pub fn set_scalar_parameter(&self, key: &str, value: f32) {
        self.set_variant(key, value.into())
    }

    pub fn set_vector2_parameter(&self, key: &str, value: Vector2) {
        self.set_variant(key, value.into())
    }

    pub fn set_vector3_parameter(&self, key: &str, value: Vector3) {
        self.set_variant(key, value.into())
    }

    pub fn set_vector4_parameter(&self, key: &str, value: Vector4) {
        self.set_variant(key, value.into())
    }

    /// The name of the property the animation targets.
    pub fn target(&self) -> Option<String> {
        self.target.borrow().clone()
    }

    pub fn set_target(&self, value: Option<String>) {
        *self.target.borrow_mut() = value;
    }

    /// The initial values of the animation: for a property of the target
    /// object, the expression that gives the property its value when the
    /// animation starts.
    pub fn initial_value_expressions(&self) -> &InitialValueExpressionCollection {
        &self.initial_value_expressions
    }

    pub(crate) fn create_snapshot(&self) -> Rc<PropertySetSnapshot> {
        Rc::new(self.property_set.snapshot())
    }

    /// The parameters of the animation in the form that is sent to the
    /// server.
    pub(crate) fn create_snapshot_source(&self) -> PropertySetSnapshotSource {
        self.property_set.snapshot_source()
    }
}

/// The initial values of a [`CompositionAnimation`]
/// (`CompositionAnimation.InitialValueExpressions`): a map from the name of a
/// property of the target object (`"Offset"`, `"Opacity"`) to an expression
/// string. When the animation starts, each expression is evaluated against the
/// target object and the parameters of the animation, and the property is set
/// to the result before the animation runs; `this.StartingValue` and
/// `this.CurrentValue` in such an expression are the value the property has
/// at that moment.
//
// Deviation (winui-composition.md): in WinUI the collection is a
// `CompositionObject` (an `IMap<String, String>`); here it is a plain
// collection owned by the animation, with the members of the map.
#[derive(Default)]
pub struct InitialValueExpressionCollection {
    expressions: RefCell<BTreeMap<String, String>>,
}

impl InitialValueExpressionCollection {
    /// Sets the initial value expression of a property. Returns `true` when it
    /// replaced an expression the property already had.
    pub fn insert(&self, property_name: &str, expression: &str) -> bool {
        self.expressions
            .borrow_mut()
            .insert(property_name.to_owned(), expression.to_owned())
            .is_some()
    }

    /// The initial value expression of a property, if it has one.
    pub fn lookup(&self, property_name: &str) -> Option<String> {
        self.expressions.borrow().get(property_name).cloned()
    }

    /// Whether the property has an initial value expression.
    pub fn has_key(&self, property_name: &str) -> bool {
        self.expressions.borrow().contains_key(property_name)
    }

    /// Removes the initial value expression of a property. Returns `true` when
    /// the property had one.
    pub fn remove(&self, property_name: &str) -> bool {
        self.expressions.borrow_mut().remove(property_name).is_some()
    }

    /// Removes every initial value expression.
    pub fn clear(&self) {
        self.expressions.borrow_mut().clear()
    }

    /// The number of initial value expressions.
    pub fn size(&self) -> usize {
        self.expressions.borrow().len()
    }

    /// The `(property name, expression)` pairs, ordered by property name.
    pub fn get_view(&self) -> Vec<(String, String)> {
        self.expressions
            .borrow()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// The expressions parsed, ordered by property name, in the form the
    /// starting animation takes them to the server (`Send`). Fails with the
    /// error of the first expression that cannot be parsed.
    pub fn parse_all(&self) -> Result<Vec<(String, Expression)>, ExpressionParseException> {
        self.expressions
            .borrow()
            .iter()
            .map(|(name, source)| ExpressionParser::parse(source).map(|parsed| (name.clone(), parsed)))
            .collect()
    }

    /// Evaluates parsed initial value expressions for a target object: for each
    /// `(property name, expression)` pair (from [`parse_all`](Self::parse_all)),
    /// the value the property starts with. `this.StartingValue`,
    /// `this.CurrentValue` and `this.FinalValue` are the value the target has
    /// for that property, `this.Target` is the target and the parameters are
    /// those of the animation. A result that is `Invalid` (an expression that
    /// does not evaluate) is left out, so the property keeps its value.
    ///
    /// This is the step the start of an animation runs on the server before
    /// the animation instance is initialized; writing the values to the
    /// properties is left to the caller, which knows the property ids.
    pub fn evaluate_parsed(
        expressions: &[(String, Expression)],
        target: &dyn IExpressionObject,
        parameters: Option<&dyn IExpressionParameterCollection>,
        foreign_function_interface: &dyn IExpressionForeignFunctionInterface,
    ) -> Vec<(String, ExpressionVariant)> {
        let mut values = Vec::with_capacity(expressions.len());
        for (property_name, expression) in expressions {
            let current = target.get_property(property_name);
            let context = ExpressionEvaluationContext {
                starting_value: current,
                current_value: current,
                final_value: current,
                target: Some(target),
                parameters,
                foreign_function_interface: Some(foreign_function_interface),
            };
            let value = expression.evaluate(&context);
            if value != ExpressionVariant::Invalid {
                values.push((property_name.clone(), value));
            }
        }
        values
    }
}
