use crate::animation::Animatable;
use crate::media::effects::{IEffect, ImmutableBlurEffect, ImmutableDropShadowEffect};
use crate::media::{Color, Colors};
use crate::reactive::{Disposable, IDisposable};
use crate::utilities::{FormatError, HandlerList};
use crate::{ferro_class, ferro_impl_classes, FerroObject, FerroObjectImpl, FerroProperty, ObjectType, Upcast};
use std::rc::Rc;

/// Base class of the mutable effects.
#[repr(C)]
pub struct Effect {
    base: Animatable,
    invalidated: HandlerList<dyn Fn()>,
}

ferro_class!(Effect: Animatable);
crate::ferro_class_info!(Effect { interfaces: [std::rc::Rc<dyn crate::media::effects::IEffect>] });
ferro_impl_classes!(Effect: FerroObjectImpl);

impl Effect {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Animatable::construct(), invalidated: HandlerList::new() }
    }

    /// Marks a property on a class deriving from [`Effect`] as affecting the
    /// effect's visual appearance: a change raises
    /// [`invalidated`](Self::invalidated).
    pub fn affects_render<T: ObjectType + Upcast<Effect>>(properties: &[&'static FerroProperty]) {
        for property in properties {
            property.changed().subscribe(|e| {
                if let Some(sender) = e.sender().downcast_ref::<T>() {
                    let effect: &Effect = sender.upcast();
                    effect.raise_invalidated();
                }
            });
        }
    }

    /// Raises the invalidated notification.
    pub fn raise_invalidated(&self) {
        if self.invalidated.is_empty() {
            return;
        }
        for (_, handler) in self.invalidated.snapshot().iter() {
            handler();
        }
    }

    /// Subscribes to invalidation of the effect: raised when a change
    /// requires the affected content to be redrawn. Disposing the returned
    /// handle unsubscribes.
    pub fn invalidated(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.invalidated.add(Rc::new(handler));
        let object: &FerroObject = self.upcast();
        let weak = object.to_weak();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                if let Some(this) = this.downcast_ref::<Effect>() {
                    this.invalidated.remove(token);
                }
            }
        })
    }

    fn parse_error(s: &str) -> FormatError {
        FormatError::from_string(format!("Unable to parse effect: {s}"))
    }

    /// Parses an effect: `blur(<radius>)` or
    /// `drop-shadow(<offset-x> <offset-y> [<blur-radius> [<color>]])`.
    pub fn parse(s: &str) -> Result<Rc<dyn IEffect>, FormatError> {
        let mut r = TokenParser::new(s);
        if r.try_consume_str("blur") {
            let radius = match (r.try_consume('('), r.try_parse_double()) {
                (true, Some(radius)) if r.try_consume(')') && r.is_eof_with_whitespace() => radius,
                _ => return Err(Self::parse_error(s)),
            };
            return Ok(Rc::new(ImmutableBlurEffect::new(radius)));
        }

        if r.try_consume_str("drop-shadow") {
            if !r.try_consume('(') {
                return Err(Self::parse_error(s));
            }
            let Some(offset_x) = r.try_parse_double() else { return Err(Self::parse_error(s)) };
            let Some(offset_y) = r.try_parse_double() else { return Err(Self::parse_error(s)) };
            let mut blur_radius = 0.0;
            let color = Colors::BLACK;
            if !r.try_consume(')') {
                match r.try_parse_double() {
                    Some(value) if value >= 0.0 => blur_radius = value,
                    _ => return Err(Self::parse_error(s)),
                }

                if !r.try_consume(')') {
                    let Some(end_of_expression) = s.rfind(')') else { return Err(Self::parse_error(s)) };

                    if !TokenParser::new(&s[end_of_expression + 1..]).is_eof_with_whitespace() {
                        return Err(Self::parse_error(s));
                    }

                    if end_of_expression < r.position() {
                        return Err(Self::parse_error(s));
                    }
                    let Some(color) = Color::try_parse(s[r.position()..end_of_expression].trim_end()) else {
                        return Err(Self::parse_error(s));
                    };

                    return Ok(Rc::new(ImmutableDropShadowEffect::new(offset_x, offset_y, blur_radius, color, 1.0)));
                }
            }
            if !r.is_eof_with_whitespace() {
                return Err(Self::parse_error(s));
            }
            return Ok(Rc::new(ImmutableDropShadowEffect::new(offset_x, offset_y, blur_radius, color, 1.0)));
        }

        Err(Self::parse_error(s))
    }
}

/// The subset of the composition expression token parser that effect parsing
/// uses.
struct TokenParser<'a> {
    s: &'a str,
    position: usize,
}

impl<'a> TokenParser<'a> {
    fn new(s: &'a str) -> Self {
        Self { s, position: 0 }
    }

    /// The offset of the next unread byte in the parsed string.
    fn position(&self) -> usize {
        self.position
    }

    fn skip_whitespace(&mut self) {
        let trimmed = self.s.trim_start_matches(char::is_whitespace);
        self.advance(self.s.len() - trimmed.len());
    }

    fn try_consume(&mut self, c: char) -> bool {
        self.skip_whitespace();
        if !self.s.starts_with(c) {
            return false;
        }
        self.advance(c.len_utf8());
        true
    }

