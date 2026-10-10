//! Port of upstream's `Controls/CarouselTests.cs` (the class is
//! `CarouselRenderTests`).

use crate::test_base::{test_font_family, CompareOptions, TestBase};
use ferroui_base::input::StandardCursorType;
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{Brush, Brushes, Color, FontWeight, SolidColorBrush};
use ferroui_base::platform::{ICursorFactory, ICursorImpl};
use ferroui_base::styling::{Selectors, Setter, Style, Styles};
use ferroui_base::{CornerRadius, FerroLocator, PixelPoint, Ref, Thickness};
use ferroui_controls::{Border, Carousel, Control, Grid, ItemsSource, TextBlock};
use ferroui_themes_simple::SimpleTheme;
use std::any::Any;
use std::rc::Rc;

fn base() -> TestBase {
    TestBase::new(r"Controls\Carousel")
}

fn font_style() -> Ref<Style> {
    Style::with_setters(
        Selectors::of_type::<TextBlock>(),
        [Setter::new(TextBlock::font_family_property(), test_font_family())],
    )
}

#[test]
fn carousel_viewport_fraction_middle_item_selected_shows_side_peeks() {
    let t = base();
    let carousel = Carousel::new();
    carousel.set_background(Some(Brushes::transparent()));
    carousel.set_viewport_fraction(0.8);
    carousel.set_selected_index(1);
    carousel.set_horizontal_alignment(HorizontalAlignment::Stretch);
    carousel.set_vertical_alignment(VerticalAlignment::Stretch);
    carousel.set_items_source(Some(ItemsSource::from_values([
        create_card("One", "#D8574B", "#F7C5BE"),
        create_card("Two", "#3E7AD9", "#BCD0F7"),
        create_card("Three", "#3D9B67", "#BEE4CB"),
    ])));

    let target = Border::new();
    target.set_width(520.0);
    target.set_height(340.0);
    target.set_background(Some(Brushes::white()));
    target.set_padding(Thickness::uniform(20.0));
    target.set_child(carousel);

    FerroLocator::current_mutable().bind::<dyn ICursorFactory>().to_constant(Rc::new(CursorFactoryStub));
    target.styles().add(SimpleTheme::new().upcast::<Styles>());
    target.styles().add(font_style());
    t.render_to_file(&target, "Carousel_ViewportFraction_MiddleItemSelected_ShowsSidePeeks");
    t.compare_images_with(
        "Carousel_ViewportFraction_MiddleItemSelected_ShowsSidePeeks",
        CompareOptions { skip_immediate: true, ..CompareOptions::default() },
    );
}

fn create_card(label: &str, background: &str, accent: &str) -> Ref<Control> {
    let card = Border::new();
    card.set_margin(Thickness::symmetric(14.0, 12.0));
    card.set_corner_radius(CornerRadius::uniform(18.0));
    card.set_clip_to_bounds(true);
    card.set_background(Some(Brush::parse(background).expect("the brush is parsed")));
    card.set_border_brush(Some(Brushes::white()));
    card.set_border_thickness(Thickness::uniform(2.0));

    let grid = Grid::new();

    let header = Border::new();
    header.set_height(56.0);
    header.set_background(Some(Brush::parse(accent).expect("the brush is parsed")));
    header.set_vertical_alignment(VerticalAlignment::Top);
    grid.children().add(header);

    let circle = Border::new();
    circle.set_width(88.0);
    circle.set_height(88.0);
    circle.set_corner_radius(CornerRadius::uniform(44.0));
    circle.set_background(Some(Brushes::white()));
    circle.set_opacity(0.9);
    circle.set_horizontal_alignment(HorizontalAlignment::Center);
    circle.set_vertical_alignment(VerticalAlignment::Center);
    grid.children().add(circle);

    let footer = Border::new();
    footer.set_background(Some(
        SolidColorBrush::with_color(Color::parse("#80000000").expect("the color is parsed")).into(),
    ));
    footer.set_vertical_alignment(VerticalAlignment::Bottom);
    footer.set_padding(Thickness::uniform(12.0));
    let text = TextBlock::new();
    text.set_text(Some(label));
    text.set_foreground(Some(Brushes::white()));
    text.set_horizontal_alignment(HorizontalAlignment::Center);
    text.set_font_weight(FontWeight::SemiBold);
    footer.set_child(text);
    grid.children().add(footer);

    card.set_child(grid);
    card.upcast()
}

struct CursorFactoryStub;

impl ICursorFactory for CursorFactoryStub {
    fn get_cursor(&self, _cursor_type: StandardCursorType) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorStub)
    }

    fn create_cursor(&self, _cursor: &Bitmap, _hot_spot: PixelPoint) -> Rc<dyn ICursorImpl> {
        Rc::new(CursorStub)
    }
}

struct CursorStub;

impl ICursorImpl for CursorStub {
    fn dispose(&self) {}

    fn as_any(&self) -> &dyn Any {
        self
    }
}
