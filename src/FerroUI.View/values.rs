//! Short typed values for the setters of the builders: key gestures, transforms, shadows, effects
//! and brushes. Each is what the text form of the value parses to, without linking its parser.
//!
//! | text                               | typed                                             |
//! |------------------------------------|---------------------------------------------------|
//! | `"Ctrl+S"`                         | `ctrl(KeyCode::S)`                                |
//! | `"Ctrl+Shift+S"`                   | `ctrl(KeyCode::S).shift()`                        |
//! | `"translate(10px, 5px) rotate(45deg)"` | `translate(10.0, 5.0).rotate(45.0)`           |
//! | `"5 5 10 2 Red"`                   | `shadow(5.0, 5.0, 10.0, Colors::RED).spread(2.0)` |
//! | `"inset 0 1 2 Black"`              | `shadow(0.0, 1.0, 2.0, Colors::BLACK).inset()`    |
//! | `"blur(5)"`                        | `blur(5.0)`                                       |
//! | `"drop-shadow(2 2 4 Black)"`       | `drop_shadow(2.0, 2.0, 4.0, Colors::BLACK)`       |
//! | `"#1E293B"`                        | `rgb(0x1E293B)`                                   |
//! | `"#801E293B"`                      | `argb(0x801E293B)`                                |
//! | a gradient in XAML                 | `linear_gradient(&[(0.0, a), (1.0, b)])`          |

use crate::builder::IntoValue;
use ferroui_base::input::{Key as KeyCode, KeyGesture, KeyModifiers};
use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush};
use ferroui_base::media::transformation::TransformOperations;
use ferroui_base::media::{
    BoxShadow, BoxShadows, Color, GradientSpreadMethod, IBrush, IEffect, ITransform, ImmutableBlurEffect,
    ImmutableDropShadowEffect,
};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{Matrix, RelativePoint, RelativeUnit};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// The gesture `Ctrl` + `key`.
pub const fn ctrl(key: KeyCode) -> KeyGesture {
    KeyGesture::new(key, KeyModifiers::CONTROL)
}

/// The gesture `Shift` + `key`.
pub const fn shift(key: KeyCode) -> KeyGesture {
    KeyGesture::new(key, KeyModifiers::SHIFT)
}

/// The gesture `Alt` + `key`.
pub const fn alt(key: KeyCode) -> KeyGesture {
    KeyGesture::new(key, KeyModifiers::ALT)
}

/// The gesture `Cmd` (`Win`, `Meta`) + `key`.
pub const fn meta(key: KeyCode) -> KeyGesture {
    KeyGesture::new(key, KeyModifiers::META)
}

/// More modifiers on a gesture: `ctrl(KeyCode::S).shift()`.
pub trait KeyGestureModifiers: Sized {
    /// Adds `Ctrl`.
    fn ctrl(self) -> KeyGesture;
    /// Adds `Shift`.
    fn shift(self) -> KeyGesture;
    /// Adds `Alt`.
    fn alt(self) -> KeyGesture;
    /// Adds `Cmd` (`Win`, `Meta`).
    fn meta(self) -> KeyGesture;
}

impl KeyGestureModifiers for KeyGesture {
    fn ctrl(self) -> KeyGesture {
        KeyGesture::new(self.key(), self.key_modifiers().union(KeyModifiers::CONTROL))
    }

    fn shift(self) -> KeyGesture {
        KeyGesture::new(self.key(), self.key_modifiers().union(KeyModifiers::SHIFT))
    }

    fn alt(self) -> KeyGesture {
        KeyGesture::new(self.key(), self.key_modifiers().union(KeyModifiers::ALT))
    }

    fn meta(self) -> KeyGesture {
        KeyGesture::new(self.key(), self.key_modifiers().union(KeyModifiers::META))
    }
}

/// A key alone is the gesture of that key.
impl IntoValue<KeyGesture> for KeyCode {
    fn into_value(self) -> KeyGesture {
        KeyGesture::from_key(self)
    }
}

impl IntoValue<Option<KeyGesture>> for KeyCode {
    fn into_value(self) -> Option<KeyGesture> {
        Some(KeyGesture::from_key(self))
    }
}

/// One step of a [`Transforms`].
#[derive(Clone, Copy, Debug, PartialEq)]
enum TransformStep {
    Translate(f64, f64),
    Rotate(f64),
    Scale(f64, f64),
    Skew(f64, f64),
    Matrix(Matrix),
}

/// Transforms applied in order: `translate(10.0, 5.0).rotate(45.0)`. A setter of a transform takes
/// it. Angles are in degrees.
#[derive(Clone, Debug, PartialEq)]
pub struct Transforms {
    steps: Vec<TransformStep>,
}

/// A move by `(x, y)`.
pub fn translate(x: f64, y: f64) -> Transforms {
    Transforms { steps: vec![TransformStep::Translate(x, y)] }
}

