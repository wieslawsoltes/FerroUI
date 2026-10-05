use crate::media::string_splitter::{split_respecting_brackets, StringSplitOptions};
use crate::media::BoxShadow;
use crate::utilities::FormatError;
use crate::Rect;
use std::fmt;
use std::rc::Rc;
use std::str::FromStr;

const SEPARATOR: char = ',';
const OPENING_PARENTHESIS: char = '(';
const CLOSING_PARENTHESIS: char = ')';

/// Represents a collection of [`BoxShadow`]s.
#[derive(Clone, Debug, Default)]
pub struct BoxShadows {
    first: BoxShadow,
    list: Option<Rc<[BoxShadow]>>,
    count: usize,
}

impl BoxShadows {
    /// Creates a collection containing a single shadow. A default shadow
    /// produces an empty collection.
    pub fn new(shadow: BoxShadow) -> Self {
        Self { first: shadow, list: None, count: if shadow == BoxShadow::default() { 0 } else { 1 } }
    }

    /// Creates a collection containing `first` followed by `rest`.
    pub fn with_rest(first: BoxShadow, rest: &[BoxShadow]) -> Self {
        Self { first, list: Some(Rc::from(rest)), count: 1 + rest.len() }
    }

    /// The number of shadows in the collection.
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// The shadow at `index`. Panics if the index is out of range.
    pub fn get(&self, index: usize) -> BoxShadow {
        if index >= self.count {
            panic!("index out of range");
        }
        if index == 0 {
            return self.first;
        }
        self.list.as_ref().expect("list is present when count > 1")[index - 1]
    }

    /// Iterates over the shadows.
    pub fn iter(&self) -> impl Iterator<Item = BoxShadow> + '_ {
        (0..self.count).map(move |index| self.get(index))
    }

    /// Parses a comma separated list of box shadows.
    pub fn parse(s: &str) -> Result<BoxShadows, FormatError> {
        let sp = split_respecting_brackets(
            s,
            &[SEPARATOR],
            OPENING_PARENTHESIS,
            CLOSING_PARENTHESIS,
            StringSplitOptions::REMOVE_EMPTY_ENTRIES,
        )?;
        if sp.is_empty() || (sp.len() == 1 && (sp[0].trim().is_empty() || sp[0] == "none")) {
            return Ok(BoxShadows::default());
        }

        let first = BoxShadow::parse(sp[0])?;
        if sp.len() == 1 {
            return Ok(BoxShadows::new(first));
        }

        let mut rest = Vec::with_capacity(sp.len() - 1);
        for part in &sp[1..] {
            rest.push(BoxShadow::parse(part)?);
        }
        Ok(BoxShadows::with_rest(first, &rest))
    }

    /// Transforms the specified bounding rectangle to account for every
    /// shadow in the collection.
    pub fn transform_bounds(&self, rect: Rect) -> Rect {
        let mut result = rect;
        for shadow in self.iter() {
            result = result.union(shadow.transform_bounds(rect));
        }
        result
    }

    /// Whether the collection contains an inset shadow.
    pub fn has_inset_shadows(&self) -> bool {
        self.iter().any(|shadow| shadow != BoxShadow::default() && shadow.is_inset)
    }
}

impl PartialEq for BoxShadows {
    fn eq(&self, other: &Self) -> bool {
        if other.count != self.count {
            return false;
        }
        (0..self.count).all(|index| self.get(index) == other.get(index))
    }
}

impl FromStr for BoxShadows {
    type Err = FormatError;
    #[inline]
    fn from_str(s: &str) -> Result<Self, FormatError> {
        BoxShadows::parse(s)
    }
}

impl fmt::Display for BoxShadows {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.count == 0 {
            return f.write_str("none");
        }
        for (index, shadow) in self.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            fmt::Display::fmt(&shadow, f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_none_returns_empty() {
        for input in ["none", " none "] {
            let bs = BoxShadows::parse(input).unwrap();
            assert_eq!(0, bs.count());
            assert_eq!(BoxShadows::default(), bs);
            assert_eq!("none", bs.to_string());
        }
    }

    #[test]
    fn parse_single_shadow_to_string_round_trip() {
        for input in [
            "0 0 5 0 #FF0000",
            "10 20 30 5 rgba(0,0,0,0.5)",
            "10 20 30 5 rgba(0, 0, 0, 0.5)",
            "  10  20  30  5  rgba(0,  0,  0,  0.5)  ",
        ] {
            let bs = BoxShadows::parse(input).unwrap();
            assert_eq!(1, bs.count());
            let reparsed = BoxShadows::parse(&bs.to_string()).unwrap();
            assert_eq!(bs, reparsed);
        }
    }

    #[test]
    fn transform_bounds_includes_shadow_expansion() {
        for (input, min_expansion) in [("0 0 5 0 #FF0000", 10.0), ("0 0 10 0 rgba(0,0,0,0.5)", 20.0)] {
            let bs = BoxShadows::parse(input).unwrap();
            let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
            let transformed = bs.transform_bounds(rect);
            assert!(transformed.width >= rect.width + min_expansion);
            assert!(transformed.height >= rect.height + min_expansion);
        }
    }

    #[test]
    fn parse_color_function_is_handled() {
        for input in ["5 5 10 0 rgba(10,20,30,0.4)", "5 5 10 0 hsla(10,20%,30%,0.4)", "5 5 10 0 hsva(10,20%,30%,0.4)"] {
            let bs = BoxShadows::parse(input).unwrap();
            assert_eq!(1, bs.count());
            let reparsed = BoxShadows::parse(&bs.to_string()).unwrap();
            assert_eq!(bs, reparsed);
        }
    }

    #[test]
    fn parse_multiple_shadows() {
        for (input, count) in [
            ("1 2 3 0 #FF0000", 1),
            ("10 20 30 5 rgba(0,0,0,0.5)", 1),
            ("1 2 3 0 #FF0000, 1 2 3 0 #FF0000", 2),
            ("10 20 30 5 rgba(0,0,0,0.5), 1 2 3 0 #FF0000", 2),
            ("10 20 30 5 rgba(0,0,0,0.5), 10 20 30 5 rgba(0,0,0,0.5)", 2),
            ("10 20 30 5 rgba(0,0,0,0.5), 10 20 30 5 rgba(0,0,0,0.5), 10 20 30 5 rgba(0,0,0,0.5)", 3),
            ("10 20 30 5 rgba(0,0,0,0.5), 10 20 30 5 #ffffff, 10 20 30 5 Red", 3),
            ("  10 20 30 5 rgba(0, 0, 0, 0.5), 10 20 30 5 rgba(0, 0, 0, 0.5), 10 20 30 5 rgba(0, 0, 0, 0.5)  ", 3),
            ("  10 20 30 5 rgba(0, 0, 0, 0.5), 10 20 30 5 #ffffff, 10 20 30 5 Red  ", 3),
        ] {
            let bs = BoxShadows::parse(input).unwrap();
            assert_eq!(count, bs.count());
            let reparsed = BoxShadows::parse(&bs.to_string()).unwrap();
            assert_eq!(bs, reparsed);
        }
    }
}
