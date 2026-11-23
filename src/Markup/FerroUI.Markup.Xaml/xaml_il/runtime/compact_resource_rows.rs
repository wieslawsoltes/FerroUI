//! Indexed compiled resource rows, with the development capture path preserved.

use super::*;

pub(super) trait Row: 'static {
    fn info(&self) -> (&'static str, i32, i32);
    fn build(&self, provider: &Rc<dyn IServiceProvider>, document: &DocumentInfo) -> Result<MarkupValue, XamlLoadError>;
}

macro_rules! row {
    ($type:ty, $build:ident) => {
        impl Row for $type {
            fn info(&self) -> (&'static str, i32, i32) { (self.key, self.line, self.position) }
            fn build(&self, provider: &Rc<dyn IServiceProvider>, document: &DocumentInfo) -> Result<MarkupValue, XamlLoadError> {
                $build(provider, document, self)
            }
        }
    };
}
row!(ResourceAliasRow, build_resource_alias);
row!(LiteralBrushRow, build_literal_brush);
row!(DynamicBrushRow, build_dynamic_brush);
row!(StaticBrushRow, build_static_brush);

/// Where the rows of a table come from: a slice of rows, or a packed table (#855) whose records
/// are decoded when a row is read. A row that cannot be read is the error of the table.
pub(super) trait RowSource: Copy + 'static {
    type Row: Row + Copy;
    fn len(&self) -> usize;
    fn row(&self, index: usize) -> Result<Self::Row, XamlLoadError>;
}

impl<R: Row + Copy> RowSource for &'static [R] {
    type Row = R;
    fn len(&self) -> usize { <[R]>::len(self) }
    fn row(&self, index: usize) -> Result<R, XamlLoadError> {
        self.get(index).copied().ok_or_else(|| super::packed::out_of_range(index))
    }
}

#[cfg(any(feature = "source-overlays", all(feature = "hot-reload", debug_assertions)))]
pub(super) fn add_rows<S: RowSource>(
    dictionary: &Ref<ResourceDictionary>, context: &Rc<XamlIlContext>, document: &'static DocumentInfo,
    rows: S,
) -> Result<(), XamlLoadError> {
    for index in 0..rows.len() {
        let row = rows.row(index)?;
        let builder = DeferredContentBuilder::try_new(move |provider| row.build(provider, document));
        let (key, line, position) = row.info();
        add_deferred_row(dictionary, key, context, builder, line, position)?;
    }
    Ok(())
}

#[cfg(not(any(feature = "source-overlays", all(feature = "hot-reload", debug_assertions))))]
mod indexed {
    use super::*;
    use std::cell::RefCell;
    use ferroui_base::controls::{DeferredContentFailed, DeferredResourceGroup, IDeferredResourceGroup, ResourceKey};
    use super::super::super::xaml_il_runtime_helpers::CompactDeferredContext;

    struct Rows<S: RowSource> {
        rows: S,
        document: &'static DocumentInfo,
        contexts: RefCell<Vec<Option<CompactDeferredContext>>>,
    }

    impl<S: RowSource> IDeferredResourceGroup for Rows<S> {
        fn try_build(
            &self, index: usize, provider: Option<&Rc<dyn IServiceProvider>>,
        ) -> Result<MarkupValue, DeferredContentFailed> {
            // Release the borrow before application code runs: a resource build
            // can reenter registration/lookup on the same dictionary.
            let context = self.contexts.borrow()[index].as_ref().expect("an indexed row has a compact capture").clone();
            let built = context.try_build(provider, |provider| self.rows.row(index)?.build(provider, self.document));
            match built {
                Ok(value) => Ok(value),
                Err(error) => {
                    crate::xaml_error_scope::report_or_raise(error);
                    Err(DeferredContentFailed)
                }
            }
        }

        fn release(&self, index: usize) {
            // The dictionary holds the value of the row: its capture is not needed again. The
            // slot stays, so the other rows keep their indices. Dropped after the borrow ends.
            let released = self.contexts.borrow_mut()[index].take();
            drop(released);
        }
    }

    pub(in super::super) fn add_rows<S: RowSource>(
        dictionary: &Ref<ResourceDictionary>, context: &Rc<XamlIlContext>, document: &'static DocumentInfo,
        rows: S,
    ) -> Result<(), XamlLoadError> {
        if rows.len() == 0 { return Ok(()); }
        let source = Rc::new(Rows {
            rows, document, contexts: RefCell::new(Vec::with_capacity(rows.len())),
        });
        let group = DeferredResourceGroup::new(source.clone());
        for index in 0..rows.len() {
            let row = rows.row(index)?;
            let (key, line, position) = row.info();
            // Capture immediately before each insertion. A preceding resource
            // notification may have changed the declaration's parent stack.
            let captured = CompactDeferredContext::capture(&super::super::super::compiled::service_provider(context))
                .map_err(|error| super::super::super::compiled::at("InvalidOperationException", error.message(), line, position))?;
            let key = ResourceKey::from(key);
            let ordinary = if captured.has_owned_values() {
                source.contexts.borrow_mut().push(None);
                Some(captured.into_deferred(super::super::super::DeferredContentBuilder::try_new(
                    move |provider| row.build(provider, document),
                )))
            } else {
                source.contexts.borrow_mut().push(Some(captured));
                None
            };
            if dictionary.contains_key(&key) {
                let error = invoked(Err::<(), _>(format!("An item with the same key has already been added. Key: {key}")), line, position);
                source.contexts.borrow_mut().pop();
                return error;
            }
            match ordinary {
                Some(content) => dictionary.add_deferred(key, content),
                None => dictionary.add_indexed_deferred(key, group.clone(), index),
            }
        }
        Ok(())
    }
}

#[cfg(not(any(feature = "source-overlays", all(feature = "hot-reload", debug_assertions))))]
pub(super) use indexed::add_rows;
