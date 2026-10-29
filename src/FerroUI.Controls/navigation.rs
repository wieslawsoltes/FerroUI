//! WinUI's navigation model of [`Frame`]: the navigation modes and cache modes, the arguments of
//! the navigation events, the journal entries and the journal itself (`Microsoft.UI.Xaml.Navigation`
//! of the type model `dxaml/xcp/tools/XCPTypesAutoGen/XamlOM/Model/Microsoft.UI.Xaml.Navigation.cs`,
//! `dxaml/xcp/dxaml/lib/NavigationHistory.cpp`, `PageStackEntry_Partial.cpp` and
//! `PageStackEntryTrackerCollection.cpp`, microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).
//!
//! The types are in the markup namespace `FerroUI.Navigation` (not in the default XML namespace
//! of the framework), so that this module's [`NavigationEventArgs`] does not collide with the
//! `NavigationEventArgs` of the upstream pages (docs/porting/winui-controls.md, section 7).

mod page_stack_entry;
pub use page_stack_entry::PageStackEntry;

use crate::{Frame, FramePage};
use crate::{IItemsList, ItemsChangedHandler, ItemsSource};
use ferroui_base::collections::{CollectionChangedHandler, FerroList};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{CollectionChange, Event, INotifyCollectionChanged, INotifyPropertyChanged};
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::{ferro_markup_enum, ferro_markup_type, BoxedValue, Ref, TypeInfo, WeakRef};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// How a navigation was requested (WinUI's `NavigationMode`, with its values).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NavigationMode {
    /// A new navigation (`Frame::navigate`).
    #[default]
    New = 0,
    /// A navigation back in the journal (`Frame::go_back`).
    Back = 1,
    /// A navigation forward in the journal (`Frame::go_forward`).
    Forward = 2,
    /// A reload of the current page.
    Refresh = 3,
}

/// Whether a frame keeps a page of a type to show it again (WinUI's `NavigationCacheMode`, with its
/// values).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NavigationCacheMode {
    /// The page is created for each navigation.
    #[default]
    Disabled = 0,
    /// The page is kept whatever the cache size of the frame.
    Required = 1,
    /// The page is kept while the cache of the frame has room for it (`Frame::cache_size`).
    Enabled = 2,
}

/// A navigation or an edit of the journal that failed. Where WinUI returns a failing `HRESULT`
/// (or throws in C#), FerroUI returns this error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationError {
    /// No page type was given (`SourcePageType` set to null).
    MissingSourcePageType,
    /// The type does not derive from [`FramePage`].
    NotAPage(String),
    /// The type has neither a default constructor nor a page factory of the frame.
    MissingConstructor(String),
    /// The page factory of the frame failed, with its message.
    FactoryFailed(String),
    /// The page factory created a page of another type.
    FactoryTypeMismatch {
        /// The type of the navigation.
        expected: String,
        /// The type of the page the factory created.
        actual: String,
    },
    /// The journal was edited during a navigation.
    NavigationPending,
    /// The entry belongs to the journal of another frame.
    EntryOwnedByAnotherFrame,
    /// The index is outside the journal.
    IndexOutOfRange {
        /// The index.
        index: usize,
        /// The number of entries.
        count: usize,
    },
    /// `go_back` with an empty back stack.
    NoBackEntry,
    /// `go_forward` with an empty forward stack.
    NoForwardEntry,
    /// A negative `CacheSize`.
    InvalidCacheSize(i32),
}

impl fmt::Display for NavigationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSourcePageType => f.write_str("SourcePageType must identify a page type."),
            Self::NotAPage(name) => write!(f, "'{name}' does not derive from FerroUI.Controls.FramePage."),
            Self::MissingConstructor(name) => write!(f, "'{name}' has no default constructor or registered page factory."),
            Self::FactoryFailed(message) => f.write_str(message),
            Self::FactoryTypeMismatch { expected, actual } => write!(f, "The factory for '{expected}' returned '{actual}'."),
            Self::NavigationPending => f.write_str("Navigation history cannot be modified during a pending navigation."),
            Self::EntryOwnedByAnotherFrame => f.write_str("The page stack entry belongs to another frame."),
            Self::IndexOutOfRange { index, count } => write!(f, "History index {index} is out of range for {count} entries."),
            Self::NoBackEntry => f.write_str("The frame has no back entry."),
            Self::NoForwardEntry => f.write_str("The frame has no forward entry."),
            Self::InvalidCacheSize(size) => write!(f, "CacheSize must be nonnegative, got {size}."),
        }
    }
}

