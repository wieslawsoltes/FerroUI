//! Sizes are computed from the metrics of the test fonts (see
//! `ferroui_base::media::text_formatting::testing`): every glyph advances
//! half an em and a line is 1.1 em high, so "1980" at the default size of 12
//! measures 24 x 13.2 where the reference tests, which run on a real font,
//! expect 27.95 x 14.52.

use crate::documents::{InlineCollection, InlineUIContainer, Run, Span, TextElement};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::test_support::{boxed_str, test_scope, TestRoot};
use crate::test_support_buttons::focus_scope;
use crate::text_box_tests::templated_text_box;
use crate::{Border, Button, ContentControl, Control, Image, TextBlock, TextBox};
use ferroui_base::input::InputElement;
use ferroui_base::data::{BindingMode, TemplateBinding};
use ferroui_base::layout::{HorizontalAlignment, LayoutHelper, VerticalAlignment};
use ferroui_base::media::{
    Brushes, DrawingImage, FontStyle, GeometryDrawing, IImage, RectangleGeometry, TextAlignment, TextDecorations,
    TextWrapping,
};
use ferroui_base::{Rect, Ref, Size, StyledElement, Thickness, Visual};
use std::rc::Rc;

fn text_block(text: &str) -> Ref<TextBlock> {
    let text_block = TextBlock::new();
    text_block.set_text(Some(text));
    text_block
}

fn inlines_of(inlines: impl IntoIterator<Item = Ref<crate::documents::Inline>>) -> InlineCollection {
    let collection = InlineCollection::new();
    for inline in inlines {
        collection.add(inline);
    }
    collection
}

fn infinity() -> Size {
    Size::new(f64::INFINITY, f64::INFINITY)
}

#[test]
fn default_binding_mode_should_be_one_way() {
    let _scope = test_scope();
    assert_eq!(
        TextBlock::text_property().get_metadata(TextBlock::TYPE).default_binding_mode(),
        BindingMode::OneWay
    );
}

#[test]
fn default_text_value_should_be_null() {
    let _scope = test_scope();
    let text_block = TextBlock::new();

    assert_eq!(text_block.text(), None);
}

#[test]
fn letter_spacing_property_uses_text_element_definition() {
    let _scope = test_scope();
    assert!(std::ptr::eq(
        TextElement::letter_spacing_property().as_property(),
        TextBlock::letter_spacing_property().as_property()
    ));
}

#[test]
fn calling_measure_should_update_text_layout() {
    let _scope = test_scope();
    let text_block = text_block("Hello World");

    let constraint = text_block.constraint();
    assert!(constraint.width.is_nan());
    assert!(constraint.height.is_nan());

    text_block.measure(Size::new(100.0, 100.0));

    let text_layout = text_block.text_layout();

    text_block.measure(Size::new(50.0, 100.0));

    assert!(!Rc::ptr_eq(&text_layout, &text_block.text_layout()));
}

#[test]
fn should_measure_min_text_with() {
    let _scope = test_scope();
    let text_block = text_block("Hello&#10;שלום&#10;Really really really really long line");
    text_block.set_horizontal_alignment(HorizontalAlignment::Center);
    text_block.set_text_alignment(TextAlignment::DetectFromContent);
    text_block.set_text_wrapping(TextWrapping::Wrap);

    text_block.measure(Size::new(1920.0, 1080.0));

    let text_layout = text_block.text_layout();

    let constraint = LayoutHelper::round_layout_size_up(Size::new(text_layout.width(), text_layout.height()), 1.0);

    assert_eq!(text_block.desired_size(), constraint);
}

