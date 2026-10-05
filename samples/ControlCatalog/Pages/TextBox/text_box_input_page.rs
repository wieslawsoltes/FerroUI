//! Port of `Pages/TextBox/TextBoxInputPage.xaml.cs`: the class of the
//! document `Pages/TextBox/TextBoxInputPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::Interactive;
use ferroui_base::{ferro_class_info, instantiate, FerroPropertyChangedEventArgs, Ref};
use ferroui_controls::{ComboBox, MaskedTextBox, SelectionChangedEventArgs, TextBlock, TextBox, UserControl};

#[repr(C)]
pub struct TextBoxInputPage {
    base: UserControl,
}

user_control_class!(TextBoxInputPage);
ferro_class_info!(TextBoxInputPage { new: TextBoxInputPage::new });
xaml_class!(TextBoxInputPage, "/Pages/TextBox/TextBoxInputPage.xaml");

impl TextBoxInputPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers belong to children of the page: they hold the page weakly.
        let weak = this.downgrade();
        this.length_box().property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.on_length_box_property_changed(e);
            }
        });
        let weak = this.downgrade();
        this.password_char_combo().selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_password_char_changed(sender, e);
            }
        });
        let weak = this.downgrade();
        this.mask_combo().selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_mask_changed(sender, e);
            }
        });

        this.update_length_status();
        this
    }

    fn length_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("LengthBox")
    }

    fn length_status(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("LengthStatus")
    }

    fn password_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("PasswordBox")
    }

    fn password_char_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("PasswordCharCombo")
    }

    fn mask_box(&self) -> Ref<MaskedTextBox> {
        self.get_control::<MaskedTextBox>("MaskBox")
    }

    fn mask_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("MaskCombo")
    }

    fn on_length_box_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == TextBox::text_property().as_property()
            || e.property() == TextBox::max_length_property().as_property()
        {
            self.update_length_status();
        }
    }

    fn on_password_char_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        // The default char, '\0', means the text is drawn as it is typed.
        self.password_box().set_password_char(match self.password_char_combo().selected_index() {
            1 => '\u{2022}',
            2 => '\0',
            _ => '*',
        });
    }

    fn on_mask_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        self.mask_box().set_mask(Some(match self.mask_combo().selected_index() {
            1 => "0000-00-00",
            2 => "LL-0000",
            _ => "(LLL) 999-0000",
        }));
    }

    fn update_length_status(&self) {
        // The length of text is counted in UTF-16 code units, as the original counts it.
        let length = self.length_box().text().map_or(0, |text| text.encode_utf16().count());
        let limit = self.length_box().max_length();
        self.length_status().set_text(Some(&if limit > 0 {
            format!("{length} / {limit} characters")
        } else {
            format!("{length} characters, no limit")
        }));
    }
}
