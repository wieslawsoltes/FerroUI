//! Custom effects (#455, not from upstream): effects defined outside the
//! framework, set on `Visual.Effect` like the built-in blur and drop shadow
//! (docs/porting/custom-effects.md).
//!
//! A custom effect is a value type that implements [`CustomEffect`]: its
//! parameters, the padding it needs and how two of its values interpolate.
//! [`ImmutableCustomEffect`] is the immutable, shared snapshot of such a
//! value. It is an [`IEffect`] and an [`IImmutableEffect`], so it is what the
//! compositor sends to the render thread: an `Arc` of plain values, never a
//! live object. A mutable effect class (a class deriving from
//! [`Effect`](super::Effect) with properties) freezes into one through
//! [`CustomEffectClass`].
//!
//! An effect whose type describes itself as an effect graph
//! ([`CustomEffect::graph`], step E2) is drawn by the render walk as that
//! graph around the content of its visual, with the shared painter of the
//! composition effects, on every backend. An effect whose type describes
//! itself as a runtime shader ([`CustomEffect::shader`], step E4) is drawn by
//! each backend with its program around the content (Skia from E4 on, Vello
//! from E5 on). What a backend draws for another custom effect is the
//! business of that backend's factory for the effect. A backend without one
//! draws the content unchanged and reports it once
//! ([`ImmutableCustomEffect::report_unhandled`]).

use super::custom_effect_graph::{load_custom_effect_graph, CustomEffectGraph};
use super::custom_effect_shader::{
    load_custom_effect_shader, visit_shader_uniforms, ShaderEffectDescription, ShaderEffectPadding, ShaderUniformType,
    ShaderUniformValue,
};
use super::{IEffect, IImmutableEffect};
use crate::animation::animators::{ColorAnimator, DoubleAnimator};
use crate::logging::{LogEventLevel, Logger};
use crate::media::Color;
use crate::rendering::composition::effects::{CompositionEffectFactoryError, GraphicsEffect};
use crate::{ObjectType, Point, Rect, Size, Thickness, Vector};
use std::any::{Any, TypeId};
use std::fmt;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// The value of a parameter of a custom effect.
///
/// The parameters are what parsing reads and writes (`tint(color=#0F9D8E,
/// strength=0.55)`), what the default interpolation interpolates, and what a
/// backend factory hands to its GPU program as uniforms.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum EffectParameterValue {
    /// `true` or `false`. Steps when interpolated.
    Bool(bool),
    /// An integer, also the value of an enumeration. Interpolated and
    /// rounded.
    Int(i32),
    /// A number.
    Float(f64),
    /// A colour: `#AARRGGBB`, `#RRGGBB` or a colour name.
    Color(Color),
    /// A point: `x,y`.
    Point(Point),
    /// A vector: `x,y`.
    Vector(Vector),
    /// A size: `width,height`.
    Size(Size),
    /// A rectangle: `x,y,width,height`.
    Rect(Rect),
    /// A name or other text. Steps when interpolated.
    Text(Arc<str>),
}