#[test]
fn calling_arrange_with_different_size_should_update_constraint_and_text_layout() {
    let _scope = test_scope();
    let text_block = text_block("Hello World");

    text_block.measure(infinity());

    let text_layout = text_block.text_layout();

    let mut constraint = LayoutHelper::round_layout_size_up(
        Size::new(text_layout.width_including_trailing_whitespace(), text_layout.height()),
        1.0,
    );

    text_block.arrange(Rect::from_size(constraint));

    // The text layout is recreated after arrange.
    let text_layout = text_block.text_layout();

    assert_eq!(text_block.constraint(), constraint);

    text_block.measure(constraint);

    assert!(Rc::ptr_eq(&text_layout, &text_block.text_layout()));

    constraint = Size::new(constraint.width + 50.0, constraint.height);

    text_block.arrange(Rect::from_size(constraint));

    assert_eq!(text_block.constraint(), constraint);

    // The text layout is recreated after arrange.
    assert!(!Rc::ptr_eq(&text_layout, &text_block.text_layout()));
}

/// Not in the reference tests: arranging at the measured constraint keeps
/// the layout created by the measure pass.
#[test]
fn arranging_at_the_measured_constraint_should_not_recreate_the_text_layout() {
    let _scope = test_scope();
    let text_block = text_block("Hello World");

    text_block.measure(Size::new(200.0, 100.0));

    let text_layout = text_block.text_layout();

    text_block.arrange(Rect::new(0.0, 0.0, 200.0, 100.0));

    assert!(Rc::ptr_eq(&text_layout, &text_block.text_layout()));

    // A layout created while the measure is invalid is left aligned: the
    // arrange pass replaces it although the constraint is unchanged.
    text_block.set_text_alignment(TextAlignment::Center);

    let text_layout = text_block.text_layout();

    assert_eq!(text_layout.text_lines()[0].start(), 0.0);

    text_block.measure(Size::new(200.0, 100.0));
    text_block.arrange(Rect::new(0.0, 0.0, 200.0, 100.0));

    assert!(!Rc::ptr_eq(&text_layout, &text_block.text_layout()));
    assert_eq!(text_block.text_layout().text_lines()[0].start(), (200.0 - 11.0 * 6.0) / 2.0);
}

#[test]
fn calling_measure_with_infinite_space_should_set_desired_size() {
    let _scope = test_scope();
    let text_block = text_block("Hello World");

    text_block.measure(infinity());

    let text_layout = text_block.text_layout();

    let constraint = LayoutHelper::round_layout_size_up(
        Size::new(text_layout.width_including_trailing_whitespace(), text_layout.height()),
        1.0,
    );

    assert_eq!(text_block.desired_size(), constraint);
}

#[test]
fn changing_inlines_collection_should_invalidate_measure() {
    let _scope = test_scope();
    let target = TextBlock::new();

    target.measure(infinity());

    assert!(target.is_measure_valid());

    target.inlines().unwrap().add(Run::with_text(Some("Hello")));

    assert!(!target.is_measure_valid());

    target.measure(infinity());

    assert!(target.is_measure_valid());
}

#[test]
fn changing_inlines_should_attach_embedded_controls_to_parents() {
    let _scope = test_scope();
    let target = TextBlock::new();

    let control = Border::new();

    let inline_ui_container = InlineUIContainer::new();
    inline_ui_container.set_child(control.clone());

    target.set_inlines(Some(inlines_of([inline_ui_container.clone().upcast()])));

    assert_eq!(control.parent(), Some(inline_ui_container.upcast::<StyledElement>()));

    assert_eq!(control.visual_parent(), Some(target.upcast::<Visual>()));
}

#[test]
fn changing_inlines_properties_should_invalidate_measure() {
    let _scope = test_scope();
    let target = TextBlock::new();

    let inline = Run::with_text(Some("Hello"));

    target.inlines().unwrap().add(inline.clone());

    target.measure(infinity());

    assert!(target.is_measure_valid());

    inline.set_foreground(Some(Brushes::green()));

    assert!(!target.is_measure_valid());
}

#[test]
fn changing_inlines_should_invalidate_measure() {
    let _scope = test_scope();
    let target = TextBlock::new();

    let inlines = inlines_of([Run::with_text(Some("Hello")).upcast()]);

    target.measure(infinity());

    assert!(target.is_measure_valid());

    target.set_inlines(Some(inlines));

    assert!(!target.is_measure_valid());
}

