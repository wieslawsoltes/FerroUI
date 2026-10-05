//! Port of `Pages/TabbedPage/TabbedPageTransitionsPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageTransitionsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::animation::{CompositePageTransition, CrossFade, IPageTransition, PageSlide, SlideAxis, TimeSpan};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ComboBox, TabPlacement, TabbedPage, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct TabbedPageTransitionsPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
}

user_control_class!(TabbedPageTransitionsPage);
ferro_class_info!(TabbedPageTransitionsPage {
    new: TabbedPageTransitionsPage::new,
    markup: {
        methods: [
            fn OnTransitionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_transition_changed(&sender, e.as_routed_event_args())
                },
            fn OnPlacementChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_placement_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TabbedPageTransitionsPage, "/Pages/TabbedPage/TabbedPageTransitionsPage.xaml");

impl TabbedPageTransitionsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);
        this
    }

    /// The field `DemoTabs`: null until `InitializeComponent()` has returned.
    fn demo_tabs(&self) -> Option<Ref<TabbedPage>> {
        self.component_initialized.get().then(|| self.get_control::<TabbedPage>("DemoTabs"))
    }

    /// `combo?.SelectedIndex` of a named combo box.
    fn selected_index(&self, name: &str) -> i32 {
        self.get_control::<ComboBox>(name).selected_index()
    }

    fn on_transition_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(demo_tabs) = self.demo_tabs() else {
            return;
        };

        let transition: Option<Rc<dyn IPageTransition>> = match self.selected_index("TransitionCombo") {
            1 => Some(Rc::new(CrossFade::with_duration(TimeSpan::from_milliseconds(250.0)))),
            2 => Some(Rc::new(PageSlide::with_duration(TimeSpan::from_milliseconds(300.0), SlideAxis::Horizontal))),
            3 => Some(Rc::new(PageSlide::with_duration(TimeSpan::from_milliseconds(300.0), SlideAxis::Vertical))),
            4 => {
                let composite = CompositePageTransition::new();
                composite.add(Rc::new(CrossFade::with_duration(TimeSpan::from_milliseconds(250.0))));
                composite.add(Rc::new(PageSlide::with_duration(
                    TimeSpan::from_milliseconds(300.0),
                    SlideAxis::Horizontal,
                )));
                Some(Rc::new(composite))
            }
            _ => None,
        };
        demo_tabs.set_page_transition(transition);
    }

    fn on_placement_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(demo_tabs) = self.demo_tabs() else {
            return;
        };
        demo_tabs.set_tab_placement(match self.selected_index("PlacementCombo") {
            1 => TabPlacement::Bottom,
            2 => TabPlacement::Left,
            3 => TabPlacement::Right,
            _ => TabPlacement::Top,
        });
    }
}
