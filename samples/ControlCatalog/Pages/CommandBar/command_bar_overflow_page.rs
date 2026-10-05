//! Port of `Pages/CommandBar/CommandBarOverflowPage.xaml.cs`: the class of the document
//! `Pages/CommandBar/CommandBarOverflowPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    CheckBox, ComboBox, CommandBar, CommandBarButton, CommandBarOverflowButtonVisibility, UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct CommandBarOverflowPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    initialized: Cell<bool>,
    primary_count: Cell<i32>,
    secondary_count: Cell<i32>,
}

user_control_class!(CommandBarOverflowPage);
ferro_class_info!(CommandBarOverflowPage {
    new: CommandBarOverflowPage::new,
    markup: {
        methods: [
            fn OnOverflowVisChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarOverflowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_overflow_vis_changed(&sender, e.as_routed_event_args())
                },
            fn OnIsOpenChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarOverflowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_is_open_changed(&sender, e.as_routed_event_args())
                },
            fn OnIsStickyChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarOverflowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_is_sticky_changed(&sender, e.as_routed_event_args())
                },
            fn OnAddPrimary(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarOverflowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_primary(&sender, e.as_routed_event_args())
                },
            fn OnAddSecondary(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarOverflowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_secondary(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CommandBarOverflowPage, "/Pages/CommandBar/CommandBarOverflowPage.xaml");

impl CommandBarOverflowPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            initialized: Cell::new(false),
            primary_count: Cell::new(0),
            secondary_count: Cell::new(0),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.initialized.set(true);
        this
    }

    fn overflow_vis_combo(&self) -> Option<Ref<ComboBox>> {
        if self.initialized.get() { self.find_control::<ComboBox>("OverflowVisCombo") } else { None }
    }

    fn is_open_check(&self) -> Option<Ref<CheckBox>> {
        if self.initialized.get() { self.find_control::<CheckBox>("IsOpenCheck") } else { None }
    }

    fn is_sticky_check(&self) -> Option<Ref<CheckBox>> {
        if self.initialized.get() { self.find_control::<CheckBox>("IsStickyCheck") } else { None }
    }

    fn demo_bar(&self) -> Option<Ref<CommandBar>> {
        if self.initialized.get() { self.find_control::<CommandBar>("DemoBar") } else { None }
    }

    fn on_overflow_vis_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(demo_bar), Some(overflow_vis_combo)) = (self.demo_bar(), self.overflow_vis_combo()) else {
            return;
        };

        demo_bar.set_overflow_button_visibility(match overflow_vis_combo.selected_index() {
            1 => CommandBarOverflowButtonVisibility::Visible,
            2 => CommandBarOverflowButtonVisibility::Collapsed,
            _ => CommandBarOverflowButtonVisibility::Auto,
        });
    }

    fn on_is_open_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(demo_bar), Some(is_open_check)) = (self.demo_bar(), self.is_open_check()) else {
            return;
        };

        demo_bar.set_is_open(is_open_check.is_checked() == Some(true));
    }

    fn on_is_sticky_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(demo_bar), Some(is_sticky_check)) = (self.demo_bar(), self.is_sticky_check()) else {
            return;
        };

        demo_bar.set_is_sticky(is_sticky_check.is_checked() == Some(true));
    }

    fn on_add_primary(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.primary_count.set(self.primary_count.get() + 1);
        let button = CommandBarButton::new();
        button.set_label(Some(&format!("Cmd {}", self.primary_count.get())));
        self.get_control::<CommandBar>("DemoBar").primary_commands().add(button.as_command_bar_element());
    }

    fn on_add_secondary(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.secondary_count.set(self.secondary_count.get() + 1);
        let button = CommandBarButton::new();
        button.set_label(Some(&format!("Sec {}", self.secondary_count.get())));
        self.get_control::<CommandBar>("DemoBar").secondary_commands().add(button.as_command_bar_element());
    }
}
