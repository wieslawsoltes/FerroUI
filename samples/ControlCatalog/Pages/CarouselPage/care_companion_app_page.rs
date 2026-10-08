//! Port of `Pages/CarouselPage/CareCompanionAppPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CareCompanionAppPage.xaml`.

use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{boxed_text, parse_color, parse_geometry};
use ferroui_base::animation::{CrossFade, TimeSpan};
use ferroui_base::collections::FerroList;
use ferroui_base::data::{BindingMode, CompiledBinding, CompiledBindingPathBuilder};
use ferroui_base::input::{Cursor, InputElement, InputElementImpl, StandardCursorType};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment};
use ferroui_base::media::{
    BoxShadow, BoxShadows, Brushes, Color, Colors, FontStyle, FontWeight, GradientStop, IBrush, LinearGradientBrush,
    SolidColorBrush, TextAlignment, TextWrapping,
};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, CornerRadius, FerroObject,
    FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref, RelativePoint, RelativeUnit,
    StyledElementImpl, Thickness, Visual, VisualImpl,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{ScrollBarVisibility, TemplatedControlImpl, UniformGrid};
use ferroui_controls::{
    Border, Button, CarouselPage, ColumnDefinitions, ContentControlImpl, ContentPage, Control, ControlImpl,
    ControlImplExt, Grid, GridLength, NavigationPage, Page, Panel, PathIcon, PipsPager, RowDefinition, ScrollViewer,
    SelectingMultiPage, StackPanel, TabPlacement, TabbedPage, TextBlock, UserControl,
};
use mini_mvvm::start_async;
use std::cell::RefCell;
use std::rc::Rc;

const HEART_ICON: &str = "M12 21.593c-5.63-5.539-11-10.297-11-14.402 0-3.791 3.068-5.191 5.281-5.191 1.312 0 4.151.501 5.719 4.457 1.59-3.968 4.464-4.447 5.726-4.447 2.54 0 5.274 1.621 5.274 5.181 0 4.069-5.136 8.625-11 14.402z";
const PLUS_ICON: &str = "M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z";
const ARROW_RIGHT_ICON: &str = "M12 4l-1.41 1.41L16.17 11H4v2h12.17l-5.58 5.59L12 20l8-8z";
const ARROW_LEFT_ICON: &str = "M20 11H7.83l5.59-5.59L12 4l-8 8 8 8 1.41-1.41L7.83 13H20v-2z";
const TREND_ICON: &str = "M16 6l2.29 2.29-4.88 4.88-4-4L2 16.59 3.41 18l6-6 4 4 6.3-6.29L22 12V6z";
const PLAN_ICON: &str = "M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-5 14H7v-2h7v2zm3-4H7v-2h10v2zm0-4H7V7h10v2z";
const MESSAGE_ICON: &str = "M20 2H4c-1.1 0-2 .9-2 2v18l4-4h14c1.1 0 2-.9 2-2V4c0-1.1-.9-2-2-2z";
const PEOPLE_ICON: &str = "M16 11c1.66 0 2.99-1.34 2.99-3S17.66 5 16 5c-1.66 0-3 1.34-3 3s1.34 3 3 3zm-8 0c1.66 0 2.99-1.34 2.99-3S9.66 5 8 5C6.34 5 5 6.34 5 8s1.34 3 3 3zm0 2c-2.33 0-7 1.17-7 3.5V19h14v-2.5c0-2.33-4.67-3.5-7-3.5zm8 0c-.29 0-.62.02-.97.05 1.16.84 1.97 1.97 1.97 3.45V19h6v-2.5c0-2.33-4.67-3.5-7-3.5z";
const PERSON_ICON: &str = "M12 2C9.243 2 7 4.243 7 7s2.243 5 5 5 5-2.243 5-5-2.243-5-5-5zM12 14c-5.523 0-10 3.582-10 8a1 1 0 001 1h18a1 1 0 001-1c0-4.418-4.477-8-10-8z";
const HOME_ICON: &str = "M10 20v-6h4v6h5v-8h3L12 3 2 12h3v8z";
const LIBRARY_ICON: &str = "M21 5c-1.11-.35-2.33-.5-3.5-.5-1.95 0-4.05.4-5.5 1.5-1.45-1.1-3.55-1.5-5.5-1.5S2.45 4.9 1 6v14.65c0 .25.25.5.5.5.1 0 .15-.05.25-.05C3.1 20.45 5.05 20 6.5 20c1.95 0 4.05.4 5.5 1.5 1.35-.85 3.8-1.5 5.5-1.5 1.65 0 3.35.3 4.75 1.05.1.05.15.05.25.05.25 0 .5-.25.5-.5V6c-.6-.45-1.25-.75-2-1z";
const BELL_ICON: &str = "M12 22c1.1 0 2-.9 2-2h-4c0 1.1.89 2 2 2zm6-6v-5c0-3.07-1.64-5.64-4.5-6.32V4c0-.83-.67-1.5-1.5-1.5s-1.5.67-1.5 1.5v.68C7.63 5.36 6 7.92 6 11v5l-2 2v1h16v-1l-2-2z";
const CHECK_ICON: &str = "M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z";
const SMILE_ICON: &str = "M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm3.5-9c.83 0 1.5-.67 1.5-1.5S16.33 8 15.5 8 14 8.67 14 9.5s.67 1.5 1.5 1.5zm-7 0c.83 0 1.5-.67 1.5-1.5S9.33 8 8.5 8 7 8.67 7 9.5 7.67 11 8.5 11zm3.5 6.5c2.33 0 4.31-1.46 5.11-3.5H6.89c.8 2.04 2.78 3.5 5.11 3.5z";
const MEDICATION_ICON: &str = "M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm-7 3c1.93 0 3.5 1.57 3.5 3.5S13.93 13 12 13s-3.5-1.57-3.5-3.5S10.07 6 12 6zm7 13H5v-.23c0-.62.28-1.2.76-1.58C7.47 15.82 9.64 15 12 15s4.53.82 6.24 2.19c.48.38.76.97.76 1.58V19z";

/// The value of a brush property.
type Brush = Option<Rc<dyn IBrush>>;

/// The colors of the page (the static fields `Primary`, `PrimaryDark`, `PrimaryLight`,
/// `BgLight`, `TextDark`, `TextMuted`, `CardBg`, `SuccessGreen` and `WarningAmber`).
fn primary() -> Color {
    parse_color("#137fec")
}

fn primary_dark() -> Color {
    parse_color("#0a5bb5")
}

fn primary_light() -> Color {
    parse_color("#e0f0ff")
}

fn bg_light() -> Color {
    parse_color("#f6f7f8")
}

fn text_dark() -> Color {
    parse_color("#111827")
}

fn text_muted() -> Color {
    parse_color("#64748b")
}

fn card_bg() -> Color {
    Colors::WHITE
}

fn success_green() -> Color {
    parse_color("#10b981")
}

fn warning_amber() -> Color {
    parse_color("#f59e0b")
}

/// `new SolidColorBrush(color)` as the value of a brush property.
fn solid(color: Color) -> Brush {
    Some(SolidColorBrush::with_color(color).into())
}

/// `Brushes.White` as the value of a brush property.
fn white() -> Brush {
    let white: Rc<dyn IBrush> = Brushes::white();
    Some(white)
}

