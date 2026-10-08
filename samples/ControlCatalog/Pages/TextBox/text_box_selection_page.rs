//! Port of `Pages/TextBox/TextBoxSelectionPage.xaml.cs`: the class of the
//! document `Pages/TextBox/TextBoxSelectionPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::animation::TimeSpan;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, FerroPropertyChangedEventArgs, Ref};
use ferroui_controls::{ComboBox, SelectionChangedEventArgs, TextBlock, TextBox, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct TextBoxSelectionPage {
    base: UserControl,
}

user_control_class!(TextBoxSelectionPage);
ferro_class_info!(TextBoxSelectionPage {
    new: TextBoxSelectionPage::new,
    markup: {
        methods: [
            fn OnSelectAll(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TextBoxSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_select_all(&sender, e.as_routed_event_args())
                },
            fn OnSelectFirstWord(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TextBoxSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_select_first_word(&sender, e.as_routed_event_args())
                },
            fn OnClearSelection(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TextBoxSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_selection(&sender, e.as_routed_event_args())
                },
            fn OnCaretToStart(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TextBoxSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_caret_to_start(&sender, e.as_routed_event_args())
                },
            fn OnCaretToEnd(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TextBoxSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_caret_to_end(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TextBoxSelectionPage, "/Pages/TextBox/TextBoxSelectionPage.xaml");

impl TextBoxSelectionPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers belong to children of the page: they hold the page weakly.
        let weak = this.downgrade();
        this.selection_box().property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_box_property_changed(e);
            }
        });
        let weak = this.downgrade();
        this.caret_blink_combo().selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_caret_blink_changed(sender, e);
            }
        });

        this.update_selection_status();
        this
    }

    fn selection_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("SelectionBox")
    }

    fn selection_status(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("SelectionStatus")
    }

    fn caret_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("CaretBox")
    }

    fn caret_blink_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("CaretBlinkCombo")
    }

    fn on_selection_box_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == TextBox::selection_start_property().as_property()
            || e.property() == TextBox::selection_end_property().as_property()
            || e.property() == TextBox::text_property().as_property()
        {
            self.update_selection_status();
        }
    }

    fn on_select_all(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.selection_box().select_all();
        self.selection_box().focus();
    }

    fn on_select_first_word(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        // Positions in text are counted in UTF-16 code units, as the original counts them.
        let text = self.selection_box().text().unwrap_or_default();
        let end = text.encode_utf16().position(|unit| unit == u16::from(b' ')).unwrap_or_else(|| text.encode_utf16().count());

        self.selection_box().set_selection_start(0);
        self.selection_box().set_selection_end(end as i32);
        self.selection_box().focus();
    }

    fn on_clear_selection(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.selection_box().clear_selection();
    }

    fn on_caret_to_start(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.caret_box().set_caret_index(0);
        self.caret_box().focus();
    }

    fn on_caret_to_end(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.caret_box().set_caret_index(self.caret_box().text().map_or(0, |text| text.encode_utf16().count() as i32));
        self.caret_box().focus();
    }

    fn on_caret_blink_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        self.caret_box().set_caret_blink_interval(match self.caret_blink_combo().selected_index() {
            1 => TimeSpan::from_milliseconds(150.0),
            2 => TimeSpan::from_seconds(1.0),
            3 => TimeSpan::ZERO,
            _ => TimeSpan::from_milliseconds(500.0),
        });
    }

    fn update_selection_status(&self) {
        let selection_box = self.selection_box();
        let selected = selection_box.selected_text();
        let shown = if selected.is_empty() { String::from("(nothing selected)") } else { selected };
        self.selection_status().set_text(Some(&format!(
            "SelectionStart: {}     SelectionEnd: {}     SelectedText: {}",
            selection_box.selection_start(),
            selection_box.selection_end(),
            shown
        )));
    }
}