impl EffectParameterValue {
    /// The value as a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Self::Bool(value) => Some(value),
            _ => None,
        }
    }

    /// The value as an integer.
    pub fn as_int(&self) -> Option<i32> {
        match *self {
            Self::Int(value) => Some(value),
            _ => None,
        }
    }

    /// The value as a number; an integer converts.
    pub fn as_float(&self) -> Option<f64> {
        match *self {
            Self::Float(value) => Some(value),
            Self::Int(value) => Some(f64::from(value)),
            _ => None,
        }
    }

    /// The value as a colour.
    pub fn as_color(&self) -> Option<Color> {
        match *self {
            Self::Color(value) => Some(value),
            _ => None,
        }
    }

    /// The value as a point; a vector converts.
    pub fn as_point(&self) -> Option<Point> {
        match *self {
            Self::Point(value) => Some(value),
            Self::Vector(value) => Some(Point::new(value.x, value.y)),
            _ => None,
        }
    }

    /// The value as a vector; a point converts.
    pub fn as_vector(&self) -> Option<Vector> {
        match *self {
            Self::Vector(value) => Some(value),
            Self::Point(value) => Some(Vector::new(value.x, value.y)),
            _ => None,
        }
    }

    /// The value as a size.
    pub fn as_size(&self) -> Option<Size> {
        match *self {
            Self::Size(value) => Some(value),
            _ => None,
        }
    }

    /// The value as a rectangle.
    pub fn as_rect(&self) -> Option<Rect> {
        match *self {
            Self::Rect(value) => Some(value),
            _ => None,
        }
    }

    /// The value as text.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(&**value),
            _ => None,
        }
    }

    /// The value at `progress` (0 to 1) of the way from `self` to `to`, or
    /// `None` when the two are of different kinds or the kind does not
    /// interpolate (booleans and text step).
    pub fn interpolate(&self, to: &Self, progress: f64) -> Option<Self> {
        let lerp = |from: f64, to: f64| DoubleAnimator::interpolate_core(progress, &from, &to);
        Some(match (self, to) {
            (Self::Int(from), Self::Int(to)) => {
                let value = lerp(f64::from(*from), f64::from(*to)).round();
                Self::Int(value.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32)
            }
            (Self::Float(from), Self::Float(to)) => Self::Float(lerp(*from, *to)),
            (Self::Color(from), Self::Color(to)) => Self::Color(ColorAnimator::interpolate_core(progress, from, to)),
            (Self::Point(from), Self::Point(to)) => Self::Point(Point::new(lerp(from.x, to.x), lerp(from.y, to.y))),
            (Self::Vector(from), Self::Vector(to)) => Self::Vector(Vector::new(lerp(from.x, to.x), lerp(from.y, to.y))),
            (Self::Size(from), Self::Size(to)) => {
                Self::Size(Size::new(lerp(from.width, to.width), lerp(from.height, to.height)))
            }
            (Self::Rect(from), Self::Rect(to)) => Self::Rect(Rect::new(
                lerp(from.x, to.x),
                lerp(from.y, to.y),
                lerp(from.width, to.width),
                lerp(from.height, to.height),
            )),
            _ => return None,
        })
    }

    /// Parses `text` as a value of the same kind as `self` (the kind of a
    /// parameter is that of its current value), in the invariant format
    /// [`Display`](fmt::Display) writes. Composite values may be wrapped in
    /// parentheses and separate their numbers with commas or spaces.
    pub fn parse_like(&self, text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        Some(match self {
            Self::Bool(_) => {
                if text.eq_ignore_ascii_case("true") {
                    Self::Bool(true)
                } else if text.eq_ignore_ascii_case("false") {
                    Self::Bool(false)
                } else {
                    return None;
                }
            }
            Self::Int(_) => Self::Int(text.parse().ok()?),
            Self::Float(_) => Self::Float(text.parse().ok()?),
            Self::Color(_) => Self::Color(Color::try_parse(text)?),
            Self::Point(_) => {
                let [x, y] = numbers::<2>(text)?;
                Self::Point(Point::new(x, y))
            }
            Self::Vector(_) => {
                let [x, y] = numbers::<2>(text)?;
                Self::Vector(Vector::new(x, y))
            }
            Self::Size(_) => {
                let [width, height] = numbers::<2>(text)?;
                Self::Size(Size::new(width, height))
            }
            Self::Rect(_) => {
                let [x, y, width, height] = numbers::<4>(text)?;
                Self::Rect(Rect::new(x, y, width, height))
            }
            Self::Text(_) => Self::Text(unquote(text).into()),
        })
    }
}