impl std::error::Error for NavigationError {}

/// The options of one navigation (WinUI's `FrameNavigationOptions`). Clones share the values.
///
/// Missing (docs/porting/winui-controls.md, section 10.4): `TransitionInfoOverride`, FerroUI's
/// frame has no navigation transitions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameNavigationOptions {
    is_navigation_stack_enabled: Rc<Cell<bool>>,
}

impl Default for FrameNavigationOptions {
    fn default() -> Self {
        Self { is_navigation_stack_enabled: Rc::new(Cell::new(true)) }
    }
}

impl FrameNavigationOptions {
    /// Creates the options of a navigation that is recorded in the journal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets whether the navigation is recorded in the journal. `true` by default.
    pub fn is_navigation_stack_enabled(&self) -> bool {
        self.is_navigation_stack_enabled.get()
    }

    /// Sets whether the navigation is recorded in the journal.
    pub fn set_is_navigation_stack_enabled(&self, value: bool) {
        self.is_navigation_stack_enabled.set(value);
    }
}

/// The arguments of `Frame.Navigating` and `FramePage::on_navigating_from` (WinUI's
/// `NavigatingCancelEventArgs`). Clones share the cancellation, also the copies markup handlers
/// receive.
#[derive(Clone)]
pub struct NavigatingCancelEventArgs {
    cancel: Rc<Cell<bool>>,
    navigation_mode: NavigationMode,
    source_page_type: &'static TypeInfo,
    parameter: Option<BoxedValue>,
}

impl PartialEq for NavigatingCancelEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.cancel, &other.cancel)
    }
}

impl NavigatingCancelEventArgs {
    pub(crate) fn new(mode: NavigationMode, entry: &PageStackEntry) -> Self {
        Self {
            cancel: Rc::new(Cell::new(false)),
            navigation_mode: mode,
            source_page_type: entry.source_page_type(),
            parameter: entry.parameter(),
        }
    }

    /// Gets whether the navigation is cancelled.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Cancels the navigation (`true`) or lets it go on.
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }

    /// Gets how the navigation was requested.
    pub fn navigation_mode(&self) -> NavigationMode {
        self.navigation_mode
    }

    /// Gets the type of the page the frame navigates to.
    pub fn source_page_type(&self) -> &'static TypeInfo {
        self.source_page_type
    }

    /// Gets the parameter of the navigation.
    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.clone()
    }
}

/// The arguments of `Frame.Navigated`, `Frame.NavigationStopped` and the navigation callbacks of
/// [`FramePage`] (WinUI's `NavigationEventArgs`). It is `ferroui_controls::navigation::NavigationEventArgs`;
/// the view builders name it `FrameNavigationEventArgs`, beside the upstream `NavigationEventArgs`
/// of the pages.
#[derive(Clone)]
pub struct NavigationEventArgs {
    identity: Rc<()>,
    content: Option<Ref<FramePage>>,
    navigation_mode: NavigationMode,
    source_page_type: &'static TypeInfo,
    parameter: Option<BoxedValue>,
}

impl PartialEq for NavigationEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.identity, &other.identity)
    }
}

impl NavigationEventArgs {
    pub(crate) fn new(content: Option<Ref<FramePage>>, mode: NavigationMode, entry: &PageStackEntry) -> Self {
        Self {
            identity: Rc::new(()),
            content,
            navigation_mode: mode,
            source_page_type: entry.source_page_type(),
            parameter: entry.parameter(),
        }
    }

    /// Gets the page the frame shows after the navigation; `None` for a stopped navigation.
    pub fn content(&self) -> Option<Ref<FramePage>> {
        self.content.clone()
    }

    /// Gets how the navigation was requested.
    pub fn navigation_mode(&self) -> NavigationMode {
        self.navigation_mode
    }