#[test]
fn changing_inlines_should_reset_inlines_parent() {
    let _scope = test_scope();
    let target = TextBlock::new();

    let run = Run::with_text(Some("Hello"));

    target.inlines().unwrap().add(run.clone());

    target.measure(infinity());

    assert!(target.is_measure_valid());

    target.set_inlines(None);

    assert_eq!(run.parent(), None);

    target.set_inlines(Some(inlines_of([run.clone().upcast()])));

    assert_eq!(run.parent(), Some(target.upcast::<StyledElement>()));
}

#[test]
fn changing_inline_host_should_propagate_to_nested_inlines() {
    let _scope = test_scope();
    let target = TextBlock::new();

    let span = Span::new();
    span.set_inlines(inlines_of([Run::with_text(Some("World")).upcast()]));

    let inlines = inlines_of([Run::with_text(Some("Hello ")).upcast(), span.clone().upcast()]);

    target.set_inlines(Some(inlines));

    assert_eq!(span.inline_host().and_then(|host| host.as_text_block()), Some(target));
}

#[test]
fn changing_inlines_should_reset_visual_children() {
    let _scope = test_scope();
    let target = TextBlock::new();

    target.inlines().unwrap().add_control(Border::new());

    target.measure(infinity());

    assert!(target.visual_children_count() > 0);

    target.set_inlines(None);

    assert_eq!(target.visual_children_count(), 0);
}

#[test]
fn changing_inlines_should_reset_inline_ui_container_visual_parent_on_measure() {
    let _scope = test_scope();
    let target = TextBlock::new();

    let control = Control::new();

    let run = InlineUIContainer::with_child(control.clone());

    target.inlines().unwrap().add(run.clone());

    target.measure(infinity());

    assert!(target.is_measure_valid());

    assert_eq!(control.visual_parent(), Some(target.clone().upcast::<Visual>()));

    target.set_inlines(None);

    assert_eq!(run.parent(), None);

    target.set_inlines(Some(inlines_of([Run::with_text(Some("Hello World")).upcast()])));

    assert_eq!(run.parent(), None);

    target.measure(infinity());

    assert_eq!(control.visual_parent(), None);
}

#[test]
fn inline_ui_container_child_should_be_arranged() {
    let _scope = test_scope();
    let target = TextBlock::new();

    let button = Button::new();
    button.set_content(boxed_str("12345678"));

    button.set_template(Some(FuncControlTemplate::for_type::<Button>(|_, scope| {
        let text_block = TextBlock::new();
        text_block.set_name(Some("PART_ContentPresenter".to_string()));
        text_block.bind_binding(
            TextBlock::text_property().as_property(),
            &TemplateBinding::new(ContentControl::content_property().as_property()),
        );
        text_block.register_in_name_scope(&**scope).upcast()
    })));

    let inlines = target.inlines().unwrap();
    inlines.add_text("123456");
    inlines.add(InlineUIContainer::with_child(button.clone()));
    inlines.add_text("123456");

    target.measure(infinity());
    target.arrange(Rect::from_size(target.desired_size()));

    assert!(button.is_measure_valid());
    // The reference asserts 58 with a real font. With the test font every
    // glyph advances 0.5 em of the default font size 12: 8 glyphs are 48.
    assert_eq!(button.desired_size().width, 48.0);

    target.arrange(Rect::from_size(Size::new(200.0, 50.0)));

    assert!(button.is_arrange_valid());

    // The reference asserts 43 with a real font: the 6 glyphs before the
    // button are 6 * 0.5 * 12 = 36 wide.
    assert_eq!(button.bounds().x, 36.0);
}