/// Exactly `N` numbers separated by commas or white space, optionally
/// inside one pair of parentheses.
fn numbers<const N: usize>(text: &str) -> Option<[f64; N]> {
    let text = text.trim();
    let text = match text.strip_prefix('(').and_then(|inner| inner.strip_suffix(')')) {
        Some(inner) => inner.trim(),
        None => text,
    };
    let mut result = [0.0; N];
    let mut count = 0;
    for part in text.split(|c: char| c == ',' || c.is_whitespace()).filter(|part| !part.is_empty()) {
        if count == N {
            return None;
        }
        result[count] = part.parse().ok()?;
        count += 1;
    }
    (count == N).then_some(result)
}

/// The text without one pair of matching outer quotes.
fn unquote(text: &str) -> &str {
    for quote in ['"', '\''] {
        if let Some(inner) = text.strip_prefix(quote).and_then(|inner| inner.strip_suffix(quote)) {
            return inner;
        }
    }
    text
}

/// Whether text is written without quotes in an effect string.
fn is_bare_text(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// The invariant format of effect strings, which
/// [`parse_like`](EffectParameterValue::parse_like) reads back.
impl fmt::Display for EffectParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(value) => write!(f, "{value}"),
            Self::Int(value) => write!(f, "{value}"),
            Self::Float(value) => write!(f, "{value}"),
            Self::Color(value) => write!(f, "{value}"),
            Self::Point(value) => write!(f, "{},{}", value.x, value.y),
            Self::Vector(value) => write!(f, "{},{}", value.x, value.y),
            Self::Size(value) => write!(f, "{},{}", value.width, value.height),
            Self::Rect(value) => write!(f, "{},{},{},{}", value.x, value.y, value.width, value.height),
            Self::Text(value) if is_bare_text(value) => f.write_str(value),
            Self::Text(value) => write!(f, "\"{value}\""),
        }
    }
}

impl From<bool> for EffectParameterValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i32> for EffectParameterValue {
    fn from(value: i32) -> Self {
        Self::Int(value)
    }
}

impl From<f64> for EffectParameterValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<Color> for EffectParameterValue {
    fn from(value: Color) -> Self {
        Self::Color(value)
    }
}

impl From<Point> for EffectParameterValue {
    fn from(value: Point) -> Self {
        Self::Point(value)
    }
}

impl From<Vector> for EffectParameterValue {
    fn from(value: Vector) -> Self {
        Self::Vector(value)
    }
}

impl From<Size> for EffectParameterValue {
    fn from(value: Size) -> Self {
        Self::Size(value)
    }
}

impl From<Rect> for EffectParameterValue {
    fn from(value: Rect) -> Self {
        Self::Rect(value)
    }
}

impl From<&str> for EffectParameterValue {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
    }
}

impl From<String> for EffectParameterValue {
    fn from(value: String) -> Self {
        Self::Text(value.into())
    }
}

impl From<Arc<str>> for EffectParameterValue {
    fn from(value: Arc<str>) -> Self {
        Self::Text(value)
    }
}

/// The name of the source parameter of the graph of a custom effect
/// ([`CustomEffect::graph`]) that is the content of the visual the effect is
/// set on: `content`.
pub const CUSTOM_EFFECT_CONTENT: &str = "content";

