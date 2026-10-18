//! `Models/TreeDataGrid/TextCell.cs`: a cell that shows its value as text and edits the text.

use super::{
    box_cell_value, cell_value_from_text, cell_value_text, BeginEditGestures, ICell, ICellBinding, IEditableObject,
    IRecyclableCell, ITextCell, ITextCellCultureOptions,
};
use ferroui_base::data::converters::composite_format::format_values_with;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::media::{TextAlignment, TextTrimming, TextWrapping};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::BoxedValue;
use ferroui_treedatagrid_core::models::NotifyingBase;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// `TextCell<T>`: a cell whose value of type `T` is shown and edited as text.
pub struct TextCell<T: Clone + PartialEq + 'static> {
    weak_self: Weak<Self>,
    notifying: NotifyingBase,
    binding: Option<Rc<dyn ICellBinding<T>>>,
    options: Option<Rc<dyn ITextCellCultureOptions>>,
    subscription: Cell<Option<u64>>,
    edit_text: RefCell<Option<String>>,
    value: RefCell<Option<T>>,
    is_editing: Cell<bool>,
    is_read_only: bool,
}

impl<T: Clone + PartialEq + 'static> TextCell<T> {
    /// A read-only cell of a constant value (`TextCell(T? value)`).
    pub fn from_value(value: Option<T>) -> Rc<Self> {
        Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            notifying: NotifyingBase::new(),
            binding: None,
            options: None,
            subscription: Cell::new(None),
            edit_text: RefCell::new(None),
            value: RefCell::new(value),
            is_editing: Cell::new(false),
            is_read_only: true,
        })
    }

    /// A cell of the value `binding` pushes; a cell that is not read only writes its changes back
    /// to the binding.
    pub fn new(
        binding: Rc<dyn ICellBinding<T>>,
        is_read_only: bool,
        options: Option<Rc<dyn ITextCellCultureOptions>>,
    ) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self| Self {
            weak_self: weak_self.clone(),
            notifying: NotifyingBase::new(),
            binding: Some(binding.clone()),
            options,
            subscription: Cell::new(None),
            edit_text: RefCell::new(None),
            value: RefCell::new(None),
            is_editing: Cell::new(false),
            is_read_only,
        });
        let weak = Rc::downgrade(&this);
        let token = binding.subscribe(Rc::new(move |value: &T| {
            if let Some(this) = weak.upgrade() {
                this.set_value(Some(value.clone()));
            }
        }));
        this.subscription.set(Some(token));
        this
    }

    /// Whether the cell's value can be changed.
    pub fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// The value of the cell.
    pub fn value(&self) -> Option<T> {
        self.value.borrow().clone()
    }

    /// Sets the value; a cell that is not read only and not editing writes it to its binding.
    pub fn set_value(&self, value: Option<T>) {
        if *self.value.borrow() == value {
            return;
        }
        // Only a cell that writes its changes back keeps a copy of the value for its binding.
        let written_back = if self.is_read_only { None } else { value.clone() };
        *self.value.borrow_mut() = value;
        self.notifying.raise_property_changed("Value");
        if !self.is_editing.get() {
            if let (Some(binding), Some(value)) = (&self.binding, written_back) {
                binding.on_next(value);
            }
        }
    }

    /// The text of the cell: the text being edited, or the value formatted with the string format
    /// of the options, or the value's text.
    pub fn text(&self) -> Option<String> {
        if self.is_editing.get() {
            return self.edit_text.borrow().clone();
        }
        let boxed = {
            let value = self.value.borrow();
            if self.options.as_ref().is_some_and(|options| options.has_default_string_format()) {
                let text = match &*value {
                    Some(value) => invariant_text(value as &dyn Any),
                    None => Some(String::new()),
                };
                if text.is_some() {
                    return text;
                }
            }
            value.as_ref().and_then(box_cell_value)
        };
        match &self.options {
            Some(options) => {
                let format = options.string_format();
                let culture = options.culture();
                Some(format_text(&format, boxed, &culture))
            }
            None => cell_value_text(boxed.as_ref()),
        }
    }

    /// Sets the text: while editing, the text being edited; otherwise the value converted from
    /// the text (a text that does not convert is ignored).
    pub fn set_text(&self, value: Option<String>) {
        if self.is_editing.get() {
            *self.edit_text.borrow_mut() = value;
            return;
        }
        match value {
            None => {
                // `Convert.ChangeType(null, typeof(T))`: null for a type that accepts it.
                if ValueTypes::accepts_null(ValueType::of::<T>()) {
                    match cell_value_from_text::<T>(None) {
                        Some(converted) => self.set_value(Some(converted)),
                        None => self.set_value(None),
                    }
                }
            }
            Some(text) => {
                if let Some(converted) = cell_value_from_text::<T>(Some(text)) {
                    self.set_value(Some(converted));
                }
            }
        }
    }

    /// Points the cell's binding at another model (`TrySetSource`); false when the binding
    /// cannot be retargeted.
    pub fn try_set_source(&self, source: Option<&BoxedValue>) -> bool {
        self.binding
            .as_ref()
            .and_then(|binding| binding.as_retargetable())
            .is_some_and(|retargetable| retargetable.try_set_root(source))
    }

    /// The cell as a shared cell.
    pub fn this(&self) -> Option<Rc<Self>> {
        self.weak_self.upgrade()
    }
}