/// A rotation by `degrees`.
pub fn rotate(degrees: f64) -> Transforms {
    Transforms { steps: vec![TransformStep::Rotate(MathUtilities::deg2rad(degrees))] }
}

/// A scale by `(x, y)`.
pub fn scale(x: f64, y: f64) -> Transforms {
    Transforms { steps: vec![TransformStep::Scale(x, y)] }
}

/// A skew by `(x, y)` degrees.
pub fn skew(x: f64, y: f64) -> Transforms {
    Transforms { steps: vec![TransformStep::Skew(MathUtilities::deg2rad(x), MathUtilities::deg2rad(y))] }
}

impl Transforms {
    /// Then a move by `(x, y)`.
    pub fn translate(mut self, x: f64, y: f64) -> Self {
        self.steps.push(TransformStep::Translate(x, y));
        self
    }

    /// Then a rotation by `degrees`.
    pub fn rotate(mut self, degrees: f64) -> Self {
        self.steps.push(TransformStep::Rotate(MathUtilities::deg2rad(degrees)));
        self
    }

    /// Then a scale by `(x, y)`.
    pub fn scale(mut self, x: f64, y: f64) -> Self {
        self.steps.push(TransformStep::Scale(x, y));
        self
    }

    /// Then a skew by `(x, y)` degrees.
    pub fn skew(mut self, x: f64, y: f64) -> Self {
        self.steps.push(TransformStep::Skew(MathUtilities::deg2rad(x), MathUtilities::deg2rad(y)));
        self
    }

    /// Then a matrix.
    pub fn matrix(mut self, matrix: Matrix) -> Self {
        self.steps.push(TransformStep::Matrix(matrix));
        self
    }

    /// The operations, as `TransformOperations::parse` gives them for the same text.
    pub fn to_operations(&self) -> Rc<TransformOperations> {
        let mut builder = TransformOperations::create_builder(self.steps.len());
        for step in &self.steps {
            match *step {
                TransformStep::Translate(x, y) => builder.append_translate(x, y),
                TransformStep::Rotate(radians) => builder.append_rotate(radians),
                TransformStep::Scale(x, y) => builder.append_scale(x, y),
                TransformStep::Skew(x, y) => builder.append_skew(x, y),
                TransformStep::Matrix(matrix) => builder.append_matrix(matrix),
            }
        }
        builder.build()
    }
}

impl IntoValue<Option<Rc<dyn ITransform>>> for Transforms {
    fn into_value(self) -> Option<Rc<dyn ITransform>> {
        Some(self.to_operations())
    }
}

/// A box shadow at the offset `(x, y)` with a blur radius and a color. `.spread(..)` and
/// `.inset()` ([`BoxShadowOptions`]) give the rest.
pub const fn shadow(offset_x: f64, offset_y: f64, blur: f64, color: Color) -> BoxShadow {
    BoxShadow { offset_x, offset_y, blur, spread: 0.0, color, is_inset: false }
}

/// The rest of a box shadow: `shadow(0.0, 1.0, 2.0, color).spread(1.0).inset()`.
pub trait BoxShadowOptions: Sized {
    /// The spread radius.
    fn spread(self, spread: f64) -> BoxShadow;
    /// An inner shadow.
    fn inset(self) -> BoxShadow;
}

impl BoxShadowOptions for BoxShadow {
    fn spread(self, spread: f64) -> BoxShadow {
        BoxShadow { spread, ..self }
    }

    fn inset(self) -> BoxShadow {
        BoxShadow { is_inset: true, ..self }
    }
}

impl IntoValue<BoxShadows> for BoxShadow {
    fn into_value(self) -> BoxShadows {
        BoxShadows::new(self)
    }
}

impl<const N: usize> IntoValue<BoxShadows> for [BoxShadow; N] {
    fn into_value(self) -> BoxShadows {
        match self.split_first() {
            Some((first, rest)) => BoxShadows::with_rest(*first, rest),
            None => BoxShadows::default(),
        }
    }
}

/// A blur of `radius`.
pub fn blur(radius: f64) -> ImmutableBlurEffect {
    ImmutableBlurEffect::new(radius)
}

/// A drop shadow at the offset `(x, y)` with a blur radius and a color.
pub fn drop_shadow(offset_x: f64, offset_y: f64, blur_radius: f64, color: Color) -> ImmutableDropShadowEffect {
    ImmutableDropShadowEffect::new(offset_x, offset_y, blur_radius, color, 1.0)
}

impl IntoValue<Option<Rc<dyn IEffect>>> for ImmutableBlurEffect {
    fn into_value(self) -> Option<Rc<dyn IEffect>> {
        Some(Rc::new(self))
    }
}

impl IntoValue<Option<Rc<dyn IEffect>>> for ImmutableDropShadowEffect {
    fn into_value(self) -> Option<Rc<dyn IEffect>> {
        Some(Rc::new(self))
    }
}

