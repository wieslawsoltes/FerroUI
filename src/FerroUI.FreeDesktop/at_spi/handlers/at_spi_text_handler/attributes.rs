use crate::at_spi::dbus::types::AtSpiAttributeSet;
use ferroui_controls::automation::provider::{TextAttribute as A, TextAttributeSet, TextAttributeValue as V};

pub(super) fn serialize(values: TextAttributeSet) -> AtSpiAttributeSet {
    let mut attributes = AtSpiAttributeSet::new();
    let mut subscript = None;
    let mut superscript = None;
    for (attribute, value) in values {
        let (name, value) = match (attribute, value) {
            (A::FontName, V::String(name)) => ("family-name", name.replace('\0', "\u{fffd}")),
            (A::FontSize, V::Number(size)) if size.is_finite() => ("size", size.to_string()),
            (A::FontWeight, V::Integer(weight)) => ("weight", weight.to_string()),
            (A::FontStyle, V::String(style)) if matches!(style.as_str(), "normal" | "italic" | "oblique") => {
                ("style", style)
            }
            (A::ForegroundColor | A::BackgroundColor, V::Color(color)) => {
                let name = if attribute == A::ForegroundColor { "fg-color" } else { "bg-color" };
                let [r, g, b] = [color.r, color.g, color.b].map(|byte| u16::from(byte) * 257);
                (name, format!("{r},{g},{b}"))
            }
            (A::UnderlineStyle | A::OverlineStyle, V::Integer(style @ (0 | 1))) => {
                let name = if attribute == A::UnderlineStyle { "underline" } else { "overline" };
                (name, if style == 1 { "single" } else { "none" }.into())
            }
            (A::StrikethroughStyle, V::Integer(style @ (0 | 1))) => ("strikethrough", (style == 1).to_string()),
            (A::IsReadOnly, V::Bool(read_only)) => ("editable", (!read_only).to_string()),
            (A::IsHidden, V::Bool(hidden)) => ("invisible", hidden.to_string()),
            (A::IsSubscript, V::Bool(enabled)) => {
                subscript = Some(enabled);
                continue;
            }
            (A::IsSuperscript, V::Bool(enabled)) => {
                superscript = Some(enabled);
                continue;
            }
            _ => continue,
        };
        attributes.insert(name.into(), value);
    }
    let position = match (subscript, superscript) {
        (Some(true), Some(false) | None) => Some("sub"),
        (Some(false) | None, Some(true)) => Some("super"),
        (Some(false), Some(false)) => Some("baseline"),
        _ => None,
    };
    if let Some(position) = position {
        attributes.insert("text-position".into(), position.into());
    }
    attributes
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::Color;

    #[test]
    fn native_values_use_at_spi_units_and_omit_inexpressible_attributes() {
        let attributes = serialize(vec![
            (A::FontSize, V::Number(14.25)),
            (A::FontStyle, V::String("oblique".into())),
            (A::ForegroundColor, V::Color(Color::from_rgb(255, 128, 0))),
            (A::UnderlineStyle, V::Integer(1)),
            (A::OverlineStyle, V::NotSupported),
            (A::StrikethroughStyle, V::Integer(0)),
            (A::IsReadOnly, V::Bool(true)),
            (A::IsHidden, V::Bool(false)),
            (A::IsSubscript, V::Bool(false)),
            (A::IsSuperscript, V::Bool(true)),
        ]);
        assert_eq!(attributes["size"], "14.25");
        assert_eq!(attributes["style"], "oblique");
        assert_eq!(attributes["fg-color"], "65535,32896,0");
        assert_eq!(attributes["underline"], "single");
        assert_eq!(attributes["strikethrough"], "false");
        assert_eq!(attributes["editable"], "false");
        assert_eq!(attributes["invisible"], "false");
        assert_eq!(attributes["text-position"], "super");
        assert!(!attributes.contains_key("overline"));
    }
}