/// An effect defined outside the framework: an immutable value of plain
/// data (numbers, colours, points; never a live object), shared with the
/// render thread as an [`ImmutableCustomEffect`].
///
/// ```ignore
/// #[derive(Clone, Debug, Default, PartialEq)]
/// struct Tint { color: Color, strength: f64 }
///
/// impl CustomEffect for Tint {
///     const NAME: &'static str = "tint";
///
///     fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
///         f("color", self.color.into());
///         f("strength", self.strength.into());
///     }
///
///     fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
///         match name {
///             "color" => value.as_color().map(|v| self.color = v).is_some(),
///             "strength" => value.as_float().map(|v| self.strength = v).is_some(),
///             _ => false,
///         }
///     }
/// }
///
/// CustomEffectRegistry::register::<Tint>()?;
/// border.set_effect(Some(ImmutableCustomEffect::new(Tint { color: Colors::TEAL, strength: 0.5 }).into()));
/// let parsed = Effect::parse("tint(color=#0F9D8E, strength=0.55)")?;
/// ```
pub trait CustomEffect: Clone + PartialEq + Default + fmt::Debug + Send + Sync + 'static {
    /// The name of the effect in effect strings: `tint` in
    /// `tint(color=#0F9D8E, strength=0.55)`. Letters, digits, `-` and `_`;
    /// matched ignoring case, `-` and `_`.
    const NAME: &'static str;

    /// Calls `f` with the name and the current value of each parameter, in
    /// a fixed order. Parsing finds a parameter here by name (ignoring case,
    /// `-` and `_`) and reads the text as a value of the kind of the current
    /// one.
    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue));

    /// Sets the parameter `name` (as [`for_each_parameter`](Self::for_each_parameter)
    /// spells it) to `value`. Returns `false` when there is no such parameter
    /// or the value is of the wrong kind.
    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool;

    /// How far the output of the effect extends beyond the bounds of its
    /// content. None by default.
    fn padding(&self) -> Thickness {
        Thickness::default()
    }

    /// The value of this effect that leaves its content unchanged, if there
    /// is one. A transition from no effect to this effect (or back)
    /// interpolates from (or to) it; without one the effect switches at
    /// half of the transition.
    fn neutral() -> Option<Self> {
        None
    }

    /// The value at `progress` (0 to 1, possibly beyond with an easing that
    /// overshoots) of the way from `self` to `to`. By default each parameter
    /// interpolates by its kind ([`EffectParameterValue::interpolate`]) and
    /// the others switch at half.
    fn interpolate(&self, to: &Self, progress: f64) -> Self {
        let mut result = if progress >= 0.5 { to.clone() } else { self.clone() };
        let mut targets = Vec::new();
        CustomEffect::for_each_parameter(to, &mut |name, value| targets.push((name, value)));
        let mut index = 0;
        CustomEffect::for_each_parameter(self, &mut |name, from| {
            if let Some((target_name, target)) = targets.get(index) {
                if *target_name == name {
                    if let Some(value) = from.interpolate(target, progress) {
                        result.set_parameter(name, value);
                    }
                }
            }
            index += 1;
        });
        result
    }

    /// The effect as an effect graph (#455, E2), in the vocabulary of the
    /// composition effects
    /// ([`GraphicsEffect`](crate::rendering::composition::effects::GraphicsEffect)):
    /// the description of the graph, whose source parameter `content`
    /// ([`CUSTOM_EFFECT_CONTENT`]) is the content of the visual the effect is
    /// set on, and whose animatable members take the values of
    /// [`for_each_graph_value`](Self::for_each_graph_value). `None` (the
    /// default) for an effect without a graph.
    ///
    /// The description is read once per type and loaded into a graph with
    /// the checks of `Compositor::create_effect_factory_with_properties`; the
    /// render walk paints the graph around the content of the visual (the
    /// composed content is the source parameter `content`, as the children
    /// of a layer visual are the first source parameter of its effect), and
    /// every frame only sets the values of the members. A graph whose source
    /// parameters are not all `content`, or that does not load, is reported
    /// once (a warning in the log area `Visual`) and the effect is drawn as
    /// an effect without a graph.
    ///
    /// The content is one layer: a graph uses it once, along one path of
    /// the graph; a second use of `content` draws nothing, except under a
    /// source-over composite of a shadow of the content and the content
    /// itself (a glow), which is drawn as the drop shadow effect of the
    /// drawing contexts.
    ///
    /// **Members.** The animatable members are named as the animatable
    /// properties of an effect factory, `EffectName.PropertyName`
    /// (`Tint.Color`), where `EffectName` is the `name` of an effect of the
    /// graph. A value of [`for_each_graph_value`](Self::for_each_graph_value)
    /// named with a dot sets that member; a value named without one (as the
    /// parameters are) sets the one animatable member of its kind of the
    /// effect of the graph with that name: a parameter `amount` (a number)
    /// sets `amount.Intensity` of a sepia effect named `amount`. A number
    /// sets a scalar member, a colour a colour member; other kinds are not
    /// members.
    fn graph() -> Option<GraphicsEffect> {
        None
    }

    /// Calls `f` with the name and the value of each animatable member of
    /// the [`graph`](Self::graph) for this value, in a fixed order (the order
    /// and the names do not depend on the value). By default the parameters
    /// ([`for_each_parameter`](Self::for_each_parameter)): each parameter is
    /// the value of the effect of the graph named after it. An effect whose
    /// members are computed from its parameters (a matrix from a strength, a
    /// clamped value) gives them here, named `EffectName.PropertyName`.
    fn for_each_graph_value(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        CustomEffect::for_each_parameter(self, f)
    }

    /// The CPU reference of the effect (decision 3 of the plan): applies the
    /// effect to `pixels`, premultiplied RGBA of eight bits a channel, `width`
    /// by `height` pixels in rows of `width * 4` bytes, the content drawn at
    /// `scale` pixels per device independent pixel over the area of the
    /// effect (its content and its padding). Returns whether the effect has
    /// a reference; `false` (the default) leaves the pixels unchanged.
    ///
    /// It is what the render tests compare the output of the GPU with, and
    /// the fallback of a backend that has no GPU program for the effect.
    /// Every effect FerroUI ships has one; an application's own effect may
    /// leave it out.
    fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool {
        let _ = (pixels, width, height, scale);
        false
    }

    /// The effect as a runtime shader (#455, E4, decision 4 of the plan): an
    /// SkSL program for Skia and a WGSL program for Vello, with the uniform
    /// convention of [`ShaderEffectDescription`] (the size and the scale of
    /// the layer, then the parameters in the order of
    /// [`for_each_parameter`](Self::for_each_parameter)), and whether the
    /// program samples the content or is an overlay. `None` (the default) for
    /// an effect without a shader.
    ///
    /// The description is read once per type, like the
    /// [`graph`](Self::graph); a frame only sets the uniforms from the
    /// parameters of the snapshot. A program that does not compile, or whose
    /// uniforms are not those of the convention, is reported once by the
    /// backend, which then draws the content unchanged.
    ///
    /// **Precedence.** A type with both a graph and a shader is drawn as its
    /// graph: the shader is not read (and the type is drawn as an effect
    /// without a graph when the graph does not load). Every backend draws a
    /// graph; a shader needs a backend that runs programs (Skia from E4 on,
    /// Vello from E5 on).
    fn shader() -> Option<ShaderEffectDescription> {
        None
    }
}

