//! Port of `Pages/ContentPage/ContentPageCustomizationPage.xaml.cs`: the class of the document
//! `Pages/ContentPage/ContentPageCustomizationPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Thickness};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Color, IBrush, SolidColorBrush};
use ferroui_controls::{ComboBox, ContentPage, Slider, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

/// `new SolidColorBrush(Color.Parse(text))`.
///
/// # Panics
/// Panics if the text is not a color (the format exception of the original).
fn solid_color_brush(text: &str) -> Rc<dyn IBrush> {
    match Color::parse(text) {
        Ok(color) => SolidColorBrush::with_color(color).into(),
        Err(error) => panic!("{error}"),
    }
}

#[repr(C)]
pub struct ContentPageCustomizationPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    initialized: Cell<bool>,
}

user_control_class!(ContentPageCustomizationPage);
ferro_class_info!(ContentPageCustomizationPage {
    new: ContentPageCustomizationPage::new,
    markup: {
        methods: [
            fn OnBackgroundChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_background_changed(&sender, e.as_routed_event_args())
                },
            fn OnHAlignChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_halign_changed(&sender, e.as_routed_event_args())
                },
            fn OnVAlignChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_valign_changed(&sender, e.as_routed_event_args())
                },
            fn OnPaddingChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_padding_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ContentPageCustomizationPage, "/Pages/ContentPage/ContentPageCustomizationPage.xaml");

impl ContentPageCustomizationPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.initialized.set(true);
        this
    }

    fn background_combo(&self) -> Option<Ref<ComboBox>> {
        if self.initialized.get() { self.find_control::<ComboBox>("BackgroundCombo") } else { None }
    }

    fn halign_combo(&self) -> Option<Ref<ComboBox>> {
        if self.initialized.get() { self.find_control::<ComboBox>("HAlignCombo") } else { None }
    }

    fn valign_combo(&self) -> Option<Ref<ComboBox>> {
        if self.initialized.get() { self.find_control::<ComboBox>("VAlignCombo") } else { None }
    }

    fn padding_slider(&self) -> Option<Ref<Slider>> {
        if self.initialized.get() { self.find_control::<Slider>("PaddingSlider") } else { None }
    }

    fn padding_label(&self) -> Option<Ref<TextBlock>> {
        if self.initialized.get() { self.find_control::<TextBlock>("PaddingLabel") } else { None }
    }

    fn sample_page(&self) -> Option<Ref<ContentPage>> {
        if self.initialized.get() { self.find_control::<ContentPage>("SamplePage") } else { None }
    }

    fn on_background_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(sample_page), Some(background_combo)) = (self.sample_page(), self.background_combo()) else {
            return;
        };

        sample_page.set_background(match background_combo.selected_index() {
            1 => Some(solid_color_brush("#E3F2FD")),
            2 => Some(solid_color_brush("#E8F5E9")),
            3 => Some(solid_color_brush("#F3E5F5")),
            4 => Some(solid_color_brush("#FFF8E1")),
            _ => None,
        });
    }

    fn on_halign_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(sample_page), Some(halign_combo)) = (self.sample_page(), self.halign_combo()) else {
            return;
        };

        sample_page.set_horizontal_content_alignment(match halign_combo.selected_index() {
            0 => HorizontalAlignment::Left,
            1 => HorizontalAlignment::Center,
            2 => HorizontalAlignment::Right,
            _ => HorizontalAlignment::Stretch,
        });
    }

    fn on_valign_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(sample_page), Some(valign_combo)) = (self.sample_page(), self.valign_combo()) else {
            return;
        };

        sample_page.set_vertical_content_alignment(match valign_combo.selected_index() {
            0 => VerticalAlignment::Top,
            1 => VerticalAlignment::Center,
            2 => VerticalAlignment::Bottom,
            _ => VerticalAlignment::Stretch,
        });
    }

    fn on_padding_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(sample_page), Some(padding_slider), Some(padding_label)) =
            (self.sample_page(), self.padding_slider(), self.padding_label())
        else {
            return;
        };

        // `(int)value`: the fraction is dropped.
        let padding = padding_slider.value() as i32;
        sample_page.set_padding(Thickness::uniform(f64::from(padding)));
        padding_label.set_text(Some(&format!("{padding} px")));
    }
}
