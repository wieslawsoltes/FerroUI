/// The CSS-like font subset accepted by text preparation backends.
#[derive(Clone, Debug, PartialEq)]
pub struct PretextFontDescriptor {
    pub size: f64,
    pub primary_family: String,
    pub weight: i32,
    pub italic: bool,
}

impl Default for PretextFontDescriptor {
    fn default() -> Self {
        Self::new(16.0, "Arial", 400, false)
    }
}

impl PretextFontDescriptor {
    pub fn new(size: f64, primary_family: impl Into<String>, weight: i32, italic: bool) -> Self {
        let primary_family = primary_family.into();
        let primary_family = if primary_family.trim().is_empty() { "Arial".into() } else { primary_family };
        Self { size, primary_family, weight, italic }
    }
}

pub struct PretextFontParser;

impl PretextFontParser {
    /// Parses the source's permissive CSS subset; absent pixel sizes use defaults.
    pub fn parse(font: &str) -> PretextFontDescriptor {
        let bytes = font.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if !bytes[index].is_ascii_digit() { index += 1; continue; }
            let start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() { index += 1; }
            if bytes.get(index) == Some(&b'.') && bytes.get(index + 1).is_some_and(u8::is_ascii_digit) {
                index += 1;
                while index < bytes.len() && bytes[index].is_ascii_digit() { index += 1; }
            }
            let number_end = index;
            while let Some(ch) = font[index..].chars().next() {
                if !ch.is_whitespace() { break; }
                index += ch.len_utf8();
            }
            if bytes.get(index..index + 2) != Some(b"px") { continue; }
            let Ok(size) = font[start..number_end].parse::<f64>() else { return PretextFontDescriptor::default(); };
            let mut family = font[index + 2..].trim();
            if family.starts_with('/') {
                family = family.find(' ').map(|space| family[space + 1..].trim()).unwrap_or("");
            }
            let mut weight = 400;
            let mut italic = false;
            for token in font[..start].split(' ').filter(|token| !token.is_empty()) {
                if token.eq_ignore_ascii_case("italic") || token.eq_ignore_ascii_case("oblique") {
                    italic = true;
                } else if token.eq_ignore_ascii_case("bold") {
                    weight = 700;
                } else if let Ok(value) = token.trim().parse::<i32>() {
                    weight = value;
                    break;
                }
            }
            let mut family = family.split(',').next().unwrap_or("").trim();
            if family.len() >= 2 && ((family.starts_with('"') && family.ends_with('"')) || (family.starts_with('\'') && family.ends_with('\''))) {
                family = family[1..family.len() - 1].trim();
            }
            return PretextFontDescriptor::new(size, family, weight, italic);
        }
        PretextFontDescriptor::default()
    }

    pub fn map_generic_family<'a>(primary_family: &'a str, sans_serif_fallback: &'a str, serif_fallback: &'a str, monospace_fallback: &'a str) -> &'a str {
        if primary_family.trim().is_empty() || primary_family.eq_ignore_ascii_case("sans-serif") || primary_family.eq_ignore_ascii_case("system-ui") {
            sans_serif_fallback
        } else if primary_family.eq_ignore_ascii_case("serif") {
            serif_fallback
        } else if primary_family.eq_ignore_ascii_case("monospace") || primary_family.eq_ignore_ascii_case("ui-monospace") {
            monospace_fallback
        } else {
            primary_family
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_font_subset_and_generic_families() {
        assert_eq!(PretextFontParser::parse("italic bold 18.5px/1.4 \"IBM Plex Sans\", serif"), PretextFontDescriptor::new(18.5, "IBM Plex Sans", 700, true));
        assert_eq!(PretextFontParser::parse("700 18px Inter").weight, 700);
        assert_eq!(PretextFontParser::parse("missing size"), PretextFontDescriptor::default());
        assert_eq!(PretextFontParser::parse("").primary_family, "Arial");
        assert_eq!(PretextFontParser::map_generic_family("SYSTEM-UI", "sans", "serif", "mono"), "sans");
        assert_eq!(PretextFontParser::map_generic_family("monospace", "sans", "serif", "mono"), "mono");
        assert_eq!(PretextFontParser::map_generic_family("Inter", "sans", "serif", "mono"), "Inter");
    }

    #[test]
    fn preserves_source_numeric_weight_token_order() {
        assert!(!PretextFontParser::parse("700 italic 16px Inter").italic);
        assert!(PretextFontParser::parse("italic 700 16px Inter").italic);
    }

    #[test]
    fn preserves_balanced_quotes_and_unicode_size_whitespace() {
        assert_eq!(PretextFontParser::parse("22\u{a0}px \"  Inter  \"").primary_family, "Inter");
        assert_eq!(PretextFontParser::parse("22\u{a0}px Inter").size, 22.0);
        assert_eq!(PretextFontParser::parse("16px \"Inter'").primary_family, "\"Inter'");
        assert_eq!(PretextFontParser::parse("700\t16px Inter").weight, 700);
        assert_eq!(PretextFontParser::map_generic_family("ui-monospace", "sans", "serif", "mono"), "mono");
    }
}