    /// Gets the type of the page of the navigation.
    pub fn source_page_type(&self) -> &'static TypeInfo {
        self.source_page_type
    }

    /// Gets the parameter of the navigation.
    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.clone()
    }
}

/// The arguments of `Frame.NavigationFailed` (WinUI's `NavigationFailedEventArgs`). Clones share
/// `Handled`.
#[derive(Clone, Debug)]
pub struct NavigationFailedEventArgs {
    handled: Rc<Cell<bool>>,
    source_page_type: &'static TypeInfo,
    exception: NavigationError,
}

impl PartialEq for NavigationFailedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.handled, &other.handled)
    }
}

impl NavigationFailedEventArgs {
    pub(crate) fn new(source_page_type: &'static TypeInfo, exception: NavigationError) -> Self {
        Self { handled: Rc::new(Cell::new(false)), source_page_type, exception }
    }

    /// Gets whether a handler handled the failure.
    pub fn handled(&self) -> bool {
        self.handled.get()
    }

    /// Marks the failure handled: `Frame::navigate` then returns `Ok(false)` instead of the error.
    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }

    /// Gets the type of the page the navigation failed to show.
    pub fn source_page_type(&self) -> &'static TypeInfo {
        self.source_page_type
    }

    /// Gets the error (WinUI's `Exception`).
    pub fn exception(&self) -> &NavigationError {
        &self.exception
    }
}

struct NavigationStackData {
    items: FerroList<Ref<PageStackEntry>>,
    frame: RefCell<Option<WeakRef<Frame>>>,
}

/// The back or the forward stack of a frame (WinUI's `IVector<PageStackEntry>` of
/// `Frame.BackStack` and `Frame.ForwardStack`): a live, observable list of entries. Clones are the
/// same list.
///
/// An edit is rejected during a navigation, with an index outside the list, or with an entry of the
/// journal of another frame (`NavigationHistory.cpp`, `PageStackEntryTrackerCollection.cpp`); the
/// edits return a `Result` where WinUI returns a failing `HRESULT`.
#[derive(Clone)]
pub struct NavigationStack(Rc<NavigationStackData>);

impl PartialEq for NavigationStack {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl NavigationStack {
    pub(crate) fn new() -> Self {
        Self(Rc::new(NavigationStackData { items: FerroList::new(), frame: RefCell::new(None) }))
    }

    pub(crate) fn attach(&self, frame: &Ref<Frame>) {
        *self.0.frame.borrow_mut() = Some(frame.downgrade());
        let weak = frame.downgrade();
        self.0.items.add_collection_changed(Rc::new(move |_| {
            if let Some(frame) = weak.upgrade() {
                frame.history_changed();
            }
        }));
    }

    fn frame(&self) -> Option<Ref<Frame>> {
        self.0.frame.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn validate_change(&self) -> Result<(), NavigationError> {
        if self.frame().is_some_and(|frame| frame.is_navigation_pending()) {
            Err(NavigationError::NavigationPending)
        } else {
            Ok(())
        }
    }

    fn validate_entry(&self, entry: &PageStackEntry) -> Result<(), NavigationError> {
        if let Some(owner) = entry.frame() {
            if self.frame().is_none_or(|frame| !owner.ptr_eq(&frame)) {
                return Err(NavigationError::EntryOwnedByAnotherFrame);
            }
        }
        Ok(())
    }

    fn release(&self, entry: &PageStackEntry) {
        if let Some(frame) = self.frame() {
            if !frame.retains_entry(entry) {
                entry.set_frame(None);
            }
        } else {
            entry.set_frame(None);
        }
    }

    /// Gets the number of entries.
    pub fn count(&self) -> usize {
        self.0.items.count()
    }

    /// Gets whether the stack has no entry.
    pub fn is_empty(&self) -> bool {
        self.0.items.is_empty()
    }

    /// Gets the entry at `index`.
    pub fn get(&self, index: usize) -> Option<Ref<PageStackEntry>> {
        self.0.items.try_get(index)
    }

    /// Gets the entries, oldest first.
    pub fn snapshot(&self) -> Rc<Vec<Ref<PageStackEntry>>> {
        self.0.items.snapshot()
    }

    /// Adds a handler of the changes of the list; returns its token.
    pub fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<Ref<PageStackEntry>>>) -> u64 {
        self.0.items.add_collection_changed(handler)
    }

