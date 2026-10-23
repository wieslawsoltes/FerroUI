use std::rc::Rc;

/// Alignment at a paragraph tab position. Values follow the rich text object model.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TabAlignment {
    #[default]
    Left = 0,
    Center = 1,
    Right = 2,
    Decimal = 3,
    /// Draws a vertical rule without moving text or consuming a tab character.
    Bar = 4,
}

/// The leader drawn across the horizontal space introduced by a tab.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TabLeader {
    #[default]
    Spaces = 0,
    Dots = 1,
    Dashes = 2,
    Lines = 3,
    ThickLines = 4,
    Equals = 5,
}

crate::ferro_markup_enum!(TabAlignment { Left, Center, Right, Decimal, Bar },
    { namespace: "FerroUI.Media.TextFormatting" });
crate::ferro_markup_enum!(TabLeader { Spaces, Dots, Dashes, Lines, ThickLines, Equals },
    { namespace: "FerroUI.Media.TextFormatting" });

/// An explicit tab measured in layout units from the paragraph's leading edge.
/// Paragraph indentation does not change this origin. In a right-to-left paragraph
/// the leading edge is the right edge. Alignment describes physical left/right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextTabStop {
    pub position: f64,
    pub alignment: TabAlignment,
    pub leader: TabLeader,
}

impl TextTabStop {
    pub const fn new(position: f64, alignment: TabAlignment, leader: TabLeader) -> Self {
        Self {
            position,
            alignment,
            leader,
        }
    }
}

/// Validation failures for an immutable paragraph tab configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextTabError {
    InvalidPosition,
    TooManyStops,
    InvalidSyntax,
    InvalidIndex,
}

impl TextTabError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::InvalidPosition => "Tab position must be finite and nonnegative",
            Self::TooManyStops => "A paragraph supports at most 63 explicit tabs",
            Self::InvalidSyntax => "Expected semicolon-separated position[, alignment[, leader]] tab stops",
            Self::InvalidIndex => "Tab index is outside the explicit tab collection",
        }
    }
}
impl std::fmt::Display for TextTabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}
impl std::error::Error for TextTabError {}

/// A value configuration, shared immutably by paragraph properties and snapshots.
/// Replacing it raises one property change; querying or cloning it allocates nothing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextTabStops(Rc<[TextTabStop]>);

impl TextTabStops {
    pub const MAX_COUNT: usize = 63;

    /// Normalizes positions to ascending order. A later stop at the same position
    /// replaces its earlier alignment and leader, just as adding that stop does.
    pub fn new(stops: impl IntoIterator<Item = TextTabStop>) -> Result<Self, TextTabError> {
        let mut result = Self::default();
        for stop in stops {
            result = result.with_tab(stop)?;
        }
        Ok(result)
    }

    pub fn as_slice(&self) -> &[TextTabStop] {
        &self.0
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn with_tab(&self, mut stop: TextTabStop) -> Result<Self, TextTabError> {
        if !stop.position.is_finite() || stop.position < 0.0 {
            return Err(TextTabError::InvalidPosition);
        }
        if stop.position == 0.0 {
            stop.position = 0.0;
        }
        let index = self
            .0
            .binary_search_by(|other| other.position.total_cmp(&stop.position));
        let mut stops = self.0.to_vec();
        match index {
            Ok(index) => stops[index] = stop,
            Err(index) => {
                if stops.len() == Self::MAX_COUNT {
                    return Err(TextTabError::TooManyStops);
                }
                stops.insert(index, stop);
            }
        }
        Ok(Self(stops.into()))
    }

    /// Removing a nonexistent position leaves the value unchanged.
    pub fn without_tab(&self, mut position: f64) -> Result<Self, TextTabError> {
        if !position.is_finite() || position < 0.0 {
            return Err(TextTabError::InvalidPosition);
        }
        if position == 0.0 {
            position = 0.0;
        }
        let Ok(index) = self.0.binary_search_by(|other| other.position.total_cmp(&position)) else {
            return Ok(self.clone());
        };
        let mut stops = self.0.to_vec();
        stops.remove(index);
        Ok(Self(stops.into()))
    }

    pub fn get(&self, index: usize) -> Result<TextTabStop, TextTabError> {
        self.0.get(index).copied().ok_or(TextTabError::InvalidIndex)
    }

    /// XAML distances are layout units. Example: `48,Left;96,Decimal,Dots`.
    pub fn parse(text: &str) -> Result<Self, TextTabError> {
        let mut result = Self::default();
        if text.trim().is_empty() {
            return Ok(result);
        }
        for item in text.split(';') {
            let mut fields = item.split(',').map(str::trim);
            let position = fields
                .next()
                .ok_or(TextTabError::InvalidSyntax)?
                .parse()
                .map_err(|_| TextTabError::InvalidSyntax)?;
            let alignment = match fields.next().unwrap_or("Left") {
                "Left" => TabAlignment::Left,
                "Center" => TabAlignment::Center,
                "Right" => TabAlignment::Right,
                "Decimal" => TabAlignment::Decimal,
                "Bar" => TabAlignment::Bar,
                _ => return Err(TextTabError::InvalidSyntax),
            };
            let leader = match fields.next().unwrap_or("Spaces") {
                "Spaces" => TabLeader::Spaces,
                "Dots" => TabLeader::Dots,
                "Dashes" => TabLeader::Dashes,
                "Lines" => TabLeader::Lines,
                "ThickLines" => TabLeader::ThickLines,
                "Equals" => TabLeader::Equals,
                _ => return Err(TextTabError::InvalidSyntax),
            };
            if fields.next().is_some() {
                return Err(TextTabError::InvalidSyntax);
            }
            result = result.with_tab(TextTabStop::new(position, alignment, leader))?;
        }
        Ok(result)
    }
}

crate::ferro_markup_type!(
    struct TextTabStops {
        namespace: "FerroUI.Media.TextFormatting",
        handles: [TextTabStops],
        constructors: [() => TextTabStops::default],
        parse: TextTabStops::parse,
    }
);