/// The type-erased form of a [`CustomEffect`], what an
/// [`ImmutableCustomEffect`] holds.
///
/// Its members have names of their own, so that they never compete with
/// those of [`CustomEffect`] in a method call.
pub(crate) trait AnyCustomEffect: fmt::Debug + Send + Sync + 'static {
    fn effect_name(&self) -> &'static str;
    fn value_as_any(&self) -> &dyn Any;
    fn value_equals(&self, other: &dyn AnyCustomEffect) -> bool;
    fn value_padding(&self) -> Thickness;
    fn visit_parameters(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue));
    fn interpolate_value(&self, to: &dyn AnyCustomEffect, progress: f64) -> Option<Arc<dyn AnyCustomEffect>>;
    fn neutral_value(&self) -> Option<Arc<dyn AnyCustomEffect>>;
    fn graph_of_type(&self) -> Option<Result<Arc<CustomEffectGraph>, CompositionEffectFactoryError>>;
    fn visit_graph_values(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue));
    fn value_cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool;
    fn shader_of_type(&self) -> Option<Arc<ShaderEffectDescription>>;
}

impl<T: CustomEffect> AnyCustomEffect for T {
    fn effect_name(&self) -> &'static str {
        T::NAME
    }

    fn value_as_any(&self) -> &dyn Any {
        self
    }

    fn value_equals(&self, other: &dyn AnyCustomEffect) -> bool {
        other.value_as_any().downcast_ref::<T>().is_some_and(|other| self == other)
    }

    fn value_padding(&self) -> Thickness {
        CustomEffect::padding(self)
    }

    fn visit_parameters(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        CustomEffect::for_each_parameter(self, f)
    }

    fn interpolate_value(&self, to: &dyn AnyCustomEffect, progress: f64) -> Option<Arc<dyn AnyCustomEffect>> {
        let to = to.value_as_any().downcast_ref::<T>()?;
        Some(Arc::new(CustomEffect::interpolate(self, to, progress)))
    }

    fn neutral_value(&self) -> Option<Arc<dyn AnyCustomEffect>> {
        T::neutral().map(|neutral| Arc::new(neutral) as Arc<dyn AnyCustomEffect>)
    }

    fn graph_of_type(&self) -> Option<Result<Arc<CustomEffectGraph>, CompositionEffectFactoryError>> {
        load_custom_effect_graph::<T>()
    }

    fn visit_graph_values(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        CustomEffect::for_each_graph_value(self, f)
    }

    fn value_cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool {
        CustomEffect::cpu_reference(self, pixels, width, height, scale)
    }

    fn shader_of_type(&self) -> Option<Arc<ShaderEffectDescription>> {
        load_custom_effect_shader::<T>()
    }
}

