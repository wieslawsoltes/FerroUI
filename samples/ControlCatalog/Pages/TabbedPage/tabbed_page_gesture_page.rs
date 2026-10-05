//! Port of `Pages/TabbedPage/TabbedPageGesturePage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageGesturePage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    CheckBox, ComboBox, Control, SelectionChangedEventArgs, TabPlacement, TabbedPage, UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct TabbedPageGesturePage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
}

user_control_class!(TabbedPageGesturePage);
ferro_class_info!(TabbedPageGesturePage {
    new: TabbedPageGesturePage::new,
    markup: {
        methods: [
            fn OnGestureEnabledChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageGesturePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_gesture_enabled_changed(&sender, e.as_routed_event_args())
                },
            fn OnPlacementChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageGesturePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_placement_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(TabbedPageGesturePage, "/Pages/TabbedPage/TabbedPageGesturePage.xaml");

impl TabbedPageGesturePage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);
        Self::enable_mouse_swipe_gesture(&this.get_control::<TabbedPage>("DemoTabs"));
        this
    }

    fn gesture_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("GestureCheck")
    }

    fn placement_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("PlacementCombo")
    }

    /// The field `DemoTabs`: null until `InitializeComponent()` has returned.
    fn demo_tabs(&self) -> Option<Ref<TabbedPage>> {
        self.component_initialized.get().then(|| self.get_control::<TabbedPage>("DemoTabs"))
    }

    fn on_gesture_enabled_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(demo_tabs) = self.demo_tabs() {
            demo_tabs.set_is_gesture_enabled(self.gesture_check().is_checked() == Some(true));
        }
    }

    fn on_placement_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(demo_tabs) = self.demo_tabs() else {
            return;
        };
        demo_tabs.set_tab_placement(match self.placement_combo().selected_index() {
            1 => TabPlacement::Bottom,
            2 => TabPlacement::Left,
            3 => TabPlacement::Right,
            _ => TabPlacement::Top,
        });
    }

    fn enable_mouse_swipe_gesture(control: &Control) {
        let recognizer = control
            .gesture_recognizers()
            .to_vec()
            .into_iter()
            .find_map(|recognizer| recognizer.cast::<SwipeGestureRecognizer>());
        if let Some(recognizer) = recognizer {
            recognizer.set_is_mouse_enabled(true);
        }
    }
}
