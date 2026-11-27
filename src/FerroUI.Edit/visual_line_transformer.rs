use crate::i_text_run_construction_context::ITextRunConstructionContext;
use crate::i_text_view_connect::ITextViewConnect;
use crate::visual_line::VisualLineBuilder;
use crate::visual_line_element::VisualLineElement;
use crate::visual_line_error::VisualLineError;
use crate::TextArea;
use ferroui_base::media::text_formatting::TextRunProperties;
use ferroui_edit_core::document::DocumentLine;
use std::cell::Cell;
use std::ops::Deref;
use std::rc::Rc;

/// Transforms generated visual elements before native shaping begins.
pub trait IVisualLineTransformer: ITextViewConnect {
    fn transform(
        &self,
        context: &dyn ITextRunConstructionContext,
        line: &mut VisualLineBuilder,
    ) -> Result<(), VisualLineError>;
}

#[derive(Clone)]
pub struct VisualLineTransformerRef(pub Rc<dyn IVisualLineTransformer>);
impl PartialEq for VisualLineTransformerRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Deref for VisualLineTransformerRef {
    type Target = dyn IVisualLineTransformer;
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
impl<T: IVisualLineTransformer + 'static> From<Rc<T>> for VisualLineTransformerRef {
    fn from(value: Rc<T>) -> Self {
        Self(value)
    }
}

pub type ColorizeVisualLine =
    dyn Fn(&dyn ITextRunConstructionContext, &mut VisualLineBuilder) -> Result<(), VisualLineError>;
pub type TextViewConnectionHandler = dyn Fn(&TextArea);

struct TransformationGuard<'a>(&'a Cell<bool>);
impl Drop for TransformationGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

/// Callback-based counterpart of a colorizing base class. Mutable access to the
/// current elements is scoped to the callback and recursive use is rejected.
pub struct ColorizingTransformer {
    colorize: Rc<ColorizeVisualLine>,
    transforming: Cell<bool>,
    added: Option<Rc<TextViewConnectionHandler>>,
    removed: Option<Rc<TextViewConnectionHandler>>,
}
impl ColorizingTransformer {
    pub fn new(
        colorize: impl Fn(
                &dyn ITextRunConstructionContext,
                &mut VisualLineBuilder,
            ) -> Result<(), VisualLineError>
            + 'static,
    ) -> Self {
        Self {
            colorize: Rc::new(colorize),
            transforming: Cell::new(false),
            added: None,
            removed: None,
        }
    }
    pub fn with_view_hooks(
        mut self,
        added: Option<Rc<TextViewConnectionHandler>>,
        removed: Option<Rc<TextViewConnectionHandler>>,
    ) -> Self {
        self.added = added;
        self.removed = removed;
        self
    }
}
impl ITextViewConnect for ColorizingTransformer {
    fn add_to_text_view(&self, view: &TextArea) {
        if let Some(handler) = &self.added {
            handler(view);
        }
    }
    fn remove_from_text_view(&self, view: &TextArea) {
        if let Some(handler) = &self.removed {
            handler(view);
        }
    }
}
impl IVisualLineTransformer for ColorizingTransformer {
    fn transform(
        &self,
        context: &dyn ITextRunConstructionContext,
        line: &mut VisualLineBuilder,
    ) -> Result<(), VisualLineError> {
        if self.transforming.replace(true) {
            return Err(VisualLineError::RecursiveTransformation);
        }
        let _guard = TransformationGuard(&self.transforming);
        (self.colorize)(context, line)
    }
}

/// Restricted document-line view used by a document colorizer.
pub struct DocumentLineColorizer<'a> {
    line: DocumentLine,
    visual_line: &'a mut VisualLineBuilder,
}
impl DocumentLineColorizer<'_> {
    pub fn document_line(&self) -> DocumentLine {
        self.line
    }
    pub fn visual_line(&self) -> &VisualLineBuilder {
        self.visual_line
    }
    pub fn visual_line_mut(&mut self) -> &mut VisualLineBuilder {
        self.visual_line
    }
    pub fn change_line_part(
        &mut self,
        start: usize,
        end: usize,
        change: impl FnMut(&mut dyn VisualLineElement),
    ) -> Result<(), VisualLineError> {
        if start < self.line.offset || start > end || end > self.line.end_offset() {
            return Err(VisualLineError::InvalidRange {
                start,
                end,
                length: self.line.end_offset(),
            });
        }
        self.visual_line.change_document_part(start, end, change)
    }
    pub fn apply_text_run_properties(
        &mut self,
        start: usize,
        end: usize,
        properties: &dyn TextRunProperties,
    ) -> Result<(), VisualLineError> {
        self.change_line_part(start, end, |element| {
            element.data_mut().set_text_run_properties(properties)
        })
    }
}
pub type ColorizeDocumentLine = dyn Fn(
    &dyn ITextRunConstructionContext,
    &mut DocumentLineColorizer<'_>,
) -> Result<(), VisualLineError>;

/// Calls a document-offset colorizer for the first logical line and each visible
/// continuation after a generated multi-line element. Hidden interior lines are skipped.
pub struct DocumentColorizingTransformer {
    colorize: Rc<ColorizeDocumentLine>,
    transforming: Cell<bool>,
    added: Option<Rc<TextViewConnectionHandler>>,
    removed: Option<Rc<TextViewConnectionHandler>>,
}
impl DocumentColorizingTransformer {
    pub fn new(
        colorize: impl Fn(
                &dyn ITextRunConstructionContext,
                &mut DocumentLineColorizer<'_>,
            ) -> Result<(), VisualLineError>
            + 'static,
    ) -> Self {
        Self {
            colorize: Rc::new(colorize),
            transforming: Cell::new(false),
            added: None,
            removed: None,
        }
    }
    pub fn with_view_hooks(
        mut self,
        added: Option<Rc<TextViewConnectionHandler>>,
        removed: Option<Rc<TextViewConnectionHandler>>,
    ) -> Self {
        self.added = added;
        self.removed = removed;
        self
    }
}
impl ITextViewConnect for DocumentColorizingTransformer {
    fn add_to_text_view(&self, view: &TextArea) {
        if let Some(handler) = &self.added {
            handler(view);
        }
    }
    fn remove_from_text_view(&self, view: &TextArea) {
        if let Some(handler) = &self.removed {
            handler(view);
        }
    }
}
impl IVisualLineTransformer for DocumentColorizingTransformer {
    fn transform(
        &self,
        context: &dyn ITextRunConstructionContext,
        line: &mut VisualLineBuilder,
    ) -> Result<(), VisualLineError> {
        if self.transforming.replace(true) {
            return Err(VisualLineError::RecursiveTransformation);
        }
        let _guard = TransformationGuard(&self.transforming);
        let first = line.first_document_line();
        (self.colorize)(
            context,
            &mut DocumentLineColorizer {
                line: first,
                visual_line: line,
            },
        )?;
        let continuation = line.source_lines()?;
        for logical in continuation
            .into_iter()
            .filter(|logical| logical.line_number != first.line_number)
        {
            (self.colorize)(
                context,
                &mut DocumentLineColorizer {
                    line: logical,
                    visual_line: line,
                },
            )?;
        }
        Ok(())
    }
}
