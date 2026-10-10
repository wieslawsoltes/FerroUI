//! Port of `Pages/KeyboardPage.xaml.cs`: the class of the document `Pages/KeyboardPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::DelegateCommand;
use ferroui_base::input::{InputElement, Key, KeyBinding, KeyEventArgs, KeyGesture, KeyModifiers, TextInputEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Border, TextBlock, TextBox, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct KeyboardPage {
    base: UserControl,
    key_down_count: Cell<i32>,
}

user_control_class!(KeyboardPage);
ferro_class_info!(KeyboardPage {
    new: KeyboardPage::new,
    markup: {
        methods: [
            fn ResetKeyboard_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<KeyboardPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.reset_keyboard_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(KeyboardPage, "/Pages/KeyboardPage.xaml");

impl KeyboardPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), key_down_count: Cell::new(0) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // Gestures without a modifier are the interesting case: on macOS the key down used to be
        // swallowed by the input context while a text input client was active.
        this.add_key_binding(KeyGesture::new(Key::Space, KeyModifiers::NONE), "Space");
        this.add_key_binding(KeyGesture::new(Key::A, KeyModifiers::NONE), "A");
        this.add_key_binding(KeyGesture::new(Key::G, KeyModifiers::CONTROL), "Ctrl+G");

        // TextBox handles TextInput, and KeyDown for keys like Backspace and the arrows, in a
        // class handler that runs before instance handlers. Subscribe with handledEventsToo so
        // those keys stay visible here.
        //
        // The text box is a descendant of the page: its handlers hold the page weakly.
        let key_down_text_box = this.key_down_text_box();
        {
            let weak = this.downgrade();
            key_down_text_box.add_handler_with(
                InputElement::key_down_event(),
                move |sender, e| {
                    if let Some(this) = weak.upgrade() {
                        this.key_down_text_box_key_down(sender, e);
                    }
                },
                Interactive::DEFAULT_ROUTES,
                true,
            );
        }
        {
            let weak = this.downgrade();
            key_down_text_box.add_handler_with(
                InputElement::text_input_event(),
                move |sender, e| {
                    if let Some(this) = weak.upgrade() {
                        this.key_down_text_box_text_input(sender, e);
                    }
                },
                Interactive::DEFAULT_ROUTES,
                true,
            );
        }
        this
    }

    fn gesture_scope(&self) -> Ref<Border> {
        self.get_control::<Border>("GestureScope")
    }

    fn gesture_text_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("GestureTextBox")
    }

    fn last_key_binding(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("LastKeyBinding")
    }

    fn key_down_text_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("KeyDownTextBox")
    }

    fn last_key_down(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("LastKeyDown")
    }

    fn key_down_count_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("KeyDownCount")
    }

    fn last_text_input(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("LastTextInput")
    }

    fn add_key_binding(&self, gesture: KeyGesture, name: &'static str) {
        let key_binding = KeyBinding::new();
        key_binding.set_gesture(Some(gesture));
        // The binding is held by a descendant of the page: its command holds the page weakly.
        let weak = self.to_ref().downgrade();
        key_binding.set_command(Some(
            DelegateCommand::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.last_key_binding().set_text(Some(name));
                }
            })
            .as_command(),
        ));
        self.gesture_scope().key_bindings().add(key_binding);
    }

    fn key_down_text_box_key_down(&self, _sender: &Interactive, e: &KeyEventArgs) {
        // While an input method is composing, the key is masked as Key.ImeProcessed but the
        // physical key and the key symbol keep their real values. The key symbol is bracketed
        // to keep whitespace symbols like the space key's visible and assertable.
        self.last_key_down().set_text(Some(&format!(
            "{}|{}|[{}]",
            e.key,
            e.physical_key,
            e.key_symbol.as_deref().unwrap_or_default()
        )));

        // Counts every key down, including the ones flagsChanged raises for modifier keys.
        self.key_down_count.set(self.key_down_count.get() + 1);
        self.key_down_count_text().set_text(Some(&self.key_down_count.get().to_string()));
    }

    fn key_down_text_box_text_input(&self, _sender: &Interactive, e: &TextInputEventArgs) {
        self.last_text_input().set_text(Some(&format!("[{}]", e.text.as_deref().unwrap_or_default())));
    }

    fn reset_keyboard_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.key_down_count.set(0);
        self.key_down_count_text().set_text(Some(""));
        self.last_key_binding().set_text(Some(""));
        self.last_key_down().set_text(Some(""));
        self.last_text_input().set_text(Some(""));
        self.gesture_text_box().set_text(Some(""));
        self.key_down_text_box().set_text(Some(""));
    }
}
