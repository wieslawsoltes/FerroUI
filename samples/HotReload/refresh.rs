//! Refresh cached application references without rerunning component constructors.
use ferroui_base::{controls::NameScopeRef, Ref, StyledElement};
use ferroui_controls::TextBlock;
use ferroui_hot_reload::{ComponentRefresh, PreparedReload, ReloadError};
use std::{cell::RefCell, rc::Rc};

#[derive(Default)]
pub(crate) struct CounterReferences {
    label: Rc<RefCell<Option<Ref<TextBlock>>>>,
}
impl CounterReferences {
    pub fn update(&self, count: i32) {
        if let Some(label) = &*self.label.borrow() {
            label.set_text(Some(&format!("Counter: {count}")));
        }
    }
}
struct Change {
    slot: Rc<RefCell<Option<Ref<TextBlock>>>>,
    before: Option<Ref<TextBlock>>,
    after: Ref<TextBlock>,
}
impl ComponentRefresh for CounterReferences {
    fn prepare(
        &self,
        _: &Ref<StyledElement>,
        scope: Option<&NameScopeRef>,
    ) -> Result<Box<dyn PreparedReload>, ReloadError> {
        let after = scope
            .and_then(|scope| scope.find("CounterStatus"))
            .and_then(|control| control.cast::<TextBlock>())
            .ok_or_else(|| ReloadError::Failed("CounterStatus must remain a TextBlock".into()))?;
        Ok(Box::new(Change {
            slot: self.label.clone(),
            before: self.label.borrow().clone(),
            after,
        }))
    }
}
impl PreparedReload for Change {
    fn commit(&mut self) -> Result<(), ReloadError> {
        // Newly authored Text/bindings take precedence over the application's display state.
        if !self.after.is_set(TextBlock::text_property()) {
            self.after.set_text(Some(&format!(
                "Counter: {}",
                super::COUNT.with(std::cell::Cell::get)
            )));
        }
        *self.slot.borrow_mut() = Some(self.after.clone());
        Ok(())
    }
    fn rollback(&mut self) {
        *self.slot.borrow_mut() = self.before.clone();
    }
}
