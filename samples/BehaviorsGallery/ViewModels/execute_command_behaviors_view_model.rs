//! Port of `ViewModels/ExecuteCommandBehaviorsViewModel.cs`.

use super::items::{text_list, ItemList};
use super::{Command, ViewModelBase};
use ferroui_base::input::ICommand;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{ferro_markup_type, BoxedValue};
use ferroui_controls::ItemsSource;
use ferroui_reactiveui::reactive_object;
use std::rc::{Rc, Weak};

/// The view model of the pages of the execute command behaviors: a log of the events the commands
/// report.
pub struct ExecuteCommandBehaviorsViewModel {
    base: ViewModelBase,
    events: ItemList,
    activated_command: Rc<dyn ICommand>,
    got_focus_command: Rc<dyn ICommand>,
    lost_focus_command: Rc<dyn ICommand>,
    pointer_entered_command: Rc<dyn ICommand>,
    pointer_pressed_command: Rc<dyn ICommand>,
    pointer_moved_command: Rc<dyn ICommand>,
    pointer_exited_command: Rc<dyn ICommand>,
    pointer_released_command: Rc<dyn ICommand>,
    pointer_capture_lost_command: Rc<dyn ICommand>,
    pointer_wheel_changed_command: Rc<dyn ICommand>,
    holding_command: Rc<dyn ICommand>,
    tapped_command: Rc<dyn ICommand>,
    double_tapped_command: Rc<dyn ICommand>,
    right_tapped_command: Rc<dyn ICommand>,
    pull_command: Rc<dyn ICommand>,
    pull_ended_command: Rc<dyn ICommand>,
    scroll_gesture_command: Rc<dyn ICommand>,
    scroll_gesture_ended_command: Rc<dyn ICommand>,
    scroll_gesture_inertia_command: Rc<dyn ICommand>,
    touch_pad_magnify_command: Rc<dyn ICommand>,
    touch_pad_rotate_command: Rc<dyn ICommand>,
    touch_pad_swipe_command: Rc<dyn ICommand>,
    key_down_command: Rc<dyn ICommand>,
    key_up_command: Rc<dyn ICommand>,
    text_input_command: Rc<dyn ICommand>,
    text_input_method_requested_command: Rc<dyn ICommand>,
    clear_log_command: Rc<dyn ICommand>,
}
reactive_object!(ExecuteCommandBehaviorsViewModel, base);

impl PartialEq for ExecuteCommandBehaviorsViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// A command that writes `message` to the log (`ReactiveCommand.Create(() => Log(message))`).
fn log_command(
    this: &Weak<ExecuteCommandBehaviorsViewModel>,
    message: &'static str,
) -> Rc<dyn ICommand> {
    let this = this.clone();
    Command::create(move || {
        if let Some(this) = this.upgrade() {
            this.log(message);
        }
    })
}

impl ExecuteCommandBehaviorsViewModel {
    /// `ExecuteCommandBehaviorsViewModel()`.
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this: &Weak<Self>| Self {
            base: ViewModelBase::new(),
            events: text_list([]),
            activated_command: log_command(this, "Window activated"),
            got_focus_command: log_command(this, "Got focus"),
            lost_focus_command: log_command(this, "Lost focus"),
            pointer_entered_command: log_command(this, "Pointer entered"),
            pointer_pressed_command: log_command(this, "Pointer pressed"),
            pointer_moved_command: log_command(this, "Pointer moved"),
            pointer_exited_command: log_command(this, "Pointer exited"),
            pointer_released_command: log_command(this, "Pointer released"),
            pointer_capture_lost_command: log_command(this, "Pointer capture lost"),
            pointer_wheel_changed_command: log_command(this, "Pointer wheel changed"),
            holding_command: log_command(this, "Holding gesture"),
            tapped_command: log_command(this, "Tapped"),
            double_tapped_command: log_command(this, "Double tapped"),
            right_tapped_command: log_command(this, "Right tapped"),
            pull_command: log_command(this, "Pull gesture"),
            pull_ended_command: log_command(this, "Pull gesture ended"),
            scroll_gesture_command: log_command(this, "Scroll gesture"),
            scroll_gesture_ended_command: log_command(this, "Scroll gesture ended"),
            scroll_gesture_inertia_command: log_command(this, "Scroll inertia starting"),
            touch_pad_magnify_command: log_command(this, "Touchpad magnify"),
            touch_pad_rotate_command: log_command(this, "Touchpad rotate"),
            touch_pad_swipe_command: log_command(this, "Touchpad swipe"),
            key_down_command: log_command(this, "Key down"),
            key_up_command: log_command(this, "Key up"),
            text_input_command: log_command(this, "Text input"),
            text_input_method_requested_command: log_command(
                this,
                "Text input method client requested",
            ),
            clear_log_command: {
                let this = this.clone();
                Command::create(move || {
                    if let Some(this) = this.upgrade() {
                        this.events.clear();
                    }
                })
            },
        })
    }

    /// `Events`.
    pub fn events(&self) -> ItemList {
        self.events.clone()
    }

    /// `Log(string)`: the message with the time, first in the log, which keeps 50 entries.
    fn log(&self, message: &str) {
        let entry = format!(
            "{} - {message}",
            DateTimeOffset::now().to_string_with("HH:mm:ss")
        );
        self.events.insert(0, Rc::new(entry) as BoxedValue);

        const MAX_ENTRIES: usize = 50;
        if self.events.count() > MAX_ENTRIES {
            self.events.remove_at(self.events.count() - 1);
        }
    }
}