/// `Brushes.Transparent` as the value of a brush property.
fn transparent() -> Brush {
    let transparent: Rc<dyn IBrush> = Brushes::transparent();
    Some(transparent)
}

/// `new SolidColorBrush(color)` as the value of a resource.
fn solid_resource(color: Color) -> Option<BoxedValue> {
    let brush: Rc<dyn IBrush> = SolidColorBrush::with_color(color).into();
    Some(Rc::new(brush))
}

/// `BoxShadows.Parse(text)`.
///
/// # Panics
/// Panics if the text is not a list of box shadows (the format exception of the original).
fn parse_box_shadows(text: &str) -> BoxShadows {
    match BoxShadows::parse(text) {
        Ok(shadows) => shadows,
        Err(error) => panic!("{error}"),
    }
}

/// `new ColumnDefinitions(text)`.
fn column_definitions(text: &str) -> ColumnDefinitions {
    match ColumnDefinitions::parse(text) {
        Ok(definitions) => definitions,
        Err(error) => panic!("{error}"),
    }
}

/// A gradient brush between two relative points with the given stops.
fn linear_gradient(start: (f64, f64), end: (f64, f64), stops: &[(Color, f64)]) -> Brush {
    let brush = LinearGradientBrush::new();
    brush.set_start_point(RelativePoint::new(start.0, start.1, RelativeUnit::Relative));
    brush.set_end_point(RelativePoint::new(end.0, end.1, RelativeUnit::Relative));
    for (color, offset) in stops {
        brush.gradient_stops().add(GradientStop::with_color_and_offset(*color, *offset));
    }
    Some(brush.into())
}

/// `new Border { Height = height }`: a gap between the cards of the home tab.
fn spacer(height: f64) -> Ref<Border> {
    let spacer = Border::new();
    spacer.set_height(height);
    spacer
}

/// `new StackPanel { Orientation = Orientation.Horizontal, HorizontalAlignment = HorizontalAlignment.Center, Spacing = 6 }`:
/// the content of a button with an arrow.
fn button_row() -> Ref<StackPanel> {
    let row = StackPanel::new();
    row.set_orientation(Orientation::Horizontal);
    row.set_horizontal_alignment(HorizontalAlignment::Center);
    row.set_spacing(6.0);
    row
}

#[repr(C)]
pub struct CareCompanionAppPage {
    base: UserControl,
    nav_page: RefCell<Option<Ref<NavigationPage>>>,
    onboarding: RefCell<Option<Ref<CarouselPage>>>,
    info_panel: RefCell<Option<Ref<ScrollViewer>>>,
}

ferro_class!(CareCompanionAppPage: UserControl);
ferro_impl_classes!(
    CareCompanionAppPage: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(CareCompanionAppPage { new: CareCompanionAppPage::new });
xaml_class!(CareCompanionAppPage, "/Pages/CarouselPage/CareCompanionAppPage.xaml");

impl ControlImpl for CareCompanionAppPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        *this.info_panel.borrow_mut() = this.find_control::<ScrollViewer>("InfoPanel");
        this.update_info_visibility();

        *this.nav_page.borrow_mut() = this.find_control::<NavigationPage>("NavPage");
        let nav_page = this.nav_page.borrow().clone();
        let Some(nav_page) = nav_page else {
            return;
        };

        let onboarding = this.build_onboarding_carousel();
        *this.onboarding.borrow_mut() = Some(onboarding.clone());
        drop(nav_page.push_async(onboarding));
    }
}

impl FerroObjectImpl for CareCompanionAppPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Visual::bounds_property().as_property() {
            this.update_info_visibility();
        }
    }
}

