//! Port of `Pages/NavigationPage/NavigationPageAppearancePage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageAppearancePage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::{boxed_text, NavigationDemoHelper};
use ferroui_base::controls::ResourceKey;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::layout::VerticalAlignment;
use ferroui_base::media::{Brushes, Color, Colors, FontWeight, IBrush, SolidColorBrush};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::primitives::RangeBaseValueChangedEventArgs;
use ferroui_controls::{
    BarLayoutBehavior, CheckBox, ComboBox, Control, NavigationPage, SelectionChangedEventArgs, Slider, StackPanel,
    TextBlock, UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct NavigationPageAppearancePage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    initialized: Cell<bool>,
    page_count: Cell<i32>,
    back_button_style: Cell<i32>,
}

user_control_class!(NavigationPageAppearancePage);
ferro_class_info!(NavigationPageAppearancePage {
    new: NavigationPageAppearancePage::new,
    markup: {
        methods: [
            fn OnHasNavBarChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAppearancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_has_nav_bar_changed(&sender, e.as_routed_event_args())
                },
            fn OnHasShadowChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAppearancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_has_shadow_changed(&sender, e.as_routed_event_args())
                },
            fn OnBarBgChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAppearancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_bar_bg_changed(&sender, e)
                    }
                },
            fn OnBarFgChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAppearancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_bar_fg_changed(&sender, e)
                    }
                },
            fn OnBarHeightChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAppearancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<RangeBaseValueChangedEventArgs>() {
                        this.on_bar_height_changed(&sender, e)
                    }
                },
            fn OnPerPageBarHeightChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAppearancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_per_page_bar_height_changed(&sender, e)
                    }
                },
            fn OnTitleStyleChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAppearancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_title_style_changed(&sender, e)
                    }
                },
            fn OnBarLayoutChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAppearancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_bar_layout_changed(&sender, e)
                    }
                },
            fn OnBackButtonStyleChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAppearancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_back_button_style_changed(&sender, e)
                    }
                },
            fn OnPush(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAppearancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageAppearancePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPageAppearancePage, "/Pages/NavigationPage/NavigationPageAppearancePage.xaml");

