//! The values of the members of enumerations and of sets of flags: the
//! constant expressions a discriminant (`B = Self::A as i32 + 1`), a constant
//! of a `bitflags!` type (`const ALL = Self::A.bits() | 1 << 4;`) or an
//! associated constant of such a type (`pub const NONE: Flags = Flags::empty();`,
//! `pub const Enter: Key = Key::Return;`) is written as, evaluated from their
//! tokens.
//!
//! What is evaluated: integer literals; `-`; `|`, `^`, `&`, `<<`, `>>`, `+`,
//! `-`, `*` with the precedence of the language; parentheses; `as` to an
//! integer type the value fits; the other members of the same type
//! (`Self::A`, `Name::A`, with `.bits()`); `Name::empty()`, `Name::all()`,
//! `Name::from_bits_retain(..)`, `Name::from_bits_truncate(..)`, `.union(..)`,
//! `.intersection(..)`, `.difference(..)` of a set of flags; `MIN` and `MAX`
//! of the integer types. Anything else (a function of the crate, a constant
//! of another type, a floating point value) has no value here.

use proc_macro2::{Delimiter, TokenTree};

use super::tokens::{ident_of, is_punct, tokens_of, Cursor};

/// The type an expression is evaluated for.
pub(crate) struct Scope<'a> {
    /// The name of the type: `Name::A` names a member of it, as `Self::A` does.
    pub type_name: &'a str,
    /// The value of the member `name` of the type (a variant, a constant of the flags, an
    /// associated constant).
    pub member: &'a dyn Fn(&str) -> Option<i64>,
    /// The union of the constants of a set of flags (`all()`); nothing for an enumeration.
    pub all: &'a dyn Fn() -> Option<i64>,
}

/// The least and the greatest value of the integer type `name`, when both are values of
/// `i64`.
fn integer_range(name: &str) -> Option<(i64, i64)> {
    Some(match name {
        "i8" => (i8::MIN.into(), i8::MAX.into()),
        "i16" => (i16::MIN.into(), i16::MAX.into()),
        "i32" => (i32::MIN.into(), i32::MAX.into()),
        "i64" | "isize" => (i64::MIN, i64::MAX),
        "u8" => (u8::MIN.into(), u8::MAX.into()),
        "u16" => (u16::MIN.into(), u16::MAX.into()),
        "u32" => (u32::MIN.into(), u32::MAX.into()),
        _ => return None,
    })
}

/// The value of the constant expression `tokens`; nothing when a part of it is not one of
/// the forms this module evaluates.
pub(crate) fn evaluate(tokens: &[TokenTree], scope: &Scope) -> Option<i64> {
    let mut cursor = Cursor::new(tokens, 0);
    let value = binary(&mut cursor, scope, 0)?;
    cursor.is_end().then_some(value)
}

/// The binary operators, the loosest binding first.
const LEVELS: &[&[&str]] = &[&["|"], &["^"], &["&"], &["<<", ">>"], &["+", "-"], &["*"]];

