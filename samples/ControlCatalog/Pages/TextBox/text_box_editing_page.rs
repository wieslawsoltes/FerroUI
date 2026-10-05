//! Port of `Pages/TextBox/TextBoxEditingPage.xaml.cs`: the class of the
//! document `Pages/TextBox/TextBoxEditingPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{InputElement, Key, KeyGesture, KeyModifiers};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, FerroObject, FerroObjectExtensions, Ref};
use ferroui_controls::{
    CheckBox, Control, MenuFlyout, MenuItem, Separator, TextBlock, TextBox, TextChangedEventArgs,
    TextChangingEventArgs, UserControl,
};
use std::cell::RefCell;
use std::rc::Rc;

const MAX_LOG_LINES: usize = 5;

#[repr(C)]
pub struct TextBoxEditingPage {
    base: UserControl,
    text_log: RefCell<Vec<String>>,
    clipboard_log: RefCell<Vec<String>>,
    cut_item: Ref<MenuItem>,
    copy_item: Ref<MenuItem>,
    paste_item: Ref<MenuItem>,
    select_all_item: Ref<MenuItem>,
}

user_control_class!(TextBoxEditingPage);
ferro_class_info!(TextBoxEditingPage {
    new: TextBoxEditingPage::new,
    markup: {
        methods: [
            fn OnUndo(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TextBoxEditingPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_undo(&sender, e.as_routed_event_args())
                },
            fn OnRedo(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TextBoxEditingPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_redo(&sender, e.as_routed_event_args())
                },
            fn OnClearTextLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TextBoxEditingPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_text_log(&sender, e.as_routed_event_args())
                },
            fn OnClearClipboardLog(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TextBoxEditingPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_clipboard_log(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TextBoxEditingPage, "/Pages/TextBox/TextBoxEditingPage.xaml");

fn menu_item(header: &str, input_gesture: Option<KeyGesture>) -> Ref<MenuItem> {
    let item = MenuItem::new();
    item.set_header(Some(Rc::new(header.to_string()) as BoxedValue));
    if input_gesture.is_some() {
        item.set_input_gesture(input_gesture);
    }
    item
}

impl TextBoxEditingPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            text_log: RefCell::new(Vec::new()),
            clipboard_log: RefCell::new(Vec::new()),
            cut_item: menu_item("Cut", Some(KeyGesture::new(Key::X, KeyModifiers::CONTROL))),
            copy_item: menu_item("Copy", Some(KeyGesture::new(Key::C, KeyModifiers::CONTROL))),
            paste_item: menu_item("Paste", Some(KeyGesture::new(Key::V, KeyModifiers::CONTROL))),
            select_all_item: menu_item("Select all", None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers belong to children of the page: they hold the page weakly.
        let weak = this.downgrade();
        this.text_events_box().text_changing(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_text_changing(sender, e);
            }
        });
        let weak = this.downgrade();
        this.text_events_box().text_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_text_changed(sender, e);
            }
        });
        let weak = this.downgrade();
        this.clipboard_box().cutting_to_clipboard(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_cutting_to_clipboard(sender, e);
            }
        });
        let weak = this.downgrade();
        this.clipboard_box().copying_to_clipboard(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_copying_to_clipboard(sender, e);
            }
        });
        let weak = this.downgrade();
        this.clipboard_box().pasting_from_clipboard(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_pasting_from_clipboard(sender, e.as_routed_event_args());
            }
        });

        this.build_context_flyout();
        this.update_text_log();
        this.update_clipboard_log();
        this
    }

    fn text_events_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("TextEventsBox")
    }

    fn text_event_log(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("TextEventLog")
    }

    fn clipboard_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("ClipboardBox")
    }

    fn clipboard_log(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("ClipboardLog")
    }

    fn block_cut_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("BlockCutCheck")
    }

    fn block_copy_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("BlockCopyCheck")
    }

    fn block_paste_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("BlockPasteCheck")
    }

    fn flyout_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("FlyoutBox")
    }

    fn undo_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("UndoBox")
    }

    fn build_context_flyout(&self) {
        // The items are fields of the page: their handlers hold the page weakly.
        let weak = self.to_ref().downgrade();
        self.cut_item.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.flyout_box().cut();
            }
        });
        let weak = self.to_ref().downgrade();
        self.copy_item.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.flyout_box().copy();
            }
        });
        let weak = self.to_ref().downgrade();
        self.paste_item.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.flyout_box().paste();
            }
        });
        let weak = self.to_ref().downgrade();
        self.select_all_item.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.flyout_box().select_all();
            }
        });

        // CanCut, CanCopy and CanPaste raise change notifications, so the menu items can simply follow them.
        let flyout_box = self.flyout_box();
        let flyout_object: &FerroObject = &flyout_box;
        self.cut_item.bind(
            InputElement::is_enabled_property(),
            FerroObjectExtensions::get_observable(flyout_object, TextBox::can_cut_property()),
            BindingPriority::LocalValue,
        );
        self.copy_item.bind(
            InputElement::is_enabled_property(),
            FerroObjectExtensions::get_observable(flyout_object, TextBox::can_copy_property()),
            BindingPriority::LocalValue,
        );
        self.paste_item.bind(
            InputElement::is_enabled_property(),
            FerroObjectExtensions::get_observable(flyout_object, TextBox::can_paste_property()),
            BindingPriority::LocalValue,
        );

        let flyout = MenuFlyout::new();
        flyout.items().add(Some(Control::boxed(&self.cut_item)));
        flyout.items().add(Some(Control::boxed(&self.copy_item)));
        flyout.items().add(Some(Control::boxed(&self.paste_item)));
        flyout.items().add(Some(Control::boxed(Separator::new())));
        flyout.items().add(Some(Control::boxed(&self.select_all_item)));

        flyout_box.set_context_flyout(flyout);
    }

    fn on_text_changing(&self, _sender: &Interactive, _e: &TextChangingEventArgs) {
        let text = self.text_events_box().text().unwrap_or_default();
        Self::log(&self.text_log, format!("TextChanging, Text is now \"{text}\""));
        self.update_text_log();
    }

    fn on_text_changed(&self, _sender: &Interactive, _e: &TextChangedEventArgs) {
        // The length of text is counted in UTF-16 code units, as the original counts it.
        let length = self.text_events_box().text().map_or(0, |text| text.encode_utf16().count());
        Self::log(&self.text_log, format!("TextChanged, {length} characters"));
        self.update_text_log();
    }

    fn on_cutting_to_clipboard(&self, _sender: &Interactive, e: &RoutedEventArgs) {
        let cancelled = self.block_cut_check().is_checked() == Some(true);
        e.set_handled(cancelled);
        Self::log(
            &self.clipboard_log,
            String::from(if cancelled { "CuttingToClipboard, cancelled" } else { "CuttingToClipboard, allowed" }),
        );
        self.update_clipboard_log();
    }

    fn on_copying_to_clipboard(&self, _sender: &Interactive, e: &RoutedEventArgs) {
        let cancelled = self.block_copy_check().is_checked() == Some(true);
        e.set_handled(cancelled);
        Self::log(
            &self.clipboard_log,
            String::from(if cancelled { "CopyingToClipboard, cancelled" } else { "CopyingToClipboard, allowed" }),
        );
        self.update_clipboard_log();
    }

    fn on_pasting_from_clipboard(&self, _sender: &Interactive, e: &RoutedEventArgs) {
        let cancelled = self.block_paste_check().is_checked() == Some(true);
        e.set_handled(cancelled);
        Self::log(
            &self.clipboard_log,
            String::from(if cancelled { "PastingFromClipboard, cancelled" } else { "PastingFromClipboard, allowed" }),
        );
        self.update_clipboard_log();
    }

    fn on_undo(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.undo_box().undo();
        self.undo_box().focus();
    }

    fn on_redo(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.undo_box().redo();
        self.undo_box().focus();
    }

    fn on_clear_text_log(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.text_log.borrow_mut().clear();
        self.update_text_log();
    }

    fn on_clear_clipboard_log(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.clipboard_log.borrow_mut().clear();
        self.update_clipboard_log();
    }

    fn log(log: &RefCell<Vec<String>>, entry: String) {
        let mut log = log.borrow_mut();
        log.insert(0, entry);
        while log.len() > MAX_LOG_LINES {
            log.pop();
        }
    }

    fn update_text_log(&self) {
        let text = {
            let log = self.text_log.borrow();
            if log.is_empty() { String::from("No text events yet.") } else { log.join("\n") }
        };
        self.text_event_log().set_text(Some(&text));
    }

    fn update_clipboard_log(&self) {
        let text = {
            let log = self.clipboard_log.borrow();
            if log.is_empty() { String::from("No clipboard events yet.") } else { log.join("\n") }
        };
        self.clipboard_log().set_text(Some(&text));
    }
}