impl NavigationPageAppearancePage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            component_initialized: Cell::new(false),
            initialized: Cell::new(false),
            page_count: Cell::new(0),
            back_button_style: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);

        // The handler of an event of the control itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    /// The field `DemoNav`: null until `InitializeComponent()` has returned.
    fn demo_nav(&self) -> Option<Ref<NavigationPage>> {
        self.component_initialized.get().then(|| self.get_control::<NavigationPage>("DemoNav"))
    }

    fn has_nav_bar_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("HasNavBarCheck")
    }

    fn has_shadow_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("HasShadowCheck")
    }

    fn bar_bg_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("BarBgCombo")
    }

    fn bar_fg_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("BarFgCombo")
    }

    fn bar_height_label(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("BarHeightLabel")
    }

    fn bar_height_slider(&self) -> Ref<Slider> {
        self.get_control::<Slider>("BarHeightSlider")
    }

    fn per_page_bar_height_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("PerPageBarHeightCombo")
    }

    fn title_style_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("TitleStyleCombo")
    }

    /// The field `BarLayoutCombo`: null until `InitializeComponent()` has returned.
    fn bar_layout_combo(&self) -> Option<Ref<ComboBox>> {
        self.component_initialized.get().then(|| self.get_control::<ComboBox>("BarLayoutCombo"))
    }

    /// The field `BackButtonStyleCombo`: null until `InitializeComponent()` has returned.
    fn back_button_style_combo(&self) -> Option<Ref<ComboBox>> {
        self.component_initialized.get().then(|| self.get_control::<ComboBox>("BackButtonStyleCombo"))
    }

    /// The field `DemoNav` once the document is loaded.
    fn nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            return;
        }

        self.initialized.set(true);
        let page = NavigationDemoHelper::make_page("Appearance", "Change bar properties using the options panel.", 0);
        drop(self.nav().push_async_with_transition(page, None));
    }

    fn on_has_nav_bar_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };
        let show = self.has_nav_bar_check().is_checked() == Some(true);
        for p in demo_nav.navigation_stack().iter() {
            NavigationPage::set_has_navigation_bar(p, show);
        }
    }

    fn on_has_shadow_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };
        demo_nav.set_has_shadow(self.has_shadow_check().is_checked() == Some(true));
    }

    /// `DemoNav.Resources[key] = brush`, or `DemoNav.Resources.Remove(key)` without a brush.
    fn set_bar_brush(demo_nav: &NavigationPage, key: &str, brush: Option<Rc<dyn IBrush>>) {
        match brush {
            Some(brush) => demo_nav.resources().set(key, Some(Rc::new(brush) as BoxedValue)),
            None => {
                demo_nav.resources().remove(&ResourceKey::from(key));
            }
        }
    }

    fn on_bar_bg_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };

        let solid = |color: Color| -> Option<Rc<dyn IBrush>> { Some(SolidColorBrush::with_color(color).into()) };
        let brush: Option<Rc<dyn IBrush>> = match self.bar_bg_combo().selected_index() {
            1 => solid(Colors::DODGER_BLUE),
            2 => solid(Colors::DARK_SLATE_GRAY),
            3 => solid(Colors::INDIGO),
            4 => solid(Colors::CRIMSON),
            5 => Some(Brushes::transparent()),
            6 => solid(Color::from_argb(80, 20, 20, 20)),
            _ => None,
        };
        Self::set_bar_brush(&demo_nav, "NavigationBarBackground", brush);
    }

    fn on_bar_fg_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };

        let brush: Option<Rc<dyn IBrush>> = match self.bar_fg_combo().selected_index() {
            1 => Some(Brushes::white()),
            2 => Some(Brushes::black()),
            3 => Some(Brushes::yellow()),
            _ => None,
        };
        Self::set_bar_brush(&demo_nav, "NavigationBarForeground", brush);
    }

    fn on_bar_height_changed(&self, _sender: &Option<BoxedValue>, _e: &RangeBaseValueChangedEventArgs) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };
        let value = self.bar_height_slider().value() as i32;
        demo_nav.set_bar_height(f64::from(value));
        self.bar_height_label().set_text(Some(&value.to_string()));
    }

    fn on_per_page_bar_height_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(current_page) = self.demo_nav().and_then(|demo_nav| demo_nav.current_page()) else {
            return;
        };

        let height = match self.per_page_bar_height_combo().selected_index() {
            1 => Some(32.0),
            2 => Some(72.0),
            _ => None,
        };
        NavigationPage::set_bar_height_override(&current_page, height);
    }

    fn on_title_style_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(current_page) = self.demo_nav().and_then(|demo_nav| demo_nav.current_page()) else {
            return;
        };

        let header: BoxedValue = match self.title_style_combo().selected_index() {
            1 => {
                let title = TextBlock::new();
                title.set_text(Some("Big Title"));
                title.set_font_size(26.0);
                title.set_font_weight(FontWeight::Black);
                title.set_vertical_alignment(VerticalAlignment::Center);
                Control::boxed(title)
            }
            2 => {
                let title = TextBlock::new();
                title.set_text(Some("Custom Title"));
                title.set_font_size(15.0);
                title.set_font_weight(FontWeight::SemiBold);
                let subtitle = TextBlock::new();
                subtitle.set_text(Some("With subtitle"));
                subtitle.set_font_size(11.0);
                subtitle.set_opacity(0.5);

                let panel = StackPanel::new();
                panel.set_spacing(0.0);
                panel.set_vertical_alignment(VerticalAlignment::Center);
                panel.children().add(title);
                panel.children().add(subtitle);
                Control::boxed(panel)
            }
            _ => boxed_text("Appearance"),
        };
        current_page.set_header(Some(header));
    }

    fn on_bar_layout_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };

        let behavior = if self.bar_layout_combo().is_some_and(|combo| combo.selected_index() == 1) {
            BarLayoutBehavior::Overlay
        } else {
            BarLayoutBehavior::Inset
        };
        for p in demo_nav.navigation_stack().iter() {
            NavigationPage::set_bar_layout_behavior(p, Some(behavior));
        }
    }

    fn on_back_button_style_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(back_button_style_combo) = self.back_button_style_combo() else {
            return;
        };
        self.back_button_style.set(back_button_style_combo.selected_index());
    }

    /// `async void`: nothing follows the push.
    fn on_push(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.page_count.set(self.page_count.get() + 1);
        let page_count = self.page_count.get();
        let page =
            NavigationDemoHelper::make_page(&format!("Page {page_count}"), "Check the back button style.", page_count);

        let back_content = match self.back_button_style.get() {
            1 => Some(boxed_text("\u{2190} Back")),
            2 => Some(boxed_text("Cancel")),
            _ => None,
        };
        NavigationPage::set_back_button_content(&page, back_content);

        if self.bar_layout_combo().is_some_and(|combo| combo.selected_index() == 1) {
            NavigationPage::set_bar_layout_behavior(&page, Some(BarLayoutBehavior::Overlay));
        }

        NavigationPage::set_has_navigation_bar(&page, self.has_nav_bar_check().is_checked() == Some(true));

        drop(self.nav().push_async(page));
    }

    /// `async void`: nothing follows the pop.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        drop(self.nav().pop_async());
    }
}