#[test]
fn inline_ui_container_child_should_be_constrained() {
    let _scope = test_scope();
    let target = TextBlock::new();

    let drawing = GeometryDrawing::new();
    drawing.set_geometry(RectangleGeometry::with_rect(Rect::new(0.0, 0.0, 500.0, 500.0)));
    let image = DrawingImage::with_drawing(&drawing);

    let image_control = Image::new();
    let source: Rc<dyn IImage> = image.into();
    image_control.set_source(Some(source));
    let container = InlineUIContainer::with_child(image_control.clone());

    let inlines = target.inlines().unwrap();
    inlines.add(Run::with_text(Some("The child should not be limited by position on line.")));
    inlines.add(container);

    target.measure(Size::new(100.0, 100.0));
    target.arrange(Rect::from_size(target.desired_size()));

    assert!(image_control.is_measure_valid());
    // Independent of the font: the image is uniformly stretched into the
    // constraint of the text block.
    assert_eq!(image_control.bounds().width, 100.0);
}

#[test]
fn setting_text_should_reset_inlines() {
    let _scope = test_scope();
    let target = TextBlock::new();

    target.inlines().unwrap().add(Run::with_text(Some("Hello World")));

    assert_eq!(target.text(), None);

    assert_eq!(target.inlines().unwrap().count(), 1);

    target.set_text(Some("1234"));

    assert_eq!(target.text().as_deref(), Some("1234"));

    assert_eq!(target.inlines().unwrap().count(), 0);
}

#[test]
fn setting_text_decorations_should_update_inlines() {
    let _scope = test_scope();
    let target = TextBlock::new();

    target.inlines().unwrap().add(Run::with_text(Some("Hello World")));

    assert_eq!(target.inlines().unwrap().count(), 1);

    assert!(target.inlines().unwrap().get(0).text_decorations().is_none());

    let underline = TextDecorations::underline();

    target.set_text_decorations(Some(underline.clone()));

    assert!(target.inlines().unwrap().get(0).text_decorations() == Some(underline));
}

#[test]
fn text_block_text_lines_should_be_empty() {
    let _scope = test_scope();
    let textblock = TextBlock::new();
    textblock.inlines().unwrap().add(Run::with_text(Some("123")));
    textblock.measure(Size::new(200.0, 200.0));
    let count = textblock.text_layout().text_lines()[0].text_runs().len();
    textblock.inlines().unwrap().clear();
    textblock.measure(Size::new(200.0, 200.0));
    let count1 = textblock.text_layout().text_lines()[0].text_runs().len();
    assert_ne!(count, count1);
}

#[test]
fn text_block_with_infinite_size_should_be_remeasured_after_text_layout_created() {
    let _scope = test_scope();

    let target = text_block("");
    let layout = target.text_layout();

    assert_eq!(layout.max_width(), 0.0);
    assert_eq!(layout.max_height(), 0.0);

    target.set_text(Some("foo"));
    target.measure(infinity());

    assert!(target.desired_size().width > 0.0);
    assert!(target.desired_size().height > 0.0);
}

#[test]
fn text_block_with_use_layout_rounding_true_should_round_desired_size() {
    let _scope = test_scope();

    let target = text_block("1980");

    target.measure(infinity());

    // 24 x 13.2 rounded up (the reference font gives 28 x 15).
    assert_eq!(target.desired_size(), Size::new(24.0, 14.0));
}

#[test]
fn text_block_with_use_layout_rounding_true_should_round_padding_and_desired_size() {
    let _scope = test_scope();

    let target = text_block("1980");
    target.set_padding(Thickness::uniform(2.25));

    target.measure(infinity());

    // The padding rounds to 2: 28 x 17.2 rounded up (the reference font
    // gives 32 x 19).
    assert_eq!(target.desired_size(), Size::new(28.0, 18.0));
}

#[test]
fn text_block_with_use_layout_rounding_false_should_not_round_desired_size() {
    let _scope = test_scope();

    let target = text_block("1980");
    target.set_use_layout_rounding(false);

    target.measure(infinity());

    assert_eq!(target.desired_size(), Size::new(24.0, 13.2));
}

#[test]
fn text_block_with_use_layout_rounding_false_should_not_round_bounds() {
    let _scope = test_scope();

    let target = text_block("1980");
    target.set_use_layout_rounding(false);

    target.measure(infinity());
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds(), Rect::new(0.0, 0.0, 24.0, 13.2));
}