impl CareCompanionAppPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            nav_page: RefCell::new(None),
            onboarding: RefCell::new(None),
            info_panel: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn update_info_visibility(&self) {
        if let Some(info_panel) = self.info_panel.borrow().as_ref() {
            info_panel.set_is_visible(self.bounds().width >= 650.0);
        }
    }

    /// `Txt` with the default opacity, alignment and wrapping.
    fn txt(text: &str, size: f64, weight: FontWeight, color: Color) -> Ref<TextBlock> {
        Self::txt_with(text, size, weight, color, 1.0, TextAlignment::Left, TextWrapping::NoWrap)
    }

    /// `Txt`.
    fn txt_with(
        text: &str,
        size: f64,
        weight: FontWeight,
        color: Color,
        opacity: f64,
        align: TextAlignment,
        wrap: TextWrapping,
    ) -> Ref<TextBlock> {
        let block = TextBlock::new();
        block.set_text(Some(text));
        block.set_font_size(size);
        block.set_font_weight(weight);
        block.set_foreground(solid(color));
        block.set_opacity(opacity);
        block.set_text_alignment(align);
        block.set_text_wrapping(wrap);
        block
    }

    /// `StyledButton` without a border.
    fn styled_button(content: BoxedValue, bg: Brush, fg: Brush, height: f64, radius: CornerRadius) -> Ref<Button> {
        Self::styled_button_with(content, bg, fg, height, radius, None, Thickness::uniform(0.0))
    }

    /// `StyledButton`. The parameters `margin`, `fontSize` and `fontWeight` of the original
    /// are never passed (and the last two are not used): the margin is the default one.
    fn styled_button_with(
        content: BoxedValue,
        bg: Brush,
        fg: Brush,
        height: f64,
        radius: CornerRadius,
        border: Brush,
        border_thick: Thickness,
    ) -> Ref<Button> {
        let btn = Button::new();
        btn.set_content(Some(content));
        btn.set_background(bg.clone());
        btn.set_foreground(fg.clone());
        btn.set_height(height);
        btn.set_corner_radius(radius);
        btn.set_margin(Thickness::uniform(0.0));
        btn.set_padding(Thickness::symmetric(16.0, 0.0));
        btn.set_horizontal_content_alignment(HorizontalAlignment::Center);
        btn.set_vertical_content_alignment(VerticalAlignment::Center);
        btn.set_border_brush(border);
        btn.set_border_thickness(border_thick);

        let over = Style::with_selector(
            Selectors::of_type::<Button>().class(":pointerover").descendant().of_type::<ContentPresenter>(),
        );
        over.add_setter(Setter::new(ContentPresenter::background_property(), bg.clone()));
        over.add_setter(Setter::new(ContentPresenter::foreground_property(), fg.clone()));
        btn.styles().add(over);

        let press = Style::with_selector(
            Selectors::of_type::<Button>().class(":pressed").descendant().of_type::<ContentPresenter>(),
        );
        press.add_setter(Setter::new(ContentPresenter::background_property(), bg));
        press.add_setter(Setter::new(ContentPresenter::foreground_property(), fg));
        btn.styles().add(press);

        btn
    }

    /// `SvgIcon`.
    fn svg_icon(data: &str, size: f64, color: Color) -> Ref<PathIcon> {
        let icon = PathIcon::new();
        icon.set_data(parse_geometry(data));
        icon.set_width(size);
        icon.set_height(size);
        icon.set_foreground(solid(color));
        icon.set_horizontal_alignment(HorizontalAlignment::Center);
        icon.set_vertical_alignment(VerticalAlignment::Center);
        icon
    }

    fn make_pips_pager(count: i32, carousel: &Ref<CarouselPage>) -> Ref<PipsPager> {
        let pager = PipsPager::new();
        pager.set_number_of_pages(count);
        pager.set_is_previous_button_visible(false);
        pager.set_is_next_button_visible(false);
        pager.set_horizontal_alignment(HorizontalAlignment::Center);

        // Deviation (DEVIATIONS.md, ControlCatalog sample): upstream builds the binding with
        // `CompiledBinding.Create<TIn, TOut>(c => c.SelectedIndex, carousel, mode: BindingMode.TwoWay)`,
        // which derives the path from an expression tree; there are no expression trees, so the
        // same one-element path (the property `SelectedIndex` of the source) is written with
        // the path builder.
        let path = CompiledBindingPathBuilder::new()
            .ferro_property(SelectingMultiPage::selected_index_property().as_property())
            .build();
        let source: BoxedValue = Rc::new(carousel.clone().upcast::<FerroObject>());
        let binding = CompiledBinding::new(path).with_source(Some(source)).with_mode(BindingMode::TwoWay);
        pager.bind_binding(PipsPager::selected_page_index_property().as_property(), &*binding);

        pager
    }

    /// `ShadowWrap`.
    fn shadow_wrap(ctrl: Ref<Button>, shadow_color: Color) -> Ref<Border> {
        let wrap = Border::new();
        wrap.set_corner_radius(CornerRadius::uniform(999.0));
        wrap.set_box_shadow(BoxShadows::new(BoxShadow {
            offset_x: 0.0,
            offset_y: 8.0,
            blur: 24.0,
            color: Color::from_argb(55, shadow_color.r, shadow_color.g, shadow_color.b),
            ..BoxShadow::default()
        }));
        wrap.set_child(ctrl);
        wrap
    }

    /// The "Skip" button of an onboarding page.
    fn skip_button(&self) -> Ref<Button> {
        let skip_btn =
            Self::styled_button(boxed_text("Skip"), transparent(), solid(text_muted()), 32.0, CornerRadius::uniform(999.0));
        skip_btn.set_horizontal_alignment(HorizontalAlignment::Right);
        skip_btn.set_margin(Thickness::new(0.0, 4.0, 8.0, 0.0));
        self.complete_onboarding_on_click(&skip_btn);
        skip_btn
    }

    /// `btn.Click += (_, _) => CompleteOnboarding()`.
    fn complete_onboarding_on_click(&self, btn: &Ref<Button>) {
        // The button ends up in a page of the navigation page of this control: its handler
        // holds this control weakly.
        let weak = self.to_ref().downgrade();
        btn.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.complete_onboarding();
            }
        });
    }

    /// `btn.Click += (_, _) => carousel.SelectedIndex = index`.
    fn select_page_on_click(btn: &Ref<Button>, carousel: &Ref<CarouselPage>, index: i32) {
        // The button ends up in a page of the carousel: its handler holds the carousel weakly.
        let weak = carousel.downgrade();
        btn.click(move |_, _| {
            if let Some(carousel) = weak.upgrade() {
                carousel.set_selected_index(index);
            }
        });
    }

    /// The layout shared by the three onboarding pages: the skip button, the scrolling middle
    /// part and the bottom area, in the rows `Auto`, `*` and `Auto` of a grid.
    fn onboarding_layout(skip_btn: Ref<Button>, middle_stack: Ref<StackPanel>, bottom_area: Ref<StackPanel>) -> Ref<Grid> {
        let middle_scroll = ScrollViewer::new();
        middle_scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        middle_scroll.set_content(Some(Control::boxed(middle_stack)));

        let grid = Grid::new();
        grid.row_definitions().add(RowDefinition::with_height(GridLength::AUTO));
        grid.row_definitions().add(RowDefinition::with_height(GridLength::STAR));
        grid.row_definitions().add(RowDefinition::with_height(GridLength::AUTO));
        Grid::set_row(&skip_btn, 0);
        Grid::set_row(&middle_scroll, 1);
        Grid::set_row(&bottom_area, 2);
        grid.children().add(skip_btn);
        grid.children().add(middle_scroll);
        grid.children().add(bottom_area);
        grid
    }

    /// The "Next" button of an onboarding page, in its shadow.
    fn next_button(carousel: &Ref<CarouselPage>, index: i32) -> Ref<Border> {
        let next_row = button_row();
        next_row.children().add(Self::txt("Next", 15.0, FontWeight::SemiBold, Colors::WHITE));
        next_row.children().add(Self::svg_icon(ARROW_RIGHT_ICON, 12.0, Colors::WHITE));

        let next_btn = Self::styled_button(
            Control::boxed(next_row),
            solid(primary()),
            white(),
            52.0,
            CornerRadius::uniform(999.0),
        );
        next_btn.set_horizontal_alignment(HorizontalAlignment::Stretch);
        Self::select_page_on_click(&next_btn, carousel, index);

        let next_btn_wrap = Self::shadow_wrap(next_btn, primary());
        next_btn_wrap.set_horizontal_alignment(HorizontalAlignment::Stretch);
        next_btn_wrap
    }

    fn build_onboarding_carousel(&self) -> Ref<CarouselPage> {
        let carousel = CarouselPage::new();
        carousel.set_background(solid(bg_light()));
        carousel.set_page_transition(Some(Rc::new(CrossFade::with_duration(TimeSpan::from_milliseconds(300.0)))));
        NavigationPage::set_has_navigation_bar(&carousel, false);

        let pips1 = Self::make_pips_pager(3, &carousel);
        let pips2 = Self::make_pips_pager(3, &carousel);
        let pips3 = Self::make_pips_pager(3, &carousel);

        let p1 = self.build_welcome_page(&carousel, pips1);
        let p2 = self.build_track_page(&carousel, pips2);
        let p3 = self.build_resources_page(pips3);

        let pages: [Ref<Page>; 3] = [p1.upcast(), p2.upcast(), p3.upcast()];
        carousel.set_pages(Some(FerroList::from_items(pages)));

        carousel
    }

    fn build_welcome_page(&self, carousel: &Ref<CarouselPage>, dots: Ref<PipsPager>) -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_background(solid(card_bg()));

        let skip_btn = self.skip_button();

        let ill_panel1 = Panel::new();
        ill_panel1.set_background(linear_gradient(
            (0.0, 0.0),
            (1.0, 1.0),
            &[(parse_color("#dbeafe"), 0.0), (parse_color("#93c5fd"), 0.5), (parse_color("#3b82f6"), 1.0)],
        ));

        let halo = Border::new();
        halo.set_width(160.0);
        halo.set_height(160.0);
        halo.set_corner_radius(CornerRadius::uniform(80.0));
        halo.set_background(solid(Color::from_argb(30, 255, 255, 255)));
        halo.set_horizontal_alignment(HorizontalAlignment::Center);
        halo.set_vertical_alignment(VerticalAlignment::Center);
        ill_panel1.children().add(halo);

        let heart = Border::new();
        heart.set_width(80.0);
        heart.set_height(80.0);
        heart.set_corner_radius(CornerRadius::uniform(20.0));
        heart.set_background(white());
        heart.set_horizontal_alignment(HorizontalAlignment::Center);
        heart.set_vertical_alignment(VerticalAlignment::Center);
        heart.set_box_shadow(parse_box_shadows("0 8 24 0 #0000001a"));
        heart.set_child(Self::svg_icon(HEART_ICON, 38.0, parse_color("#3b82f6")));
        ill_panel1.children().add(heart);

        let badge_dot = Border::new();
        badge_dot.set_width(8.0);
        badge_dot.set_height(8.0);
        badge_dot.set_corner_radius(CornerRadius::uniform(4.0));
        badge_dot.set_background(solid(success_green()));
        badge_dot.set_vertical_alignment(VerticalAlignment::Center);

        let badge_row = StackPanel::new();
        badge_row.set_orientation(Orientation::Horizontal);
        badge_row.set_spacing(6.0);
        badge_row.children().add(badge_dot);
        badge_row.children().add(Self::txt("Your health, simplified", 10.0, FontWeight::SemiBold, text_dark()));

        let badge = Border::new();
        badge.set_background(white());
        badge.set_corner_radius(CornerRadius::uniform(999.0));
        badge.set_padding(Thickness::symmetric(10.0, 6.0));
        badge.set_horizontal_alignment(HorizontalAlignment::Left);
        badge.set_vertical_alignment(VerticalAlignment::Bottom);
        badge.set_margin(Thickness::new(24.0, 0.0, 0.0, 24.0));
        badge.set_box_shadow(parse_box_shadows("0 4 12 0 #0000001a"));
        badge.set_child(badge_row);
        ill_panel1.children().add(badge);

        let plus = Border::new();
        plus.set_width(44.0);
        plus.set_height(44.0);
        plus.set_corner_radius(CornerRadius::uniform(22.0));
        plus.set_background(solid(Color::from_argb(50, 255, 255, 255)));
        plus.set_horizontal_alignment(HorizontalAlignment::Right);
        plus.set_vertical_alignment(VerticalAlignment::Top);
        plus.set_margin(Thickness::new(0.0, 20.0, 28.0, 0.0));
        plus.set_child(Self::svg_icon(PLUS_ICON, 20.0, Colors::WHITE));
        ill_panel1.children().add(plus);

        let img_card1 = Border::new();
        img_card1.set_height(210.0);
        img_card1.set_corner_radius(CornerRadius::uniform(20.0));
        img_card1.set_clip_to_bounds(true);
        img_card1.set_margin(Thickness::new(20.0, 6.0, 20.0, 0.0));
        img_card1.set_child(ill_panel1);

        let text_area = StackPanel::new();
        text_area.set_margin(Thickness::new(28.0, 20.0, 28.0, 0.0));
        text_area.set_spacing(10.0);
        let title_stack1 = StackPanel::new();
        title_stack1.set_spacing(2.0);
        title_stack1.set_horizontal_alignment(HorizontalAlignment::Center);
        title_stack1.children().add(Self::txt_with(
            "Welcome to Your",
            26.0,
            FontWeight::Bold,
            text_dark(),
            1.0,
            TextAlignment::Center,
            TextWrapping::NoWrap,
        ));
        title_stack1.children().add(Self::txt_with(
            "Care Companion",
            28.0,
            FontWeight::ExtraBold,
            primary(),
            1.0,
            TextAlignment::Center,
            TextWrapping::NoWrap,
        ));
        text_area.children().add(title_stack1);
        text_area.children().add(Self::txt_with(
            "We are here to support you through every step of your treatment journey. Track symptoms, manage appointments, and stay connected.",
            13.0,
            FontWeight::Normal,
            text_muted(),
            1.0,
            TextAlignment::Center,
            TextWrapping::Wrap,
        ));

        let next_btn_wrap = Self::next_button(carousel, 1);

        let bottom_area = StackPanel::new();
        bottom_area.set_margin(Thickness::new(24.0, 16.0, 24.0, 36.0));
        bottom_area.set_spacing(20.0);
        bottom_area.children().add(dots);
        bottom_area.children().add(next_btn_wrap);

        let middle_stack = StackPanel::new();
        middle_stack.set_spacing(0.0);
        middle_stack.children().add(img_card1);
        middle_stack.children().add(text_area);

        page.set_content(Some(Control::boxed(Self::onboarding_layout(skip_btn, middle_stack, bottom_area))));
        page
    }

    fn build_track_page(&self, carousel: &Ref<CarouselPage>, dots: Ref<PipsPager>) -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_background(solid(card_bg()));

        let skip_btn = self.skip_button();

        let ill_panel2 = Panel::new();
        ill_panel2.set_background(linear_gradient(
            (0.0, 0.0),
            (1.0, 1.0),
            &[(parse_color("#0ea5e9"), 0.0), (parse_color("#6366f1"), 1.0)],
        ));

        let bar_h: [f64; 7] = [48.0, 72.0, 40.0, 96.0, 64.0, 80.0, 56.0];
        let bar_d: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];
        let chart_inner = StackPanel::new();
        chart_inner.set_orientation(Orientation::Horizontal);
        chart_inner.set_horizontal_alignment(HorizontalAlignment::Center);
        chart_inner.set_vertical_alignment(VerticalAlignment::Bottom);
        chart_inner.set_spacing(6.0);
        chart_inner.set_margin(Thickness::new(0.0, 0.0, 0.0, 10.0));
        for (ci, (height, day)) in bar_h.into_iter().zip(bar_d).enumerate() {
            let bar_col = StackPanel::new();
            bar_col.set_spacing(3.0);
            bar_col.set_vertical_alignment(VerticalAlignment::Bottom);

            let bar = Border::new();
            bar.set_width(20.0);
            bar.set_height(height);
            bar.set_corner_radius(CornerRadius::new(5.0, 5.0, 0.0, 0.0));
            bar.set_background(solid(if ci == 3 { Colors::WHITE } else { Color::from_argb(160, 255, 255, 255) }));
            bar.set_vertical_alignment(VerticalAlignment::Bottom);
            bar_col.children().add(bar);
            bar_col.children().add(Self::txt_with(
                day,
                9.0,
                FontWeight::Medium,
                Colors::WHITE,
                0.7,
                TextAlignment::Center,
                TextWrapping::NoWrap,
            ));
            chart_inner.children().add(bar_col);
        }

        let chart = Border::new();
        chart.set_background(solid(Color::from_argb(40, 255, 255, 255)));
        chart.set_corner_radius(CornerRadius::uniform(16.0));
        chart.set_padding(Thickness::new(14.0, 14.0, 14.0, 6.0));
        chart.set_horizontal_alignment(HorizontalAlignment::Center);
        chart.set_vertical_alignment(VerticalAlignment::Center);
        chart.set_child(chart_inner);
        ill_panel2.children().add(chart);

        let score_stack = StackPanel::new();
        score_stack.set_spacing(1.0);
        score_stack.children().add(Self::txt("Weekly Score", 9.0, FontWeight::SemiBold, text_muted()));
        score_stack.children().add(Self::txt("\u{2191} 18%", 13.0, FontWeight::Bold, parse_color("#0ea5e9")));

        let score = Border::new();
        score.set_background(white());
        score.set_corner_radius(CornerRadius::uniform(12.0));
        score.set_padding(Thickness::symmetric(10.0, 7.0));
        score.set_horizontal_alignment(HorizontalAlignment::Left);
        score.set_vertical_alignment(VerticalAlignment::Top);
        score.set_margin(Thickness::new(22.0, 20.0, 0.0, 0.0));
        score.set_box_shadow(parse_box_shadows("0 4 12 0 #0000001a"));
        score.set_child(score_stack);
        ill_panel2.children().add(score);

        let trend = Border::new();
        trend.set_width(36.0);
        trend.set_height(36.0);
        trend.set_corner_radius(CornerRadius::uniform(18.0));
        trend.set_background(solid(Color::from_argb(50, 255, 255, 255)));
        trend.set_horizontal_alignment(HorizontalAlignment::Right);
        trend.set_vertical_alignment(VerticalAlignment::Top);
        trend.set_margin(Thickness::new(0.0, 22.0, 24.0, 0.0));
        trend.set_child(Self::svg_icon(TREND_ICON, 16.0, Colors::WHITE));
        ill_panel2.children().add(trend);

        let img_card2 = Border::new();
        img_card2.set_height(210.0);
        img_card2.set_corner_radius(CornerRadius::uniform(20.0));
        img_card2.set_clip_to_bounds(true);
        img_card2.set_margin(Thickness::new(20.0, 6.0, 20.0, 0.0));
        img_card2.set_child(ill_panel2);

        let icon_badge = Border::new();
        icon_badge.set_width(52.0);
        icon_badge.set_height(52.0);
        icon_badge.set_corner_radius(CornerRadius::uniform(14.0));
        icon_badge.set_background(solid(parse_color("#eff6ff")));
        icon_badge.set_margin(Thickness::new(0.0, 16.0, 0.0, 0.0));
        icon_badge.set_horizontal_alignment(HorizontalAlignment::Center);
        icon_badge.set_child(Self::svg_icon(TREND_ICON, 24.0, primary()));

        let text_area = StackPanel::new();
        text_area.set_margin(Thickness::new(28.0, 10.0, 28.0, 0.0));
        text_area.set_spacing(10.0);
        text_area.children().add(Self::txt_with(
            "Track and Understand",
            24.0,
            FontWeight::Bold,
            text_dark(),
            1.0,
            TextAlignment::Center,
            TextWrapping::Wrap,
        ));
        text_area.children().add(Self::txt_with(
            "Easily log your symptoms and side effects to share with your medical team for better care.",
            13.0,
            FontWeight::Normal,
            text_muted(),
            1.0,
            TextAlignment::Center,
            TextWrapping::Wrap,
        ));

        let back_row = button_row();
        back_row.children().add(Self::svg_icon(ARROW_LEFT_ICON, 12.0, text_dark()));
        back_row.children().add(Self::txt("Back", 15.0, FontWeight::SemiBold, text_dark()));

        let back_btn = Self::styled_button(
            Control::boxed(back_row),
            solid(parse_color("#f3f4f6")),
            solid(text_dark()),
            52.0,
            CornerRadius::uniform(999.0),
        );
        back_btn.set_horizontal_alignment(HorizontalAlignment::Stretch);
        Self::select_page_on_click(&back_btn, carousel, 0);

        let next_btn_wrap2 = Self::next_button(carousel, 2);

        let nav_grid = Grid::new();
        nav_grid.set_column_definitions(column_definitions("*,16,*"));
        Grid::set_column(&back_btn, 0);
        Grid::set_column(&next_btn_wrap2, 2);
        nav_grid.children().add(back_btn);
        nav_grid.children().add(next_btn_wrap2);

        let bottom_area = StackPanel::new();
        bottom_area.set_margin(Thickness::new(24.0, 16.0, 24.0, 36.0));
        bottom_area.set_spacing(20.0);
        bottom_area.children().add(dots);
        bottom_area.children().add(nav_grid);

        let middle_stack = StackPanel::new();
        middle_stack.set_spacing(0.0);
        middle_stack.children().add(img_card2);
        middle_stack.children().add(icon_badge);
        middle_stack.children().add(text_area);

        page.set_content(Some(Control::boxed(Self::onboarding_layout(skip_btn, middle_stack, bottom_area))));
        page
    }

    /// `BuildResourcesPage`; the original does not use its parameter `carousel`.
    fn build_resources_page(&self, dots: Ref<PipsPager>) -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_background(solid(card_bg()));

        let skip_btn = self.skip_button();

        let ill_panel = Panel::new();
        ill_panel.set_background(solid(parse_color("#eef4ff")));

        let document = Border::new();
        document.set_width(72.0);
        document.set_height(72.0);
        document.set_corner_radius(CornerRadius::uniform(36.0));
        document.set_background(white());
        document.set_horizontal_alignment(HorizontalAlignment::Center);
        document.set_vertical_alignment(VerticalAlignment::Center);
        document.set_margin(Thickness::new(0.0, 0.0, 0.0, 32.0));
        document.set_box_shadow(parse_box_shadows("0 4 16 0 #0000001a"));
        document.set_child(Self::svg_icon(PLAN_ICON, 32.0, primary()));
        ill_panel.children().add(document);

        let message = Border::new();
        message.set_width(36.0);
        message.set_height(36.0);
        message.set_corner_radius(CornerRadius::uniform(18.0));
        message.set_background(solid(parse_color("#10b981")));
        message.set_horizontal_alignment(HorizontalAlignment::Right);
        message.set_vertical_alignment(VerticalAlignment::Top);
        message.set_margin(Thickness::new(0.0, 22.0, 44.0, 0.0));
        message.set_child(Self::svg_icon(MESSAGE_ICON, 17.0, Colors::WHITE));
        ill_panel.children().add(message);

        let people = Border::new();
        people.set_width(30.0);
        people.set_height(30.0);
        people.set_corner_radius(CornerRadius::uniform(15.0));
        people.set_background(solid(parse_color("#8b5cf6")));
        people.set_horizontal_alignment(HorizontalAlignment::Right);
        people.set_vertical_alignment(VerticalAlignment::Center);
        people.set_margin(Thickness::new(0.0, 0.0, 20.0, 0.0));
        people.set_child(Self::svg_icon(PEOPLE_ICON, 15.0, Colors::WHITE));
        ill_panel.children().add(people);

        let avatar = Border::new();
        avatar.set_width(40.0);
        avatar.set_height(40.0);
        avatar.set_corner_radius(CornerRadius::uniform(20.0));
        avatar.set_background(linear_gradient(
            (0.0, 0.0),
            (1.0, 1.0),
            &[(parse_color("#93c5fd"), 0.0), (primary(), 1.0)],
        ));
        avatar.set_horizontal_alignment(HorizontalAlignment::Left);
        avatar.set_vertical_alignment(VerticalAlignment::Center);
        avatar.set_margin(Thickness::new(32.0, -20.0, 0.0, 0.0));
        avatar.set_child(Self::svg_icon(PERSON_ICON, 22.0, Colors::WHITE));
        ill_panel.children().add(avatar);

        let cs_icon_border = Border::new();
        cs_icon_border.set_width(28.0);
        cs_icon_border.set_height(28.0);
        cs_icon_border.set_corner_radius(CornerRadius::uniform(6.0));
        cs_icon_border.set_background(solid(parse_color("#eff6ff")));
        cs_icon_border.set_vertical_alignment(VerticalAlignment::Center);
        cs_icon_border.set_child(Self::svg_icon(PEOPLE_ICON, 14.0, primary()));

        let cs_text = StackPanel::new();
        cs_text.set_spacing(1.0);
        cs_text.set_vertical_alignment(VerticalAlignment::Center);
        cs_text.children().add(Self::txt("Community Support", 10.0, FontWeight::SemiBold, text_dark()));
        cs_text.children().add(Self::txt_with(
            "Connect with others and experts.",
            9.0,
            FontWeight::Normal,
            text_muted(),
            1.0,
            TextAlignment::Left,
            TextWrapping::Wrap,
        ));

        let cs_inner = StackPanel::new();
        cs_inner.set_orientation(Orientation::Horizontal);
        cs_inner.set_spacing(8.0);
        cs_inner.children().add(cs_icon_border);
        cs_inner.children().add(cs_text);

        let community = Border::new();
        community.set_background(white());
        community.set_corner_radius(CornerRadius::uniform(10.0));
        community.set_padding(Thickness::symmetric(8.0, 7.0));
        community.set_horizontal_alignment(HorizontalAlignment::Left);
        community.set_vertical_alignment(VerticalAlignment::Bottom);
        community.set_margin(Thickness::new(16.0, 0.0, 64.0, 14.0));
        community.set_box_shadow(parse_box_shadows("0 2 8 0 #0000001a"));
        community.set_child(cs_inner);
        ill_panel.children().add(community);

        let ill_card = Border::new();
        ill_card.set_height(210.0);
        ill_card.set_corner_radius(CornerRadius::uniform(20.0));
        ill_card.set_clip_to_bounds(true);
        ill_card.set_margin(Thickness::new(20.0, 6.0, 20.0, 0.0));
        ill_card.set_child(ill_panel);

        let text_area = StackPanel::new();
        text_area.set_margin(Thickness::new(28.0, 20.0, 28.0, 0.0));
        text_area.set_spacing(10.0);
        text_area.children().add(Self::txt_with(
            "Stay Informed and Connected",
            24.0,
            FontWeight::Bold,
            text_dark(),
            1.0,
            TextAlignment::Center,
            TextWrapping::Wrap,
        ));
        text_area.children().add(Self::txt_with(
            "Access expert resources and manage your appointments all in one place.",
            13.0,
            FontWeight::Normal,
            text_muted(),
            1.0,
            TextAlignment::Center,
            TextWrapping::Wrap,
        ));

        let gs_row = button_row();
        gs_row.children().add(Self::txt("Get Started", 15.0, FontWeight::SemiBold, Colors::WHITE));
        gs_row.children().add(Self::svg_icon(ARROW_RIGHT_ICON, 12.0, Colors::WHITE));

        let get_started_btn = Self::styled_button(
            Control::boxed(gs_row),
            solid(primary()),
            white(),
            52.0,
            CornerRadius::uniform(999.0),
        );
        get_started_btn.set_horizontal_alignment(HorizontalAlignment::Stretch);
        self.complete_onboarding_on_click(&get_started_btn);

        let get_started_wrap = Self::shadow_wrap(get_started_btn, primary());
        get_started_wrap.set_horizontal_alignment(HorizontalAlignment::Stretch);

        let login_row = StackPanel::new();
        login_row.set_orientation(Orientation::Horizontal);
        login_row.set_horizontal_alignment(HorizontalAlignment::Center);
        login_row.set_vertical_alignment(VerticalAlignment::Center);
        login_row.set_spacing(0.0);

        let login_prompt = TextBlock::new();
        login_prompt.set_text(Some("Already have an account? "));
        login_prompt.set_font_size(13.0);
        login_prompt.set_foreground(solid(text_muted()));
        login_prompt.set_vertical_alignment(VerticalAlignment::Center);
        login_row.children().add(login_prompt);

        let login_link = TextBlock::new();
        login_link.set_text(Some("Log In"));
        login_link.set_font_size(13.0);
        login_link.set_font_weight(FontWeight::SemiBold);
        login_link.set_foreground(solid(primary()));
        login_link.set_vertical_alignment(VerticalAlignment::Center);
        login_link.set_cursor(Some(Cursor::new(StandardCursorType::Hand)));
        // The text ends up in a page of the navigation page of this control: its handler holds
        // this control weakly.
        let weak = self.to_ref().downgrade();
        login_link.add_handler(InputElement::pointer_released_event(), move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.complete_onboarding();
            }
        });
        login_row.children().add(login_link);

        let bottom_area = StackPanel::new();
        bottom_area.set_margin(Thickness::new(24.0, 16.0, 24.0, 28.0));
        bottom_area.set_spacing(16.0);
        bottom_area.children().add(dots);
        bottom_area.children().add(get_started_wrap);
        bottom_area.children().add(login_row);

        let middle_stack = StackPanel::new();
        middle_stack.set_spacing(0.0);
        middle_stack.children().add(ill_card);
        middle_stack.children().add(text_area);

        page.set_content(Some(Control::boxed(Self::onboarding_layout(skip_btn, middle_stack, bottom_area))));
        page
    }

    /// `async void`: the onboarding carousel is removed once the dashboard is pushed.
    fn complete_onboarding(&self) {
        let nav_page = self.nav_page.borrow().clone();
        let Some(nav_page) = nav_page else {
            return;
        };
        if self.onboarding.borrow().is_none() {
            return;
        }

        let pushed = nav_page.push_async(Self::build_dashboard());
        let this = self.to_ref();
        drop(start_async(async move {
            if pushed.await.is_err() {
                return;
            }
            // `_navPage.RemovePage(_onboarding)`: the fields are read again after the wait.
            let nav_page = this.nav_page.borrow().clone().expect("the navigation page of the document");
            let onboarding = this.onboarding.borrow().clone().expect("the onboarding carousel");
            nav_page.remove_page(onboarding);
            *this.onboarding.borrow_mut() = None;
        }));
    }

    fn build_dashboard() -> Ref<TabbedPage> {
        let tp = TabbedPage::new();
        tp.set_background(solid(bg_light()));
        tp.set_tab_placement(TabPlacement::Bottom);
        let white_brush: Rc<dyn IBrush> = Brushes::white();
        tp.resources().set("TabbedPageTabStripBackground", Some(Rc::new(white_brush)));
        tp.resources().set("TabbedPageTabItemHeaderForegroundSelected", solid_resource(primary()));
        tp.resources().set("TabbedPageTabItemHeaderForegroundUnselected", solid_resource(text_muted()));
        NavigationPage::set_has_navigation_bar(&tp, false);

        let home = Self::build_home_tab();
        home.set_header(Some(boxed_text("Home")));
        let home_icon = PathIcon::new();
        home_icon.set_data(parse_geometry(HOME_ICON));
        home.set_icon(Some(Control::boxed(home_icon)));

        let pages: [Ref<Page>; 5] = [
            home.upcast(),
            Self::placeholder_tab("Care Plan", PLAN_ICON, "Your personalized care plan will appear here.").upcast(),
            Self::placeholder_tab("Messages", MESSAGE_ICON, "Messages from your care team will appear here.").upcast(),
            Self::placeholder_tab("Library", LIBRARY_ICON, "Educational resources and guides will appear here.")
                .upcast(),
            Self::placeholder_tab("Profile", PERSON_ICON, "Your profile and settings will appear here.").upcast(),
        ];
        tp.set_pages(Some(FerroList::from_items(pages)));

        tp
    }

    fn placeholder_tab(header: &str, icon_data: &str, message: &str) -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_header(Some(boxed_text(header)));
        let icon = PathIcon::new();
        icon.set_data(parse_geometry(icon_data));
        page.set_icon(Some(Control::boxed(icon)));
        page.set_background(solid(bg_light()));

        let content = StackPanel::new();
        content.set_vertical_alignment(VerticalAlignment::Center);
        content.set_horizontal_alignment(HorizontalAlignment::Center);
        content.set_spacing(12.0);
        content.set_margin(Thickness::uniform(32.0));
        content.children().add(Self::svg_icon(icon_data, 48.0, parse_color("#d1d5db")));
        content.children().add(Self::txt_with(
            header,
            20.0,
            FontWeight::Bold,
            text_dark(),
            1.0,
            TextAlignment::Center,
            TextWrapping::NoWrap,
        ));
        content.children().add(Self::txt_with(
            message,
            13.0,
            FontWeight::Normal,
            text_muted(),
            1.0,
            TextAlignment::Center,
            TextWrapping::Wrap,
        ));
        page.set_content(Some(Control::boxed(content)));
        page
    }

    fn build_home_tab() -> Ref<ContentPage> {
        let page = ContentPage::new();
        page.set_background(solid(bg_light()));
        let scroll = ScrollViewer::new();
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        let root = StackPanel::new();
        root.set_spacing(0.0);

        let header_border = Border::new();
        header_border.set_background(white());
        header_border.set_padding(Thickness::new(16.0, 20.0, 16.0, 16.0));
        let h_grid = Grid::new();
        h_grid.set_column_definitions(column_definitions("*,Auto"));

        let greet_stack = StackPanel::new();
        greet_stack.set_spacing(2.0);
        greet_stack.children().add(Self::txt("Tuesday, Oct 24", 12.0, FontWeight::Normal, text_muted()));
        greet_stack.children().add(Self::txt("Good Morning, Sarah", 20.0, FontWeight::Bold, text_dark()));
        h_grid.children().add(greet_stack);

        let bell_container = Panel::new();
        bell_container.set_width(40.0);
        bell_container.set_height(40.0);
        bell_container.set_vertical_alignment(VerticalAlignment::Center);

        let bell = Border::new();
        bell.set_width(40.0);
        bell.set_height(40.0);
        bell.set_corner_radius(CornerRadius::uniform(20.0));
        bell.set_background(solid(parse_color("#f3f4f6")));
        bell.set_child(Self::svg_icon(BELL_ICON, 20.0, text_dark()));
        bell_container.children().add(bell);

        let bell_dot = Border::new();
        bell_dot.set_width(10.0);
        bell_dot.set_height(10.0);
        bell_dot.set_corner_radius(CornerRadius::uniform(5.0));
        let red: Rc<dyn IBrush> = Brushes::red();
        bell_dot.set_background(Some(red));
        bell_dot.set_border_brush(white());
        bell_dot.set_border_thickness(Thickness::uniform(1.5));
        bell_dot.set_horizontal_alignment(HorizontalAlignment::Right);
        bell_dot.set_vertical_alignment(VerticalAlignment::Top);
        bell_dot.set_margin(Thickness::new(0.0, 1.0, 1.0, 0.0));
        bell_container.children().add(bell_dot);

        Grid::set_column(&bell_container, 1);
        h_grid.children().add(bell_container);
        header_border.set_child(h_grid);
        root.children().add(header_border);
        root.children().add(spacer(12.0));

        let weekly_card = Border::new();
        weekly_card.set_background(linear_gradient(
            (0.0, 0.5),
            (1.0, 0.5),
            &[(primary(), 0.0), (primary_dark(), 1.0)],
        ));
        weekly_card.set_corner_radius(CornerRadius::uniform(16.0));
        weekly_card.set_padding(Thickness::uniform(16.0));
        weekly_card.set_margin(Thickness::symmetric(16.0, 0.0));
        let weekly_inner = StackPanel::new();
        weekly_inner.set_spacing(14.0);

        let weekly_title_row = Grid::new();
        weekly_title_row.set_column_definitions(column_definitions("*,Auto"));
        let weekly_title_stack = StackPanel::new();
        weekly_title_stack.set_spacing(2.0);
        weekly_title_stack.children().add(Self::txt("Weekly Progress", 16.0, FontWeight::Bold, Colors::WHITE));
        weekly_title_stack.children().add(Self::txt_with(
            "You're on a 5-day streak!",
            12.0,
            FontWeight::Normal,
            Colors::WHITE,
            0.8,
            TextAlignment::Left,
            TextWrapping::NoWrap,
        ));
        weekly_title_row.children().add(weekly_title_stack);

        let trend_badge = Border::new();
        trend_badge.set_width(40.0);
        trend_badge.set_height(40.0);
        trend_badge.set_corner_radius(CornerRadius::uniform(10.0));
        trend_badge.set_background(white());
        trend_badge.set_vertical_alignment(VerticalAlignment::Center);
        trend_badge.set_child(Self::svg_icon(TREND_ICON, 20.0, primary()));
        Grid::set_column(&trend_badge, 1);
        weekly_title_row.children().add(trend_badge);
        weekly_inner.children().add(weekly_title_row);

        let day_labels: [&str; 7] = ["M", "T", "W", "T", "F", "S", "S"];
        let day_grid = UniformGrid::new();
        day_grid.set_rows(1);
        for (i, day_label) in day_labels.iter().enumerate() {
            let is_current = i == 4;
            let is_past = i < 4;

            let inner_circle = Border::new();
            if is_current {
                inner_circle.set_width(38.0);
                inner_circle.set_height(38.0);
                inner_circle.set_corner_radius(CornerRadius::uniform(19.0));
                inner_circle.set_background(white());
                inner_circle.set_child(Self::svg_icon(CHECK_ICON, 18.0, primary()));
            } else if is_past {
                inner_circle.set_width(30.0);
                inner_circle.set_height(30.0);
                inner_circle.set_corner_radius(CornerRadius::uniform(15.0));
                inner_circle.set_background(solid(Color::from_argb(55, 255, 255, 255)));
                inner_circle.set_child(Self::svg_icon(CHECK_ICON, 13.0, Colors::WHITE));
            } else {
                inner_circle.set_width(30.0);
                inner_circle.set_height(30.0);
                inner_circle.set_corner_radius(CornerRadius::uniform(15.0));
                inner_circle.set_background(solid(Color::from_argb(25, 255, 255, 255)));
            }

            let circle_wrap = Border::new();
            circle_wrap.set_width(40.0);
            circle_wrap.set_height(40.0);
            circle_wrap.set_horizontal_alignment(HorizontalAlignment::Center);
            circle_wrap.set_child(&inner_circle);
            inner_circle.set_horizontal_alignment(HorizontalAlignment::Center);
            inner_circle.set_vertical_alignment(VerticalAlignment::Center);

            let day_col = StackPanel::new();
            day_col.set_spacing(4.0);
            day_col.set_horizontal_alignment(HorizontalAlignment::Center);
            day_col.children().add(circle_wrap);
            day_col.children().add(Self::txt_with(
                day_label,
                10.0,
                FontWeight::Medium,
                Colors::WHITE,
                0.75,
                TextAlignment::Center,
                TextWrapping::NoWrap,
            ));
            day_grid.children().add(day_col);
        }

        weekly_inner.children().add(day_grid);
        weekly_card.set_child(weekly_inner);
        root.children().add(weekly_card);
        root.children().add(spacer(12.0));

        let symptom_card = Border::new();
        symptom_card.set_background(white());
        symptom_card.set_corner_radius(CornerRadius::uniform(16.0));
        symptom_card.set_padding(Thickness::uniform(16.0));
        symptom_card.set_margin(Thickness::symmetric(16.0, 0.0));
        symptom_card.set_box_shadow(parse_box_shadows("0 1 4 0 #0000000a"));
        let s_inner = StackPanel::new();
        s_inner.set_spacing(12.0);
        let s_top_row = StackPanel::new();
        s_top_row.set_orientation(Orientation::Horizontal);
        s_top_row.set_spacing(12.0);
        s_top_row.set_vertical_alignment(VerticalAlignment::Center);

        let s_icon = Border::new();
        s_icon.set_width(48.0);
        s_icon.set_height(48.0);
        s_icon.set_corner_radius(CornerRadius::uniform(12.0));
        s_icon.set_background(solid(parse_color("#fff7ed")));
        s_icon.set_child(Self::svg_icon(SMILE_ICON, 24.0, parse_color("#f97316")));
        s_top_row.children().add(s_icon);

        let s_text_stack = StackPanel::new();
        s_text_stack.set_spacing(2.0);
        s_text_stack.set_vertical_alignment(VerticalAlignment::Center);
        s_text_stack.children().add(Self::txt("How are you feeling?", 15.0, FontWeight::Bold, text_dark()));
        s_text_stack.children().add(Self::txt("Track your symptoms daily.", 12.0, FontWeight::Normal, text_muted()));
        s_top_row.children().add(s_text_stack);
        s_inner.children().add(s_top_row);

        let log_btn = Self::styled_button(
            boxed_text("Log Symptoms"),
            solid(primary()),
            white(),
            44.0,
            CornerRadius::uniform(10.0),
        );
        log_btn.set_horizontal_alignment(HorizontalAlignment::Stretch);
        s_inner.children().add(log_btn);
        symptom_card.set_child(s_inner);
        root.children().add(symptom_card);

        root.children().add(spacer(16.0));
        let sched_header = Grid::new();
        sched_header.set_column_definitions(column_definitions("*,Auto"));
        sched_header.set_margin(Thickness::new(16.0, 0.0, 16.0, 8.0));
        sched_header.children().add(Self::txt("Today's Schedule", 16.0, FontWeight::Bold, text_dark()));
        let see_all_txt = TextBlock::new();
        see_all_txt.set_text(Some("See All"));
        see_all_txt.set_font_size(13.0);
        see_all_txt.set_font_weight(FontWeight::SemiBold);
        see_all_txt.set_foreground(solid(primary()));
        see_all_txt.set_vertical_alignment(VerticalAlignment::Center);
        see_all_txt.set_cursor(Some(Cursor::new(StandardCursorType::Hand)));
        Grid::set_column(&see_all_txt, 1);
        sched_header.children().add(see_all_txt);
        root.children().add(sched_header);

        root.children().add(Self::build_schedule_item(
            warning_amber(),
            MEDICATION_ICON,
            parse_color("#fef3c7"),
            warning_amber(),
            "Tamoxifen (20mg)",
            "09:00 AM",
            "Take with food",
            "Mark as Done",
            true,
        ));

        root.children().add(spacer(8.0));

        root.children().add(Self::build_schedule_item(
            primary(),
            PERSON_ICON,
            primary_light(),
            primary(),
            "Dr. Emily Chen",
            "02:30 PM",
            "Oncologist \u{2022} Video Consultation",
            "Join Video Call",
            false,
        ));

        root.children().add(spacer(12.0));
        let no_more = TextBlock::new();
        no_more.set_text(Some("No more events for today. Rest well."));
        no_more.set_font_size(12.0);
        no_more.set_font_style(FontStyle::Italic);
        no_more.set_foreground(solid(text_muted()));
        no_more.set_text_alignment(TextAlignment::Center);
        no_more.set_margin(Thickness::new(16.0, 0.0, 16.0, 0.0));
        root.children().add(no_more);
        root.children().add(spacer(24.0));

        scroll.set_content(Some(Control::boxed(root)));
        page.set_content(Some(Control::boxed(scroll)));
        page
    }

    #[allow(clippy::too_many_arguments)]
    fn build_schedule_item(
        bullet_color: Color,
        icon_data: &str,
        icon_bg: Color,
        icon_fg: Color,
        title: &str,
        time: &str,
        subtitle: &str,
        action_label: &str,
        is_check: bool,
    ) -> Ref<Border> {
        let card = Border::new();
        card.set_background(white());
        card.set_corner_radius(CornerRadius::uniform(16.0));
        card.set_padding(Thickness::uniform(16.0));
        card.set_margin(Thickness::symmetric(16.0, 0.0));
        card.set_box_shadow(parse_box_shadows("0 1 4 0 #0000000a"));

        let outer_row = Grid::new();
        outer_row.set_column_definitions(column_definitions("8,*"));
        let bullet = Border::new();
        bullet.set_width(8.0);
        bullet.set_height(8.0);
        bullet.set_corner_radius(CornerRadius::uniform(4.0));
        bullet.set_background(solid(bullet_color));
        bullet.set_horizontal_alignment(HorizontalAlignment::Center);
        bullet.set_vertical_alignment(VerticalAlignment::Top);
        bullet.set_margin(Thickness::new(0.0, 4.0, 0.0, 0.0));
        outer_row.children().add(bullet);

        let right_stack = StackPanel::new();
        right_stack.set_spacing(8.0);
        right_stack.set_margin(Thickness::new(10.0, 0.0, 0.0, 0.0));
        Grid::set_column(&right_stack, 1);

        let top_row = Grid::new();
        top_row.set_column_definitions(column_definitions("Auto,*,Auto"));
        let icon = Border::new();
        icon.set_width(36.0);
        icon.set_height(36.0);
        icon.set_corner_radius(CornerRadius::uniform(8.0));
        icon.set_background(solid(icon_bg));
        icon.set_vertical_alignment(VerticalAlignment::Center);
        icon.set_child(Self::svg_icon(icon_data, 18.0, icon_fg));
        top_row.children().add(icon);

        let title_txt = Self::txt(title, 14.0, FontWeight::SemiBold, text_dark());
        title_txt.set_vertical_alignment(VerticalAlignment::Center);
        title_txt.set_margin(Thickness::new(10.0, 0.0, 6.0, 0.0));
        Grid::set_column(&title_txt, 1);
        top_row.children().add(title_txt);

        let time_badge = Border::new();
        time_badge.set_background(solid(parse_color("#f3f4f6")));
        time_badge.set_corner_radius(CornerRadius::uniform(999.0));
        time_badge.set_padding(Thickness::symmetric(8.0, 3.0));
        time_badge.set_vertical_alignment(VerticalAlignment::Center);
        time_badge.set_child(Self::txt(time, 10.0, FontWeight::Medium, text_muted()));
        Grid::set_column(&time_badge, 2);
        top_row.children().add(time_badge);
        right_stack.children().add(top_row);
        right_stack.children().add(Self::txt(subtitle, 12.0, FontWeight::Normal, text_muted()));

        let action_btn = if is_check {
            Self::styled_button_with(
                boxed_text(action_label),
                solid(parse_color("#f0fdf4")),
                solid(success_green()),
                36.0,
                CornerRadius::uniform(8.0),
                solid(parse_color("#bbf7d0")),
                Thickness::uniform(1.0),
            )
        } else {
            Self::styled_button(boxed_text(action_label), solid(primary()), white(), 36.0, CornerRadius::uniform(8.0))
        };

        action_btn.set_horizontal_alignment(HorizontalAlignment::Stretch);
        right_stack.children().add(action_btn);

        outer_row.children().add(right_stack);
        card.set_child(outer_row);
        card
    }
}
