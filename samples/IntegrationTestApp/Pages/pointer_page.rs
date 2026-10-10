//! Port of `Pages/PointerPage.xaml.cs`: the class of the document `Pages/PointerPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::DelegateCommand;
use ferroui_base::input::{InputElement, PointerPressedEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, Visual};
use ferroui_controls::{Button, Control, TextBlock, TopLevel, UserControl, Window};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct PointerPage {
    base: UserControl,
}

user_control_class!(PointerPage);
ferro_class_info!(PointerPage {
    new: PointerPage::new,
    markup: {
        methods: [
            fn PointerPageShowDialog_PointerPressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PointerPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.pointer_page_show_dialog_pointer_pressed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(PointerPage, "/Pages/PointerPage.xaml");

impl PointerPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn pointer_capture_status(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("PointerCaptureStatus")
    }

    /// # Panics
    /// Panics if the page is not attached to a window (the internal exception of the managed
    /// original).
    fn pointer_page_show_dialog_pointer_pressed(&self, _sender: &Option<BoxedValue>, e: &PointerPressedEventArgs) {
        let visual: &Visual = self;
        let window = TopLevel::get_top_level(Some(visual))
            .and_then(|top_level| top_level.cast::<Window>())
            .unwrap_or_else(|| panic!("PointerPage is not attached to a Window."));
        let captured = e.pointer().captured().and_then(|captured| captured.cast::<Control>());

        if let Some(captured) = &captured {
            // `CaptureLost`: says that nothing holds the pointer and removes itself. The handler
            // is held by the control it is added to, which may be an ancestor of the page: it
            // holds the page and the control weakly.
            let token: Rc<RefCell<Option<RoutedEventHandlerToken>>> = Rc::new(RefCell::new(None));
            let weak = self.to_ref().downgrade();
            let weak_captured = captured.downgrade();
            let handler_token = token.clone();
            let added = captured.add_handler(InputElement::pointer_capture_lost_event(), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.pointer_capture_status().set_text(Some("None"));
                }
                let token = handler_token.borrow_mut().take();
                if let (Some(captured), Some(token)) = (weak_captured.upgrade(), token) {
                    captured.remove_handler(InputElement::pointer_capture_lost_event(), token);
                }
            });
            *token.borrow_mut() = Some(added);
        }

        // `captured?.ToString() ?? "None"`: a control says the full name of its class.
        let status = captured.as_ref().map_or_else(|| String::from("None"), |captured| captured.get_type().full_name());
        self.pointer_capture_status().set_text(Some(&status));

        let dialog = Window::new();
        dialog.set_width(200.0);
        dialog.set_height(200.0);

        let button = Button::new();
        button.set_content(Some(Rc::new(String::from("Close")) as BoxedValue));
        {
            // The button is the content of the dialog: its command holds the dialog weakly.
            let weak_dialog = dialog.downgrade();
            button.set_command(Some(
                DelegateCommand::new(move || {
                    if let Some(dialog) = weak_dialog.upgrade() {
                        dialog.close();
                    }
                })
                .as_command(),
            ));
        }
        dialog.set_content(Some(Control::boxed(&button)));

        drop(dialog.show_dialog(&window));
    }
}