#[test]
fn text_block_with_fractional_line_height_should_not_cull_last_line_at_fractional_scaling() {
    let _scope = test_scope();

    let target = text_block("first second third");
    target.set_font_size(16.0);
    target.set_line_height(20.8);
    target.set_text_wrapping(TextWrapping::Wrap);
    target.set_width(50.0);
    target.set_horizontal_alignment(HorizontalAlignment::Left);
    target.set_vertical_alignment(VerticalAlignment::Top);
    let root = TestRoot::with_child(target.clone());
    root.set_layout_scaling(1.25);

    root.measure(infinity());
    root.arrange(Rect::from_size(root.desired_size()));

    assert_eq!(target.text_layout().text_lines().len(), 3);
}

#[test]
fn text_block_with_use_layout_rounding_false_should_not_round_padding_in_measure_override() {
    let _scope = test_scope();

    let target = text_block("1980");
    target.set_use_layout_rounding(false);
    target.set_padding(Thickness::uniform(2.25));

    target.measure(infinity());

    assert_eq!(target.desired_size(), Size::new(28.5, 17.7));
}

#[test]
fn text_block_with_use_layout_rounding_false_should_not_round_padding_in_arrange_override() {
    let _scope = test_scope();

    let target = text_block("1980");
    target.set_use_layout_rounding(false);
    target.set_padding(Thickness::uniform(2.25));

    target.measure(infinity());
    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds(), Rect::new(0.0, 0.0, 28.5, 17.7));
}

#[test]
fn measure_and_arrange_should_use_width_including_trailing_whitespace_for_bounds() {
    let _scope = test_scope();

    let target = text_block("fy");
    target.set_font_style(FontStyle::Italic);
    target.set_font_size(48.0);
    target.set_use_layout_rounding(false);
    target.set_padding(Thickness::new(3.0, 2.0, 5.0, 4.0));

    target.measure(infinity());

    let text_layout = target.text_layout();
    let expected_size = Size::new(text_layout.width_including_trailing_whitespace(), text_layout.height())
        .inflate(target.padding());

    assert_eq!(target.desired_size(), expected_size);

    target.arrange(Rect::from_size(target.desired_size()));

    assert_eq!(target.bounds(), Rect::from_size(expected_size));
}

#[test]
fn should_shape_inlines_when_text_layout_is_created_before_measure() {
    let _scope = test_scope();

    let target = TextBlock::new();
    target.set_inlines(Some(inlines_of([Run::with_text(Some("Hello World")).upcast()])));

    // A render pass that runs before the queued measure reads the layout
    // while the runs have not been built yet.
    let _ = target.text_layout();

    target.measure(Size::new(1000.0, 1000.0));

    assert!(target.desired_size().width > 0.0, "DesiredSize was {:?}", target.desired_size());
}

#[test]
fn should_shape_inlines_when_text_layout_is_created_between_content_change_and_measure() {
    let _scope = test_scope();

    let target = TextBlock::new();
    target.set_inlines(Some(InlineCollection::new()));

    target.measure(Size::new(1000.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 1000.0, 1000.0));

    target.set_inlines(Some(inlines_of([Run::with_text(Some("Hello World")).upcast()])));

    let _ = target.text_layout();

    target.measure(Size::new(1000.0, 1000.0));

    assert!(target.desired_size().width > 0.0, "DesiredSize was {:?}", target.desired_size());
}

#[test]
fn should_not_cache_shaped_runs_built_outside_of_measure_from_text() {
    let _scope = test_scope();

    let target = TextBlock::new();
    target.set_inlines(Some(InlineCollection::new()));

    target.measure(Size::new(1000.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 1000.0, 1000.0));

    target.set_inlines(Some(inlines_of([Run::with_text(Some("Hello World")).upcast()])));

    let _ = target.text_layout();

    // Measuring at a different constraint drops the layout, so a wrong
    // result here can only come from shaped runs that were cached outside
    // of the measure pass.
    target.measure(Size::new(900.0, 1000.0));

    assert!(target.desired_size().width > 0.0, "DesiredSize was {:?}", target.desired_size());
}