/// The immutable snapshot of a custom effect: what `Visual.Effect` holds
/// for a custom effect value and what crosses to the render thread.
///
/// It is `Send + Sync` and compares by value. Cloning shares the value.
#[derive(Clone, Debug)]
pub struct ImmutableCustomEffect {
    value: Arc<dyn AnyCustomEffect>,
}

const _: () = {
    fn assert_send_sync<T: Send + Sync>() {}
    let _ = assert_send_sync::<ImmutableCustomEffect>;
};

impl ImmutableCustomEffect {
    /// The snapshot of `value`.
    pub fn new<T: CustomEffect>(value: T) -> Self {
        Self { value: Arc::new(value) }
    }

    /// The name of the effect ([`CustomEffect::NAME`]).
    pub fn name(&self) -> &'static str {
        self.value.effect_name()
    }

    /// The type of the value, by which a backend finds its factory for the
    /// effect.
    pub fn value_type_id(&self) -> TypeId {
        self.value.value_as_any().type_id()
    }

    /// Whether the value is a `T`.
    pub fn is<T: CustomEffect>(&self) -> bool {
        self.value.value_as_any().is::<T>()
    }

    /// The value, when it is a `T`.
    pub fn downcast_ref<T: CustomEffect>(&self) -> Option<&T> {
        self.value.value_as_any().downcast_ref::<T>()
    }

    /// How far the output of the effect extends beyond the bounds of its
    /// content ([`CustomEffect::padding`]): each side at least zero, a value
    /// that is not finite read as zero. A shader effect whose padding is
    /// [`ShaderEffectPadding::SampleRadius`] adds its sample radius to each
    /// side.
    pub fn padding(&self) -> Thickness {
        let side = |value: f64| if value.is_finite() { value.max(0.0) } else { 0.0 };
        let padding = self.value.value_padding();
        let extra = match self.shader() {
            Some(shader) if shader.padding == ShaderEffectPadding::SampleRadius => side(shader.sample_radius),
            _ => 0.0,
        };
        Thickness::new(
            side(padding.left) + extra,
            side(padding.top) + extra,
            side(padding.right) + extra,
            side(padding.bottom) + extra,
        )
    }

    /// Calls `f` with the name and value of each parameter
    /// ([`CustomEffect::for_each_parameter`]), without allocating.
    pub fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        self.value.visit_parameters(f)
    }

    /// The parameters with their values.
    pub fn parameters(&self) -> Vec<(&'static str, EffectParameterValue)> {
        let mut parameters = Vec::new();
        self.for_each_parameter(&mut |name, value| parameters.push((name, value)));
        parameters
    }

    /// The value of the parameter `name` (ignoring case, `-` and `_`).
    pub fn parameter(&self, name: &str) -> Option<EffectParameterValue> {
        let key = normalize_identifier(name);
        let mut found = None;
        self.for_each_parameter(&mut |candidate, value| {
            if found.is_none() && normalize_identifier(candidate) == key {
                found = Some(value);
            }
        });
        found
    }

    /// The effect at `progress` of the way from `self` to `to`
    /// ([`CustomEffect::interpolate`]); `None` when the two are effects of
    /// different types.
    pub fn interpolate(&self, to: &ImmutableCustomEffect, progress: f64) -> Option<ImmutableCustomEffect> {
        self.value.interpolate_value(&*to.value, progress).map(|value| Self { value })
    }

    /// The value of this effect's type that leaves its content unchanged
    /// ([`CustomEffect::neutral`]).
    pub fn neutral(&self) -> Option<ImmutableCustomEffect> {
        self.value.neutral_value().map(|value| Self { value })
    }

    /// The graph of the type of the effect, loaded once per type
    /// ([`CustomEffect::graph`]); `None` when the type has none or it did not
    /// load.
    pub(crate) fn graph(&self) -> Option<Arc<CustomEffectGraph>> {
        self.value.graph_of_type().and_then(Result::ok)
    }

    /// Whether the type of the effect has a graph that loaded
    /// ([`CustomEffect::graph`]): the render walk draws the effect as that
    /// graph around the content of its visual.
    pub fn has_graph(&self) -> bool {
        self.graph().is_some()
    }

    /// Why the graph of the type of the effect did not load, as an effect
    /// factory reports it; `None` when it loaded or the type has none.
    pub fn graph_error(&self) -> Option<CompositionEffectFactoryError> {
        self.value.graph_of_type().and_then(Result::err)
    }

    /// The animatable members of the graph of the type: for each value of
    /// [`for_each_graph_value`](Self::for_each_graph_value), in order, its
    /// name and the full name of the member it sets
    /// (`EffectName.PropertyName`). Empty without a graph.
    pub fn graph_members(&self) -> Vec<(&'static str, String)> {
        self.graph().map(|graph| graph.members.clone()).unwrap_or_default()
    }

    /// Calls `f` with the name and value of each animatable member of the
    /// graph of the effect ([`CustomEffect::for_each_graph_value`]).
    pub fn for_each_graph_value(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        self.value.visit_graph_values(f)
    }

    /// Applies the CPU reference of the effect to premultiplied RGBA pixels
    /// ([`CustomEffect::cpu_reference`]); returns whether the effect has one.
    pub fn cpu_reference(&self, pixels: &mut [u8], width: usize, height: usize, scale: f64) -> bool {
        self.value.value_cpu_reference(pixels, width, height, scale)
    }

    /// The runtime shader of the type of the effect, read once per type
    /// ([`CustomEffect::shader`]); `None` when the type has none, or has a
    /// graph, which takes precedence.
    pub fn shader(&self) -> Option<Arc<ShaderEffectDescription>> {
        self.value.shader_of_type()
    }

    /// Whether the type of the effect is a runtime shader effect
    /// ([`shader`](Self::shader)).
    pub fn has_shader(&self) -> bool {
        self.shader().is_some()
    }

    /// Calls `f` with the name and the value of each uniform of the shader of
    /// the effect by the convention of [`ShaderEffectDescription`]:
    /// `layer_size` (the area of the effect in device pixels), `layer_scale`
    /// (device pixels per device independent pixel), then the parameters in
    /// order (text skipped), named as
    /// [`CustomEffect::for_each_parameter`] names them. Without allocating:
    /// what a backend calls each frame to fill the uniforms of the program.
    pub fn for_each_shader_uniform(
        &self,
        layer_size: [f32; 2],
        layer_scale: f32,
        f: &mut dyn FnMut(&'static str, ShaderUniformValue),
    ) {
        visit_shader_uniforms(self, layer_size, layer_scale, f)
    }

    /// The uniforms a program of the shader of this effect's type declares by
    /// the convention, in order: their names (a parameter's with every `-`
    /// written `_`) and types. What a backend checks a compiled program
    /// against, once.
    pub fn shader_uniform_layout(&self) -> Vec<(String, ShaderUniformType)> {
        let mut layout = Vec::new();
        self.for_each_shader_uniform([0.0; 2], 1.0, &mut |name, value| {
            layout.push((super::shader_uniform_name(name), value.ty()));
        });
        layout
    }

    /// Reports that `backend` has no factory for this effect, so that it
    /// draws the content of the effect unchanged. The first report of each
    /// backend and effect name is logged (a warning in the log area named
    /// by `backend`) and returns `true`; later ones return `false`. Called by
    /// the drawing contexts of the backends on the render thread.
    pub fn report_unhandled(&self, backend: &'static str) -> bool {
        static REPORTED: Mutex<Vec<(&'static str, &'static str)>> = Mutex::new(Vec::new());

        let name = self.name();
        {
            let mut reported = REPORTED.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if reported.contains(&(backend, name)) {
                return false;
            }
            reported.push((backend, name));
        }
        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, backend) {
            logger.log(
                None,
                &format!("The custom effect {name} has no factory in this backend; its content is drawn unchanged."),
            );
        }
        true
    }
}

