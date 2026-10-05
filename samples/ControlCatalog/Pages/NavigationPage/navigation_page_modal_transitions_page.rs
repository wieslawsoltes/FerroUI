//! Port of `Pages/NavigationPage/NavigationPageModalTransitionsPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageModalTransitionsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::boxed_text;
use ferroui_base::animation::{CrossFade, IPageTransition, PageSlide, SlideAxis, TimeSpan};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Color, FontWeight, SolidColorBrush, TextAlignment, TextWrapping};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::primitives::RangeBaseValueChangedEventArgs;
use ferroui_controls::{
    ComboBox, ContentPage, Control, NavigationPage, SelectionChangedEventArgs, Slider, StackPanel, TextBlock,
    UserControl,
};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

const MODAL_COLORS: [Color; 6] = [
    Color::from_rgb(237, 231, 246),
    Color::from_rgb(255, 243, 224),
    Color::from_rgb(224, 247, 250),
    Color::from_rgb(232, 245, 233),
    Color::from_rgb(255, 235, 238),
    Color::from_rgb(227, 242, 253),
];

#[repr(C)]
pub struct NavigationPageModalTransitionsPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    modal_count: Cell<i32>,
    initialized: Cell<bool>,
    /// `ModalColors`: one brush per color, shared by the modals of the control.
    modal_colors: Vec<Ref<SolidColorBrush>>,
}

user_control_class!(NavigationPageModalTransitionsPage);
ferro_class_info!(NavigationPageModalTransitionsPage {
    new: NavigationPageModalTransitionsPage::new,
    markup: {
        methods: [
            fn OnTransitionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageModalTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_transition_changed(&sender, e)
                    }
                },
            fn OnDurationChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageModalTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<RangeBaseValueChangedEventArgs>() {
                        this.on_duration_changed(&sender, e)
                    }
                },
            fn OnOpenModal(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageModalTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_open_modal(&sender, e.as_routed_event_args())
                },
            fn OnPopModal(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageModalTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop_modal(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPageModalTransitionsPage, "/Pages/NavigationPage/NavigationPageModalTransitionsPage.xaml");

impl NavigationPageModalTransitionsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            component_initialized: Cell::new(false),
            modal_count: Cell::new(0),
            initialized: Cell::new(false),
            modal_colors: MODAL_COLORS.iter().map(|color| SolidColorBrush::with_color(*color)).collect(),
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

    fn transition_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("TransitionCombo")
    }

    fn duration_slider(&self) -> Ref<Slider> {
        self.get_control::<Slider>("DurationSlider")
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    /// The field `DurationLabel`: null until `InitializeComponent()` has returned.
    fn duration_label(&self) -> Option<Ref<TextBlock>> {
        self.component_initialized.get().then(|| self.get_control::<TextBlock>("DurationLabel"))
    }

    /// The field `DemoNav` once the document is loaded.
    fn nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    /// `async void`.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if self.initialized.get() {
            self.update_transition();
            return;
        }

        self.initialized.set(true);
        let this = self.to_ref();
        drop(start_async(async move {
            let text = TextBlock::new();
            text.set_text(Some("Select a modal transition type and tap 'Open Modal'."));
            text.set_font_size(13.0);
            text.set_opacity(0.7);
            text.set_text_wrapping(TextWrapping::Wrap);
            text.set_text_alignment(TextAlignment::Center);
            text.set_max_width(260.0);
            text.set_horizontal_alignment(HorizontalAlignment::Center);
            text.set_vertical_alignment(VerticalAlignment::Center);

            let page = ContentPage::new();
            page.set_header(Some(boxed_text("Modal Transitions")));
            page.set_content(Some(Control::boxed(text)));
            page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
            page.set_vertical_content_alignment(VerticalAlignment::Stretch);
            if this.nav().push_async_with_transition(page, None).await.is_err() {
                return;
            }
            this.update_transition();
        }));
    }

    fn on_transition_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        self.update_transition();
    }

    fn on_duration_changed(&self, _sender: &Option<BoxedValue>, _e: &RangeBaseValueChangedEventArgs) {
        let Some(duration_label) = self.duration_label() else {
            return;
        };
        duration_label.set_text(Some(&format!("{} ms", self.duration_slider().value() as i32)));
        self.update_transition();
    }

    /// `async void`.
    fn on_open_modal(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            this.modal_count.set(this.modal_count.get() + 1);
            let modal_count = this.modal_count.get();

            let title = TextBlock::new();
            title.set_text(Some(&format!("Modal {modal_count}")));
            title.set_font_size(18.0);
            title.set_font_weight(FontWeight::SemiBold);
            title.set_horizontal_alignment(HorizontalAlignment::Center);

            let body = TextBlock::new();
            body.set_text(Some(&format!("Presented with {}.", this.get_transition_name())));
            body.set_font_size(13.0);
            body.set_opacity(0.7);
            body.set_text_wrapping(TextWrapping::Wrap);
            body.set_text_alignment(TextAlignment::Center);
            body.set_max_width(240.0);

            let panel = StackPanel::new();
            panel.set_horizontal_alignment(HorizontalAlignment::Center);
            panel.set_vertical_alignment(VerticalAlignment::Center);
            panel.set_spacing(8.0);
            panel.children().add(title);
            panel.children().add(body);

            let modal = ContentPage::new();
            modal.set_header(Some(boxed_text(&format!("Modal {modal_count}"))));
            let background = &this.modal_colors[modal_count as usize % this.modal_colors.len()];
            modal.set_background(Some(background.into()));
            modal.set_content(Some(Control::boxed(panel)));
            modal.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
            modal.set_vertical_content_alignment(VerticalAlignment::Stretch);

            if this.nav().push_modal_async(modal).await.is_err() {
                return;
            }
            this.status_text().set_text(Some(&format!("Modals: {}", this.nav().modal_stack().len())));
        }));
    }

    /// `async void`.
    fn on_pop_modal(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let this = self.to_ref();
        drop(start_async(async move {
            if this.nav().pop_modal_async().await.is_err() {
                return;
            }
            this.status_text().set_text(Some(&format!("Modals: {}", this.nav().modal_stack().len())));
        }));
    }

    fn update_transition(&self) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };

        let duration = TimeSpan::from_milliseconds(self.duration_slider().value());
        let transition: Option<Rc<dyn IPageTransition>> = match self.transition_combo().selected_index() {
            1 => Some(Rc::new(CrossFade::with_duration(duration))),
            2 => None,
            _ => Some(Rc::new(PageSlide::with_duration(duration, SlideAxis::Vertical))),
        };
        demo_nav.set_modal_transition(transition);
    }

    fn get_transition_name(&self) -> &'static str {
        match self.transition_combo().selected_index() {
            1 => "CrossFade",
            2 => "no transition",
            _ => "PageSlide (from Bottom)",
        }
    }
}
