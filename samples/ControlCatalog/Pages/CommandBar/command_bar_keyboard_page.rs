//! Port of `Pages/CommandBar/CommandBarKeyboardPage.xaml.cs`: the class of the document
//! `Pages/CommandBar/CommandBarKeyboardPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_base::input::{FocusChangedEventArgs, InputElement, NavigationMethod};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_controls::{CommandBar, CommandBarButton, CommandBarToggleButton, TextBlock, UserControl};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct CommandBarKeyboardPage {
    base: UserControl,
    log: RefCell<Vec<String>>,
    /// The subscriptions `OnLoaded` adds and `OnUnloaded` removes.
    opened: RefCell<Vec<RoutedEventHandlerToken>>,
    closed: RefCell<Vec<RoutedEventHandlerToken>>,
    item_focused: RefCell<Vec<(Ref<InputElement>, RoutedEventHandlerToken)>>,
}

user_control_class!(CommandBarKeyboardPage);
ferro_class_info!(CommandBarKeyboardPage {
    new: CommandBarKeyboardPage::new,
    markup: {
        methods: [
            fn OnOpenOverflow(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarKeyboardPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_open_overflow(&sender, e.as_routed_event_args())
                },
            fn OnClearLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarKeyboardPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_log(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CommandBarKeyboardPage, "/Pages/CommandBar/CommandBarKeyboardPage.xaml");

impl CommandBarKeyboardPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            log: RefCell::new(Vec::new()),
            opened: RefCell::new(Vec::new()),
            closed: RefCell::new(Vec::new()),
            item_focused: RefCell::new(Vec::new()),
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

    fn demo_bar(&self) -> Ref<CommandBar> {
        self.get_control::<CommandBar>("DemoBar")
    }

    fn focus_log_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("FocusLogText")
    }

    /// The elements whose focus is logged: `BtnCopy`, `BtnPaste`, `BtnBold`,
    /// `BtnShare`, `BtnDelete` and `BtnExport`.
    fn focus_items(&self) -> [Ref<InputElement>; 6] {
        [
            self.get_control::<CommandBarButton>("BtnCopy").upcast(),
            self.get_control::<CommandBarButton>("BtnPaste").upcast(),
            self.get_control::<CommandBarToggleButton>("BtnBold").upcast(),
            self.get_control::<CommandBarButton>("BtnShare").upcast(),
            self.get_control::<CommandBarButton>("BtnDelete").upcast(),
            self.get_control::<CommandBarButton>("BtnExport").upcast(),
        ]
    }

    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        // The handlers belong to children of the page: they hold the page weakly.
        let demo_bar = self.demo_bar();
        let weak = self.to_ref().downgrade();
        self.opened.borrow_mut().push(demo_bar.opened(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_opened(sender, e);
            }
        }));
        let weak = self.to_ref().downgrade();
        self.closed.borrow_mut().push(demo_bar.closed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_closed(sender, e);
            }
        }));
        for item in self.focus_items() {
            let weak = self.to_ref().downgrade();
            let token = item.add_handler(InputElement::got_focus_event(), move |sender, e: &FocusChangedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_item_focused(sender, e);
                }
            });
            self.item_focused.borrow_mut().push((item, token));
        }
    }

    fn on_unloaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        // One subscription of each handler is removed, as `-=` removes one.
        let demo_bar = self.demo_bar();
        let opened = self.opened.borrow_mut().pop();
        if let Some(token) = opened {
            demo_bar.remove_handler(CommandBar::opened_event(), token);
        }
        let closed = self.closed.borrow_mut().pop();
        if let Some(token) = closed {
            demo_bar.remove_handler(CommandBar::closed_event(), token);
        }
        for item in self.focus_items() {
            let subscription = {
                let mut subscriptions = self.item_focused.borrow_mut();
                let index = subscriptions.iter().rposition(|(subscribed, _)| subscribed.ptr_eq(&item));
                index.map(|index| subscriptions.remove(index))
            };
            if let Some((_, token)) = subscription {
                item.remove_handler(InputElement::got_focus_event(), token);
            }
        }
    }

    fn on_opened(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.append_log("Opened. Use arrow keys to navigate.");
    }

    fn on_closed(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.append_log("Closed");
    }

    fn on_item_focused(&self, sender: &Interactive, e: &FocusChangedEventArgs) {
        let sender = sender.to_ref();
        let label = if let Some(btn) = sender.cast::<CommandBarButton>() {
            btn.label().unwrap_or_else(|| String::from("(unnamed)"))
        } else if let Some(t) = sender.cast::<CommandBarToggleButton>() {
            t.label().unwrap_or_else(|| String::from("(unnamed)"))
        } else {
            sender.get_type().name().to_string()
        };

        let method = match e.navigation_method {
            NavigationMethod::Directional => "arrow key",
            NavigationMethod::Tab => "Tab",
            NavigationMethod::Pointer => "pointer",
            _ => "unspecified",
        };

        self.append_log(&format!("Focus: {label} ({method})"));
    }

    fn on_open_overflow(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.demo_bar().set_is_open(true);
    }

    fn on_clear_log(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.log.borrow_mut().clear();
        self.focus_log_text().set_text(Some("Log cleared."));
    }

    fn append_log(&self, message: &str) {
        let text = {
            let mut log = self.log.borrow_mut();
            log.push(message.to_string());
            if log.len() > 10 {
                log.remove(0);
            }
            let lines: Vec<String> = log.iter().enumerate().map(|(i, entry)| format!("{:>2}. {entry}", i + 1)).collect();
            lines.join("\n")
        };

        self.focus_log_text().set_text(Some(&text));
    }
}
