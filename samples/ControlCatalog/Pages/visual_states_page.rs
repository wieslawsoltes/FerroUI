//! Not from upstream: the page of WinUI's visual state manager
//! (docs/porting/winui-calculator-gaps.md), the class of the document
//! `Pages/VisualStatesPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ContentPage, Control, StackPanel, TextBlock, VisualStateManager};
use std::rc::Rc;

#[repr(C)]
pub struct VisualStatesPage {
    base: ContentPage,
}

content_page_class!(VisualStatesPage);
ferro_class_info!(VisualStatesPage {
    new: VisualStatesPage::new,
    markup: {
        methods: [
            fn GoToState_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<VisualStatesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.go_to_state_on_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(VisualStatesPage, "/Pages/VisualStatesPage.xaml");

impl VisualStatesPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// Goes to the state named by the `Tag` of the button.
    fn go_to_state_on_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let state = sender
            .as_ref()
            .and_then(Control::from_boxed)
            .and_then(|button| button.tag())
            .and_then(|tag| tag.downcast_ref::<String>().cloned());
        if let Some(state) = state {
            VisualStateManager::go_to_state(self, &state, true);
        }
        self.show_current_states();
    }

    fn show_current_states(&self) {
        let root = self.get_control::<StackPanel>("LayoutRoot");
        let groups = VisualStateManager::get_visual_state_groups(&root);
        let text = groups
            .snapshot()
            .iter()
            .map(|group| {
                let state = group.current_state().map(|state| state.name()).unwrap_or_else(|| "(none)".to_string());
                format!("{}: {state}", group.name())
            })
            .collect::<Vec<_>>()
            .join(", ");
        self.get_control::<TextBlock>("CurrentStates").set_text(Some(&text));
    }
}
