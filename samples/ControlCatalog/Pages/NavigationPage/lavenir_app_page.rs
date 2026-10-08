//! Port of `Pages/NavigationPage/LAvenirAppPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/LAvenirAppPage.xaml`.

use super::{LAvenirDishDetailView, LAvenirMenuView, LAvenirProfileView, LAvenirReservationsView};
use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::{boxed_text, parse_color, parse_geometry};
use ferroui_base::animation::{PageSlide, SlideAxis, TimeSpan};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{Brushes, Color, FontWeight, IBrush, SolidColorBrush, TextAlignment};
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, CornerRadius, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, Ref, StyledElementImpl, Thickness, Visual, VisualImpl,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::shapes::Ellipse;
use ferroui_controls::templates::FuncDataTemplate;
use ferroui_controls::{
    Border, Button, ColumnDefinitions, ContentControlImpl, ContentPage, Control, ControlImpl, ControlImplExt,
    DrawerPage, Grid, NavigationPage, Page, PathIcon, ScrollViewer, StackPanel, TabPlacement, TabbedPage,
    TextBlock, UserControl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// The colors of the page (the static fields `Primary`, `BgDark`, `BgLight`, `TextDark`,
/// `TextMuted` and `BorderLight`).
fn primary() -> Color {
    parse_color("#4b2bee")
}

fn bg_dark() -> Color {
    parse_color("#131022")
}

fn bg_light() -> Color {
    parse_color("#f6f6f8")
}

fn text_dark() -> Color {
    parse_color("#1e293b")
}

fn text_muted() -> Color {
    parse_color("#94a3b8")
}

fn border_light() -> Color {
    parse_color("#e2e8f0")
}

/// `new SolidColorBrush(color)` as the value of a brush property.
fn solid(color: Color) -> Option<Rc<dyn IBrush>> {
    Some(SolidColorBrush::with_color(color).into())
}

/// `Brushes.White` as the value of a brush property.
fn white() -> Option<Rc<dyn IBrush>> {
    let white: Rc<dyn IBrush> = Brushes::white();
    Some(white)
}

/// `new SolidColorBrush(color)` as the value of a resource.
fn solid_resource(color: Color) -> Option<BoxedValue> {
    let brush: Rc<dyn IBrush> = SolidColorBrush::with_color(color).into();
    Some(Rc::new(brush))
}

/// A brush of `Brushes` as the value of a resource.
fn brush_resource(brush: Rc<dyn IBrush>) -> Option<BoxedValue> {
    Some(Rc::new(brush))
}

/// `new PathIcon { Data = Geometry.Parse(data) }` as the icon of a page.
fn page_icon(data: &str) -> Option<BoxedValue> {
    let icon = PathIcon::new();
    icon.set_data(parse_geometry(data));
    Some(Control::boxed(icon))
}

#[repr(C)]
pub struct LAvenirAppPage {
    base: UserControl,
    nav_page: RefCell<Option<Ref<NavigationPage>>>,
    drawer_page: RefCell<Option<Ref<DrawerPage>>>,
    info_panel: RefCell<Option<Ref<ScrollViewer>>>,
}

ferro_class!(LAvenirAppPage: UserControl);
ferro_impl_classes!(
    LAvenirAppPage: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(LAvenirAppPage {
    new: LAvenirAppPage::new,
    markup: {
        methods: [
            fn OnMenuItemClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<LAvenirAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_menu_item_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(LAvenirAppPage, "/Pages/NavigationPage/LAvenirAppPage.xaml");

impl ControlImpl for LAvenirAppPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

        *this.info_panel.borrow_mut() = this.find_control::<ScrollViewer>("InfoPanel");
        this.update_info_panel_visibility();
    }
}

impl FerroObjectImpl for LAvenirAppPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Visual::bounds_property().as_property() {
            this.update_info_panel_visibility();
        }
    }
}

impl LAvenirAppPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            nav_page: RefCell::new(None),
            drawer_page: RefCell::new(None),
            info_panel: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        *this.nav_page.borrow_mut() = this.find_control::<NavigationPage>("NavPage");
        *this.drawer_page.borrow_mut() = this.find_control::<DrawerPage>("DrawerPageControl");

        let nav_page = this.nav_page.borrow().clone();
        if let Some(nav_page) = nav_page {
            drop(nav_page.push_async(this.build_menu_tabbed_page()));
        }
        this
    }

    fn update_info_panel_visibility(&self) {
        if let Some(info_panel) = self.info_panel.borrow().as_ref() {
            info_panel.set_is_visible(self.bounds().width >= 650.0);
        }
    }

    fn build_menu_tabbed_page(&self) -> Ref<TabbedPage> {
        let tp = TabbedPage::new();
        tp.set_background(solid(bg_light()));
        tp.set_tab_placement(TabPlacement::Bottom);
        tp.set_page_transition(Some(Rc::new(PageSlide::with_duration(
            TimeSpan::from_milliseconds(200.0),
            SlideAxis::Horizontal,
        ))));
        tp.resources().set("TabItemHeaderFontSize", Some(Rc::new(12.0_f64)));
        tp.resources().set("TabbedPageTabStripBackground", brush_resource(Brushes::white()));
        tp.resources().set("TabbedPageTabStripBorderThickness", Some(Rc::new(Thickness::new(0.0, 1.0, 0.0, 0.0))));
        tp.resources().set("TabbedPageTabStripBorderBrush", solid_resource(border_light()));
        tp.resources().set("TabbedPageTabItemHeaderForegroundSelected", solid_resource(primary()));
        tp.resources().set("TabbedPageTabItemHeaderForegroundUnselected", solid_resource(text_muted()));

        // `new FuncDataTemplate<object>(...)`: the template matches any data.
        tp.set_indicator_template(Some(FuncDataTemplate::new(
            |_| true,
            |_, _| {
                let indicator = Ellipse::new();
                indicator.set_width(5.0);
                indicator.set_height(5.0);
                indicator.set_margin(Thickness::new(0.0, 10.0, 0.0, 0.0));
                indicator.set_horizontal_alignment(HorizontalAlignment::Center);
                indicator.set_fill(solid(primary()));
                Some(indicator.upcast())
            },
            false,
        )));

        let header = TextBlock::new();
        header.set_text(Some("L'Avenir"));
        header.set_font_size(18.0);
        header.set_font_weight(FontWeight::Bold);
        header.set_foreground(solid(text_dark()));
        header.set_vertical_alignment(VerticalAlignment::Center);
        header.set_text_alignment(TextAlignment::Center);
        tp.set_header(Some(Control::boxed(header)));

        let search_icon = PathIcon::new();
        search_icon.set_data(parse_geometry("M9.5,3A6.5,6.5 0 0,1 16,9.5C16,11.11 15.41,12.59 14.44,13.73L14.71,14H15.5L20.5,19L19,20.5L14,15.5V14.71L13.73,14.44C12.59,15.41 11.11,16 9.5,16A6.5,6.5 0 0,1 3,9.5A6.5,6.5 0 0,1 9.5,3M9.5,5C7,5 5,7 5,9.5C5,12 7,14 9.5,14C12,14 14,12 14,9.5C14,7 12,5 9.5,5Z"));
        search_icon.set_width(18.0);
        search_icon.set_height(18.0);

        let search_btn = Button::new();
        search_btn.set_width(40.0);
        search_btn.set_height(40.0);
        search_btn.set_corner_radius(CornerRadius::uniform(12.0));
        let transparent: Rc<dyn IBrush> = Brushes::transparent();
        search_btn.set_background(Some(transparent));
        search_btn.set_foreground(solid(text_dark()));
        search_btn.set_padding(Thickness::uniform(8.0));
        search_btn.set_border_thickness(Thickness::uniform(0.0));
        search_btn.set_content(Some(Control::boxed(search_icon)));
        search_btn.set_vertical_alignment(VerticalAlignment::Center);
        NavigationPage::set_top_command_bar(&tp, search_btn);

        let menu_view = LAvenirMenuView::new();
        // The view ends up in a page of the navigation page of this control: its action holds
        // this control weakly.
        let weak = self.to_ref().downgrade();
        menu_view.set_dish_selected(Some(Rc::new(
            move |name: &str, price: &str, description: &str, image_file: &str| {
                if let Some(this) = weak.upgrade() {
                    this.push_dish_detail(name, price, description, image_file);
                }
            },
        )));

        let menu_page = ContentPage::new();
        menu_page.set_content(Some(Control::boxed(menu_view)));
        menu_page.set_background(solid(bg_light()));
        menu_page.set_header(Some(boxed_text("Menu")));
        menu_page.set_icon(page_icon(
            "M11 9H9V2H7v7H5V2H3v7c0 2.12 1.66 3.84 3.75 3.97V22h2.5v-9.03C11.34 12.84 13 11.12 13 9V2h-2v7zm5-3v8h2.5v8H21V2c-2.76 0-5 2.24-5 4z",
        ));

        let reservations_page = ContentPage::new();
        reservations_page.set_content(Some(Control::boxed(LAvenirReservationsView::new())));
        reservations_page.set_background(solid(bg_light()));
        reservations_page.set_header(Some(boxed_text("Reservations")));
        reservations_page.set_icon(page_icon(
            "M19 3h-1V1h-2v2H8V1H6v2H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V8h14v11zM9 10H7v2h2v-2zm4 0h-2v2h2v-2zm4 0h-2v2h2v-2z",
        ));

        let profile_page = ContentPage::new();
        profile_page.set_content(Some(Control::boxed(LAvenirProfileView::new())));
        profile_page.set_background(solid(bg_light()));
        profile_page.set_header(Some(boxed_text("Profile")));
        profile_page.set_icon(page_icon(
            "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 3c1.66 0 3 1.34 3 3s-1.34 3-3 3-3-1.34-3-3 1.34-3 3-3zm0 14.2c-2.5 0-4.71-1.28-6-3.22.03-1.99 4-3.08 6-3.08 1.99 0 5.97 1.09 6 3.08-1.29 1.94-3.5 3.22-6 3.22z",
        ));

        let pages: [Ref<Page>; 3] = [menu_page.upcast(), reservations_page.upcast(), profile_page.upcast()];
        tp.set_pages(Some(FerroList::from_items(pages)));
        tp
    }

    /// `async void`: nothing follows the push.
    fn push_dish_detail(&self, name: &str, price: &str, description: &str, image_file: &str) {
        let nav_page = self.nav_page.borrow().clone();
        let Some(nav_page) = nav_page else {
            return;
        };

        let detail = ContentPage::new();
        detail.set_content(Some(Control::boxed(LAvenirDishDetailView::with_dish(
            name,
            price,
            description,
            image_file,
        ))));
        detail.set_background(solid(bg_dark()));
        detail.set_header(Some(boxed_text(name)));
        NavigationPage::set_bottom_command_bar(&detail, Self::build_floating_bar(price));

        nav_page.set_background(solid(bg_dark()));
        nav_page.resources().set("NavigationBarBackground", solid_resource(bg_dark()));
        nav_page.resources().set("NavigationBarForeground", brush_resource(Brushes::white()));

        // The page ends up in the navigation page of this control: its handler holds this
        // control weakly.
        let weak = self.to_ref().downgrade();
        detail.navigated_from(move |_| {
            let Some(this) = weak.upgrade() else {
                return;
            };
            let nav_page = this.nav_page.borrow().clone();
            if let Some(nav_page) = nav_page {
                nav_page.set_background(solid(bg_light()));
                nav_page.resources().set("NavigationBarBackground", solid_resource(bg_light()));
                nav_page.resources().set("NavigationBarForeground", solid_resource(text_dark()));
            }
        });

        drop(nav_page.push_async(detail));
    }

    fn build_floating_bar(price: &str) -> Ref<Border> {
        let bg_dark = bg_dark();
        let bar = Border::new();
        bar.set_corner_radius(CornerRadius::uniform(16.0));
        bar.set_background(solid(Color::from_argb(178, bg_dark.r, bg_dark.g, bg_dark.b)));
        bar.set_border_brush(solid(Color::from_argb(51, 255, 255, 255)));
        bar.set_border_thickness(Thickness::uniform(1.0));
        bar.set_padding(Thickness::symmetric(16.0, 12.0));
        bar.set_margin(Thickness::new(16.0, 8.0, 16.0, 8.0));

        let bar_grid = Grid::new();
        bar_grid.set_column_definitions(match ColumnDefinitions::parse("*,Auto") {
            Ok(definitions) => definitions,
            Err(error) => panic!("{error}"),
        });

        let info = StackPanel::new();
        info.set_vertical_alignment(VerticalAlignment::Center);

        let title = TextBlock::new();
        title.set_text(Some("Add to Order"));
        title.set_font_size(14.0);
        title.set_font_weight(FontWeight::Bold);
        title.set_foreground(white());
        info.children().add(title);

        let price_text = TextBlock::new();
        price_text.set_text(Some(price));
        price_text.set_font_size(12.0);
        price_text.set_font_weight(FontWeight::Medium);
        price_text.set_foreground(solid(text_muted()));
        info.children().add(price_text);
        bar_grid.children().add(info);

        let add_btn = Button::new();
        add_btn.set_content(Some(boxed_text("Add")));
        add_btn.set_width(80.0);
        add_btn.set_height(40.0);
        add_btn.set_corner_radius(CornerRadius::uniform(10.0));
        add_btn.set_background(solid(primary()));
        add_btn.set_foreground(white());
        add_btn.set_font_weight(FontWeight::Bold);
        add_btn.set_font_size(14.0);
        add_btn.set_horizontal_content_alignment(HorizontalAlignment::Center);
        add_btn.set_vertical_content_alignment(VerticalAlignment::Center);

        let hover_style = Style::with_selector(
            Selectors::of_type::<Button>().class(":pointerover").descendant().of_type::<ContentPresenter>(),
        );
        hover_style.add_setter(Setter::new(ContentPresenter::background_property(), solid(parse_color("#3d22cc"))));
        hover_style.add_setter(Setter::new(ContentPresenter::foreground_property(), white()));
        add_btn.styles().add(hover_style);

        let press_style = Style::with_selector(
            Selectors::of_type::<Button>().class(":pressed").descendant().of_type::<ContentPresenter>(),
        );
        press_style.add_setter(Setter::new(ContentPresenter::background_property(), solid(parse_color("#3518b0"))));
        press_style.add_setter(Setter::new(ContentPresenter::foreground_property(), white()));
        add_btn.styles().add(press_style);

        Grid::set_column(&add_btn, 1);
        bar_grid.children().add(add_btn);
        bar.set_child(bar_grid);
        bar
    }

    fn on_menu_item_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        // `sender is not Button btn || btn.Tag is not string`.
        let btn = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<Button>());
        let Some(btn) = btn else {
            return;
        };
        if !btn.tag().is_some_and(|tag| tag.downcast_ref::<String>().is_some()) {
            return;
        }

        if let Some(drawer_page) = self.drawer_page.borrow().as_ref() {
            drawer_page.set_is_open(false);
        }

        let nav_page = self.nav_page.borrow().clone();
        if let Some(nav_page) = nav_page {
            drop(nav_page.pop_to_root_async());
        }
    }
}
