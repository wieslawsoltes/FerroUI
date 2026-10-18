//! `Models/TreeDataGrid/TextColumnOptions.cs`: the options of a text column.

use super::{BeginEditGestures, ColumnOptions, ICellOptions, IColumnOptions, ITextCellOptions};
use ferroui_base::media::{TextAlignment, TextTrimming, TextWrapping};
use ferroui_base::utilities::CultureInfo;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// The culture of the options of a text cell (`ITextCellOptions.Culture`), which the view contract
/// of the options does not state: an extension of [`ITextCellOptions`].
pub trait ITextCellCultureOptions: ITextCellOptions {
    /// The culture the string format is applied with.
    fn culture(&self) -> CultureInfo;
}

/// `TextColumnOptions<TModel>`.
pub struct TextColumnOptions<M: 'static> {
    base: ColumnOptions<M>,
    is_text_search_enabled: Cell<bool>,
    string_format: RefCell<String>,
    culture: RefCell<CultureInfo>,
    text_trimming: RefCell<Rc<dyn TextTrimming>>,
    text_wrapping: Cell<TextWrapping>,
    text_alignment: Cell<TextAlignment>,
}

impl<M: 'static> Default for TextColumnOptions<M> {
    fn default() -> Self {
        Self {
            base: ColumnOptions::default(),
            is_text_search_enabled: Cell::new(false),
            string_format: RefCell::new("{0}".to_string()),
            culture: RefCell::new(CultureInfo::current_culture()),
            text_trimming: RefCell::new(<dyn TextTrimming>::character_ellipsis()),
            text_wrapping: Cell::new(TextWrapping::NoWrap),
            text_alignment: Cell::new(TextAlignment::Left),
        }
    }
}

impl<M: 'static> TextColumnOptions<M> {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the column takes part in text searches.
    pub fn is_text_search_enabled(&self) -> bool {
        self.is_text_search_enabled.get()
    }

    pub fn set_is_text_search_enabled(&self, value: bool) {
        self.is_text_search_enabled.set(value);
    }

    /// The format string of the cells of the column.
    pub fn string_format(&self) -> String {
        self.string_format.borrow().clone()
    }

    pub fn set_string_format(&self, value: impl Into<String>) {
        *self.string_format.borrow_mut() = value.into();
    }

    /// The culture used with the string format.
    pub fn culture(&self) -> CultureInfo {
        self.culture.borrow().clone()
    }

    pub fn set_culture(&self, value: CultureInfo) {
        *self.culture.borrow_mut() = value;
    }

    /// The text trimming of the cells of the column.
    pub fn text_trimming(&self) -> Rc<dyn TextTrimming> {
        self.text_trimming.borrow().clone()
    }

    pub fn set_text_trimming(&self, value: Rc<dyn TextTrimming>) {
        *self.text_trimming.borrow_mut() = value;
    }

    /// The text wrapping of the cells of the column.
    pub fn text_wrapping(&self) -> TextWrapping {
        self.text_wrapping.get()
    }

    pub fn set_text_wrapping(&self, value: TextWrapping) {
        self.text_wrapping.set(value);
    }

    /// The text alignment of the cells of the column.
    pub fn text_alignment(&self) -> TextAlignment {
        self.text_alignment.get()
    }

    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.text_alignment.set(value);
    }
}

impl<M: 'static> Deref for TextColumnOptions<M> {
    type Target = ColumnOptions<M>;

    fn deref(&self) -> &ColumnOptions<M> {
        &self.base
    }
}

impl<M: 'static> IColumnOptions<M> for TextColumnOptions<M> {
    fn column_options(&self) -> &ColumnOptions<M> {
        &self.base
    }
}

impl<M: 'static> ICellOptions for TextColumnOptions<M> {
    fn begin_edit_gestures(&self) -> BeginEditGestures {
        self.base.begin_edit_gestures()
    }
}

impl<M: 'static> ITextCellOptions for TextColumnOptions<M> {
    fn string_format(&self) -> String {
        TextColumnOptions::string_format(self)
    }

    fn text_trimming(&self) -> Rc<dyn TextTrimming> {
        TextColumnOptions::text_trimming(self)
    }

    fn text_wrapping(&self) -> TextWrapping {
        TextColumnOptions::text_wrapping(self)
    }

    fn text_alignment(&self) -> TextAlignment {
        TextColumnOptions::text_alignment(self)
    }
}

impl<M: 'static> ITextCellCultureOptions for TextColumnOptions<M> {
    fn culture(&self) -> CultureInfo {
        TextColumnOptions::culture(self)
    }
}
