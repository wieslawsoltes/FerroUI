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

#[cfg(any(feature = "source-overlays", all(feature = "hot-reload", debug_assertions)))]
pub(super) fn add_rows<R: Row>(
    dictionary: &Ref<ResourceDictionary>, context: &Rc<XamlIlContext>, document: &'static DocumentInfo,
    rows: &'static [R],
) -> Result<(), XamlLoadError> {
    for row in rows {
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

    struct Rows<R: Row> {
        rows: &'static [R],
        document: &'static DocumentInfo,
        contexts: RefCell<Vec<Option<CompactDeferredContext>>>,
    }

    impl<R: Row> IDeferredResourceGroup for Rows<R> {
        fn try_build(
            &self, index: usize, provider: Option<&Rc<dyn IServiceProvider>>,
        ) -> Result<MarkupValue, DeferredContentFailed> {
            // Release the borrow before application code runs: a resource build
            // can reenter registration/lookup on the same dictionary.
            let context = self.contexts.borrow()[index].as_ref().expect("an indexed row has a compact capture").clone();
            match context.try_build(provider, |provider| self.rows[index].build(provider, self.document)) {
                Ok(value) => Ok(value),
                Err(error) => match crate::xaml_error_scope::report(error) {
                    Ok(()) => Err(DeferredContentFailed),
                    Err(error) => crate::xaml_error_scope::throw(error),
                },
            }
        }
    }

    pub(in super::super) fn add_rows<R: Row>(
        dictionary: &Ref<ResourceDictionary>, context: &Rc<XamlIlContext>, document: &'static DocumentInfo,
        rows: &'static [R],
    ) -> Result<(), XamlLoadError> {
        if rows.is_empty() { return Ok(()); }
        let source = Rc::new(Rows {
            rows, document, contexts: RefCell::new(Vec::with_capacity(rows.len())),
        });
        let group = DeferredResourceGroup::new(source.clone());
        for (index, row) in rows.iter().enumerate() {
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
