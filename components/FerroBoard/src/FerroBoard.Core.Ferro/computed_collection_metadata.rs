//! Native computed rows, shared keys and explicit lazy-query snapshots for markup.
use ferroboard_core::layouts::{
    ComputedKey, ComputedKeys, ComputedRow, ComputedRows, ResourceTextListError,
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
};
use std::rc::Rc;
type Result<T> = std::result::Result<T, MarkupInvokeError>;
fn index(value: i32) -> Result<usize> {
    usize::try_from(value).map_err(|_| {
        crate::resource_error_metadata::failure(ResourceTextListError::IndexOutOfRange)
    })
}
fn count(value: usize) -> Result<i32> {
    i32::try_from(value).map_err(|_| {
        MarkupInvokeError::Failed("computed collection count exceeds markup i32".into())
    })
}
#[derive(Clone)]
pub struct ComputedKeyReadOnlyList(ComputedRow);
impl PartialEq for ComputedKeyReadOnlyList {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl Eq for ComputedKeyReadOnlyList {}
impl ComputedKeyReadOnlyList {
    pub fn from_row(value: ComputedRow) -> Self {
        Self(value)
    }
    pub fn source(&self) -> ComputedRow {
        self.0.clone()
    }
    pub fn count(&self) -> Result<i32> {
        count(self.0.len())
    }
    pub fn get(&self, value: i32) -> Result<Option<Rc<ComputedKey>>> {
        self.0
            .get_nullable(index(value)?)
            .map_err(crate::resource_error_metadata::failure)
    }
}
#[derive(Clone)]
pub struct ComputedRowsReadOnlyList(ComputedRows);
impl PartialEq for ComputedRowsReadOnlyList {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}
impl Eq for ComputedRowsReadOnlyList {}
impl ComputedRowsReadOnlyList {
    pub fn from_rows(value: ComputedRows) -> Self {
        Self(value)
    }
    pub fn source(&self) -> ComputedRows {
        self.0.clone()
    }
    pub fn count(&self) -> Result<i32> {
        count(self.0.len())
    }
    pub fn get(&self, value: i32) -> Result<Option<ComputedKeyReadOnlyList>> {
        self.0
            .get_nullable(index(value)?)
            .map(|value| value.map(ComputedKeyReadOnlyList::from_row))
            .map_err(crate::resource_error_metadata::failure)
    }
}
/// Owns a lazy query over actual Core rows; every iterator starts independently.
#[derive(Clone)]
pub struct ComputedKeyQuery(Rc<ComputedKeys>);
impl PartialEq for ComputedKeyQuery {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for ComputedKeyQuery {}
impl ComputedKeyQuery {
    pub fn from_query(value: ComputedKeys) -> Self {
        Self(Rc::new(value))
    }
    pub fn iter(&self) -> ComputedKeys {
        self.0.iter()
    }
    pub fn snapshot(&self) -> Result<ComputedKeyReadOnlyList> {
        let values = self
            .iter()
            .map(|value| value.map_err(crate::derived_key_metadata::failure))
            .collect::<Result<Vec<_>>>()?;
        Ok(ComputedKeyReadOnlyList::from_row(
            ComputedRow::from_nullable(values),
        ))
    }
}
#[doc(hidden)]
pub struct ComputedKeyReadOnlyListMetadata;
#[doc(hidden)]
pub struct ComputedRowsReadOnlyListMetadata;
#[doc(hidden)]
pub struct ComputedKeyQueryMetadata;
ferroui_base::ferro_markup_type!(class ComputedKeyReadOnlyListMetadata as "ComputedKeyList"{namespace:"FerroBoard.Layouts",this:ComputedKeyReadOnlyList,handles:[ComputedKeyReadOnlyList,Option<ComputedKeyReadOnlyList>],properties:[Count:i32{try_get:ComputedKeyReadOnlyList::count}],indexers:[(i32)->Option<Rc<ComputedKey>>{try_get:ComputedKeyReadOnlyList::get}]});
ferroui_base::ferro_markup_type!(class ComputedRowsReadOnlyListMetadata as "ComputedRows"{namespace:"FerroBoard.Layouts",this:ComputedRowsReadOnlyList,handles:[ComputedRowsReadOnlyList,Option<ComputedRowsReadOnlyList>],properties:[Count:i32{try_get:ComputedRowsReadOnlyList::count}],indexers:[(i32)->Option<ComputedKeyReadOnlyList>{try_get:ComputedRowsReadOnlyList::get}]});
ferroui_base::ferro_markup_type!(class ComputedKeyQueryMetadata as "ComputedKeyQuery"{namespace:"FerroBoard.Layouts",this:ComputedKeyQuery,handles:[ComputedKeyQuery,Option<ComputedKeyQuery>],methods:[try fn Snapshot()->ComputedKeyReadOnlyList=>ComputedKeyQuery::snapshot]});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::computed_collection_metadata::ComputedKeyReadOnlyListMetadata,crate::computed_collection_metadata::ComputedRowsReadOnlyListMetadata,crate::computed_collection_metadata::ComputedKeyQueryMetadata],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[
        ComputedKeyReadOnlyListMetadata::MARKUP,
        ComputedRowsReadOnlyListMetadata::MARKUP,
        ComputedKeyQueryMetadata::MARKUP,
    ]);
    ValueTypes::register_global(|| {
        ValueTypes::register_nullable::<ComputedKeyReadOnlyList>();
        ValueTypes::register_nullable::<ComputedRowsReadOnlyList>();
        ValueTypes::register_nullable::<ComputedKeyQuery>();
    });
}
