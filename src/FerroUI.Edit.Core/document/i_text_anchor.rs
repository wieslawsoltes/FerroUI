use super::{AnchorMovementType, DocumentError, DocumentSubscription, TextAnchor, TextLocation};
use std::any::Any;
use std::rc::Rc;

/// A live position that follows document edits. Coordinate reads fail after
/// deletion; movement policy and deletion state remain available.
pub trait ITextAnchor: Any {
    fn offset(&self) -> Result<usize, DocumentError>;
    fn location(&self) -> Result<TextLocation, DocumentError>;
    fn movement_type(&self) -> AnchorMovementType;
    fn set_movement_type(&self, value: AnchorMovementType);
    fn survive_deletion(&self) -> bool;
    fn set_survive_deletion(&self, value: bool);
    fn is_deleted(&self) -> bool;
    fn deleted(&self, handler: Rc<dyn Fn()>) -> DocumentSubscription;

    fn line(&self) -> Result<usize, DocumentError> {
        Ok(self.location()?.line)
    }
    fn column(&self) -> Result<usize, DocumentError> {
        Ok(self.location()?.column)
    }

    /// Compares the underlying position identity, including deleted positions.
    /// Separately allocated interface wrappers can refer to the same anchor.
    fn is_same_anchor_as(&self, other: &dyn ITextAnchor) -> bool;
}

impl TextAnchor {
    pub fn as_text_anchor(&self) -> Rc<dyn ITextAnchor> {
        Rc::new(self.clone())
    }
}

impl ITextAnchor for TextAnchor {
    fn offset(&self) -> Result<usize, DocumentError> {
        TextAnchor::offset(self)
    }
    fn location(&self) -> Result<TextLocation, DocumentError> {
        TextAnchor::location(self)
    }
    fn movement_type(&self) -> AnchorMovementType {
        TextAnchor::movement_type(self)
    }
    fn set_movement_type(&self, value: AnchorMovementType) {
        TextAnchor::set_movement_type(self, value);
    }
    fn survive_deletion(&self) -> bool {
        TextAnchor::survive_deletion(self)
    }
    fn set_survive_deletion(&self, value: bool) {
        TextAnchor::set_survive_deletion(self, value);
    }
    fn is_deleted(&self) -> bool {
        TextAnchor::is_deleted(self)
    }
    fn deleted(&self, handler: Rc<dyn Fn()>) -> DocumentSubscription {
        TextAnchor::deleted(self, move || handler())
    }
    fn is_same_anchor_as(&self, other: &dyn ITextAnchor) -> bool {
        let other: &dyn Any = other;
        other
            .downcast_ref::<Self>()
            .is_some_and(|other| self == other)
    }
}
