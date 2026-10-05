//! Port of `Pages/CommandBar/CommandBarEventsPage.xaml.cs`: the class of the document
//! `Pages/CommandBar/CommandBarEventsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, FerroPropertyChangedEventArgs, Ref};

use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::media::StreamGeometry;
use ferroui_base::reactive::IDisposable;
use ferroui_controls::{
    Button, CheckBox, CommandBar, CommandBarButton, CommandBarElementList, Control, ICommandBarElement, PathIcon,
    TextBlock, UserControl,
};
use mini_mvvm::MiniCommand;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// `item as CommandBarButton`.
fn as_button(item: &Rc<dyn ICommandBarElement>) -> Option<Ref<CommandBarButton>> {
    item.as_object().and_then(|object| object.to_ref().cast::<CommandBarButton>())
}

/// The text of a boolean, as the original prints it.
fn boolean_text(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

#[repr(C)]
pub struct CommandBarEventsPage {
    base: UserControl,
    log: RefCell<Vec<String>>,
    primary_count: Cell<i32>,
    secondary_count: Cell<i32>,
    /// The subscriptions `OnLoaded` adds and `OnUnloaded` removes, per event of the bar.
    bar_handlers: RefCell<Vec<[RoutedEventHandlerToken; 4]>>,
    bar_property_changed: RefCell<Vec<Rc<dyn IDisposable>>>,
    /// The subscription of `OnCommandItemClick` of each button that has one.
    item_clicks: RefCell<Vec<(Ref<CommandBarButton>, RoutedEventHandlerToken)>>,
}

user_control_class!(CommandBarEventsPage);
ferro_class_info!(CommandBarEventsPage {
    new: CommandBarEventsPage::new,
    markup: {
        methods: [
            fn OnIsOpenChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_is_open_changed(&sender, e.as_routed_event_args())
                },
            fn OnAddPrimary(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_primary(&sender, e.as_routed_event_args())
                },
            fn OnRemovePrimary(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_remove_primary(&sender, e.as_routed_event_args())
                },
            fn OnAddSecondary(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_add_secondary(&sender, e.as_routed_event_args())
                },
            fn OnRemoveSecondary(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_remove_secondary(&sender, e.as_routed_event_args())
                },
            fn OnClearLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarEventsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_log(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CommandBarEventsPage, "/Pages/CommandBar/CommandBarEventsPage.xaml");

impl CommandBarEventsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            log: RefCell::new(Vec::new()),
            primary_count: Cell::new(3),
            secondary_count: Cell::new(2),
            bar_handlers: RefCell::new(Vec::new()),
            bar_property_changed: RefCell::new(Vec::new()),
            item_clicks: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers of the events of the page itself hold it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        let weak = this.downgrade();
        this.unloaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_unloaded(sender, e);
            }
        });
        this
    }

    fn is_open_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("IsOpenCheck")
    }

    fn state_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StateText")
    }

    fn demo_bar(&self) -> Ref<CommandBar> {
        self.get_control::<CommandBar>("DemoBar")
    }

    fn event_log_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("EventLogText")
    }

    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        // The handlers belong to children of the page: they hold the page weakly.
        let demo_bar = self.demo_bar();
        let handler = |handler: fn(&CommandBarEventsPage, &Interactive, &RoutedEventArgs)| {
            let weak = self.to_ref().downgrade();
            move |sender: &Interactive, e: &RoutedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    handler(&this, sender, e);
                }
            }
        };
        self.bar_handlers.borrow_mut().push([
            demo_bar.opening(handler(Self::on_opening)),
            demo_bar.opened(handler(Self::on_opened)),
            demo_bar.closing(handler(Self::on_closing)),
            demo_bar.closed(handler(Self::on_closed)),
        ]);
        let weak = self.to_ref().downgrade();
        self.bar_property_changed.borrow_mut().push(demo_bar.property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.on_bar_property_changed(e);
            }
        }));

        self.attach_item_handlers(&demo_bar.primary_commands());
        self.attach_item_handlers(&demo_bar.secondary_commands());

        self.append_log("Ready");
        self.refresh_state();
    }

    fn on_unloaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        // One subscription of each handler is removed, as `-=` removes one.
        let demo_bar = self.demo_bar();
        let handlers = self.bar_handlers.borrow_mut().pop();
        if let Some([opening, opened, closing, closed]) = handlers {
            demo_bar.remove_handler(CommandBar::opening_event(), opening);
            demo_bar.remove_handler(CommandBar::opened_event(), opened);
            demo_bar.remove_handler(CommandBar::closing_event(), closing);
            demo_bar.remove_handler(CommandBar::closed_event(), closed);
        }
        let property_changed = self.bar_property_changed.borrow_mut().pop();
        if let Some(property_changed) = property_changed {
            property_changed.dispose();
        }

        self.detach_item_handlers(&demo_bar.primary_commands());
        self.detach_item_handlers(&demo_bar.secondary_commands());
    }

    fn on_is_open_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.demo_bar().set_is_open(self.is_open_check().is_checked() == Some(true));
        self.refresh_state();
    }

    fn on_add_primary(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.primary_count.set(self.primary_count.get() + 1);
        let button = self.create_button(&format!("Primary {}", self.primary_count.get()));
        self.demo_bar().primary_commands().add(button.as_command_bar_element());
        self.append_log(&format!("Primary +, {}", self.demo_bar().primary_commands().count()));
        self.refresh_state();
    }

    fn on_remove_primary(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.remove_last_command(&self.demo_bar().primary_commands(), "Primary");
    }

    fn on_add_secondary(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.secondary_count.set(self.secondary_count.get() + 1);
        let button = self.create_button(&format!("Secondary {}", self.secondary_count.get()));
        self.demo_bar().secondary_commands().add(button.as_command_bar_element());
        self.append_log(&format!("Secondary +, {}", self.demo_bar().secondary_commands().count()));
        self.refresh_state();
    }

    fn on_remove_secondary(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.remove_last_command(&self.demo_bar().secondary_commands(), "Secondary");
    }

    fn on_clear_log(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.log.borrow_mut().clear();
        self.event_log_text().set_text(Some("Log cleared"));
    }

    fn on_opening(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.append_log("Opening");
        self.refresh_state();
    }

    fn on_opened(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.append_log("Opened");
        self.refresh_state();
    }

    fn on_closing(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.append_log("Closing");
        self.refresh_state();
    }

    fn on_closed(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.append_log("Closed");
        self.refresh_state();
    }

    fn on_bar_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == CommandBar::is_open_property().as_property()
            || e.property() == CommandBar::has_secondary_commands_property().as_property()
            || e.property() == CommandBar::is_overflow_button_visible_property().as_property()
        {
            self.refresh_state();
        }
    }

    fn refresh_state(&self) {
        let demo_bar = self.demo_bar();
        self.state_text().set_text(Some(&format!(
            "IsOpen: {}\nHasSecondaryCommands: {}\nIsOverflowButtonVisible: {}\nPrimary: {}\nSecondary: {}\nOverflowItems: {}",
            boolean_text(demo_bar.is_open()),
            boolean_text(demo_bar.has_secondary_commands()),
            boolean_text(demo_bar.is_overflow_button_visible()),
            demo_bar.primary_commands().count(),
            demo_bar.secondary_commands().count(),
            demo_bar.overflow_items().count()
        )));

        self.is_open_check().set_is_checked(Some(demo_bar.is_open()));
    }

    fn on_command_item_click(&self, sender: &Interactive, _e: &RoutedEventArgs) {
        if let Some(button) = sender.to_ref().cast::<CommandBarButton>() {
            self.append_log(&format!(
                "Click, {}, {}",
                button.label().unwrap_or_default(),
                Self::describe_placement(&button)
            ));
        }
    }

    fn create_button(&self, label: &str) -> Ref<CommandBarButton> {
        let button = CommandBarButton::new();
        button.set_label(Some(label));
        let icon = PathIcon::new();
        match StreamGeometry::parse("M19,13H13V19H11V13H5V11H11V5H13V11H19V13Z") {
            Ok(data) => icon.set_data(data),
            Err(error) => panic!("{error}"),
        }
        button.set_icon(Some(Control::boxed(icon)));

        self.attach_item_handler(&button.as_command_bar_element());
        button
    }

    fn attach_item_handlers(&self, items: &CommandBarElementList) {
        for item in items.snapshot().iter() {
            self.attach_item_handler(item);
        }
    }

    fn detach_item_handlers(&self, items: &CommandBarElementList) {
        for item in items.snapshot().iter() {
            if let Some(button) = as_button(item) {
                self.remove_item_click(&button);
            }
        }
    }

    fn attach_item_handler(&self, item: &Rc<dyn ICommandBarElement>) {
        let Some(button) = as_button(item) else {
            return;
        };

        self.remove_item_click(&button);
        // The handler and the command belong to a child of the page: they hold the page and the button weakly.
        let weak = self.to_ref().downgrade();
        let token = button.click(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_command_item_click(sender, e);
            }
        });
        self.item_clicks.borrow_mut().push((button.clone(), token));

        let weak = self.to_ref().downgrade();
        let weak_button = button.downgrade();
        button.set_command(Some(
            MiniCommand::create(move || {
                if let (Some(this), Some(button)) = (weak.upgrade(), weak_button.upgrade()) {
                    this.append_log(&format!(
                        "Command, {}, {}",
                        button.label().unwrap_or_default(),
                        Self::describe_placement(&button)
                    ));
                }
            })
            .as_command(),
        ));
    }

    /// `button.Click -= OnCommandItemClick`.
    fn remove_item_click(&self, button: &Ref<CommandBarButton>) {
        let subscription = {
            let mut subscriptions = self.item_clicks.borrow_mut();
            let index = subscriptions.iter().rposition(|(subscribed, _)| subscribed.ptr_eq(button));
            index.map(|index| subscriptions.remove(index))
        };
        if let Some((_, token)) = subscription {
            button.remove_handler(Button::click_event(), token);
        }
    }

    fn remove_last_command(&self, items: &CommandBarElementList, bucket_name: &str) {
        if items.count() == 0 {
            return;
        }

        let item = items.get(items.count() - 1);
        let button = as_button(&item);
        let label = match &button {
            Some(button) => button.label().unwrap_or_else(|| String::from("(unnamed)")),
            None => item.as_object().map_or_else(String::new, |object| object.get_type().name().to_string()),
        };

        if let Some(command_bar_button) = &button {
            self.remove_item_click(command_bar_button);
        }

        items.remove_at(items.count() - 1);
        self.append_log(&format!("{bucket_name} -, {label}, {}", items.count()));
        self.refresh_state();
    }

    fn describe_placement(button: &CommandBarButton) -> &'static str {
        if button.is_in_overflow() { "overflow" } else { "primary" }
    }

    fn append_log(&self, message: &str) {
        let text = {
            let mut log = self.log.borrow_mut();
            log.push(message.to_string());
            if log.len() > 12 {
                log.remove(0);
            }
            let lines: Vec<String> =
                log.iter().enumerate().map(|(index, entry)| format!("{:>2}. {entry}", index + 1)).collect();
            lines.join("\n")
        };

        self.event_log_text().set_text(Some(&text));
    }
}
