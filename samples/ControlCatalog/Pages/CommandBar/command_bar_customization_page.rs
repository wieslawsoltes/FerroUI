//! Port of `Pages/CommandBar/CommandBarCustomizationPage.xaml.cs`: the class of the document
//! `Pages/CommandBar/CommandBarCustomizationPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, RelativePoint, RelativeUnit, CornerRadius, Thickness};
use ferroui_base::media::{Brushes, Color, GradientStop, IBrush, LinearGradientBrush, SolidColorBrush};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{ComboBox, ComboBoxItem, CommandBar, Control, Slider, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

/// The tag of the selected item of `combo`, when the item is a `ComboBoxItem`
/// whose tag is text (`SelectedItem is ComboBoxItem { Tag: string }`).
fn selected_tag(combo: &ComboBox) -> Option<String> {
    let item = combo.selected_item().and_then(|item| Control::from_boxed(&item))?.cast::<ComboBoxItem>()?;
    item.tag().and_then(|tag| tag.downcast_ref::<String>().cloned())
}

/// `Color.Parse(text)`.
///
/// # Panics
/// Panics if the text is not a color (the format exception of the original).
fn parse_color(text: &str) -> Color {
    match Color::parse(text) {
        Ok(color) => color,
        Err(error) => panic!("{error}"),
    }
}

#[repr(C)]
pub struct CommandBarCustomizationPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    initialized: Cell<bool>,
}

user_control_class!(CommandBarCustomizationPage);
ferro_class_info!(CommandBarCustomizationPage {
    new: CommandBarCustomizationPage::new,
    markup: {
        methods: [
            fn OnBgPresetChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_bg_preset_changed(&sender, e.as_routed_event_args())
                },
            fn OnFgChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_fg_changed(&sender, e.as_routed_event_args())
                },
            fn OnRadiusChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_radius_changed(&sender, e.as_routed_event_args())
                },
            fn OnBorderChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_border_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CommandBarCustomizationPage, "/Pages/CommandBar/CommandBarCustomizationPage.xaml");

impl CommandBarCustomizationPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.initialized.set(true);
        this
    }

    fn live_bar(&self) -> Option<Ref<CommandBar>> {
        if self.initialized.get() { self.find_control::<CommandBar>("LiveBar") } else { None }
    }

    fn bg_preset_combo(&self) -> Option<Ref<ComboBox>> {
        if self.initialized.get() { self.find_control::<ComboBox>("BgPresetCombo") } else { None }
    }

    fn fg_combo(&self) -> Option<Ref<ComboBox>> {
        if self.initialized.get() { self.find_control::<ComboBox>("FgCombo") } else { None }
    }

    fn radius_slider(&self) -> Option<Ref<Slider>> {
        if self.initialized.get() { self.find_control::<Slider>("RadiusSlider") } else { None }
    }

    fn radius_label(&self) -> Option<Ref<TextBlock>> {
        if self.initialized.get() { self.find_control::<TextBlock>("RadiusLabel") } else { None }
    }

    fn border_slider(&self) -> Option<Ref<Slider>> {
        if self.initialized.get() { self.find_control::<Slider>("BorderSlider") } else { None }
    }

    fn border_label(&self) -> Option<Ref<TextBlock>> {
        if self.initialized.get() { self.find_control::<TextBlock>("BorderLabel") } else { None }
    }

    fn on_bg_preset_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(live_bar), Some(bg_preset_combo)) = (self.live_bar(), self.bg_preset_combo()) else {
            return;
        };

        let Some(preset) = selected_tag(&bg_preset_combo) else {
            live_bar.clear_value(TemplatedControl::background_property());
            return;
        };

        match preset.as_str() {
            "Gradient" => {
                let brush = LinearGradientBrush::new();
                brush.set_start_point(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
                brush.set_end_point(RelativePoint::new(1.0, 0.0, RelativeUnit::Relative));
                brush.gradient_stops().add(GradientStop::with_color_and_offset(parse_color("#3F51B5"), 0.0));
                brush.gradient_stops().add(GradientStop::with_color_and_offset(parse_color("#E91E63"), 1.0));
                live_bar.set_background(Some(brush.into()));
            }
            "Transparent" => {
                let brush: Rc<dyn IBrush> = Brushes::transparent();
                live_bar.set_background(Some(brush));
            }
            _ => live_bar.set_background(Some(SolidColorBrush::with_color(parse_color(&preset)).into())),
        }
    }

    fn on_fg_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(live_bar), Some(fg_combo)) = (self.live_bar(), self.fg_combo()) else {
            return;
        };

        if let Some(color) = selected_tag(&fg_combo) {
            live_bar.set_foreground(Some(SolidColorBrush::with_color(parse_color(&color)).into()));
            return;
        }

        live_bar.clear_value(TemplatedControl::foreground_property());
    }

    fn on_radius_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(live_bar), Some(radius_slider), Some(radius_label)) =
            (self.live_bar(), self.radius_slider(), self.radius_label())
        else {
            return;
        };

        // `(int)value`: the fraction is dropped.
        let r = radius_slider.value() as i32;
        live_bar.set_corner_radius(CornerRadius::uniform(f64::from(r)));
        radius_label.set_text(Some(&r.to_string()));
    }

    fn on_border_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(live_bar), Some(border_slider), Some(border_label)) =
            (self.live_bar(), self.border_slider(), self.border_label())
        else {
            return;
        };

        // `(int)value`: the fraction is dropped.
        let t = border_slider.value() as i32;
        live_bar.set_border_thickness(Thickness::uniform(f64::from(t)));
        border_label.set_text(Some(&t.to_string()));

        if t > 0 {
            let brush: Rc<dyn IBrush> = Brushes::gray();
            live_bar.set_border_brush(Some(brush));
        } else {
            live_bar.set_border_brush(None);
        }
    }
}
