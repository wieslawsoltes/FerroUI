//! Port of `Pages/WindowDecorationsPage.xaml.cs`: the class of the document
//! `Pages/WindowDecorationsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::ShowWindowTest;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Visual, WeakRef};
use ferroui_controls::{CheckBox, Control, TextBox, TopLevel, UserControl, Window};
use std::rc::Rc;

#[repr(C)]
pub struct WindowDecorationsPage {
    base: UserControl,
}

user_control_class!(WindowDecorationsPage);
ferro_class_info!(WindowDecorationsPage {
    new: WindowDecorationsPage::new,
    markup: {
        methods: [
            fn ApplyWindowDecorations_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<WindowDecorationsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.apply_window_decorations_click(&sender, e.as_routed_event_args())
                },
            fn ShowNewWindowDecorations_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<WindowDecorationsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.show_new_window_decorations_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(WindowDecorationsPage, "/Pages/WindowDecorationsPage.xaml");

/// The text of a boolean, as the managed original writes it.
fn boolean_text(value: bool) -> &'static str {
    if value {
        "True"
    } else {
        "False"
    }
}

impl WindowDecorationsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn window_extend_client_area_to_decorations_hint(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("WindowExtendClientAreaToDecorationsHint")
    }

    fn window_show_title_area_control(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("WindowShowTitleAreaControl")
    }

    fn window_title_bar_height_hint(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("WindowTitleBarHeightHint")
    }

    fn window_decoration_properties(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("WindowDecorationProperties")
    }

    /// # Panics
    /// Panics if the check box of the hint is in its third state (a null value in the managed
    /// original).
    fn set_window_decorations(&self, window: &Ref<Window>) {
        window.set_extend_client_area_to_decorations_hint(
            self.window_extend_client_area_to_decorations_hint().is_checked().expect("the check box is checked or not"),
        );
        // `int.TryParse`, which allows white space around the number.
        let hint = self.window_title_bar_height_hint().text().unwrap_or_default();
        window.set_extend_client_area_title_bar_height_hint(match hint.trim().parse::<i32>() {
            Ok(val) => f64::from(val) / window.desktop_scaling(),
            Err(_) => -1.0,
        });

        if let Some(show_window_test) = window.cast::<ShowWindowTest>() {
            if self.window_show_title_area_control().is_checked() == Some(true) {
                show_window_test.show_title_area_control();
            }
        }

        self.adjust_offsets(window);

        let transparent: Rc<dyn IBrush> = Brushes::transparent();
        window.set_background(Some(transparent));
        // `WindowOnPropertyChanged`. The window holds the handlers of its events and may be
        // the window of the page: the handler holds both weakly.
        let weak = self.to_ref().downgrade();
        let weak_window: WeakRef<Window> = window.downgrade();
        window.property_changed(move |e| {
            let (Some(this), Some(window)) = (weak.upgrade(), weak_window.upgrade()) else {
                return;
            };
            if e.property() == Window::off_screen_margin_property().as_property()
                || e.property() == Window::window_decoration_margin_property().as_property()
            {
                this.adjust_offsets(&window);
            }
        });
    }

    /// `AdjustOffsets`, a local function of `SetWindowDecorations` in the managed original.
    ///
    /// # Panics
    /// Panics if the content of the window is not a control (an invalid cast in the managed
    /// original).
    fn adjust_offsets(&self, window: &Ref<Window>) {
        let scaling = window.desktop_scaling();

        window.set_padding(window.off_screen_margin());
        let content = window.content().and_then(|content| Control::from_boxed(&content));
        content.expect("the content of the window is a control").set_margin(window.window_decoration_margin());

        self.window_decoration_properties().set_text(Some(&format!(
            "{} {} {}",
            window.off_screen_margin().top * scaling,
            window.window_decoration_margin().top * scaling,
            boolean_text(window.is_extended_into_window_decorations())
        )));
    }

    /// # Panics
    /// Panics if the page is not attached to a window (the internal exception of the managed
    /// original).
    fn apply_window_decorations_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let visual: &Visual = self;
        let window = TopLevel::get_top_level(Some(visual))
            .and_then(|top_level| top_level.cast::<Window>())
            .unwrap_or_else(|| panic!("WindowDecorationsPage is not attached to a Window."));
        self.set_window_decorations(&window);
    }

    fn show_new_window_decorations_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let window = ShowWindowTest::new();
        self.set_window_decorations(&window.clone().upcast());
        window.show();
    }
}
