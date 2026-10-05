//! Tests of the column header of the table view.

use crate::primitives::{TemplatedControl, Thumb};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::test_support::TestRoot;
use crate::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
use crate::{GridLength, GridUnitType, TableViewColumn, TableViewColumnHeader};
use ferroui_base::input::VectorEventArgs;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::styling::{ControlTheme, Setter};
use ferroui_base::{Ref, Vector};
use std::rc::Rc;

/// A running unit test application with the text services of the tests
/// registered over its services.
struct AppScope {
    // Dropped in this order: the text services first.
    _text: TextTestScope,
    _app: UnitTestApplicationScope,
}

fn start() -> AppScope {
    let app = UnitTestApplication::start(TestServices::mock_platform_render_interface());
    AppScope { _text: TextTestScope::new(), _app: app }
}

fn table_view_column_header_theme(resizer_width: f64) -> Ref<ControlTheme> {
    let template: Rc<dyn IControlTemplate> = FuncControlTemplate::for_type::<TableViewColumnHeader>(move |_, scope| {
        let thumb = Thumb::new();
        thumb.set_name(Some("PART_Resizer".to_string()));
        thumb.set_width(resizer_width);
        thumb.register_in_name_scope(&**scope).upcast()
    });

    ControlTheme::with_setters(
        TableViewColumnHeader::TYPE,
        [Setter::new(TemplatedControl::template_property(), Some(template))],
    )
}

fn resizer(header: &TableViewColumnHeader) -> Ref<Thumb> {
    let mut thumbs: Vec<Ref<Thumb>> =
        header.get_visual_descendants().into_iter().filter_map(|visual| visual.cast::<Thumb>()).collect();
    assert_eq!(thumbs.len(), 1);
    thumbs.remove(0)
}

fn raise_drag_delta(thumb: &Thumb, vector: Vector) {
    let mut e = VectorEventArgs::new();
    e.set_routed_event(Some(Thumb::drag_delta_event()));
    e.vector = vector;
    thumb.raise_event(&e);
}

#[test]
fn dragging_resizer_sets_column_width_to_pixel_value() {
    let _app = start();

    let column = TableViewColumn::new();
    column.set_width(GridLength::new(1.0, GridUnitType::Star));
    column.set_can_user_resize(Some(true));
    column.set_actual_width(100.0);

    let header = TableViewColumnHeader::new();
    header.set_column(Some(column.clone()));
    header.set_theme(table_view_column_header_theme(0.0));

    let root = TestRoot::new();
    root.set_child(header.clone());
    root.execute_initial_layout_pass();

    let thumb = resizer(&header);
    raise_drag_delta(&thumb, Vector::new(15.0, 0.0));

    assert_eq!(column.width(), GridLength::from_pixels(115.0));
}

#[test]
fn dragging_resizer_clamps_column_width_to_resizer_width() {
    let _app = start();

    let column = TableViewColumn::new();
    column.set_width(GridLength::from_pixels(50.0));
    column.set_can_user_resize(Some(true));
    column.set_actual_width(50.0);

    let header = TableViewColumnHeader::new();
    header.set_column(Some(column.clone()));
    header.set_theme(table_view_column_header_theme(6.0));

    let root = TestRoot::new();
    root.set_child(header.clone());
    root.execute_initial_layout_pass();

    let thumb = resizer(&header);
    raise_drag_delta(&thumb, Vector::new(-500.0, 0.0));

    assert_eq!(column.width(), GridLength::from_pixels(6.0));
}
