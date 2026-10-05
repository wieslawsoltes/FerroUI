//! Port of `Pages/CommandBar/CommandBarDynamicOverflowPage.xaml.cs`: the class of the document
//! `Pages/CommandBar/CommandBarDynamicOverflowPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    Border, CheckBox, CommandBar, CommandBarButton, CommandBarSeparator, ICommandBarElement, Slider, TextBlock,
    UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct CommandBarDynamicOverflowPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    initialized: Cell<bool>,
}

user_control_class!(CommandBarDynamicOverflowPage);
ferro_class_info!(CommandBarDynamicOverflowPage {
    new: CommandBarDynamicOverflowPage::new,
    markup: {
        methods: [
            fn OnWidthChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarDynamicOverflowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_width_changed(&sender, e.as_routed_event_args())
                },
            fn OnDynamicOverflowChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarDynamicOverflowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_dynamic_overflow_changed(&sender, e.as_routed_event_args())
                },
            fn OnSecondaryVisibilityChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarDynamicOverflowPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_secondary_visibility_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CommandBarDynamicOverflowPage, "/Pages/CommandBar/CommandBarDynamicOverflowPage.xaml");

impl CommandBarDynamicOverflowPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.initialized.set(true);

        let demo_bar = this.get_control::<CommandBar>("DemoBar");
        if this.get_control::<CheckBox>("SecondaryVisibleCheck").is_checked() != Some(true) {
            let command = this.get_control::<CommandBarButton>("DemoSecondaryCommand").as_command_bar_element();
            demo_bar.secondary_commands().remove(&command);
        }

        // The collections belong to a child of the page: their handlers hold the page weakly.
        let weak = this.downgrade();
        demo_bar.overflow_items().add_collection_changed(Rc::new(move |_| {
            if let Some(this) = weak.upgrade() {
                this.on_overflow_changed();
            }
        }));
        let weak = this.downgrade();
        demo_bar.visible_primary_commands().add_collection_changed(Rc::new(move |_| {
            if let Some(this) = weak.upgrade() {
                this.on_overflow_changed();
            }
        }));

        this.update_status();
        this
    }

    fn width_slider(&self) -> Option<Ref<Slider>> {
        if self.initialized.get() { self.find_control::<Slider>("WidthSlider") } else { None }
    }

    fn width_label(&self) -> Option<Ref<TextBlock>> {
        if self.initialized.get() { self.find_control::<TextBlock>("WidthLabel") } else { None }
    }

    fn dynamic_overflow_check(&self) -> Option<Ref<CheckBox>> {
        if self.initialized.get() { self.find_control::<CheckBox>("DynamicOverflowCheck") } else { None }
    }

    fn secondary_visible_check(&self) -> Option<Ref<CheckBox>> {
        if self.initialized.get() { self.find_control::<CheckBox>("SecondaryVisibleCheck") } else { None }
    }

    fn bar_container(&self) -> Option<Ref<Border>> {
        if self.initialized.get() { self.find_control::<Border>("BarContainer") } else { None }
    }

    fn demo_bar(&self) -> Option<Ref<CommandBar>> {
        if self.initialized.get() { self.find_control::<CommandBar>("DemoBar") } else { None }
    }

    fn demo_secondary_command(&self) -> Option<Ref<CommandBarButton>> {
        if self.initialized.get() { self.find_control::<CommandBarButton>("DemoSecondaryCommand") } else { None }
    }

    fn on_width_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(bar_container), Some(width_slider), Some(width_label)) =
            (self.bar_container(), self.width_slider(), self.width_label())
        else {
            return;
        };

        // `(int)value`: the fraction is dropped.
        let width = width_slider.value() as i32;
        bar_container.set_width(f64::from(width));
        width_label.set_text(Some(&width.to_string()));
    }

    fn on_dynamic_overflow_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(demo_bar), Some(dynamic_overflow_check)) = (self.demo_bar(), self.dynamic_overflow_check()) else {
            return;
        };

        demo_bar.set_is_dynamic_overflow_enabled(dynamic_overflow_check.is_checked() == Some(true));
    }

    fn on_secondary_visibility_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let (Some(demo_bar), Some(demo_secondary_command), Some(secondary_visible_check)) =
            (self.demo_bar(), self.demo_secondary_command(), self.secondary_visible_check())
        else {
            return;
        };

        let command = demo_secondary_command.as_command_bar_element();
        let should_include = secondary_visible_check.is_checked() == Some(true);
        let is_included = demo_bar.secondary_commands().contains(&command);

        if should_include && !is_included {
            demo_bar.secondary_commands().add(command);
        } else if !should_include && is_included {
            demo_bar.secondary_commands().remove(&command);
        }

        self.update_status();
    }

    fn on_overflow_changed(&self) {
        self.update_status();
    }

    fn update_status(&self) {
        let demo_bar = self.get_control::<CommandBar>("DemoBar");
        let is_separator = |item: &Rc<dyn ICommandBarElement>| {
            item.as_object().is_some_and(|object| object.is::<CommandBarSeparator>())
        };

        let mut visible_primary_command_count = 0;
        let mut visible_primary_separator_count = 0;

        for item in demo_bar.visible_primary_commands().snapshot().iter() {
            if is_separator(item) {
                visible_primary_separator_count += 1;
            } else {
                visible_primary_command_count += 1;
            }
        }

        let mut overflow_command_count = 0;
        let mut overflow_separator_count = 0;
        let mut has_synthetic_overflow_divider = false;

        for item in demo_bar.overflow_items().snapshot().iter() {
            if is_separator(item) {
                overflow_separator_count += 1;

                if !demo_bar.primary_commands().contains(item) && !demo_bar.secondary_commands().contains(item) {
                    has_synthetic_overflow_divider = true;
                }
            } else {
                overflow_command_count += 1;
            }
        }

        self.get_control::<TextBlock>("StatusText").set_text(Some(&format!(
            "Visible primary: {visible_primary_command_count} commands, {visible_primary_separator_count} separators\n\
             Overflow items: {overflow_command_count} commands, {overflow_separator_count} separators\n\
             Synthetic overflow divider: {}",
            if has_synthetic_overflow_divider { "present" } else { "absent" }
        )));
    }
}