#[test]
fn should_shape_inlines_added_to_the_collection_before_measure() {
    let _scope = test_scope();

    let target = TextBlock::new();
    target.set_inlines(Some(InlineCollection::new()));

    target.measure(Size::new(1000.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 1000.0, 1000.0));

    target.inlines().unwrap().add(Run::with_text(Some("Hello World")));

    let _ = target.text_layout();

    target.measure(Size::new(1000.0, 1000.0));

    assert!(target.desired_size().width > 0.0, "DesiredSize was {:?}", target.desired_size());
}

#[test]
fn should_remeasure_embedded_controls_when_the_constraint_changes() {
    let _scope = test_scope();

    let child = text_block("Hello World Hello World");
    child.set_text_wrapping(TextWrapping::Wrap);
    let target = TextBlock::new();
    target.set_inlines(Some(inlines_of([InlineUIContainer::with_child(child).upcast()])));

    target.measure(Size::new(1000.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 1000.0, 1000.0));

    let wide = target.desired_size();

    target.measure(Size::new(60.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 60.0, 1000.0));

    let narrow = target.desired_size();

    assert!(narrow.width < wide.width, "wide {wide:?}, narrow {narrow:?}");
    assert!(narrow.height > wide.height, "wide {wide:?}, narrow {narrow:?}");
}

fn sized_border(width: f64, height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border
}

#[test]
fn should_remeasure_embedded_controls_when_the_child_invalidates_its_measure() {
    let _scope = test_scope();

    let child = sized_border(20.0, 20.0);
    let target = TextBlock::new();
    target.set_inlines(Some(inlines_of([InlineUIContainer::with_child(child.clone()).upcast()])));

    target.measure(Size::new(1000.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 1000.0, 1000.0));

    let before = target.desired_size();

    child.set_width(80.0);

    // Stands in for the layout manager propagating the child's invalidation
    // to its parent.
    target.invalidate_measure();

    target.measure(Size::new(1000.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 1000.0, 1000.0));

    assert!(target.desired_size().width > before.width, "before {before:?}, after {:?}", target.desired_size());
}

#[test]
fn changing_line_spacing_should_invalidate_measure() {
    let _scope = test_scope();

    let target = text_block("Hello World\nHello World");

    target.measure(Size::new(1000.0, 1000.0));

    let before = target.desired_size();

    target.set_line_spacing(20.0);

    target.measure(Size::new(1000.0, 1000.0));

    assert!(target.desired_size().height > before.height, "before {before:?}, after {:?}", target.desired_size());
}

#[test]
fn should_shape_the_latest_inlines_when_content_changes_twice_before_measure() {
    let _scope = test_scope();

    let target = TextBlock::new();
    target.set_inlines(Some(inlines_of([Run::with_text(Some("Hello World")).upcast()])));

    target.measure(Size::new(1000.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 1000.0, 1000.0));

    target.set_inlines(Some(inlines_of([Run::with_text(Some("A")).upcast()])));

    // A render pass builds the layout from the first change, before the
    // queued measure.
    let _ = target.text_layout();

    target.set_inlines(Some(inlines_of([Run::with_text(Some("Hello World Hello World")).upcast()])));

    target.measure(Size::new(1000.0, 1000.0));

    let expected = TextBlock::new();
    expected.set_inlines(Some(inlines_of([Run::with_text(Some("Hello World Hello World")).upcast()])));

    expected.measure(Size::new(1000.0, 1000.0));

    assert_eq!(target.desired_size(), expected.desired_size());
}

