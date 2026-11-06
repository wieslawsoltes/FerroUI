// Derived from ScreenKeyboardControl; see NOTICE.md.
use super::{EnterAction, InputAttributes};
use crate::text::Utf16Text;
use ferroui_system::events::Event;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

#[derive(Clone, Debug)]
pub enum TargetReplaceError {
    ArgumentNull(&'static str),
    NullReference,
    Native(Arc<dyn std::error::Error + Send + Sync>),
}
impl PartialEq for TargetReplaceError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ArgumentNull(a), Self::ArgumentNull(b)) => a == b,
            (Self::NullReference, Self::NullReference) => true,
            (Self::Native(a), Self::Native(b)) => Arc::ptr_eq(a, b),
            _ => false,
        }
    }
}
impl Eq for TargetReplaceError {}
impl std::fmt::Display for TargetReplaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ArgumentNull(p) => write!(f, "required replacement argument is missing: {p}"),
            Self::NullReference => f.write_str("required input-target data is missing"),
            Self::Native(e) => std::fmt::Display::fmt(e.as_ref(), f),
        }
    }
}
impl std::error::Error for TargetReplaceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Native(e) = self {
            Some(e.as_ref())
        } else {
            None
        }
    }
}
pub type TargetReplaceResult = Result<(), TargetReplaceError>;
pub trait ITextInputTarget {
    fn attributes(&self) -> Arc<InputAttributes>;
    fn text(&self) -> Utf16Text;
    fn selection_start(&self) -> i32;
    fn selection_length(&self) -> i32;
    fn replace(&self, start: i32, length: i32, text: Option<&Utf16Text>) -> TargetReplaceResult;
    fn select(&self, start: i32, length: i32);
    fn perform_enter_action(&self, action: EnterAction) -> bool;
    fn changed(&self) -> &Event<dyn ITextInputTarget>;
}

// Interface views name the same underlying source object; vtable addresses are not identities.
impl PartialEq for dyn ITextInputTarget {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
impl Eq for dyn ITextInputTarget {}

pub type EnterActionHandler = Rc<dyn Fn(EnterAction) -> bool>;
pub struct TextBufferTarget {
    text: RefCell<Utf16Text>,
    selection_start: Cell<i32>,
    selection_length: Cell<i32>,
    attributes: RefCell<Arc<InputAttributes>>,
    performed_actions: Rc<RefCell<Vec<EnterAction>>>,
    enter_action_handler: RefCell<Option<EnterActionHandler>>,
    changed: Event<dyn ITextInputTarget>,
    edited: Event<dyn ITextInputTarget>,
}
impl Default for TextBufferTarget {
    fn default() -> Self {
        Self::new(Utf16Text::default(), None)
    }
}
impl PartialEq for TextBufferTarget {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for TextBufferTarget {}
impl TextBufferTarget {
    pub fn new(text: Utf16Text, attributes: Option<Arc<InputAttributes>>) -> Self {
        let length = i32::try_from(text.len()).expect(".NET string length exceeds i32");
        Self {
            text: RefCell::new(text),
            selection_start: Cell::new(length),
            selection_length: Cell::new(0),
            attributes: RefCell::new(attributes.unwrap_or_else(InputAttributes::shared_default)),
            performed_actions: Rc::default(),
            enter_action_handler: RefCell::default(),
            changed: Event::new(),
            edited: Event::new(),
        }
    }
    pub fn set_attributes(&self, value: Arc<InputAttributes>) {
        *self.attributes.borrow_mut() = value;
    }
    pub fn set_text(&self, value: Option<Utf16Text>) {
        let value = value.unwrap_or_default();
        let length = i32::try_from(value.len()).expect(".NET string length exceeds i32");
        *self.text.borrow_mut() = value;
        self.selection_start
            .set(self.selection_start.get().min(length));
        self.selection_length.set(
            self.selection_length
                .get()
                .min(length - self.selection_start.get()),
        );
        self.changed.raise(self);
    }
    pub fn performed_actions(&self) -> Rc<RefCell<Vec<EnterAction>>> {
        self.performed_actions.clone()
    }
    pub fn enter_action_handler(&self) -> Option<EnterActionHandler> {
        self.enter_action_handler.borrow().clone()
    }
    pub fn set_enter_action_handler(&self, handler: Option<EnterActionHandler>) {
        let previous = self.enter_action_handler.replace(handler);
        // Captures can run user code on drop, including replacing this handler.
        drop(previous);
    }
    pub fn edited(&self) -> &Event<dyn ITextInputTarget> {
        &self.edited
    }
    pub fn move_caret_externally(&self, position: i32) {
        self.selection_start
            .set(position.clamp(0, self.text.borrow().len() as i32));
        self.selection_length.set(0);
        self.changed.raise(self);
    }
    /// Source ToString, returning UTF-16 so an invalid surrogate is never silently replaced.
    pub fn to_text(&self) -> Utf16Text {
        let mut text = self.text.borrow().units().to_vec();
        let start = self.selection_start.get() as usize;
        let length = self.selection_length.get() as usize;
        text.insert(
            start + length,
            if length > 0 { b']' as u16 } else { b'|' as u16 },
        );
        if length > 0 {
            text.insert(start, b'[' as u16);
        }
        Utf16Text::from_units(text)
    }
}
impl ITextInputTarget for TextBufferTarget {
    fn attributes(&self) -> Arc<InputAttributes> {
        self.attributes.borrow().clone()
    }
    fn text(&self) -> Utf16Text {
        self.text.borrow().clone()
    }
    fn selection_start(&self) -> i32 {
        self.selection_start.get()
    }
    fn selection_length(&self) -> i32 {
        self.selection_length.get()
    }
    fn replace(&self, start: i32, length: i32, text: Option<&Utf16Text>) -> TargetReplaceResult {
        let original = self.text();
        let start = start.clamp(0, original.len() as i32) as usize;
        let length = length.clamp(0, (original.len() - start) as i32) as usize;
        let text = text.ok_or(TargetReplaceError::ArgumentNull("value"))?;
        let mut result = Vec::with_capacity(original.len() - length + text.len());
        result.extend_from_slice(&original.units()[..start]);
        result.extend_from_slice(text.units());
        result.extend_from_slice(&original.units()[start + length..]);
        *self.text.borrow_mut() = Utf16Text::from_units(result);
        self.selection_start
            .set(i32::try_from(start + text.len()).expect(".NET string length exceeds i32"));
        self.selection_length.set(0);
        self.edited.raise(self);
        Ok(())
    }
    fn select(&self, start: i32, length: i32) {
        let size = self.text.borrow().len() as i32;
        let start = start.clamp(0, size);
        self.selection_start.set(start);
        self.selection_length.set(length.clamp(0, size - start));
        self.edited.raise(self);
    }
    fn perform_enter_action(&self, action: EnterAction) -> bool {
        self.performed_actions.borrow_mut().push(action);
        let handler = self.enter_action_handler();
        handler.is_some_and(|handler| handler(action))
    }
    fn changed(&self) -> &Event<dyn ITextInputTarget> {
        &self.changed
    }
}