type Vm = Rc<ExecuteCommandBehaviorsViewModel>;

ferro_markup_type!(class ExecuteCommandBehaviorsViewModel {
    this: Rc<ExecuteCommandBehaviorsViewModel>,
    handles: [ExecuteCommandBehaviorsViewModel, Rc<ExecuteCommandBehaviorsViewModel>, Option<Rc<ExecuteCommandBehaviorsViewModel>>],
    constructors: [() => ExecuteCommandBehaviorsViewModel::new],
    properties: [
        Events: ItemsSource { get: |this: &Vm| ItemsSource::from(this.events()) },
        ActivatedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.activated_command.clone() },
        GotFocusCommand: Rc<dyn ICommand> { get: |this: &Vm| this.got_focus_command.clone() },
        LostFocusCommand: Rc<dyn ICommand> { get: |this: &Vm| this.lost_focus_command.clone() },
        PointerEnteredCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pointer_entered_command.clone() },
        PointerPressedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pointer_pressed_command.clone() },
        PointerMovedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pointer_moved_command.clone() },
        PointerExitedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pointer_exited_command.clone() },
        PointerReleasedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pointer_released_command.clone() },
        PointerCaptureLostCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pointer_capture_lost_command.clone() },
        PointerWheelChangedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pointer_wheel_changed_command.clone() },
        HoldingCommand: Rc<dyn ICommand> { get: |this: &Vm| this.holding_command.clone() },
        TappedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.tapped_command.clone() },
        DoubleTappedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.double_tapped_command.clone() },
        RightTappedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.right_tapped_command.clone() },
        PullCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pull_command.clone() },
        PullEndedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.pull_ended_command.clone() },
        ScrollGestureCommand: Rc<dyn ICommand> { get: |this: &Vm| this.scroll_gesture_command.clone() },
        ScrollGestureEndedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.scroll_gesture_ended_command.clone() },
        ScrollGestureInertiaCommand: Rc<dyn ICommand> { get: |this: &Vm| this.scroll_gesture_inertia_command.clone() },
        TouchPadMagnifyCommand: Rc<dyn ICommand> { get: |this: &Vm| this.touch_pad_magnify_command.clone() },
        TouchPadRotateCommand: Rc<dyn ICommand> { get: |this: &Vm| this.touch_pad_rotate_command.clone() },
        TouchPadSwipeCommand: Rc<dyn ICommand> { get: |this: &Vm| this.touch_pad_swipe_command.clone() },
        KeyDownCommand: Rc<dyn ICommand> { get: |this: &Vm| this.key_down_command.clone() },
        KeyUpCommand: Rc<dyn ICommand> { get: |this: &Vm| this.key_up_command.clone() },
        TextInputCommand: Rc<dyn ICommand> { get: |this: &Vm| this.text_input_command.clone() },
        TextInputMethodRequestedCommand: Rc<dyn ICommand> { get: |this: &Vm| this.text_input_method_requested_command.clone() },
        ClearLogCommand: Rc<dyn ICommand> { get: |this: &Vm| this.clear_log_command.clone() },
    ],
    notify_property_changed: ExecuteCommandBehaviorsViewModel,
});