/// The text of the default format of a column (`{0}`) for a value whose text is the same in every
/// culture: a string is its own text, a number without a sign has no symbol of a culture, and a
/// missing value has no text (as `string.Format` writes null). `None` for any other value, which
/// the composite formatter formats. A cell reads its text at every realization, and this is its
/// common case: the value is not boxed and the format is not parsed.
fn invariant_text(value: &dyn Any) -> Option<String> {
    // The cells of untyped models hold their values boxed.
    if let Some(boxed) = value.downcast_ref::<BoxedValue>() {
        return invariant_text(boxed.as_any());
    }
    if let Some(boxed) = value.downcast_ref::<Option<BoxedValue>>() {
        return match boxed {
            Some(boxed) => invariant_text(boxed.as_any()),
            None => Some(String::new()),
        };
    }
    if let Some(text) = value.downcast_ref::<String>() {
        return Some(text.clone());
    }
    if let Some(text) = value.downcast_ref::<Option<String>>() {
        return Some(text.clone().unwrap_or_default());
    }
    macro_rules! unsigned {
        ($($ty:ty),*) => {
            $(if let Some(v) = value.downcast_ref::<$ty>() {
                return Some(v.to_string());
            }
            if let Some(v) = value.downcast_ref::<Option<$ty>>() {
                return Some(v.map(|v| v.to_string()).unwrap_or_default());
            })*
        };
    }
    macro_rules! signed {
        ($($ty:ty),*) => {
            $(if let Some(v) = value.downcast_ref::<$ty>() {
                return (*v >= 0).then(|| v.to_string());
            }
            if let Some(v) = value.downcast_ref::<Option<$ty>>() {
                return match v {
                    None => Some(String::new()),
                    Some(v) => (*v >= 0).then(|| v.to_string()),
                };
            })*
        };
    }
    unsigned!(u8, u16, u32, u64, usize);
    signed!(i8, i16, i32, i64, isize);
    None
}

/// `string.Format(culture, format, value)`.
fn format_text(format: &str, value: Option<BoxedValue>, culture: &CultureInfo) -> String {
    match format_values_with(format, std::slice::from_ref(&value), culture) {
        Ok(text) => text,
        // A format the value does not take shows the value's own text.
        Err(_) => cell_value_text(value.as_ref()).unwrap_or_default(),
    }
}

impl<T: Clone + PartialEq + 'static> IEditableObject for TextCell<T> {
    fn begin_edit(&self) {
        if !self.is_editing.get() && !self.is_read_only {
            // As the reference: the text is read once editing has begun, so the edit starts from
            // the edit text the control supplies.
            self.is_editing.set(true);
            let text = self.text();
            *self.edit_text.borrow_mut() = text;
        }
    }

    fn cancel_edit(&self) {
        if self.is_editing.get() {
            self.is_editing.set(false);
            *self.edit_text.borrow_mut() = None;
        }
    }

    fn end_edit(&self) {
        if self.is_editing.get() {
            let text = self.edit_text.borrow_mut().take();
            self.is_editing.set(false);
            self.set_text(text);
        }
    }
}

impl<T: Clone + PartialEq + 'static> INotifyPropertyChanged for TextCell<T> {
    fn property_changed(&self) -> &Event<str> {
        self.notifying.property_changed()
    }
}

impl<T: Clone + PartialEq + 'static> ICell for TextCell<T> {
    fn can_edit(&self) -> bool {
        !self.is_read_only
    }

    fn edit_gestures(&self) -> BeginEditGestures {
        self.options.as_ref().map(|o| o.begin_edit_gestures()).unwrap_or(BeginEditGestures::DEFAULT)
    }

    fn value(&self) -> Option<BoxedValue> {
        self.value.borrow().as_ref().and_then(box_cell_value)
    }

    fn as_editable(&self) -> Option<&dyn IEditableObject> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_text_cell(&self) -> Option<&dyn ITextCell> {
        Some(self)
    }

    fn as_notify_property_changed(&self) -> Option<&dyn INotifyPropertyChanged> {
        Some(self)
    }

    fn as_recyclable(&self) -> Option<&dyn IRecyclableCell> {
        Some(self)
    }

    fn dispose(&self) {
        if let (Some(binding), Some(token)) = (&self.binding, self.subscription.take()) {
            binding.unsubscribe(token);
        }
    }
}

impl<T: Clone + PartialEq + 'static> ITextCell for TextCell<T> {
    fn text(&self) -> Option<String> {
        TextCell::text(self)
    }

    fn set_text(&self, value: Option<String>) {
        TextCell::set_text(self, value)
    }

    fn text_trimming(&self) -> Rc<dyn TextTrimming> {
        match &self.options {
            Some(options) => options.text_trimming(),
            None => <dyn TextTrimming>::none(),
        }
    }

    fn text_wrapping(&self) -> TextWrapping {
        self.options.as_ref().map(|o| o.text_wrapping()).unwrap_or(TextWrapping::NoWrap)
    }

    fn text_alignment(&self) -> TextAlignment {
        self.options.as_ref().map(|o| o.text_alignment()).unwrap_or(TextAlignment::Left)
    }
}

impl<T: Clone + PartialEq + 'static> IRecyclableCell for TextCell<T> {
    fn try_suspend(&self) -> bool {
        let suspended = self
            .binding
            .as_ref()
            .and_then(|binding| binding.as_retargetable())
            .is_some_and(|retargetable| retargetable.try_suspend_root());
        if !suspended {
            return false;
        }

        *self.value.borrow_mut() = None;
        *self.edit_text.borrow_mut() = None;
        self.is_editing.set(false);
        true
    }
}
