//! Port of `Pages/CarouselPage/SanctuaryShowcasePage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/SanctuaryShowcasePage.xaml`.

use super::SanctuaryMainPage;
use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::parse_geometry;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Thickness};
use ferroui_controls::{
    Button, CarouselPage, ColumnDefinitions, ContentPage, Control, Grid, NavigationPage, PathIcon, TextBlock,
    UserControl,
};
use mini_mvvm::start_async;
use std::rc::Rc;

const CLOSE_ICON: &str = "M4.397 4.397a1 1 0 0 1 1.414 0L12 10.585l6.19-6.188a1 1 0 0 1 1.414 1.414L13.413 12l6.19 6.189a1 1 0 0 1-1.414 1.414L12 13.413l-6.189 6.19a1 1 0 0 1-1.414-1.414L10.585 12 4.397 5.811a1 1 0 0 1 0-1.414z";

#[repr(C)]
pub struct SanctuaryShowcasePage {
    base: UserControl,
}

user_control_class!(SanctuaryShowcasePage);
ferro_class_info!(SanctuaryShowcasePage {
    new: SanctuaryShowcasePage::new,
    markup: {
        methods: [
            fn OnPage1CTA(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<SanctuaryShowcasePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_page1_cta(&sender, e.as_routed_event_args())
                },
            fn OnPage2CTA(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<SanctuaryShowcasePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_page2_cta(&sender, e.as_routed_event_args())
                },
            fn OnPage3CTA(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<SanctuaryShowcasePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_page3_cta(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(SanctuaryShowcasePage, "/Pages/CarouselPage/SanctuaryShowcasePage.xaml");

impl SanctuaryShowcasePage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn demo_carousel(&self) -> Ref<CarouselPage> {
        self.get_control::<CarouselPage>("DemoCarousel")
    }

    fn on_page1_cta(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.demo_carousel().set_selected_index(1);
    }

    fn on_page2_cta(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.demo_carousel().set_selected_index(2);
    }

    /// `async void`: the page of the carousel is removed once the main page is pushed.
    fn on_page3_cta(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(nav) = self.find_ancestor_of_type::<NavigationPage>(false) else {
            return;
        };

        let carousel_wrapper = nav.navigation_stack().last().cloned();

        let header_grid = Grid::new();
        header_grid.set_column_definitions(match ColumnDefinitions::parse("*, Auto") {
            Ok(definitions) => definitions,
            Err(error) => panic!("{error}"),
        });
        let header_text = TextBlock::new();
        header_text.set_text(Some("Sanctuary"));
        header_text.set_vertical_alignment(VerticalAlignment::Center);
        header_grid.children().add(header_text);

        let close_icon = PathIcon::new();
        close_icon.set_data(parse_geometry(CLOSE_ICON));
        let close_btn = Button::new();
        close_btn.set_content(Some(Control::boxed(close_icon)));
        let transparent: Rc<dyn IBrush> = Brushes::transparent();
        close_btn.set_background(Some(transparent));
        close_btn.set_border_thickness(Thickness::uniform(0.0));
        close_btn.set_padding(Thickness::symmetric(8.0, 4.0));
        close_btn.set_vertical_alignment(VerticalAlignment::Center);
        Grid::set_column(&close_btn, 1);
        header_grid.children().add(close_btn.clone());
        // The button ends up in a page of `nav`: its handler holds `nav` weakly.
        let weak_nav = nav.downgrade();
        close_btn.click(move |_, _| {
            if let Some(nav) = weak_nav.upgrade() {
                drop(nav.pop_async_with_transition(None));
            }
        });

        let main_page = ContentPage::new();
        main_page.set_header(Some(Control::boxed(header_grid)));
        main_page.set_content(Some(Control::boxed(SanctuaryMainPage::new())));
        NavigationPage::set_has_back_button(&main_page, false);

        let pushed = nav.push_async(main_page);
        drop(start_async(async move {
            if pushed.await.is_err() {
                return;
            }

            if let Some(carousel_wrapper) = carousel_wrapper {
                nav.remove_page(carousel_wrapper);
            }
        }));
    }
}
