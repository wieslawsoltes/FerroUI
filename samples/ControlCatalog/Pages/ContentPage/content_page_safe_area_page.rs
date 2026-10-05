//! Port of `Pages/ContentPage/ContentPageSafeAreaPage.xaml.cs`: the class of the document
//! `Pages/ContentPage/ContentPageSafeAreaPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Thickness};
use ferroui_controls::{Border, CheckBox, ContentPage, Slider, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct ContentPageSafeAreaPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    initialized: Cell<bool>,
}

user_control_class!(ContentPageSafeAreaPage);
ferro_class_info!(ContentPageSafeAreaPage {
    new: ContentPageSafeAreaPage::new,
    markup: {
        methods: [
            fn OnAutoApplyChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageSafeAreaPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_auto_apply_changed(&sender, e.as_routed_event_args())
                },
            fn OnInsetChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ContentPageSafeAreaPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_inset_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ContentPageSafeAreaPage, "/Pages/ContentPage/ContentPageSafeAreaPage.xaml");

impl ContentPageSafeAreaPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.initialized.set(true);

        // The handler of an event of the page itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn sample_page(&self) -> Option<Ref<ContentPage>> {
        if self.initialized.get() { self.find_control::<ContentPage>("SamplePage") } else { None }
    }

    fn auto_apply_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("AutoApplyCheck")
    }

    fn top_value(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("TopValue")
    }

    fn top_slider(&self) -> Ref<Slider> {
        self.get_control::<Slider>("TopSlider")
    }

    fn bottom_value(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("BottomValue")
    }

    fn bottom_slider(&self) -> Ref<Slider> {
        self.get_control::<Slider>("BottomSlider")
    }

    fn left_value(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("LeftValue")
    }

    fn left_slider(&self) -> Ref<Slider> {
        self.get_control::<Slider>("LeftSlider")
    }

    fn right_value(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("RightValue")
    }

    fn right_slider(&self) -> Ref<Slider> {
        self.get_control::<Slider>("RightSlider")
    }

    fn safe_area_info(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("SafeAreaInfo")
    }

    fn auto_apply_info(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("AutoApplyInfo")
    }

    fn top_inset_indicator(&self) -> Ref<Border> {
        self.get_control::<Border>("TopInsetIndicator")
    }

    fn bottom_inset_indicator(&self) -> Ref<Border> {
        self.get_control::<Border>("BottomInsetIndicator")
    }

    fn left_inset_indicator(&self) -> Ref<Border> {
        self.get_control::<Border>("LeftInsetIndicator")
    }

    fn right_inset_indicator(&self) -> Ref<Border> {
        self.get_control::<Border>("RightInsetIndicator")
    }

    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.sync_indicators();
    }

    fn on_auto_apply_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(sample_page) = self.sample_page() else {
            return;
        };

        sample_page.set_automatically_apply_safe_area_padding(self.auto_apply_check().is_checked() == Some(true));
        self.sync_indicators();
    }

    fn on_inset_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.sync_indicators();
    }

    fn sync_indicators(&self) {
        let Some(sample_page) = self.sample_page() else {
            return;
        };

        // `(int)value`: the fraction is dropped.
        let top = self.top_slider().value() as i32;
        let bottom = self.bottom_slider().value() as i32;
        let left = self.left_slider().value() as i32;
        let right = self.right_slider().value() as i32;

        self.top_value().set_text(Some(&top.to_string()));
        self.bottom_value().set_text(Some(&bottom.to_string()));
        self.left_value().set_text(Some(&left.to_string()));
        self.right_value().set_text(Some(&right.to_string()));

        self.top_inset_indicator().set_is_visible(top > 0);
        self.top_inset_indicator().set_height(f64::from(top));

        self.bottom_inset_indicator().set_is_visible(bottom > 0);
        self.bottom_inset_indicator().set_height(f64::from(bottom));

        self.left_inset_indicator().set_is_visible(left > 0);
        self.left_inset_indicator().set_width(f64::from(left));
        self.left_inset_indicator().set_margin(Thickness::new(0.0, f64::from(top), 0.0, f64::from(bottom)));

        self.right_inset_indicator().set_is_visible(right > 0);
        self.right_inset_indicator().set_width(f64::from(right));
        self.right_inset_indicator().set_margin(Thickness::new(0.0, f64::from(top), 0.0, f64::from(bottom)));

        let insets = Thickness::new(f64::from(left), f64::from(top), f64::from(right), f64::from(bottom));
        sample_page.set_safe_area_padding(insets);

        self.safe_area_info().set_text(Some(&format!("SafeAreaPadding: L={left} T={top} R={right} B={bottom}")));
        let auto_apply = sample_page.automatically_apply_safe_area_padding();
        self.auto_apply_info().set_text(Some(&format!(
            "AutoApply: {}  \u{2192}  {}",
            if auto_apply { "True" } else { "False" },
            if auto_apply { "insets absorbed by presenter" } else { "insets ignored" }
        )));
    }
}
