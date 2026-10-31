//! The experimental part of DynamicData (`DynamicData/Experimental`): a watcher of the items of a
//! cache by key ([`IWatcher`], `as_watcher` of [`ExperimentalEx`]).

mod experimental_ex;
mod i_watcher;
mod subject_with_ref_count;
mod watcher;

pub use experimental_ex::ExperimentalEx;
pub use i_watcher::IWatcher;
pub(crate) use subject_with_ref_count::SubjectWithRefCount;
pub(crate) use watcher::Watcher;