#[test]
fn should_remeasure_embedded_controls_when_the_child_changes_while_measure_is_invalid() {
    let _scope = test_scope();

    let child = sized_border(20.0, 20.0);
    let target = TextBlock::new();
    target.set_inlines(Some(inlines_of([InlineUIContainer::with_child(child.clone()).upcast()])));

    target.measure(Size::new(1000.0, 1000.0));
    target.arrange(Rect::new(0.0, 0.0, 1000.0, 1000.0));

    target.invalidate_measure();

    // A render pass builds the layout while the child still has its old size.
    let _ = target.text_layout();

    // The block is already measure invalid, so this raises no further
    // invalidation on it.
    child.set_width(80.0);

    target.measure(Size::new(1000.0, 1000.0));

    let expected = TextBlock::new();
    expected.set_inlines(Some(inlines_of([InlineUIContainer::with_child(sized_border(80.0, 20.0)).upcast()])));

    expected.measure(Size::new(1000.0, 1000.0));

    assert_eq!(target.desired_size(), expected.desired_size());
}

#[test]
fn can_call_measure_without_invalidate_text_layout() {
    let _scope = test_scope();
    let target = TextBlock::new();

    let text_box = TextBox::new();
    text_box.set_text(Some("Hello"));
    target.inlines().unwrap().add_control(text_box);

    target.measure(infinity());

    target.invalidate_measure();

    target.measure(infinity());
}

#[test]
fn embedded_control_should_keep_focus() {
    let _scope = test_scope();
    let _focus = focus_scope();
    let target = TextBlock::new();

    let root = TestRoot::with_child(target.clone());

    let text_box = templated_text_box(Some("Hello"));

    target.inlines().unwrap().add_control(text_box.clone());

    target.measure(infinity());

    text_box.focus();

    let focus_manager = root.presentation_source().unwrap().input_root().focus_manager().unwrap();
    let text_box_element: Ref<InputElement> = text_box.clone().upcast();

    assert_eq!(Some(text_box_element.clone()), focus_manager.get_focused_element());

    target.invalidate_measure();

    assert_eq!(Some(text_box_element.clone()), focus_manager.get_focused_element());

    target.measure(infinity());

    assert_eq!(Some(text_box_element), focus_manager.get_focused_element());
}

/// The family of the font the first run of the first line of a text block is drawn with.
fn first_run_family(text_block: &TextBlock) -> String {
    let text_layout = text_block.text_layout();
    let runs = text_layout.text_lines()[0].text_runs();

    runs[0]
        .downcast_ref::<ferroui_base::media::text_formatting::ShapedTextRun>()
        .expect("a shaped run")
        .glyph_run()
        .glyph_typeface()
        .family_name()
        .to_owned()
}

#[test]
fn text_of_a_family_the_font_manager_does_not_have_is_laid_out_with_the_default_family() {
    use ferroui_base::media::text_formatting::testing::DEFAULT_FAMILY;
    use ferroui_base::media::{FontFamily, FontManager};

    let _scope = test_scope();

    let reference = text_block("Hello World");

    reference.measure(infinity());

    assert_eq!(first_run_family(&reference), DEFAULT_FAMILY);

    // Named alone, and as the first of a list none of which the font manager has.
    for family in ["Cascadia Mono", "Cascadia Mono,Consolas,Menlo,DejaVu Sans Mono"] {
        // A text block of the family.
        let target = text_block("Hello World");

        target.set_font_family(FontFamily::new(family));
        target.measure(infinity());

        assert_eq!(target.desired_size(), reference.desired_size(), "{family}");
        assert_eq!(first_run_family(&target), DEFAULT_FAMILY, "{family}");

        // A run of the family in a text block of the default one.
        let target = TextBlock::new();
        let run = Run::with_text(Some("Hello World"));

        run.set_font_family(FontFamily::new(family));
        target.inlines().unwrap().add(run);
        target.measure(infinity());

        assert_eq!(target.desired_size(), reference.desired_size(), "{family}");
        assert_eq!(first_run_family(&target), DEFAULT_FAMILY, "{family}");
    }

    // Asking did not make them families of the system fonts.
    let system_fonts = FontManager::current().system_fonts();

    assert!(system_fonts.font_families().iter().all(|font_family| font_family.name() != "Cascadia Mono"));
}