    /// Removes the handler of `token`.
    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.0.items.remove_collection_changed(token)
    }

    /// Adds an entry at the end (the newest).
    pub fn add(&self, entry: Ref<PageStackEntry>) -> Result<(), NavigationError> {
        self.insert(self.count(), entry)
    }

    /// Inserts an entry at `index`.
    pub fn insert(&self, index: usize, entry: Ref<PageStackEntry>) -> Result<(), NavigationError> {
        self.validate_change()?;
        self.validate_entry(&entry)?;
        if index > self.count() {
            return Err(NavigationError::IndexOutOfRange { index, count: self.count() });
        }
        entry.set_frame(self.frame().as_ref());
        self.0.items.insert(index, entry);
        Ok(())
    }

    /// Replaces the entry at `index`.
    pub fn set(&self, index: usize, entry: Ref<PageStackEntry>) -> Result<(), NavigationError> {
        self.validate_change()?;
        self.validate_entry(&entry)?;
        let old = self.get(index).ok_or(NavigationError::IndexOutOfRange { index, count: self.count() })?;
        entry.set_frame(self.frame().as_ref());
        self.0.items.set(index, entry);
        self.release(&old);
        Ok(())
    }

    /// Removes the entry at `index`.
    pub fn remove_at(&self, index: usize) -> Result<(), NavigationError> {
        self.validate_change()?;
        let old = self.get(index).ok_or(NavigationError::IndexOutOfRange { index, count: self.count() })?;
        self.0.items.remove_at(index);
        self.release(&old);
        Ok(())
    }

    /// Removes every entry.
    pub fn clear(&self) -> Result<(), NavigationError> {
        self.validate_change()?;
        self.clear_internal();
        Ok(())
    }

    pub(crate) fn push_internal(&self, entry: Ref<PageStackEntry>) {
        entry.set_frame(self.frame().as_ref());
        self.0.items.add(entry);
    }

    pub(crate) fn pop_internal(&self) {
        if let Some(index) = self.count().checked_sub(1) {
            let old = self.0.items.get(index);
            self.0.items.remove_at(index);
            self.release(&old);
        }
    }

    pub(crate) fn clear_internal(&self) {
        let old = self.snapshot();
        self.0.items.clear();
        for entry in old.iter() {
            self.release(entry);
        }
    }
}

impl INotifyCollectionChanged for NavigationStack {
    fn collection_changed(&self) -> &Event<CollectionChange> {
        self.0.items.collection_changed()
    }
}

impl INotifyPropertyChanged for NavigationStack {
    fn property_changed(&self) -> &Event<str> {
        self.0.items.property_changed()
    }
}