    fn try_consume_str(&mut self, s: &str) -> bool {
        self.skip_whitespace();
        if !self.s.starts_with(s) {
            return false;
        }
        self.advance(s.len());
        true
    }

    fn advance(&mut self, c: usize) {
        self.s = &self.s[c..];
        self.position += c;
    }

    fn try_parse_double(&mut self) -> Option<f64> {
        self.skip_whitespace();
        if self.s.is_empty() {
            return None;
        }
        let mut len = 0;
        let mut dot_count = 0;
        for (c, ch) in self.s.bytes().enumerate() {
            if ch.is_ascii_digit() {
                len = c + 1;
            } else if ch == b'.' && dot_count == 0 {
                len = c + 1;
                dot_count += 1;
            } else if ch == b'-' {
                if len != 0 {
                    return None;
                }
                len = c + 1;
            } else {
                break;
            }
        }

        let span = &self.s[..len];
        // A lone sign or decimal point is not a number.
        if !span.bytes().any(|b| b.is_ascii_digit()) {
            return None;
        }
        let res = span.parse::<f64>().ok()?;
        self.advance(len);
        Some(res)
    }

    fn is_eof_with_whitespace(&mut self) -> bool {
        self.skip_whitespace();
        self.s.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::effects::EffectExtensions;

    #[test]
    fn parse_parses_blur() {
        let effect = Effect::parse("blur(123.34)").unwrap();
        let effect = effect.as_any().downcast_ref::<ImmutableBlurEffect>().unwrap();
        assert_eq!(123.34, effect.radius());
    }

    const BLACK: u32 = 0xff000000;

    #[test]
    fn parse_parses_drop_shadow() {
        for (s, x, y, r, color) in [
            ("drop-shadow(10 20)", 10.0, 20.0, 0.0, BLACK),
            ("drop-shadow( 10  20 ) ", 10.0, 20.0, 0.0, BLACK),
            ("drop-shadow( 10  20 30 ) ", 10.0, 20.0, 30.0, BLACK),
            ("drop-shadow(10  20 30)", 10.0, 20.0, 30.0, BLACK),
            ("drop-shadow(-10  -20 30)", -10.0, -20.0, 30.0, BLACK),
            ("drop-shadow(10 20 30 #ffff00ff)", 10.0, 20.0, 30.0, 0xffff00ff),
            ("drop-shadow ( 10 20 30 #ffff00ff ) ", 10.0, 20.0, 30.0, 0xffff00ff),
            ("drop-shadow(10 20 30 red)", 10.0, 20.0, 30.0, 0xffff0000),
            ("drop-shadow ( 10   20   30 red  ) ", 10.0, 20.0, 30.0, 0xffff0000),
            ("drop-shadow(10 20 30 rgba(100, 30, 45, 90%))", 10.0, 20.0, 30.0, 0xe6641e2d),
            ("drop-shadow(10 20 30  rgba(100, 30, 45, 90%) ) ", 10.0, 20.0, 30.0, 0xe6641e2d),
        ] {
            let effect = Effect::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"));
            let effect = effect.as_any().downcast_ref::<ImmutableDropShadowEffect>().unwrap();
            assert_eq!(x, effect.offset_x(), "{s}");
            assert_eq!(y, effect.offset_y(), "{s}");
            assert_eq!(r, effect.blur_radius(), "{s}");
            assert_eq!(1.0, effect.opacity(), "{s}");
            assert_eq!(color, effect.color().to_uint32(), "{s}");
        }
    }

    #[test]
    fn invalid_effect_parse_fails() {
        for b in ["blur", "blur(", "blur()", "blur(123", "blur(aaab)", "drop-shadow(-10  -20 -30)"] {
            assert!(Effect::parse(b).is_err(), "{b}");
        }
    }

    #[test]
    fn padding_is_correctly_calculated() {
        for (effect, left, top, right, bottom) in [
            ("blur(2.5)", 4.0, 4.0, 4.0, 4.0),
            ("blur(0)", 0.0, 0.0, 0.0, 0.0),
            ("drop-shadow(10 15)", 0.0, 0.0, 10.0, 15.0),
            ("drop-shadow(10 15 5)", 0.0, 0.0, 16.0, 21.0),
            ("drop-shadow(0 0 5)", 6.0, 6.0, 6.0, 6.0),
            ("drop-shadow(3 3 5)", 3.0, 3.0, 9.0, 9.0),
        ] {
            let parsed = Effect::parse(effect).unwrap();
            let padding = EffectExtensions::get_effect_output_padding(Some(&*parsed));
            assert_eq!(left, padding.left, "{effect}");
            assert_eq!(top, padding.top, "{effect}");
            assert_eq!(right, padding.right, "{effect}");
            assert_eq!(bottom, padding.bottom, "{effect}");
        }
    }

    // --- not from upstream ---

    #[test]
    fn parse_rejects_lone_sign_and_trailing_garbage() {
        for s in ["blur(-)", "blur(.)", "blur(1) x", "drop-shadow(1 2 3 red) x", "drop-shadow(1 2 3 nocolor)", ""] {
            assert!(Effect::parse(s).is_err(), "{s}");
        }
    }
}
