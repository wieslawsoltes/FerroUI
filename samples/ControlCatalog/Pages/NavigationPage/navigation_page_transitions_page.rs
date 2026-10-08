//! Port of `Pages/NavigationPage/NavigationPageTransitionsPage.xaml.cs`: the class of the document
//! `Pages/NavigationPage/NavigationPageTransitionsPage.xaml`.

use super::transitions::{
    CompositeTransition, FadeThroughTransition, PageSlideTransition, PageSlideTransitionAxis, ParallaxSlideTransition,
};
use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::NavigationDemoHelper;
use ferroui_base::animation::{CrossFade, IPageTransition, PageSlide, SlideAxis, TimeSpan};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::primitives::RangeBaseValueChangedEventArgs;
use ferroui_controls::{ComboBox, NavigationPage, SelectionChangedEventArgs, Slider, TextBlock, UserControl};
use mini_mvvm::start_async;
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct NavigationPageTransitionsPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    initialized: Cell<bool>,
    page_count: Cell<i32>,
}

user_control_class!(NavigationPageTransitionsPage);
ferro_class_info!(NavigationPageTransitionsPage {
    new: NavigationPageTransitionsPage::new,
    markup: {
        methods: [
            fn OnTransitionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_transition_changed(&sender, e)
                    }
                },
            fn OnDurationChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<RangeBaseValueChangedEventArgs>() {
                        this.on_duration_changed(&sender, e)
                    }
                },
            fn OnPush(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_push(&sender, e.as_routed_event_args())
                },
            fn OnPop(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<NavigationPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_pop(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(NavigationPageTransitionsPage, "/Pages/NavigationPage/NavigationPageTransitionsPage.xaml");

impl NavigationPageTransitionsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            component_initialized: Cell::new(false),
            initialized: Cell::new(false),
            page_count: Cell::new(0),
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

    /// The field `DemoNav` once the document is loaded.
    fn nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("DemoNav")
    }

    fn transition_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("TransitionCombo")
    }

    fn duration_slider(&self) -> Ref<Slider> {
        self.get_control::<Slider>("DurationSlider")
    }

    /// The field `DurationLabel`: null until `InitializeComponent()` has returned.
    fn duration_label(&self) -> Option<Ref<TextBlock>> {
        self.component_initialized.get().then(|| self.get_control::<TextBlock>("DurationLabel"))
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
            let page = NavigationDemoHelper::make_page("Transitions", "Choose a transition type and push pages.", 0);
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

    /// `async void`: nothing follows the push.
    fn on_push(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.page_count.set(self.page_count.get() + 1);
        let page_count = self.page_count.get();
        let page = NavigationDemoHelper::make_page(
            &format!("Page {page_count}"),
            &format!("Pushed with {}.", self.get_transition_name()),
            page_count,
        );
        drop(self.nav().push_async(page));
    }

    /// `async void`: nothing follows the pop.
    fn on_pop(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        drop(self.nav().pop_async());
    }

    fn update_transition(&self) {
        let Some(demo_nav) = self.demo_nav() else {
            return;
        };
        let duration = TimeSpan::from_milliseconds(self.duration_slider().value());
        let transition: Option<Rc<dyn IPageTransition>> = match self.transition_combo().selected_index() {
            1 => Some(Rc::new(PageSlide::with_duration(duration, SlideAxis::Horizontal))),
            3 => Some(Rc::new(ParallaxSlideTransition::with_duration(duration))),
            4 => Some(Rc::new(CrossFade::with_duration(duration))),
            5 => Some(Rc::new(FadeThroughTransition::with_duration(duration))),
            6 => Some(Rc::new(PageSlideTransition::with_duration(duration, PageSlideTransitionAxis::Horizontal))),
            7 => Some(Rc::new(PageSlideTransition::with_duration(duration, PageSlideTransitionAxis::Vertical))),
            8 => Some(Rc::new(CompositeTransition::with_duration(duration))),
            9 => None,
            _ => Some(Rc::new(PageSlide::with_duration(duration, SlideAxis::Horizontal))),
        };
        demo_nav.set_page_transition(transition);
    }

    fn get_transition_name(&self) -> &'static str {
        match self.transition_combo().selected_index() {
            1 => "Page Slide",
            3 => "Parallax Slide",
            4 => "Cross Fade",
            5 => "Fade Through",
            6 => "Page Slide (Horizontal)",
            7 => "Page Slide (Vertical)",
            8 => "Composite (Slide + Fade)",
            9 => "no transition",
            _ => "Page Slide",
        }
    }
}
