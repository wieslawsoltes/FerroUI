//! Port of `Pages/NavigationPage/NavigationPageCurvedHeaderPage.xaml.cs`: the class of the
//! document `Pages/NavigationPage/NavigationPageCurvedHeaderPage.xaml`.

use super::{CurvedHeaderHomeScrollView, CurvedHeaderProfileScrollView};
use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::parse_color;
use ferroui_base::input::{Cursor, InputElement, InputElementImpl, PointerReleasedEventArgs, StandardCursorType};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, Layoutable, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{
    Brushes, Color, Colors, FontWeight, GradientStop, IBrush, IImageBrushSource, ImageBrush, LinearGradientBrush,
    SolidColorBrush, StreamGeometry, Stretch, SweepDirection, TextAlignment,
};
use ferroui_base::platform::{AssetLoader, IGeometryContext};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, CornerRadius, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Point, Ref, RelativePoint, RelativeUnit, Size, StyledElementImpl, Thickness, Visual,
    VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::shapes::{Ellipse, Path};
use ferroui_controls::{
    BarLayoutBehavior, Canvas, ContentControlImpl, ContentPage, Control, ControlImpl, ControlImplExt, NavigationPage,
    Panel, ScrollViewer, StackPanel, TextBlock, TextBox, UserControl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// The colors of the page (the static fields `BgLight`, `TextDark` and `TextMuted`; the field
/// `Primary` of the original is not used).
fn bg_light() -> Color {
    parse_color("#f6f7f8")
}

fn text_dark() -> Color {
    parse_color("#111827")
}

fn text_muted() -> Color {
    parse_color("#64748b")
}

const DOME_H: f64 = 32.0;
const HOME_HEADER_FLAT_H: f64 = 130.0;
const PROFILE_HEADER_FLAT_H: f64 = 110.0;
const AVATAR_HOME_SIZE: f64 = 72.0;
const AVATAR_PROFILE_SIZE: f64 = 88.0;

const ASSET_BASE: &str = "ferres://ControlCatalog/Assets/CurvedHeader/";

/// `new SolidColorBrush(color)` as the value of a brush property.
fn solid(color: Color) -> Option<Rc<dyn IBrush>> {
    Some(SolidColorBrush::with_color(color).into())
}

fn white() -> Option<Rc<dyn IBrush>> {
    let white: Rc<dyn IBrush> = Brushes::white();
    Some(white)
}

fn transparent() -> Option<Rc<dyn IBrush>> {
    let transparent: Rc<dyn IBrush> = Brushes::transparent();
    Some(transparent)
}

/// The bitmap of the asset `name`; `None` when the asset cannot be opened or decoded (the
/// `catch` of the original).
fn load_img(name: &str) -> Option<Rc<Bitmap>> {
    let uri = Uri::absolute(&format!("{ASSET_BASE}{name}")).ok()?;
    let mut stream = AssetLoader::open(&uri, None).ok()?;
    Some(Rc::new(Bitmap::from_stream(&mut stream).ok()?))
}

fn img_brush(name: &str, fallback: Option<Rc<dyn IBrush>>) -> Option<Rc<dyn IBrush>> {
    match load_img(name) {
        Some(bmp) => {
            let source: Rc<dyn IImageBrushSource> = bmp;
            let brush = ImageBrush::with_source(Some(source));
            brush.set_stretch(Stretch::UniformToFill);
            Some(brush.into())
        }
        None => fallback,
    }
}

#[repr(C)]
pub struct NavigationPageCurvedHeaderPage {
    base: UserControl,
    nav_page: RefCell<Option<Ref<NavigationPage>>>,
    info_panel: RefCell<Option<Ref<ScrollViewer>>>,
}

ferro_class!(NavigationPageCurvedHeaderPage: UserControl);
ferro_impl_classes!(
    NavigationPageCurvedHeaderPage: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(NavigationPageCurvedHeaderPage { new: NavigationPageCurvedHeaderPage::new });
xaml_class!(NavigationPageCurvedHeaderPage, "/Pages/NavigationPage/NavigationPageCurvedHeaderPage.xaml");

impl ControlImpl for NavigationPageCurvedHeaderPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

        *this.info_panel.borrow_mut() = this.find_control::<ScrollViewer>("InfoPanel");
        this.update_info_panel_visibility();
    }
}

impl FerroObjectImpl for NavigationPageCurvedHeaderPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Visual::bounds_property().as_property() {
            this.update_info_panel_visibility();
        }
    }
}

impl NavigationPageCurvedHeaderPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), nav_page: RefCell::new(None), info_panel: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        *this.nav_page.borrow_mut() = this.find_control::<NavigationPage>("NavPage");
        let nav_page = this.nav_page.borrow().clone();
        if let Some(nav_page) = nav_page {
            drop(nav_page.push_async(this.build_home_page()));
        }
        this
    }

    fn update_info_panel_visibility(&self) {
        if let Some(info_panel) = self.info_panel.borrow().as_ref() {
            info_panel.set_is_visible(self.bounds().width >= 650.0);
        }
    }

    /// `async () => { if (_navPage != null) await _navPage.PushAsync(BuildProfilePage()); }`,
    /// started: nothing follows the push.
    fn push_profile_page(&self) {
        let nav_page = self.nav_page.borrow().clone();
        if let Some(nav_page) = nav_page {
            drop(nav_page.push_async(self.build_profile_page()));
        }
    }

    fn build_home_page(&self) -> Ref<ContentPage> {
        let bg_path = Path::new();
        bg_path.set_fill(solid(Colors::WHITE));

        let welcome = TextBlock::new();
        welcome.set_text(Some("Welcome back, Alex"));
        welcome.set_font_size(20.0);
        welcome.set_font_weight(FontWeight::Bold);
        welcome.set_foreground(solid(text_dark()));
        welcome.set_text_alignment(TextAlignment::Center);

        let ready = TextBlock::new();
        ready.set_text(Some("Ready to explore?"));
        ready.set_font_size(14.0);
        ready.set_foreground(solid(text_muted()));
        ready.set_text_alignment(TextAlignment::Center);

        let search = TextBox::new();
        search.set_margin(Thickness::new(0.0, 10.0, 0.0, 0.0));
        search.set_placeholder_text(Some("Search for products..."));
        search.set_corner_radius(CornerRadius::uniform(999.0));
        search.set_height(40.0);
        search.set_padding(Thickness::symmetric(16.0, 10.0));
        search.set_font_size(14.0);
        search.set_text_alignment(TextAlignment::Center);
        search.set_vertical_content_alignment(VerticalAlignment::Center);
        search.set_background(solid(parse_color("#f1f5f9")));
        search.set_border_thickness(Thickness::uniform(0.0));
        search.styles().add(Style::with_setters(
            Selectors::of_type::<TextBox>().template().of_type::<TextBlock>().name("PART_PlaceholderText"),
            [
                Setter::new(TextBlock::text_alignment_property(), TextAlignment::Center),
                Setter::new(Layoutable::horizontal_alignment_property(), HorizontalAlignment::Stretch),
            ],
        ));

        let header_content = StackPanel::new();
        header_content.set_horizontal_alignment(HorizontalAlignment::Center);
        header_content.set_spacing(4.0);
        header_content.set_margin(Thickness::new(24.0, 20.0, 24.0, 0.0));
        header_content.children().add(welcome);
        header_content.children().add(ready);
        header_content.children().add(search);

        let avatar = Ellipse::new();
        avatar.set_width(AVATAR_HOME_SIZE);
        avatar.set_height(AVATAR_HOME_SIZE);
        avatar.set_fill(img_brush("avatar.jpg", solid(parse_color("#93c5fd"))));
        let avatar_ring = Ellipse::new();
        avatar_ring.set_width(AVATAR_HOME_SIZE + 6.0);
        avatar_ring.set_height(AVATAR_HOME_SIZE + 6.0);
        avatar_ring.set_stroke(white());
        avatar_ring.set_stroke_thickness(3.0);
        avatar_ring.set_fill(transparent());

        let header_panel = Panel::new();
        header_panel.set_vertical_alignment(VerticalAlignment::Top);

        // The views and the canvas end up in a page of the navigation page of this control:
        // their handlers hold the control weakly.
        let home_scroll = CurvedHeaderHomeScrollView::new();
        let weak = self.to_ref().downgrade();
        home_scroll.set_navigate_requested(Some(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.push_profile_page();
            }
        })));

        {
            let (bg_path, avatar, avatar_ring) = (bg_path.clone(), avatar.clone(), avatar_ring.clone());
            header_panel.size_changed(move |_, args| {
                let w = args.new_size().width;
                if w <= 1.0 {
                    return;
                }

                bg_path.set_data(Self::build_dome_geometry(w, HOME_HEADER_FLAT_H, DOME_H));

                let dome_tip_y = HOME_HEADER_FLAT_H + DOME_H;
                Canvas::set_left(&avatar, (w - AVATAR_HOME_SIZE) / 2.0);
                Canvas::set_top(&avatar, dome_tip_y - AVATAR_HOME_SIZE / 2.0);
                Canvas::set_left(&avatar_ring, (w - (AVATAR_HOME_SIZE + 6.0)) / 2.0);
                Canvas::set_top(&avatar_ring, dome_tip_y - (AVATAR_HOME_SIZE + 6.0) / 2.0);
            });
        }

        let avatar_canvas = Canvas::new();
        avatar_canvas.set_is_hit_test_visible(true);
        avatar_canvas.set_cursor(Some(Cursor::new(StandardCursorType::Hand)));
        avatar_canvas.children().add(avatar_ring);
        avatar_canvas.children().add(avatar);
        let weak = self.to_ref().downgrade();
        avatar_canvas.add_handler(InputElement::pointer_released_event(), move |_, _: &PointerReleasedEventArgs| {
            if let Some(this) = weak.upgrade() {
                this.push_profile_page();
            }
        });

        header_panel.children().add(bg_path);
        header_panel.children().add(header_content);
        header_panel.children().add(avatar_canvas);

        let root = Panel::new();
        root.children().add(home_scroll);
        root.children().add(header_panel);

        let page = ContentPage::new();
        page.set_background(solid(bg_light()));
        page.set_content(Some(Control::boxed(root)));
        NavigationPage::set_has_navigation_bar(&page, false);
        page
    }

    fn build_profile_page(&self) -> Ref<ContentPage> {
        let fill = LinearGradientBrush::new();
        fill.set_start_point(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
        fill.set_end_point(RelativePoint::new(1.0, 1.0, RelativeUnit::Relative));
        fill.gradient_stops().add(GradientStop::with_color_and_offset(parse_color("#137fec"), 0.0));
        fill.gradient_stops().add(GradientStop::with_color_and_offset(parse_color("#0a4fa8"), 1.0));
        let bg_path = Path::new();
        bg_path.set_fill(Some(fill.into()));

        let name = TextBlock::new();
        name.set_text(Some("Alex Johnson"));
        name.set_font_size(20.0);
        name.set_font_weight(FontWeight::Bold);
        name.set_foreground(white());
        name.set_text_alignment(TextAlignment::Center);

        let role = TextBlock::new();
        role.set_text(Some("UI/UX Designer \u{b7} San Francisco"));
        role.set_font_size(13.0);
        role.set_foreground(solid(Color::from_argb(210, 255, 255, 255)));
        role.set_text_alignment(TextAlignment::Center);

        let profile_content = StackPanel::new();
        profile_content.set_horizontal_alignment(HorizontalAlignment::Center);
        profile_content.set_spacing(3.0);
        profile_content.set_margin(Thickness::new(24.0, 52.0, 24.0, 0.0));
        profile_content.children().add(name);
        profile_content.children().add(role);

        let avatar = Ellipse::new();
        avatar.set_width(AVATAR_PROFILE_SIZE);
        avatar.set_height(AVATAR_PROFILE_SIZE);
        avatar.set_fill(img_brush("avatar.jpg", solid(parse_color("#60a5fa"))));
        let avatar_ring = Ellipse::new();
        avatar_ring.set_width(AVATAR_PROFILE_SIZE + 6.0);
        avatar_ring.set_height(AVATAR_PROFILE_SIZE + 6.0);
        avatar_ring.set_stroke(white());
        avatar_ring.set_stroke_thickness(3.0);
        avatar_ring.set_fill(transparent());

        let header_panel = Panel::new();
        header_panel.set_vertical_alignment(VerticalAlignment::Top);

        let profile_scroll = CurvedHeaderProfileScrollView::new();

        {
            let (bg_path, avatar, avatar_ring) = (bg_path.clone(), avatar.clone(), avatar_ring.clone());
            header_panel.size_changed(move |_, args| {
                let w = args.new_size().width;
                if w <= 1.0 {
                    return;
                }

                bg_path.set_data(Self::build_dome_geometry(w, PROFILE_HEADER_FLAT_H, DOME_H));

                let tip_y = PROFILE_HEADER_FLAT_H + DOME_H;
                Canvas::set_left(&avatar, (w - AVATAR_PROFILE_SIZE) / 2.0);
                Canvas::set_top(&avatar, tip_y - AVATAR_PROFILE_SIZE / 2.0);
                Canvas::set_left(&avatar_ring, (w - (AVATAR_PROFILE_SIZE + 6.0)) / 2.0);
                Canvas::set_top(&avatar_ring, tip_y - (AVATAR_PROFILE_SIZE + 6.0) / 2.0);
            });
        }

        let avatar_canvas = Canvas::new();
        avatar_canvas.set_is_hit_test_visible(false);
        avatar_canvas.children().add(avatar_ring);
        avatar_canvas.children().add(avatar);

        header_panel.children().add(bg_path);
        header_panel.children().add(profile_content);
        header_panel.children().add(avatar_canvas);

        let root = Panel::new();
        root.children().add(profile_scroll);
        root.children().add(header_panel);

        let page = ContentPage::new();
        page.set_background(solid(bg_light()));
        page.set_content(Some(Control::boxed(root)));
        NavigationPage::set_bar_layout_behavior(&page, Some(BarLayoutBehavior::Overlay));
        page
    }

    fn build_dome_geometry(w: f64, flat_h: f64, dome_h: f64) -> Ref<StreamGeometry> {
        let sg = StreamGeometry::new();
        {
            let mut ctx = sg.open();
            ctx.begin_figure(Point::new(0.0, 0.0), true);
            ctx.line_to(Point::new(w, 0.0), true);
            ctx.line_to(Point::new(w, flat_h), true);
            ctx.arc_to(Point::new(0.0, flat_h), Size::new(w / 2.0, dome_h), 0.0, false, SweepDirection::Clockwise, true);
            ctx.end_figure(true);
            ctx.dispose();
        }
        sg
    }
}
