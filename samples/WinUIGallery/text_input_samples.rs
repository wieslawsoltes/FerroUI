//! The code behind of the pinned NumberBox page in Rust: the constructor and
//! `SpinButtonPlacementGroup_SelectionChanged` of `NumberBoxPage.xaml.cs`. The TextBlock and
//! TextBox pages have no code-behind upstream, and the PasswordBox handlers are in
//! `input_samples.rs`. The controls own editing and parsing; the sample owns only its options.

use ferroui_base::Ref;
use ferroui_controls::{
    Control, INumberBoxNumberFormatter, NumberBox, NumberBoxSpinButtonPlacementMode, RadioButtons,
};
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id == "NumberBox" {
        wire_number_box(root);
    }
}

/// The formatter `SetNumberBoxNumberFormatter` of `NumberBoxPage.xaml.cs` gives the dollar box:
/// a `DecimalFormatter` with one integer digit and two fraction digits whose
/// `IncrementNumberRounder` rounds to an increment of 0.25, half up.
///
/// Deviation: the source's `DecimalFormatter` uses the decimal separator of the user's region;
/// this formatter writes and reads the invariant `.`.
pub(crate) struct DollarAmountFormatter;

impl DollarAmountFormatter {
    /// `IncrementNumberRounder.Increment`.
    pub(crate) const INCREMENT: f64 = 0.25;

    /// `IncrementNumberRounder.RoundDouble` with `RoundingAlgorithm.RoundHalfUp`: the nearest
    /// multiple of the increment, a value halfway between two rounded towards positive infinity.
    pub(crate) fn round(value: f64) -> f64 {
        (value / Self::INCREMENT + 0.5).floor() * Self::INCREMENT
    }
}

impl INumberBoxNumberFormatter for DollarAmountFormatter {
    fn format_double(&self, value: f64) -> String {
        // `FractionDigits = 2`; `IntegerDigits = 1` keeps the zero of a value below one.
        let rounded = Self::round(value);
        // A negative value that rounds to zero is written without its sign.
        let rounded = if rounded == 0.0 { 0.0 } else { rounded };
        format!("{rounded:.2}")
    }

    fn parse_double(&self, text: &str) -> Option<f64> {
        // `DecimalFormatter.ParseDouble` reads a decimal number: an optional sign, digits and one
        // decimal separator; no exponent, no infinity and no NaN.
        let digits = text.strip_prefix(['-', '+']).unwrap_or(text);
        let mut separators = 0;
        let mut has_digit = false;
        for character in digits.chars() {
            match character {
                '0'..='9' => has_digit = true,
                '.' => separators += 1,
                _ => return None,
            }
        }
        if !has_digit || separators > 1 {
            return None;
        }
        text.parse().ok()
    }
}

/// The constructor (`SetNumberBoxNumberFormatter`) and `SpinButtonPlacementGroup_SelectionChanged`
/// of `NumberBoxPage.xaml.cs`: the dollar box gets the rounding formatter, and the selected option
/// places the spin buttons (0 Inline, 1 Compact).
fn wire_number_box(root: &Ref<Control>) {
    root.get_control::<NumberBox>("FormattedNumberBox")
        .set_number_formatter(Rc::new(DollarAmountFormatter));
    let group = root.get_control::<RadioButtons>("SpinButtonPlacementGroup");
    let weak_group = group.downgrade();
    let weak_box = root
        .get_control::<NumberBox>("NumberBoxSpinButtonPlacementExample")
        .downgrade();
    group.selection_changed(move |_, _| {
        let (Some(group), Some(number_box)) = (weak_group.upgrade(), weak_box.upgrade()) else {
            return;
        };
        match group.selected_index() {
            0 => {
                number_box.set_spin_button_placement_mode(NumberBoxSpinButtonPlacementMode::Inline)
            }
            1 => {
                number_box.set_spin_button_placement_mode(NumberBoxSpinButtonPlacementMode::Compact)
            }
            _ => {}
        }
    });
}
