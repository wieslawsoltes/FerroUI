//! Port of `Pages/ButtonsPage.xaml.cs`: the class of the document
//! `Pages/ButtonsPage.xaml`.

use crate::controls::{sample_page_class, SamplePage};
use crate::markup::xaml_class;
use ferroui_base::input::ICommand;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Button, TextBlock};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

#[repr(C)]
pub struct ButtonsPage {
    base: SamplePage,
    click_count: Cell<i32>,
    repeat_button_click_count: Cell<i32>,
    command_count: Cell<i32>,
    can_count: Cell<bool>,
    count_command: RefCell<Option<Rc<RelayCommand>>>,
}

sample_page_class!(ButtonsPage);
ferro_class_info!(ButtonsPage {
    new: ButtonsPage::new,
    markup: {
        properties: [
            CountCommand: Option<Rc<dyn ICommand>> {
                get: |this: &Ref<ButtonsPage>| this.count_command().map(|command| command as Rc<dyn ICommand>)
            },
            CanCount: bool {
                get: |this: &Ref<ButtonsPage>| this.can_count(),
                set: |this: &Ref<ButtonsPage>, value: bool| this.set_can_count(value)
            },
        ],
        methods: [
            fn OnDemoButtonClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ButtonsPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.on_demo_button_click(&sender, args.as_routed_event_args())
                },
            fn OnModeButtonClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ButtonsPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.on_mode_button_click(&sender, args.as_routed_event_args())
                },
            fn OnHotKeyButtonClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ButtonsPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.on_hot_key_button_click(&sender, args.as_routed_event_args())
                },
            fn OnRepeatButtonClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ButtonsPage>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.on_repeat_button_click(&sender, args.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ButtonsPage, "/Pages/ButtonsPage.xaml");

/// `time` or `times`: the plural suffix of a count.
fn plural(count: i32) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

impl ButtonsPage {
    pub fn construct() -> Self {
        Self {
            base: SamplePage::construct(),
            click_count: Cell::new(0),
            repeat_button_click_count: Cell::new(0),
            command_count: Cell::new(0),
            can_count: Cell::new(true),
            count_command: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        let execute = {
            let weak = this.downgrade();
            move || {
                let Some(this) = weak.upgrade() else { return };
                this.command_count.set(this.command_count.get() + 1);
                let count = this.command_count.get();
                this.mode_status().set_text(Some(&format!("Command executed {count} time{}.", plural(count))));
            }
        };
        let can_execute = {
            let weak = this.downgrade();
            move || weak.upgrade().is_some_and(|this| this.can_count())
        };
        *this.count_command.borrow_mut() = Some(RelayCommand::new(execute, can_execute));
        this.set_data_context(Some(Rc::new(this.clone()) as BoxedValue));
        this
    }

    fn click_status(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("ClickStatus")
    }

    fn mode_status(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("ModeStatus")
    }

    fn mode_button(&self) -> Ref<Button> {
        self.get_control::<Button>("ModeButton")
    }

    fn repeat_button_text_block(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("RepeatButtonTextBlock")
    }

    /// The command of the page; `None` until the constructor has created it.
    pub fn count_command(&self) -> Option<Rc<RelayCommand>> {
        self.count_command.borrow().clone()
    }

    pub fn can_count(&self) -> bool {
        self.can_count.get()
    }

    /// # Panics
    /// Panics if the command does not exist yet (a null reference in the
    /// managed original).
    pub fn set_can_count(&self, value: bool) {
        self.can_count.set(value);
        self.count_command().expect("the count command").raise_can_execute_changed();
    }

    fn on_demo_button_click(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        self.click_count.set(self.click_count.get() + 1);
        let count = self.click_count.get();
        self.click_status().set_text(Some(&format!("Click raised {count} time{}.", plural(count))));
    }

    fn on_mode_button_click(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        let click_mode = self.mode_button().click_mode();
        self.mode_status().set_text(Some(&format!("Click raised with ClickMode.{click_mode:?}.")));
    }

    fn on_hot_key_button_click(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        self.mode_status().set_text(Some("Click raised by the hot key or the pointer."));
    }

    fn on_repeat_button_click(&self, _sender: &Option<BoxedValue>, _args: &RoutedEventArgs) {
        self.repeat_button_click_count.set(self.repeat_button_click_count.get() + 1);
        self.repeat_button_text_block()
            .set_text(Some(&format!("Repeat Button: {}", self.repeat_button_click_count.get())));
    }
}

/// `ButtonsPage.RelayCommand`: a command made of two callbacks.
pub struct RelayCommand {
    execute: Box<dyn Fn()>,
    can_execute: Box<dyn Fn() -> bool>,
    can_execute_changed: RefCell<Vec<(u64, Rc<dyn Fn()>)>>,
    next_token: Cell<u64>,
    this: Weak<RelayCommand>,
}

impl RelayCommand {
    pub fn new(execute: impl Fn() + 'static, can_execute: impl Fn() -> bool + 'static) -> Rc<RelayCommand> {
        Rc::new_cyclic(|this| RelayCommand {
            execute: Box::new(execute),
            can_execute: Box::new(can_execute),
            can_execute_changed: RefCell::new(Vec::new()),
            next_token: Cell::new(0),
            this: this.clone(),
        })
    }

    pub fn raise_can_execute_changed(&self) {
        let handlers: Vec<Rc<dyn Fn()>> = self.can_execute_changed.borrow().iter().map(|(_, h)| h.clone()).collect();
        for handler in handlers {
            handler();
        }
    }
}

impl ICommand for RelayCommand {
    fn can_execute(&self, _parameter: Option<&BoxedValue>) -> bool {
        (self.can_execute)()
    }

    fn execute(&self, _parameter: Option<&BoxedValue>) {
        (self.execute)()
    }

    fn can_execute_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.next_token.get();
        self.next_token.set(token + 1);
        self.can_execute_changed.borrow_mut().push((token, handler));
        let this = self.this.clone();
        Disposable::create(move || {
            if let Some(this) = this.upgrade() {
                this.can_execute_changed.borrow_mut().retain(|(t, _)| *t != token);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn relay_command_runs_its_callbacks() {
        let runs = Rc::new(Cell::new(0));
        let allowed = Rc::new(Cell::new(true));
        let command = RelayCommand::new(
            {
                let runs = runs.clone();
                move || runs.set(runs.get() + 1)
            },
            {
                let allowed = allowed.clone();
                move || allowed.get()
            },
        );

        assert!(command.can_execute(None));
        command.execute(None);
        assert_eq!(1, runs.get());
        allowed.set(false);
        assert!(!command.can_execute(None));
    }

    #[test]
    fn relay_command_raises_can_execute_changed_until_unsubscribed() {
        let command = RelayCommand::new(|| {}, || true);
        let changes = Rc::new(Cell::new(0));
        let subscription = command.can_execute_changed({
            let changes = changes.clone();
            Rc::new(move || changes.set(changes.get() + 1))
        });

        command.raise_can_execute_changed();
        assert_eq!(1, changes.get());
        subscription.dispose();
        command.raise_can_execute_changed();
        assert_eq!(1, changes.get());
    }

    #[test]
    fn plural_suffix() {
        assert_eq!("", plural(1));
        assert_eq!("s", plural(0));
        assert_eq!("s", plural(2));
    }
}
