//! Port of upstream's `BugRepros.cs`.

use crate::test_base::TestBase;
use ferroui_base::layout::Orientation;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::Thickness;
use ferroui_controls::{Border, StackPanel};
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new("BugRepros")
}

#[test]
fn sibling_visuals_with_opacity_should_not_affect_each_other() {
    let t = base();
    let brushes: [Rc<dyn IBrush>; 10] = [
        Brushes::red(),
        Brushes::green(),
        Brushes::blue(),
        Brushes::yellow(),
        Brushes::magenta(),
        Brushes::cyan(),
        Brushes::orange(),
        Brushes::purple(),
        Brushes::pink(),
        Brushes::brown(),
    ];

    let stack_panel = StackPanel::new();
    stack_panel.set_orientation(Orientation::Vertical);
    stack_panel.set_width(300.0);
    stack_panel.set_height(500.0);
    stack_panel.set_background(Some(Brushes::white()));

    for brush in &brushes {
        let border = Border::new();
        border.set_width(280.0);
        border.set_height(40.0);
        border.set_border_thickness(Thickness::uniform(2.0));
        border.set_margin(Thickness::uniform(5.0));
        border.set_background(Some(brush.clone()));
        border.set_opacity(0.3);
        stack_panel.children().add(border);
    }

    t.render_to_file(&stack_panel, "Sibling_Visuals_With_Opacity_Should_Not_Affect_Each_Other");
    t.compare_images("Sibling_Visuals_With_Opacity_Should_Not_Affect_Each_Other");
}