impl PartialEq for ImmutableCustomEffect {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.value, &other.value) || self.value.value_equals(&*other.value)
    }
}

impl From<ImmutableCustomEffect> for Rc<dyn IEffect> {
    fn from(value: ImmutableCustomEffect) -> Self {
        Rc::new(value)
    }
}

/// The effect string: `name(parameter=value, ...)`, which
/// [`Effect::parse`](super::Effect::parse) reads back when the effect is
/// registered ([`CustomEffectRegistry`](super::CustomEffectRegistry)).
impl fmt::Display for ImmutableCustomEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.name())?;
        let mut result = Ok(());
        let mut first = true;
        self.for_each_parameter(&mut |name, value| {
            if result.is_ok() {
                let separator = if first { "" } else { ", " };
                first = false;
                result = write!(f, "{separator}{name}={value}");
            }
        });
        result?;
        f.write_str(")")
    }
}

impl IEffect for ImmutableCustomEffect {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_custom_effect(&self) -> Option<&ImmutableCustomEffect> {
        Some(self)
    }

    fn as_immutable_effect(&self) -> Option<&dyn IImmutableEffect> {
        Some(self)
    }

    fn into_immutable_effect(self: Rc<Self>) -> Option<Arc<dyn IImmutableEffect>> {
        Some(Arc::new(ImmutableCustomEffect::clone(&self)))
    }
}

impl IImmutableEffect for ImmutableCustomEffect {
    /// Whether `other` is a custom effect of the same type with the same
    /// values; a mutable effect class is compared by its snapshot.
    fn equals(&self, other: Option<&dyn IEffect>) -> bool {
        let Some(other) = other else { return false };
        match other.as_custom_effect() {
            Some(other) => self == other,
            None => super::EffectExtensions::to_custom_effect(other).is_some_and(|other| *self == other),
        }
    }
}

/// A mutable effect class whose values freeze into a custom effect: a class
/// deriving from [`Effect`](super::Effect) that constructs its base with
/// [`Effect::construct_custom`](super::Effect::construct_custom). Its handle
/// converts to `Rc<dyn IEffect>` as the built-in effect classes do; the
/// compositor sends its snapshot to the render thread and takes a new one
/// when the effect raises `invalidated`.
pub trait CustomEffectClass: ObjectType {
    /// The snapshot of the current values.
    fn to_custom_effect(&self) -> ImmutableCustomEffect;
}

/// The identifier `name` reduced to lowercase letters and digits, as
/// effect and parameter names are matched.
pub(crate) fn normalize_identifier(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}
