//! Port of `Pages/TabbedPage/TabbedPageKeyboardPage.xaml.cs`: the class of the document
//! `Pages/TabbedPage/TabbedPageKeyboardPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::value_text;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    CheckBox, ComboBox, ContentPage, PageSelectionChangedEventArgs, SelectionChangedEventArgs, TabPlacement,
    TabbedPage, TextBlock, UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct TabbedPageKeyboardPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
}

user_control_class!(TabbedPageKeyboardPage);
ferro_class_info!(TabbedPageKeyboardPage {
    new: TabbedPageKeyboardPage::new,
    markup: {
        methods: [
            fn OnPlacementChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageKeyboardPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_placement_changed(&sender, e)
                    }
                },
            fn OnKeyboardChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageKeyboardPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_keyboard_changed(&sender, e.as_routed_event_args())
                },
            fn OnSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TabbedPageKeyboardPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PageSelectionChangedEventArgs>() {
                        this.on_selection_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(TabbedPageKeyboardPage, "/Pages/TabbedPage/TabbedPageKeyboardPage.xaml");

impl TabbedPageKeyboardPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);
        this
    }

    /// A named text block: null until `InitializeComponent()` has returned.
    fn text(&self, name: &str) -> Option<Ref<TextBlock>> {
        self.component_initialized.get().then(|| self.get_control::<TextBlock>(name))
    }

    /// The field `DemoTabs`: null until `InitializeComponent()` has returned.
    fn demo_tabs(&self) -> Option<Ref<TabbedPage>> {
        self.component_initialized.get().then(|| self.get_control::<TabbedPage>("DemoTabs"))
    }

    fn on_placement_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(demo_tabs) = self.demo_tabs() else {
            return;
        };
        demo_tabs.set_tab_placement(match self.get_control::<ComboBox>("PlacementCombo").selected_index() {
            1 => TabPlacement::Bottom,
            2 => TabPlacement::Left,
            3 => TabPlacement::Right,
            _ => TabPlacement::Top,
        });
        self.update_arrow_key_labels();
    }

    fn update_arrow_key_labels(&self) {
        let Some(arrow_keys_header) = self.text("ArrowKeysHeader") else {
            return;
        };
        let placement = self.get_control::<TabbedPage>("DemoTabs").tab_placement();
        let vertical = matches!(placement, TabPlacement::Left | TabPlacement::Right);
        arrow_keys_header.set_text(Some(if vertical { "Left / Right placement" } else { "Top / Bottom placement" }));
        self.get_control::<TextBlock>("NextKeyText").set_text(Some(if vertical { "\u{2193}" } else { "\u{2192}" }));
        self.get_control::<TextBlock>("PrevKeyText").set_text(Some(if vertical { "\u{2191}" } else { "\u{2190}" }));
    }

    fn on_keyboard_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(demo_tabs) = self.demo_tabs() {
            demo_tabs.set_is_keyboard_navigation_enabled(
                self.get_control::<CheckBox>("KeyboardCheck").is_checked() == Some(true),
            );
        }
    }

    fn on_selection_changed(&self, _sender: &Option<BoxedValue>, e: &PageSelectionChangedEventArgs) {
        if let Some(status_text) = self.text("StatusText") {
            let header = e
                .current_page()
                .and_then(|page| page.cast::<ContentPage>())
                .and_then(|page| value_text(&page.header()))
                .unwrap_or_default();
            status_text.set_text(Some(&format!(
                "Selected: {header} ({})",
                self.get_control::<TabbedPage>("DemoTabs").selected_index()
            )));
        }
    }
}
