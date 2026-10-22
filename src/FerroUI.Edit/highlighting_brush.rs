use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Brush, Color, IBrush, ISolidColorBrush};
use ferroui_base::StyledElement;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// A native brush could not be constructed from a highlighting value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HighlightingBrushError {
    InvalidValue(String),
    MissingContext,
    MissingResource(String),
    InvalidResource(String),
}

impl fmt::Display for HighlightingBrushError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue(value) => write!(f, "Invalid highlighting brush {value:?}"),
            Self::MissingContext => f.write_str("A resource brush needs a styled-element context"),
            Self::MissingResource(key) => write!(f, "Highlighting resource {key} was not found"),
            Self::InvalidResource(key) => {
                write!(f, "Highlighting resource {key} is not a brush or color")
            }
        }
    }
}

impl std::error::Error for HighlightingBrushError {}

/// Resolves a brush when native text is constructed. The optional context is a
/// real FerroUI styled element, so implementations can use its resource tree.
pub trait HighlightingBrush {
    fn get_brush(
        &self,
        context: Option<&StyledElement>,
    ) -> Result<Rc<dyn IBrush>, HighlightingBrushError>;

    fn get_color(
        &self,
        context: Option<&StyledElement>,
    ) -> Result<Option<Color>, HighlightingBrushError> {
        Ok(self
            .get_brush(context)?
            .as_solid_color_brush()
            .map(ISolidColorBrush::color))
    }
}

/// An immutable solid brush. Equality and hashing use the source color.
#[derive(Clone, Debug)]
pub struct SimpleHighlightingBrush {
    brush: Rc<ImmutableSolidColorBrush>,
}

impl SimpleHighlightingBrush {
    pub fn new(color: Color) -> Self {
        Self {
            brush: Rc::new(ImmutableSolidColorBrush::new(color)),
        }
    }

    /// Freezes a mutable brush, retaining opacity and transforms as well as color.
    pub fn from_brush(brush: &dyn ISolidColorBrush) -> Self {
        Self {
            brush: Rc::new(ImmutableSolidColorBrush::from_brush(brush)),
        }
    }

    pub fn color(&self) -> Color {
        self.brush.color()
    }
}

impl HighlightingBrush for SimpleHighlightingBrush {
    fn get_brush(
        &self,
        _context: Option<&StyledElement>,
    ) -> Result<Rc<dyn IBrush>, HighlightingBrushError> {
        Ok(self.brush.clone())
    }
}

impl PartialEq for SimpleHighlightingBrush {
    fn eq(&self, other: &Self) -> bool {
        self.color() == other.color()
    }
}

impl Eq for SimpleHighlightingBrush {}

impl Hash for SimpleHighlightingBrush {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.color().hash(state);
    }
}

impl fmt::Display for SimpleHighlightingBrush {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.color().fmt(f)
    }
}

/// A resource-backed extension of the highlighting brush contract. Resolution
/// uses the current logical resource tree and theme each time it is requested.
#[derive(Clone, Debug)]
pub struct ResourceHighlightingBrush {
    key: ResourceKey,
}

impl ResourceHighlightingBrush {
    pub fn new(key: impl Into<ResourceKey>) -> Self {
        Self { key: key.into() }
    }

    pub fn key(&self) -> &ResourceKey {
        &self.key
    }
}

impl HighlightingBrush for ResourceHighlightingBrush {
    fn get_brush(
        &self,
        context: Option<&StyledElement>,
    ) -> Result<Rc<dyn IBrush>, HighlightingBrushError> {
        let context = context.ok_or(HighlightingBrushError::MissingContext)?;
        let value = context
            .try_find_resource(&self.key, context.actual_theme_variant().as_ref())
            .flatten()
            .ok_or_else(|| HighlightingBrushError::MissingResource(format!("{:?}", self.key)))?;
        if let Some(brush) = value.downcast_ref::<Rc<dyn IBrush>>() {
            return Ok(brush.clone());
        }
        if let Some(color) = value.downcast_ref::<Color>() {
            return Ok(Rc::new(ImmutableSolidColorBrush::new(*color)));
        }
        if let Some(converted) =
            ValueTypes::try_convert(Some(&value), ValueType::of::<Rc<dyn IBrush>>()).flatten()
        {
            if let Some(brush) = converted.downcast_ref::<Rc<dyn IBrush>>() {
                return Ok(brush.clone());
            }
        }
        Err(HighlightingBrushError::InvalidResource(format!(
            "{:?}",
            self.key
        )))
    }
}

/// Parses native color syntax first, then resolves a resource name through the
/// supplied styled element. An explicit `resource:` prefix disambiguates a
/// resource whose name is also a known color.
pub fn resolve_highlighting_brush(
    value: &str,
    context: Option<&StyledElement>,
) -> Result<Rc<dyn IBrush>, HighlightingBrushError> {
    if let Some(key) = value.strip_prefix("resource:") {
        return ResourceHighlightingBrush::new(key).get_brush(context);
    }
    if let Ok(brush) = Brush::parse(value) {
        return Ok(brush);
    }
    if context.is_some() {
        ResourceHighlightingBrush::new(value).get_brush(context)
    } else {
        Err(HighlightingBrushError::InvalidValue(value.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::media::{Colors, SolidColorBrush};
    use ferroui_controls::TextBlock;
    use std::collections::hash_map::DefaultHasher;

    #[test]
    fn simple_brush_freezes_native_values_and_uses_color_equality() {
        let _scope = crate::tests::test_scope();
        let mutable = SolidColorBrush::new();
        mutable.set_color(Colors::RED);
        let native: Rc<dyn IBrush> = mutable.clone().into();
        let brush = SimpleHighlightingBrush::from_brush(native.as_solid_color_brush().unwrap());
        mutable.set_color(Colors::BLUE);
        assert_eq!(brush.get_color(None).unwrap(), Some(Colors::RED));
        let same = SimpleHighlightingBrush::new(Colors::RED);
        assert_eq!(brush, same);
        let mut left = DefaultHasher::new();
        let mut right = DefaultHasher::new();
        brush.hash(&mut left);
        same.hash(&mut right);
        assert_eq!(left.finish(), right.finish());
    }

    #[test]
    fn resource_brush_reads_the_native_resource_tree_each_time() {
        let _scope = crate::tests::test_scope();
        let owner = TextBlock::new();
        owner.resources().add_value("Accent", Colors::RED);
        let brush = ResourceHighlightingBrush::new("Accent");
        assert_eq!(brush.get_color(Some(&owner)).unwrap(), Some(Colors::RED));
        owner.resources().set("Accent", Some(Rc::new(Colors::BLUE)));
        assert_eq!(brush.get_color(Some(&owner)).unwrap(), Some(Colors::BLUE));
        owner.resources().set("Red", Some(Rc::new(Colors::GREEN)));
        assert_eq!(
            resolve_highlighting_brush("resource:Red", Some(&owner))
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Colors::GREEN
        );
        assert!(matches!(
            brush.get_brush(None),
            Err(HighlightingBrushError::MissingContext)
        ));
        assert!(ResourceHighlightingBrush::new("Missing")
            .get_brush(Some(&owner))
            .is_err());
        owner.resources().set("Invalid", Some(Rc::new(12usize)));
        assert!(ResourceHighlightingBrush::new("Invalid")
            .get_brush(Some(&owner))
            .is_err());
    }
}
