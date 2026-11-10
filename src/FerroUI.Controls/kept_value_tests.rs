//! Not ports: the copies the controls keep of the value properties their layout reads many times
//! a frame (docs/porting/desktop-performance.md, section 9.28) are the values `get_value` reads
//! after every kind of change.

use crate::{Border, ColumnDefinition, Decorator, GridLength, GridUnitType, RowDefinition};
use ferroui_base::data::BindingPriority;
use ferroui_base::media::BackgroundSizing;
use ferroui_base::{CornerRadius, FerroObjectImplExt, Thickness};

#[test]
fn the_kept_values_of_a_border_follow_local_style_and_cleared_values() {
    let border = Border::new();
    assert_eq!(Thickness::default(), border.border_thickness());
    assert_eq!(CornerRadius::default(), border.corner_radius());
    assert_eq!(BackgroundSizing::default(), border.background_sizing());
    assert_eq!(Thickness::default(), border.padding());

    border.set_border_thickness(Thickness::uniform(2.0));
    assert_eq!(Thickness::uniform(2.0), border.border_thickness());
    // The other kept values are read again with it, and are still their own.
    assert_eq!(CornerRadius::default(), border.corner_radius());

    border.set_corner_radius(CornerRadius::uniform(3.0));
    border.set_background_sizing(BackgroundSizing::OuterBorderEdge);
    border.set_padding(Thickness::uniform(5.0));
    assert_eq!(Thickness::uniform(2.0), border.border_thickness());
    assert_eq!(CornerRadius::uniform(3.0), border.corner_radius());
    assert_eq!(BackgroundSizing::OuterBorderEdge, border.background_sizing());
    assert_eq!(Thickness::uniform(5.0), border.padding());

    // A style value under the local one: the local value stays effective until it is cleared.
    let _style = border.set_value_with_priority(
        Border::border_thickness_property(),
        Thickness::uniform(7.0),
        BindingPriority::Style,
    );
    assert_eq!(Thickness::uniform(2.0), border.border_thickness());
    border.clear_value(Border::border_thickness_property());
    assert_eq!(Thickness::uniform(7.0), border.border_thickness());

    border.clear_value(Border::corner_radius_property());
    border.clear_value(Border::background_sizing_property());
    border.clear_value(Decorator::padding_property());
    assert_eq!(CornerRadius::default(), border.corner_radius());
    assert_eq!(BackgroundSizing::default(), border.background_sizing());
    assert_eq!(Thickness::default(), border.padding());
}

#[test]
fn the_kept_sizes_of_a_definition_follow_its_properties() {
    let column = ColumnDefinition::new();
    assert_eq!(column.width(), column.user_size());
    assert_eq!(column.min_width(), column.user_min_size());
    assert_eq!(column.max_width(), column.user_max_size());

    column.set_width(GridLength::new(40.0, GridUnitType::Pixel));
    column.set_min_width(10.0);
    column.set_max_width(90.0);
    assert_eq!(GridLength::new(40.0, GridUnitType::Pixel), column.user_size());
    assert_eq!(10.0, column.user_min_size());
    assert_eq!(90.0, column.user_max_size());

    column.set_width(GridLength::new(2.0, GridUnitType::Star));
    assert_eq!(GridLength::new(2.0, GridUnitType::Star), column.user_size());
    column.clear_value(ColumnDefinition::width_property());
    column.clear_value(ColumnDefinition::min_width_property());
    column.clear_value(ColumnDefinition::max_width_property());
    assert_eq!(column.width(), column.user_size());
    assert_eq!(column.min_width(), column.user_min_size());
    assert_eq!(column.max_width(), column.user_max_size());

    let row = RowDefinition::new();
    row.set_height(GridLength::auto());
    assert_eq!(GridLength::auto(), row.user_size());
    row.set_height(GridLength::new(12.0, GridUnitType::Pixel));
    row.set_min_height(4.0);
    row.set_max_height(30.0);
    assert_eq!(GridLength::new(12.0, GridUnitType::Pixel), row.user_size());
    assert_eq!(4.0, row.user_min_size());
    assert_eq!(30.0, row.user_max_size());
}
