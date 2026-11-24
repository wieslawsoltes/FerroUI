use crate::parallax_expressions as expressions;
use crate::{ParallaxSourceOffsetKind, ParallaxView};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::Brushes;
use ferroui_base::rendering::composition::server::ServerCompositionVisual;
use ferroui_base::rendering::composition::ElementComposition;
use ferroui_base::{Rect, Ref, Size, Vector};
use ferroui_controls::presenters::ScrollContentPresenter;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use ferroui_controls::testing::{CompositorTestServices, MockWindowImpl, TestServices, UnitTestApplication};
use ferroui_controls::{Border, Control, Grid, ScrollViewer, StackPanel, Window};

#[test]
fn the_defaults_are_those_of_winui() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let view = ParallaxView::new();
    assert_eq!(view.horizontal_shift(), 0.0);
    assert_eq!(view.vertical_shift(), 0.0);
    assert_eq!(view.horizontal_source_offset_kind(), ParallaxSourceOffsetKind::Relative);
    assert_eq!(view.vertical_source_offset_kind(), ParallaxSourceOffsetKind::Relative);
    assert!(view.is_horizontal_shift_clamped() && view.is_vertical_shift_clamped());
    assert_eq!((view.max_horizontal_shift_ratio(), view.max_vertical_shift_ratio()), (1.0, 1.0));
    assert!(view.clip_to_bounds());
}

#[test]
fn the_child_is_measured_with_the_shift_and_arranged_beyond_the_view() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let view = ParallaxView::new();
    let child = Border::new();
    child.set_width(f64::NAN);
    child.set_horizontal_alignment(HorizontalAlignment::Stretch);
    child.set_vertical_alignment(VerticalAlignment::Stretch);
    view.set_child(Some(child.clone().upcast::<Control>()));
    view.set_vertical_shift(100.0);
    view.measure(Size::new(200.0, 100.0));
    assert_eq!(view.desired_size(), Size::new(200.0, 100.0));
    view.arrange(Rect::new(0.0, 0.0, 200.0, 100.0));
    assert_eq!(child.bounds().height, 200.0, "the view and the shift");
    assert_eq!(child.bounds().y, 0.0);
}

/// The clamped expression of the source, with its three ranges.
#[test]
fn the_clamped_expression_has_the_three_ranges_of_the_source() {
    let text = expressions::parallax(&expressions::VERTICAL, true, 100.0, "s", "e");
    assert!(text.starts_with("(-source.Translation.Y <= (s)) ? 0.0 :"), "{text}");
    assert!(text.contains("-Min(maxRatio * Max(0.0, (e) - (s)), shift)"), "{text}");
    let negative = expressions::parallax(&expressions::HORIZONTAL, true, -100.0, "s", "e");
    assert!(negative.contains("-shift"), "{negative}");
    assert!(negative.ends_with(": 0.0)"), "{negative}");
}

#[test]
fn the_offset_expressions_follow_the_kind_and_the_place_of_the_view() {
    assert!(expressions::start_offset(true, true, 0.0).contains("parallaxViewOffset"));
    assert!(!expressions::start_offset(true, false, 0.0).contains("parallaxViewOffset"));
    assert_eq!(expressions::start_offset(false, false, -5.0), "startOffset");
    assert!(expressions::start_offset(false, false, 5.0).contains("source.Scale.X"));
    assert!(expressions::end_offset(true, false, 0.0, 500.0, 100.0).contains("contentSize"));
    assert!(expressions::end_offset(false, false, 10.0, 500.0, 100.0).starts_with("Max(0.0, endOffset"));
    assert!(expressions::end_offset(false, false, 900.0, 500.0, 100.0).contains("+ endOffset - contentSize + viewportSize"));
}

#[test]
fn every_expression_parses() {
    use ferroui_base::rendering::composition::expressions::ExpressionParser;
    let mut texts = Vec::new();
    for axis in [&expressions::HORIZONTAL, &expressions::VERTICAL] {
        for clamped in [true, false] {
            for shift in [100.0, -100.0] {
                texts.push(expressions::parallax(axis, clamped, shift, "startOffset * source.Scale.X", "Max(0.0, endOffset)"));
            }
        }
    }
    for relative in [true, false] {
        for inside in [true, false] {
            for offset in [-5.0, 5.0, 900.0] {
                texts.push(expressions::start_offset(relative, inside, offset));
                for (content, viewport) in [(500.0, 100.0), (100.0, 500.0)] {
                    texts.push(expressions::end_offset(relative, inside, offset, content, viewport));
                }
            }
        }
    }
    for text in texts {
        assert!(ExpressionParser::parse(&text).is_ok(), "{text}");
    }
}