/// An opaque color from its red, green and blue: `#1E293B` is `rgb(0x1E293B)`. A setter of a brush
/// takes it, and every element that names one color shares one brush.
///
/// # Panics
/// When `value` has more than 24 bits (a color with alpha is [`argb`]). In a constant, this is a
/// compile error.
pub const fn rgb(value: u32) -> Color {
    assert!(value <= 0xFF_FFFF, "rgb takes 0xRRGGBB; use argb for a color with alpha");
    Color::from_uint32(0xFF00_0000 | value)
}

/// A color from its alpha, red, green and blue: `#801E293B` is `argb(0x801E293B)`.
pub const fn argb(value: u32) -> Color {
    Color::from_uint32(value)
}

/// The gradients whose brush is kept.
const KEPT_GRADIENTS: usize = 256;

ferroui_base::ferro_thread_local! {
    static GRADIENTS: RefCell<HashMap<Vec<u64>, Rc<dyn IBrush>>> = RefCell::new(HashMap::new());
}

/// A linear gradient: stops of an offset from 0 to 1 and a color, from the top left to the bottom
/// right unless [`horizontal`](Self::horizontal), [`vertical`](Self::vertical) or
/// [`points`](Self::points) say otherwise. A setter of a brush takes it, and every element that
/// names the same gradient shares one brush, as for a color.
#[derive(Clone, Debug, PartialEq)]
pub struct LinearGradient {
    stops: Vec<(f64, Color)>,
    start: RelativePoint,
    end: RelativePoint,
    opacity: f64,
    spread_method: GradientSpreadMethod,
}

/// A linear gradient of `stops`: `linear_gradient(&[(0.0, rgb(0x1E293B)), (1.0, Colors::WHITE)])`.
pub fn linear_gradient(stops: &[(f64, Color)]) -> LinearGradient {
    LinearGradient {
        stops: stops.to_vec(),
        start: RelativePoint::TOP_LEFT,
        end: RelativePoint::BOTTOM_RIGHT,
        opacity: 1.0,
        spread_method: GradientSpreadMethod::default(),
    }
}

impl LinearGradient {
    /// From the left edge to the right edge.
    pub fn horizontal(self) -> Self {
        let (start, end) = ((0.0, 0.5), (1.0, 0.5));
        self.points(
            RelativePoint::new(start.0, start.1, RelativeUnit::Relative),
            RelativePoint::new(end.0, end.1, RelativeUnit::Relative),
        )
    }

    /// From the top edge to the bottom edge.
    pub fn vertical(self) -> Self {
        let (start, end) = ((0.5, 0.0), (0.5, 1.0));
        self.points(
            RelativePoint::new(start.0, start.1, RelativeUnit::Relative),
            RelativePoint::new(end.0, end.1, RelativeUnit::Relative),
        )
    }

    /// From `start` to `end`.
    pub fn points(mut self, start: RelativePoint, end: RelativePoint) -> Self {
        self.start = start;
        self.end = end;
        self
    }

    /// The opacity of the brush.
    pub fn opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity;
        self
    }

    /// What the gradient does outside of its points.
    pub fn spread_method(mut self, spread_method: GradientSpreadMethod) -> Self {
        self.spread_method = spread_method;
        self
    }

    /// The value of the gradient as a key of the kept brushes.
    fn key(&self) -> Vec<u64> {
        let mut key = Vec::with_capacity(self.stops.len() * 2 + 8);
        for (offset, color) in &self.stops {
            key.push(offset.to_bits());
            key.push(u64::from(color.to_uint32()));
        }
        for point in [self.start, self.end] {
            key.push(point.point.x.to_bits());
            key.push(point.point.y.to_bits());
            key.push(point.unit as u64);
        }
        key.push(self.opacity.to_bits());
        key.push(self.spread_method as u64);
        key
    }

    fn make(&self) -> Rc<dyn IBrush> {
        let stops: Vec<_> = self.stops.iter().map(|&(offset, color)| ImmutableGradientStop::new(offset, color)).collect();
        Rc::new(ImmutableLinearGradientBrush::new(
            &stops,
            self.opacity,
            None,
            None,
            self.spread_method,
            Some(self.start),
            Some(self.end),
            None,
        ))
    }

    /// The brush of the gradient: the same object for the same gradient, on one thread, for the
    /// first [`KEPT_GRADIENTS`] gradients.
    pub fn to_brush(&self) -> Rc<dyn IBrush> {
        GRADIENTS
            .try_with(|gradients| {
                let mut gradients = gradients.borrow_mut();
                let key = self.key();
                if let Some(brush) = gradients.get(&key) {
                    return brush.clone();
                }
                let brush = self.make();
                if gradients.len() < KEPT_GRADIENTS {
                    gradients.insert(key, brush.clone());
                }
                brush
            })
            .unwrap_or_else(|_| self.make())
    }
}

impl IntoValue<Option<Rc<dyn IBrush>>> for LinearGradient {
    fn into_value(self) -> Option<Rc<dyn IBrush>> {
        Some(self.to_brush())
    }
}
