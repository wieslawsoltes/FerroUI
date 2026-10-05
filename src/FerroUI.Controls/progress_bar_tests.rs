//! The reference implementation has no unit tests for `ProgressBar`; these
//! tests are specific to this port.

use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{test_scope, TestRoot};
use crate::{Border, Panel, ProgressBar};
use ferroui_base::layout::Orientation;
use ferroui_base::Ref;

fn progress_bar() -> Ref<ProgressBar> {
    let target = ProgressBar::new();
    target.set_template(Some(FuncControlTemplate::new(|_, scope| {
        let indicator = Border::new();
        indicator.set_name(Some("PART_Indicator".to_string()));
        let panel = Panel::new();
        panel.children().add(indicator.register_in_name_scope(&**scope));
        panel.upcast()
    })));
    target
}

#[test]
fn progress_bar_sizes_indicator_and_reports_percentage() {
    let _scope = test_scope();
    let target = progress_bar();
    target.set_width(200.0);
    target.set_height(10.0);
    target.set_minimum(0.0);
    target.set_maximum(200.0);
    target.set_range_value(50.0);

    let root = TestRoot::with_child(target.clone());
    root.execute_initial_layout_pass();

    let indicator = target.get_template_descendants().into_iter().find_map(|x| x.cast::<Border>()).unwrap();
    assert_eq!(50.0, indicator.width());
    assert!(indicator.height().is_nan());
    assert_eq!(25.0, target.percentage());
    assert!(target.classes().contains(":horizontal"));

    target.set_orientation(Orientation::Vertical);
    assert!(indicator.width().is_nan());
    assert_eq!(2.5, indicator.height());
    assert!(target.classes().contains(":vertical"));
}

#[test]
fn progress_bar_indeterminate_updates_template_settings() {
    let _scope = test_scope();
    let target = progress_bar();
    target.set_width(100.0);
    target.set_height(10.0);

    let root = TestRoot::with_child(target.clone());
    root.execute_initial_layout_pass();

    target.set_is_indeterminate(true);

    assert!(target.classes().contains(":indeterminate"));
    let settings = target.template_settings();
    assert_eq!(40.0, settings.container_width());
    assert_eq!(60.0, settings.container2_width());
    assert_eq!(40.0 * -1.8, settings.container_animation_start_position());
    assert_eq!(120.0, settings.container_animation_end_position());
    assert_eq!(-90.0, settings.container2_animation_start_position());
    assert_eq!(60.0 * 1.66, settings.container2_animation_end_position());
    assert_eq!(-100.0, settings.indeterminate_starting_offset());
    assert_eq!(100.0, settings.indeterminate_ending_offset());
}