impl IItemsList for NavigationStack {
    fn count(&self) -> usize {
        NavigationStack::count(self)
    }
    fn get_at(&self, index: usize) -> Option<BoxedValue> {
        self.0.items.get_at(index)
    }
    fn index_of(&self, item: &Option<BoxedValue>) -> i32 {
        IItemsList::index_of(&self.0.items, item)
    }
    fn is_notifying(&self) -> bool {
        true
    }
    fn add_collection_changed(&self, handler: Rc<ItemsChangedHandler>) -> Option<u64> {
        IItemsList::add_collection_changed(&self.0.items, handler)
    }
    fn remove_collection_changed(&self, token: u64) {
        IItemsList::remove_collection_changed(&self.0.items, token);
    }
    fn identity(&self) -> Option<usize> {
        Some(Rc::as_ptr(&self.0) as usize)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl From<NavigationStack> for ItemsSource {
    fn from(stack: NavigationStack) -> Self {
        let list: Rc<dyn IItemsList> = Rc::new(stack);
        Self::from(list)
    }
}

/// The index of an indexer of markup as an index of the list; a negative index is outside it.
fn stack_entry(stack: &NavigationStack, index: i32) -> Result<Ref<PageStackEntry>, NavigationError> {
    let count = stack.count();
    usize::try_from(index)
        .ok()
        .and_then(|index| stack.get(index))
        .ok_or(NavigationError::IndexOutOfRange { index: usize::try_from(index).unwrap_or(usize::MAX), count })
}

ferro_markup_enum!(NavigationMode { New, Back, Forward, Refresh }, { namespace: "FerroUI.Navigation" });
ferro_markup_enum!(NavigationCacheMode { Disabled, Required, Enabled }, { namespace: "FerroUI.Navigation" });
ferro_markup_type!(class FrameNavigationOptions {
    namespace: "FerroUI.Navigation",
    handles: [FrameNavigationOptions, Option<FrameNavigationOptions>],
    constructors: [() => FrameNavigationOptions::new],
    properties: [
        IsNavigationStackEnabled: bool { get: FrameNavigationOptions::is_navigation_stack_enabled, set: FrameNavigationOptions::set_is_navigation_stack_enabled },
    ],
});
ferro_markup_type!(class NavigatingCancelEventArgs {
    namespace: "FerroUI.Navigation",
    handles: [NavigatingCancelEventArgs],
    properties: [
        Cancel: bool { get: NavigatingCancelEventArgs::cancel, set: NavigatingCancelEventArgs::set_cancel },
        NavigationMode: NavigationMode { get: NavigatingCancelEventArgs::navigation_mode },
        SourcePageType: &'static TypeInfo { get: NavigatingCancelEventArgs::source_page_type },
        Parameter: Option<BoxedValue> { get: NavigatingCancelEventArgs::parameter },
    ],
});
ferro_markup_type!(class NavigationEventArgs {
    namespace: "FerroUI.Navigation",
    handles: [NavigationEventArgs],
    properties: [
        Content: Option<Ref<FramePage>> { get: NavigationEventArgs::content },
        NavigationMode: NavigationMode { get: NavigationEventArgs::navigation_mode },
        SourcePageType: &'static TypeInfo { get: NavigationEventArgs::source_page_type },
        Parameter: Option<BoxedValue> { get: NavigationEventArgs::parameter },
    ],
});
ferro_markup_type!(class NavigationFailedEventArgs {
    namespace: "FerroUI.Navigation",
    handles: [NavigationFailedEventArgs],
    properties: [
        Handled: bool { get: NavigationFailedEventArgs::handled, set: NavigationFailedEventArgs::set_handled },
        SourcePageType: &'static TypeInfo { get: NavigationFailedEventArgs::source_page_type },
        Exception: String { get: |args: &NavigationFailedEventArgs| args.exception().to_string() },
    ],
});
ferro_markup_type!(class NavigationStack {
    namespace: "FerroUI.Navigation",
    handles: [NavigationStack, Option<NavigationStack>],
    properties: [Count: i32 { get: |stack: &NavigationStack| i32::try_from(stack.count()).unwrap_or(i32::MAX) }],
    methods: [
        try fn Add(Ref<PageStackEntry>) => NavigationStack::add,
        try fn Clear() => NavigationStack::clear,
        try fn RemoveAt(usize) => NavigationStack::remove_at,
    ],
    indexers: [
        (i32) -> Ref<PageStackEntry> { try_get: |stack: &NavigationStack, index: i32| stack_entry(stack, index) },
    ],
});

/// The markup types of this module, listed with the value types of the crate
/// (`markup_types/values.rs`).
pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[
    <NavigationMode as MarkupTyped>::MARKUP,
    <NavigationCacheMode as MarkupTyped>::MARKUP,
    <FrameNavigationOptions as MarkupTyped>::MARKUP,
    <NavigatingCancelEventArgs as MarkupTyped>::MARKUP,
    <NavigationEventArgs as MarkupTyped>::MARKUP,
    <NavigationFailedEventArgs as MarkupTyped>::MARKUP,
    <NavigationStack as MarkupTyped>::MARKUP,
];

/// Registers the nullable forms of the value types of this module, and the journal as an items
/// source and a notifying collection of bindings.
pub(crate) fn register_value_types() {
    ValueTypes::register_nullable::<NavigationMode>();
    ValueTypes::register_nullable::<NavigationCacheMode>();
    ValueTypes::register_nullable::<NavigationStack>();
    ValueTypes::register_nullable::<FrameNavigationOptions>();
    ItemsSource::register_binding_conversion::<NavigationStack>();
    ferroui_base::data::model::ModelTypes::register_notifying_collection::<NavigationStack>();
}