/// The operator of `operators` at the cursor: one mark, or two written together.
fn operator(cursor: &Cursor, operators: &[&'static str]) -> Option<&'static str> {
    let at = |offset: usize, character: char| cursor.peek_at(offset).is_some_and(|token| is_punct(token, character));
    operators.iter().copied().find(|operator| {
        let mut characters = operator.chars();
        match (characters.next(), characters.next()) {
            (Some(first), Some(second)) => at(0, first) && at(1, second),
            // `|` and `&` are not the first half of `||` and `&&`, `<` and `>` no operators.
            (Some(first), None) => at(0, first) && !at(1, first),
            _ => false,
        }
    })
}

fn binary(cursor: &mut Cursor, scope: &Scope, level: usize) -> Option<i64> {
    let Some(operators) = LEVELS.get(level) else { return cast(cursor, scope) };
    let mut value = binary(cursor, scope, level + 1)?;
    while let Some(found) = operator(cursor, operators) {
        for _ in 0..found.len() {
            cursor.next();
        }
        let right = binary(cursor, scope, level + 1)?;
        value = match found {
            "|" => value | right,
            "^" => value ^ right,
            "&" => value & right,
            "<<" => value.checked_shl(u32::try_from(right).ok()?)?,
            ">>" => value.checked_shr(u32::try_from(right).ok()?)?,
            "+" => value.checked_add(right)?,
            "-" => value.checked_sub(right)?,
            _ => value.checked_mul(right)?,
        };
    }
    Some(value)
}

/// `value as T`: the value, when it is a value of the integer type `T`.
fn cast(cursor: &mut Cursor, scope: &Scope) -> Option<i64> {
    let value = unary(cursor, scope)?;
    while cursor.eat_ident("as") {
        let (least, greatest) = integer_range(&cursor.take_ident("an integer type").ok()?)?;
        if value < least || value > greatest {
            return None;
        }
    }
    Some(value)
}

fn unary(cursor: &mut Cursor, scope: &Scope) -> Option<i64> {
    if cursor.eat_punct('-') {
        return unary(cursor, scope)?.checked_neg();
    }
    let mut value = primary(cursor, scope)?;
    // The methods of a set of flags on a value: `.bits()`, `.union(other)`, ..
    while cursor.is_punct('.') {
        cursor.next();
        let method = cursor.take_ident("a method").ok()?;
        let (arguments, _) = cursor.take_group(Delimiter::Parenthesis, "the arguments").ok()?;
        value = match (method.as_str(), arguments.is_empty()) {
            ("bits", true) => value,
            ("union", false) => value | evaluate(&arguments, scope)?,
            ("intersection", false) => value & evaluate(&arguments, scope)?,
            ("difference", false) => value & !evaluate(&arguments, scope)?,
            _ => return None,
        };
    }
    Some(value)
}

fn primary(cursor: &mut Cursor, scope: &Scope) -> Option<i64> {
    match cursor.next()? {
        TokenTree::Literal(literal) => match syn::Lit::new(literal.clone()) {
            syn::Lit::Int(integer) => integer.base10_parse::<i64>().ok(),
            _ => None,
        },
        TokenTree::Group(group) if matches!(group.delimiter(), Delimiter::Parenthesis | Delimiter::None) => evaluate(&tokens_of(group.stream()), scope),
        token @ TokenTree::Ident(_) => {
            // `Type::NAME`, `Type::function(arguments)`.
            let head = ident_of(token)?;
            if !cursor.eat_path_separator() {
                return None;
            }
            let name = cursor.take_ident("a name").ok()?;
            let own = head == "Self" || head == scope.type_name;
            if cursor.is_group(Delimiter::Parenthesis) {
                let (arguments, _) = cursor.take_group(Delimiter::Parenthesis, "the arguments").ok()?;
                return match (own, name.as_str(), arguments.is_empty()) {
                    (true, "empty", true) => Some(0),
                    (true, "all", true) => (scope.all)(),
                    (true, "from_bits_retain" | "from_bits_truncate", false) => evaluate(&arguments, scope),
                    _ => None,
                };
            }
            if own {
                return (scope.member)(&name);
            }
            let (least, greatest) = integer_range(&head)?;
            match name.as_str() {
                "MIN" => Some(least),
                "MAX" => Some(greatest),
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> Option<i64> {
        let tokens = tokens_of(text.parse().expect("tokens"));
        let member = |name: &str| match name {
            "A" => Some(1),
            "B" => Some(2),
            "D" => Some(8),
            _ => None,
        };
        evaluate(&tokens, &Scope { type_name: "Flags", member: &member, all: &|| Some(11) })
    }

    /// Not from upstream: the constant expressions of discriminants and of flags are
    /// evaluated with the precedence of the language.
    #[test]
    fn constant_expressions_are_evaluated() {
        assert_eq!(value("3"), Some(3));
        assert_eq!(value("0x10"), Some(16));
        assert_eq!(value("1_000i32"), Some(1000));
        assert_eq!(value("-1"), Some(-1));
        assert_eq!(value("1 << 4"), Some(16));
        assert_eq!(value("1 << 2 | 1 << 0"), Some(5));
        assert_eq!(value("256 >> 4"), Some(16));
        assert_eq!(value("1 + 2 * 3"), Some(7));
        assert_eq!(value("(1 + 2) * 3"), Some(9));
        assert_eq!(value("7 & 3 ^ 1"), Some(2));
        assert_eq!(value("6 - 1 - 2"), Some(3));
        assert_eq!(value("i32::MAX"), Some(2147483647));
        assert_eq!(value("i32::MIN"), Some(-2147483648));
        assert_eq!(value("u8::MAX as i32"), Some(255));
        assert_eq!(value("Self::A as i32 + 1"), Some(2));
    }

    /// Not from upstream: the members of the type and the functions of a set of flags.
    #[test]
    fn members_of_the_type_are_looked_up() {
        assert_eq!(value("Self::B"), Some(2));
        assert_eq!(value("Flags::B"), Some(2));
        assert_eq!(value("Self::A.bits() | Self::B.bits()"), Some(3));
        assert_eq!(value("Flags::empty()"), Some(0));
        assert_eq!(value("Self::all()"), Some(11));
        assert_eq!(value("Flags::from_bits_retain(Self::A.bits() | 4)"), Some(5));
        assert_eq!(value("Flags::from_bits_truncate(6)"), Some(6));
        assert_eq!(value("Self::A.union(Self::D)"), Some(9));
        assert_eq!(value("Self::all().difference(Flags::B)"), Some(9));
        assert_eq!(value("Self::all().intersection(Flags::B)"), Some(2));
    }

    /// Not from upstream: what is not one of the forms has no value, and nothing is guessed.
    #[test]
    fn other_expressions_have_no_value() {
        assert_eq!(value("Self::Missing"), None);
        assert_eq!(value("Other::A"), None);
        assert_eq!(value("A"), None);
        assert_eq!(value("compute()"), None);
        assert_eq!(value("Flags::compute()"), None);
        assert_eq!(value("1.5"), None);
        assert_eq!(value("1 +"), None);
        assert_eq!(value("1 2"), None);
        assert_eq!(value("1 || 2"), None);
        assert_eq!(value("1 < 2"), None);
        assert_eq!(value("-1 as u32"), None);
        assert_eq!(value("u64::MAX"), None);
        assert_eq!(value("i64::MAX + 1"), None);
        assert_eq!(value("1 << 64"), None);
        assert_eq!(value("Self::A.complement()"), None);
    }
}
