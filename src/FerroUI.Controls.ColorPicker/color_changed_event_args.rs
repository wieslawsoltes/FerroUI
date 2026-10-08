// Portions of this source file are adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

use ferroui_base::media::Color;

/// Holds the details of a ColorChanged event.
///
/// HSV color information is intentionally not provided.
/// Use [`Color::to_hsv`] to obtain it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorChangedEventArgs {
    old_color: Color,
    new_color: Color,
}

impl ColorChangedEventArgs {
    /// Initializes a new instance of the [`ColorChangedEventArgs`] class.
    ///
    /// `old_color` is the old/original color from before the change event;
    /// `new_color` is the new/updated color that triggered the change event.
    pub fn new(old_color: Color, new_color: Color) -> Self {
        Self { old_color, new_color }
    }

    /// Gets the old/original color from before the change event.
    pub fn old_color(&self) -> Color {
        self.old_color
    }

    /// Gets the new/updated color that triggered the change event.
    pub fn new_color(&self) -> Color {
        self.new_color
    }
}
