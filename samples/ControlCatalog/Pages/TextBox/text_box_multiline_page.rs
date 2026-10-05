//! Port of `Pages/TextBox/TextBoxMultilinePage.xaml.cs`: the class of the
//! document `Pages/TextBox/TextBoxMultilinePage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    ComboBox, Control, SelectionChangedEventArgs, TextBlock, TextBox, TextChangedEventArgs, UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct TextBoxMultilinePage {
    base: UserControl,
    /// The subscription of `OnNewLineBoxLoaded`, which removes itself.
    new_line_box_loaded: Cell<Option<RoutedEventHandlerToken>>,
}

user_control_class!(TextBoxMultilinePage);
ferro_class_info!(TextBoxMultilinePage {
    new: TextBoxMultilinePage::new,
    markup: {
        methods: [
            fn OnAppendLine(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TextBoxMultilinePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_append_line(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TextBoxMultilinePage, "/Pages/TextBox/TextBoxMultilinePage.xaml");

impl TextBoxMultilinePage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), new_line_box_loaded: Cell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers belong to children of the page: they hold the page weakly.
        let weak = this.downgrade();
        this.new_line_combo().selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_new_line_changed(sender, e);
            }
        });
        let weak = this.downgrade();
        this.new_line_box().text_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_new_line_text_changed(sender, e);
            }
        });
        let weak = this.downgrade();
        let token = this.new_line_box().loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_new_line_box_loaded(sender, e);
            }
        });
        this.new_line_box_loaded.set(Some(token));

        this.apply_new_line();
        this
    }

    fn new_line_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("NewLineBox")
    }

    fn new_line_status(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("NewLineStatus")
    }

    fn new_line_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("NewLineCombo")
    }

    fn on_new_line_box_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        if let Some(token) = self.new_line_box_loaded.take() {
            self.new_line_box().remove_handler(Control::loaded_event(), token);
        }
        self.update_status();
    }

    fn on_new_line_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        self.apply_new_line();
    }

    fn on_new_line_text_changed(&self, _sender: &Interactive, _e: &TextChangedEventArgs) {
        self.update_status();
    }

    fn on_append_line(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let new_line_box = self.new_line_box();
        let text = new_line_box.text().unwrap_or_default() + &new_line_box.new_line() + "Appended from code";
        new_line_box.set_text(Some(&text));
        // The length of text is counted in UTF-16 code units, as the original counts it.
        new_line_box.set_caret_index(new_line_box.text().map_or(0, |text| text.encode_utf16().count() as i32));
        new_line_box.focus();
    }

    fn apply_new_line(&self) {
        self.new_line_box().set_new_line(match self.new_line_combo().selected_index() {
            1 => "\r\n",
            2 => "\r",
            _ => "\n",
        });

        self.update_status();
    }

    fn update_status(&self) {
        // GetLineCount reports the laid out lines, so it is only meaningful once the box has a presenter.
        let new_line_box = self.new_line_box();
        let line_count = new_line_box.get_line_count();
        let lines = if line_count < 0 { String::from("not measured yet") } else { line_count.to_string() };
        self.new_line_status().set_text(Some(&format!(
            "NewLine: {}     Visual lines: {}     Text length: {}",
            Self::escape(&new_line_box.new_line()),
            lines,
            new_line_box.text().map_or(0, |text| text.encode_utf16().count())
        )));
    }

    fn escape(value: &str) -> String {
        let mut builder = String::new();
        for character in value.chars() {
            match character {
                '\r' => builder.push_str("\\r"),
                '\n' => builder.push_str("\\n"),
                '\t' => builder.push_str("\\t"),
                _ => builder.push(character),
            }
        }

        builder
    }
}

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn escape_spells_out_the_control_characters() {
        assert_eq!("a\\r\\n\\tb", TextBoxMultilinePage::escape("a\r\n\tb"));
    }
}
