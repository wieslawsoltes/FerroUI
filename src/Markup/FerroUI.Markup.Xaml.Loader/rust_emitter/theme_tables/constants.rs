//! Only non-fallible, built-in literal constructors qualify. The typed call emitted by
//! XamlX remains the fallback for other receivers, key types, values and source-info writes.

use super::{text_literal, Site, SiteBody, ThemeTables};
use ferroui_xaml_binary::{Kind, Value};

pub(super) enum Constant {
    Color(u32),
    Double(u64),
    Thickness([u64; 4]),
    CornerRadius([u64; 4]),
}

impl Constant {
    pub fn layout(&self) -> (Kind, &'static str, &'static str) {
        match self {
            Self::Color(_) => (Kind::Color, "colors", "u32"),
            Self::Double(_) => (Kind::Double, "doubles", "u64"),
            Self::Thickness(_) => (Kind::Thickness, "thicknesses", "[u64; 4]"),
            Self::CornerRadius(_) => (Kind::CornerRadius, "corner_radii", "[u64; 4]"),
        }
    }

    pub fn value(&self) -> Value<'static> {
        match *self {
            Self::Color(v) => Value::Argb(v),
            Self::Double(v) => Value::Double(v),
            Self::Thickness(v) | Self::CornerRadius(v) => Value::Quad(v),
        }
    }

    fn number(operand: &str) -> Option<u64> {
        Some(operand.strip_suffix("_f64")?.parse::<f64>().ok()?.to_bits())
    }

    fn recognize(value: &str, prelude: &[String]) -> Option<Self> {
        if let [bound] = prelude {
            let local = value
                .strip_prefix("::core::clone::Clone::clone(&")?
                .strip_suffix(')')?;
            let color = bound
                .trim_start()
                .strip_prefix(&format!(
                    "let {local} = ::ferroui_base::media::Color::__markup_FromUInt32_2("
                ))?
                .strip_suffix("_u32);")?;
            return Some(Self::Color(color.parse().ok()?));
        }
        if !prelude.is_empty() {
            return None;
        }
        if let Some(bits) = Self::number(value) {
            return Some(Self::Double(bits));
        }
        for (prefix, make) in [
            (
                "::ferroui_base::Thickness::__markup_new_3(",
                Self::Thickness as fn([u64; 4]) -> Self,
            ),
            (
                "::ferroui_base::CornerRadius::__markup_new_3(",
                Self::CornerRadius,
            ),
        ] {
            if let Some(args) = value.strip_prefix(prefix) {
                let mut args = args.strip_suffix(')')?.split(", ").map(Self::number);
                let values = [args.next()??, args.next()??, args.next()??, args.next()??];
                if args.next().is_none() {
                    return Some(make(values));
                }
            }
        }
        None
    }
}

impl ThemeTables {
    pub fn constant(
        &mut self,
        dictionary: &str,
        args: &[String],
        prelude: &[String],
        call: &str,
        line: i32,
        column: i32,
    ) -> Option<String> {
        let [receiver, key, value] = args else { return None };
        if *receiver != format!("&rt::cast({dictionary}.clone(), {line}, {column})?") {
            return None;
        }
        let key = text_literal(key.strip_prefix("rt::string_object(")?.strip_suffix(')')?)?;
        let operand = value.strip_prefix("rt::to_object(")?.strip_suffix(')')?;
        let constant = Constant::recognize(operand, prelude)?;
        let direct = match prelude {
            [bound] => format!("{}\n{call};", bound.trim_start()),
            _ => format!("{call};"),
        };
        let index = self.sites.len();
        self.sites.push(Site {
            dictionary: format!("&{dictionary}"),
            direct,
            key,
            line: line.try_into().ok()?,
            column: column.try_into().ok()?,
            body: SiteBody::Constant(constant),
        });
        Some(format!("@theme-site {index}"))
    }
}
