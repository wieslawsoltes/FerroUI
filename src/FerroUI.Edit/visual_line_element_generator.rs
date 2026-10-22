use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::i_text_view_connect::ITextViewConnect;
use crate::visual_line_element::VisualLineElement;
use crate::visual_line_error::VisualLineError;
use std::ops::Deref;
use std::rc::Rc;

/// Replaces document text with visual elements while a line is constructed.
/// Context is borrowed for each call; a generator never retains a borrowed line.
pub trait VisualLineElementGenerator: ITextViewConnect {
    fn start_generation(
        &self,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<(), VisualLineError> {
        Ok(())
    }
    fn finish_generation(&self) {}
    fn runs_on_long_lines(&self) -> bool {
        false
    }
    fn get_first_interested_offset(
        &self,
        start_offset: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError>;
    fn construct_element(
        &self,
        offset: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError>;
}

/// A shared, identity-preserving generator value for native observable collections.
#[derive(Clone)]
pub struct VisualLineElementGeneratorRef(pub Rc<dyn VisualLineElementGenerator>);
impl PartialEq for VisualLineElementGeneratorRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Deref for VisualLineElementGeneratorRef {
    type Target = dyn VisualLineElementGenerator;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl<T: VisualLineElementGenerator + 'static> From<Rc<T>> for VisualLineElementGeneratorRef {
    fn from(value: Rc<T>) -> Self {
        Self(value)
    }
}