fn scroll_template(_control: &Ref<ScrollViewer>, scope: &NameScopeRef) -> Ref<Control> {
    let presenter = ScrollContentPresenter::new();
    presenter.set_name(Some("PART_ContentPresenter".to_string()));
    presenter.register_in_name_scope(&**scope).upcast()
}

fn scroller(content: Ref<Control>) -> Ref<ScrollViewer> {
    let scroller = ScrollViewer::new();
    scroller.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(scroll_template)));
    scroller.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    scroller.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    scroller.set_width(100.0);
    scroller.set_height(100.0);
    scroller.set_horizontal_alignment(HorizontalAlignment::Left);
    scroller.set_vertical_alignment(VerticalAlignment::Top);
    scroller.set_content(Some(Control::boxed(&content)));
    scroller
}

fn block(height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_width(100.0);
    border.set_height(height);
    border.set_background(Some(Brushes::blue()));
    border
}

fn view_of(shift: f64) -> (Ref<ParallaxView>, Ref<Border>) {
    let view = ParallaxView::new();
    view.set_width(100.0);
    view.set_height(100.0);
    view.set_horizontal_alignment(HorizontalAlignment::Left);
    view.set_vertical_alignment(VerticalAlignment::Top);
    let child = block(100.0);
    child.set_vertical_alignment(VerticalAlignment::Top);
    view.set_child(Some(child.clone().upcast::<Control>()));
    view.set_vertical_shift(shift);
    (view, child)
}

fn show(services: &CompositorTestServices, content: Ref<Control>) -> Ref<Window> {
    let platform = MockWindowImpl::window(400.0, 300.0);
    services.setup(&platform);
    let window = Window::with_impl(platform);
    window.set_content(Some(Control::boxed(&content)));
    window.show();
    services.run_jobs();
    services.run_jobs();
    window
}

/// The translation the compositor gives the child.
fn moved(services: &CompositorTestServices, child: &Ref<Border>) -> f32 {
    services.run_jobs();
    services.run_jobs();
    let visual = ElementComposition::get_element_visual(child).expect("the visual of the child");
    let server = visual.compositor().server().get::<ServerCompositionVisual>(visual.server()).expect("the server visual");
    server.translation().y as f32
}

#[test]
fn the_child_follows_a_source_beside_it_over_the_whole_range() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let (view, child) = view_of(90.0);
    let source = scroller(block(1000.0).upcast());
    let grid = Grid::new();
    grid.children().add(view.clone());
    grid.children().add(source.clone());
    let _window = show(&services, grid.upcast());
    assert_eq!((source.extent().height, source.viewport().height), (1000.0, 100.0));
    view.set_source(Some(source.clone().upcast::<Control>()));
    assert_eq!(moved(&services, &child), 0.0);
    // The range is 0..900, so the child moves a tenth of the scrolling.
    source.set_offset(Vector::new(0.0, 450.0));
    assert_eq!(moved(&services, &child), -45.0);
    source.set_offset(Vector::new(0.0, 900.0));
    assert_eq!(moved(&services, &child), -90.0);
    // A shift of zero stops the motion and puts the child back.
    view.set_vertical_shift(0.0);
    assert_eq!(moved(&services, &child), 0.0);
}

#[test]
fn a_view_inside_its_source_moves_while_it_is_on_screen() {
    let services = CompositorTestServices::start(TestServices::styled_window());
    let (view, child) = view_of(50.0);
    let content = StackPanel::new();
    content.children().add(block(500.0));
    content.children().add(view.clone());
    content.children().add(block(500.0));
    let source = scroller(content.upcast());
    let _window = show(&services, source.clone().upcast());
    view.set_source(Some(source.clone().upcast::<Control>()));
    moved(&services, &child);
    // On screen from 400 (its top reaches the bottom of the viewport) to 600 (its bottom leaves
    // the top): 50 over 200, a quarter of the scrolling.
    source.set_offset(Vector::new(0.0, 300.0));
    assert_eq!(moved(&services, &child), 0.0);
    source.set_offset(Vector::new(0.0, 500.0));
    assert_eq!(moved(&services, &child), -25.0);
    source.set_offset(Vector::new(0.0, 700.0));
    assert_eq!(moved(&services, &child), -50.0);
}
