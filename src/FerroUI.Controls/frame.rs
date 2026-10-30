//! WinUI's `Frame` (`dxaml/xcp/dxaml/lib/Frame_Partial.cpp`, `NavigationHistory.cpp` and
//! `NavigationCache.cpp` of microsoft-ui-xaml, commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`;
//! MIT, Copyright (c) Microsoft Corporation, see NOTICE.md).

use crate::navigation::{
    FrameNavigationOptions, NavigatingCancelEventArgs, NavigationCacheMode, NavigationError, NavigationEventArgs,
    NavigationFailedEventArgs, NavigationMode, NavigationStack, PageStackEntry,
};
use crate::primitives::TemplatedControlImpl;
use crate::{ContentControl, ContentControlImpl, Control, ControlImpl, FramePage};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, DirectProperty,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl, StyledProperty, TypeInfo,
    VisualImpl,
};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

/// Creates the pages of a type for a [`Frame`] (`Frame::register_page_factory`), with a recoverable
/// error, in place of the default constructor of the type.
pub type PageFactory = dyn Fn() -> Result<Ref<FramePage>, NavigationError>;

/// The cache size of WinUI's frame (`Frame::InitialTransientCacheSize`).
const INITIAL_TRANSIENT_CACHE_SIZE: i32 = 10;

/// A content control that navigates between pages of the WinUI model (WinUI's `Frame`): it shows a
/// [`FramePage`] of the type of a navigation and keeps a journal of the pages it showed (the back
/// and the forward stack) and a cache of pages (`CacheSize`, `FramePage::navigation_cache_mode`).
///
/// A navigation is synchronous: `navigate`, `go_back` and `go_forward` raise `Navigating`, which
/// can cancel it, call the callbacks of the old and the new page, raise `Navigated`, and return; a
/// navigation requested by one of those handlers while the frame navigates is ignored. A page is
/// created by the page factory registered for its type (`register_page_factory`) or by the default
/// constructor of its type. The upstream pages (`Page`, `NavigationPage`) are another model;
/// docs/porting/winui-controls.md, section 7, compares them.
///
/// The WinUI theme has the template (WinUI's `Frame` style of `generic.xaml`); the Fluent and Simple
/// themes have none. Missing (docs/porting/winui-controls.md, section 10.4): the navigation
/// transitions (`ContentTransitions`, the `NavigationTransitionInfo` of `Navigate`),
/// `GetNavigationState` and `SetNavigationState`.
#[repr(C)]
pub struct Frame {
    base: ContentControl,
    back_stack: NavigationStack,
    forward_stack: NavigationStack,
    current: RefCell<Option<Ref<PageStackEntry>>>,
    pending: RefCell<Option<Ref<PageStackEntry>>>,
    in_navigate: Cell<bool>,
    committing: Cell<bool>,
    can_go_back: Cell<bool>,
    can_go_forward: Cell<bool>,
    back_stack_depth: Cell<i32>,
    source_page_type: Cell<Option<&'static TypeInfo>>,
    cache_size: Cell<i32>,
    factories: RefCell<HashMap<&'static TypeInfo, Rc<PageFactory>>>,
    permanent: RefCell<HashMap<&'static TypeInfo, Ref<FramePage>>>,
    transient: RefCell<VecDeque<(&'static TypeInfo, Ref<FramePage>)>>,
    navigating: HandlerList<dyn Fn(&NavigatingCancelEventArgs)>,
    navigated: HandlerList<dyn Fn(&NavigationEventArgs)>,
    navigation_stopped: HandlerList<dyn Fn(&NavigationEventArgs)>,
    navigation_failed: HandlerList<dyn Fn(&NavigationFailedEventArgs)>,
}

ferro_class!(Frame: ContentControl);
ferro_class_info!(Frame {
    new: Frame::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Content,
        properties: [
            BackStack: NavigationStack { get: Frame::back_stack },
            ForwardStack: NavigationStack { get: Frame::forward_stack },
            SourcePageType: Option<&'static TypeInfo> {
                get: Frame::source_page_type,
                set: |frame: &Ref<Frame>, value: Option<&'static TypeInfo>| frame.set_source_page_type_or_log(value)
            },
        ],
        methods: [
            try fn Navigate(&'static TypeInfo) -> bool => |frame: &Ref<Frame>, ty: &'static TypeInfo| frame.navigate(ty, None),
            try fn Navigate(&'static TypeInfo, Option<BoxedValue>) -> bool => Frame::navigate,
            try fn NavigateToType(&'static TypeInfo, Option<BoxedValue>, FrameNavigationOptions) -> bool => |frame: &Ref<Frame>, ty: &'static TypeInfo, parameter: Option<BoxedValue>, options: FrameNavigationOptions| frame.navigate_to_type(ty, parameter, &options),
            try fn GoBack() => Frame::go_back,
            try fn GoForward() => Frame::go_forward,
        ],
        events: [
            Navigating(Option<BoxedValue>, NavigatingCancelEventArgs) => |this: &Ref<Frame>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.navigating(move |args| { handler.invoke(&[into_markup_value(weak.upgrade()), into_markup_value(args.clone())]); })
            },
            Navigated(Option<BoxedValue>, NavigationEventArgs) => |this: &Ref<Frame>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.navigated(move |args| { handler.invoke(&[into_markup_value(weak.upgrade()), into_markup_value(args.clone())]); })
            },
            NavigationStopped(Option<BoxedValue>, NavigationEventArgs) => |this: &Ref<Frame>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.navigation_stopped(move |args| { handler.invoke(&[into_markup_value(weak.upgrade()), into_markup_value(args.clone())]); })
            },
            NavigationFailed(Option<BoxedValue>, NavigationFailedEventArgs) => |this: &Ref<Frame>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.navigation_failed(move |args| { handler.invoke(&[into_markup_value(weak.upgrade()), into_markup_value(args.clone())]); })
            },
        ],
    },
});
ferro_impl_classes!(Frame: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl, TemplatedControlImpl, ContentControlImpl);

impl FerroObjectImpl for Frame {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.back_stack.attach(&this.to_ref());
        this.forward_stack.attach(&this.to_ref());
    }
}

ferro_properties! {
    impl Frame {
        /// Defines the `CanGoBack` property: whether the back stack has an entry (read-only).
        pub fn can_go_back_property() -> DirectProperty<Frame, bool> {
            FerroProperty::register_direct::<Frame, _>("CanGoBack", Frame::can_go_back, None, false)
        }
        /// Defines the `CanGoForward` property: whether the forward stack has an entry (read-only).
        pub fn can_go_forward_property() -> DirectProperty<Frame, bool> {
            FerroProperty::register_direct::<Frame, _>("CanGoForward", Frame::can_go_forward, None, false)
        }
        /// Defines the `BackStackDepth` property: the number of entries of the back stack (read-only).
        pub fn back_stack_depth_property() -> DirectProperty<Frame, i32> {
            FerroProperty::register_direct::<Frame, _>("BackStackDepth", Frame::back_stack_depth, None, 0)
        }
        /// Defines the `SourcePageType` property: the type of the page of the last navigation; setting
        /// it navigates to a page of the type. An invalid value is logged and ignored (WinUI's setter
        /// fails with the error of `Navigate`).
        pub fn source_page_type_property() -> DirectProperty<Frame, Option<&'static TypeInfo>> {
            FerroProperty::register_direct::<Frame, _>(
                "SourcePageType",
                Frame::source_page_type,
                Some(|frame, value| frame.set_source_page_type_or_log(value)),
                None,
            )
        }
        /// Defines the `CurrentSourcePageType` property: the type of the page the frame shows (read-only).
        pub fn current_source_page_type_property() -> DirectProperty<Frame, Option<&'static TypeInfo>> {
            FerroProperty::register_direct::<Frame, _>("CurrentSourcePageType", Frame::current_source_page_type, None, None)
        }
        /// Defines the `CacheSize` property: the number of pages of the cache mode `Enabled` the frame
        /// keeps, 10 by default. A negative value is coerced to 0 and logged.
        pub fn cache_size_property() -> DirectProperty<Frame, i32> {
            FerroProperty::register_direct::<Frame, _>(
                "CacheSize",
                Frame::cache_size,
                Some(|frame, value| frame.set_cache_size_or_log(value)),
                INITIAL_TRANSIENT_CACHE_SIZE,
            )
        }
        /// Defines the `IsNavigationStackEnabled` property: whether navigations are recorded in the
        /// journal. `true` by default.
        pub fn is_navigation_stack_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<Frame, _>("IsNavigationStackEnabled", true)
        }
    }
}

/// Releases the navigation guard, also when an application callback unwinds.
struct NavigationGuard<'a>(&'a Frame);

impl Drop for NavigationGuard<'_> {
    fn drop(&mut self) {
        self.0.pending.borrow_mut().take();
        self.0.committing.set(false);
        self.0.in_navigate.set(false);
    }
}

/// Logs a value of a property of the frame that the frame does not take.
fn log_invalid(message: &str) {
    if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::CONTROL) {
        logger.log(None, message);
    }
}

impl Frame {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            back_stack: NavigationStack::new(),
            forward_stack: NavigationStack::new(),
            current: RefCell::new(None),
            pending: RefCell::new(None),
            in_navigate: Cell::new(false),
            committing: Cell::new(false),
            can_go_back: Cell::new(false),
            can_go_forward: Cell::new(false),
            back_stack_depth: Cell::new(0),
            source_page_type: Cell::new(None),
            cache_size: Cell::new(INITIAL_TRANSIENT_CACHE_SIZE),
            factories: RefCell::new(HashMap::new()),
            permanent: RefCell::new(HashMap::new()),
            transient: RefCell::new(VecDeque::new()),
            navigating: HandlerList::new(),
            navigated: HandlerList::new(),
            navigation_stopped: HandlerList::new(),
            navigation_failed: HandlerList::new(),
        }
    }

    /// Creates a frame with an empty journal.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the back stack (live; clones are the same list).
    pub fn back_stack(&self) -> NavigationStack {
        self.back_stack.clone()
    }

    /// Gets the forward stack (live; clones are the same list).
    pub fn forward_stack(&self) -> NavigationStack {
        self.forward_stack.clone()
    }

    /// Gets whether the back stack has an entry.
    pub fn can_go_back(&self) -> bool {
        self.can_go_back.get()
    }

    /// Gets whether the forward stack has an entry.
    pub fn can_go_forward(&self) -> bool {
        self.can_go_forward.get()
    }

    /// Gets the number of entries of the back stack.
    pub fn back_stack_depth(&self) -> i32 {
        self.back_stack_depth.get()
    }

    /// Gets the type of the page of the last navigation recorded in the journal.
    pub fn source_page_type(&self) -> Option<&'static TypeInfo> {
        self.source_page_type.get()
    }

    /// Gets the type of the page the frame shows, as recorded in the journal.
    pub fn current_source_page_type(&self) -> Option<&'static TypeInfo> {
        self.source_page_type.get()
    }

    /// Gets the number of pages of the cache mode `Enabled` the frame keeps.
    pub fn cache_size(&self) -> i32 {
        self.cache_size.get()
    }

    /// Gets whether navigations are recorded in the journal.
    pub fn is_navigation_stack_enabled(&self) -> bool {
        self.get_value(Self::is_navigation_stack_enabled_property())
    }

    /// Sets whether navigations are recorded in the journal.
    pub fn set_is_navigation_stack_enabled(&self, value: bool) {
        self.set_value(Self::is_navigation_stack_enabled_property(), value);
    }

    /// Gets the page the frame shows.
    pub fn current_page(&self) -> Option<Ref<FramePage>> {
        self.content().and_then(|content| ValueTypes::as_object(&*content)).and_then(|object| object.cast::<FramePage>())
    }

    /// Sets the number of pages of the cache mode `Enabled` the frame keeps, and drops the least
    /// recently shown pages beyond it (`NavigationCache::ChangeTransientCacheSize`). A negative size
    /// is an error and changes nothing.
    pub fn set_cache_size(&self, value: i32) -> Result<(), NavigationError> {
        if value < 0 {
            return Err(NavigationError::InvalidCacheSize(value));
        }
        // The pages dropped from the cache may run code when they are dropped: drop them outside
        // the borrow of the cache.
        let evicted = {
            let mut cache = self.transient.borrow_mut();
            let count = cache.len().saturating_sub(value as usize);
            cache.drain(..count).collect::<Vec<_>>()
        };
        self.set_and_raise_cell(Self::cache_size_property(), &self.cache_size, value);
        drop(evicted);
        Ok(())
    }

    /// The setter of the property `CacheSize` (markup, bindings, styles): WinUI reads the size as
    /// an unsigned number, so a negative size is an unbounded cache there; a value from markup must
    /// not end the application, so a negative size is coerced to 0 and logged.
    fn set_cache_size_or_log(&self, value: i32) {
        if value < 0 {
            log_invalid(&format!("Frame.CacheSize must be 0 or more; {value} is coerced to 0."));
        }
        // A size of 0 or more is always accepted.
        let _ = self.set_cache_size(value.max(0));
    }

    /// Navigates to a page of `value` unless the frame shows a page of it already. `None` is an
    /// error, as in WinUI's `Navigate`. Rust callers get the error; the property setter logs it.
    pub fn set_source_page_type(&self, value: Option<&'static TypeInfo>) -> Result<bool, NavigationError> {
        match value {
            Some(ty) if self.source_page_type() == Some(ty) => Ok(true),
            Some(ty) => self.navigate(ty, None),
            None => Err(NavigationError::MissingSourcePageType),
        }
    }

    /// The setter of the property `SourcePageType`: an invalid type or a failed navigation is
    /// logged, where WinUI's setter fails with the error of `Navigate`.
    fn set_source_page_type_or_log(&self, value: Option<&'static TypeInfo>) {
        if let Err(error) = self.set_source_page_type(value) {
            log_invalid(&format!("Frame.SourcePageType: {error}"));
        }
    }

    /// Registers how the frame creates the pages of `source_page_type`, in place of the default
    /// constructor of the type. Pages already in the cache stay there.
    pub fn register_page_factory(
        &self,
        source_page_type: &'static TypeInfo,
        factory: Rc<PageFactory>,
    ) -> Result<(), NavigationError> {
        if !FramePage::TYPE.is_assignable_from(source_page_type) {
            return Err(NavigationError::NotAPage(source_page_type.full_name()));
        }
        let previous = self.factories.borrow_mut().insert(source_page_type, factory);
        drop(previous);
        Ok(())
    }

    /// Removes the page factory of `source_page_type`; returns whether there was one.
    pub fn unregister_page_factory(&self, source_page_type: &'static TypeInfo) -> bool {
        let removed = self.factories.borrow_mut().remove(source_page_type);
        removed.is_some()
    }

    /// Navigates to a page of `source_page_type` with `parameter` (WinUI's `Navigate`).
    ///
    /// `Ok(true)` when the request was taken, also when a handler cancelled it, as WinUI's
    /// `Navigate` returns; `Ok(false)` for a request made during a navigation and for a failure a
    /// `NavigationFailed` handler handled; the error of a failure no handler handled.
    pub fn navigate(&self, source_page_type: &'static TypeInfo, parameter: Option<BoxedValue>) -> Result<bool, NavigationError> {
        self.navigate_to_type(source_page_type, parameter, &FrameNavigationOptions::default())
    }

    /// Navigates to a page of `source_page_type` with `parameter` and `options` (WinUI's
    /// `NavigateToType`); returns as [`navigate`](Self::navigate).
    pub fn navigate_to_type(
        &self,
        source_page_type: &'static TypeInfo,
        parameter: Option<BoxedValue>,
        options: &FrameNavigationOptions,
    ) -> Result<bool, NavigationError> {
        if self.in_navigate.replace(true) {
            return Ok(false);
        }
        let _guard = NavigationGuard(self);
        let entry = PageStackEntry::new(source_page_type, parameter);
        entry.set_frame(Some(&self.to_ref()));
        self.perform_navigation(entry, NavigationMode::New, options.is_navigation_stack_enabled())
    }

    /// Navigates to the newest entry of the back stack (WinUI's `GoBack`).
    pub fn go_back(&self) -> Result<(), NavigationError> {
        if self.in_navigate.replace(true) {
            return Ok(());
        }
        let _guard = NavigationGuard(self);
        let entry = self
            .back_stack
            .count()
            .checked_sub(1)
            .and_then(|index| self.back_stack.get(index))
            .ok_or(NavigationError::NoBackEntry)?;
        self.perform_navigation(entry, NavigationMode::Back, true).map(|_| ())
    }

    /// Navigates to the newest entry of the forward stack (WinUI's `GoForward`).
    pub fn go_forward(&self) -> Result<(), NavigationError> {
        if self.in_navigate.replace(true) {
            return Ok(());
        }
        let _guard = NavigationGuard(self);
        let entry = self
            .forward_stack
            .count()
            .checked_sub(1)
            .and_then(|index| self.forward_stack.get(index))
            .ok_or(NavigationError::NoForwardEntry)?;
        self.perform_navigation(entry, NavigationMode::Forward, true).map(|_| ())
    }

    /// `Frame::NavigateImpl` and `NavigateWithTransitionInfo`: `Navigating`, the page's
    /// `OnNavigatingFrom`, the page from the cache or a new one, the content, the journal,
    /// `Navigated`, the old page's `OnNavigatedFrom` and the new page's `OnNavigatedTo`.
    fn perform_navigation(
        &self,
        entry: Ref<PageStackEntry>,
        mode: NavigationMode,
        stack_enabled: bool,
    ) -> Result<bool, NavigationError> {
        *self.pending.borrow_mut() = Some(entry.clone());
        let cancel = NavigatingCancelEventArgs::new(mode, &entry);
        for (_, handler) in self.navigating.snapshot().iter() {
            handler(&cancel);
        }
        let old_page = self.current_page();
        if !cancel.cancel() {
            if let Some(page) = &old_page {
                page.on_navigating_from(&cancel);
            }
        }
        if cancel.cancel() {
            let args = NavigationEventArgs::new(None, mode, &entry);
            for (_, handler) in self.navigation_stopped.snapshot().iter() {
                handler(&args);
            }
            return Ok(true);
        }
        let page = match self.get_page(entry.source_page_type()) {
            Ok(page) => page,
            Err(error) => {
                let args = NavigationFailedEventArgs::new(entry.source_page_type(), error.clone());
                for (_, handler) in self.navigation_failed.snapshot().iter() {
                    handler(&args);
                }
                return if args.handled() { Ok(false) } else { Err(error) };
            }
        };
        page.prepare(&self.to_ref(), entry.source_page_type());
        self.set_content(Some(Control::boxed(page.clone())));
        if self.is_navigation_stack_enabled() && stack_enabled {
            self.commit(&entry, mode);
        }
        // A navigation outside the journal changes neither the source type nor the current entry.
        self.pending.borrow_mut().take();
        let args = NavigationEventArgs::new(Some(page.clone()), mode, &entry);
        for (_, handler) in self.navigated.snapshot().iter() {
            handler(&args);
        }
        if let Some(old_page) = old_page {
            old_page.on_navigated_from(&args);
        }
        page.on_navigated_to(&args);
        Ok(true)
    }

    /// `NavigationHistory::CommitNavigation`: the old entry goes to the back stack (a new or a
    /// forward navigation) or to the forward stack (a back navigation).
    fn commit(&self, entry: &Ref<PageStackEntry>, mode: NavigationMode) {
        self.committing.set(true);
        let old = self.current.replace(Some(entry.clone()));
        let (back, forward) = match mode {
            NavigationMode::New => (self.back_stack.count() + usize::from(old.is_some()), 0),
            NavigationMode::Back => {
                (self.back_stack.count().saturating_sub(1), self.forward_stack.count() + usize::from(old.is_some()))
            }
            NavigationMode::Forward => {
                (self.back_stack.count() + usize::from(old.is_some()), self.forward_stack.count().saturating_sub(1))
            }
            NavigationMode::Refresh => (self.back_stack.count(), self.forward_stack.count()),
        };
        self.set_flags(back, forward);
        self.set_source_type(Some(entry.source_page_type()));
        match mode {
            NavigationMode::New => {
                if let Some(old) = old {
                    self.back_stack.push_internal(old);
                }
                self.forward_stack.clear_internal();
            }
            NavigationMode::Back => {
                if let Some(old) = old {
                    self.forward_stack.push_internal(old);
                }
                self.back_stack.pop_internal();
            }
            NavigationMode::Forward => {
                if let Some(old) = old {
                    self.back_stack.push_internal(old);
                }
                self.forward_stack.pop_internal();
            }
            NavigationMode::Refresh => {}
        }
        self.committing.set(false);
    }

    /// `NavigationCache::GetContent`: the cached page of the type, or a new one from the factory
    /// or the default constructor, cached by its cache mode.
    fn get_page(&self, ty: &'static TypeInfo) -> Result<Ref<FramePage>, NavigationError> {
        if !FramePage::TYPE.is_assignable_from(ty) {
            return Err(NavigationError::NotAPage(ty.full_name()));
        }
        if let Some(page) = self.permanent.borrow().get(&ty).cloned() {
            return Ok(page);
        }
        {
            let mut cache = self.transient.borrow_mut();
            if let Some(index) = cache.iter().position(|(key, _)| *key == ty) {
                if let Some(cached) = cache.remove(index) {
                    let page = cached.1.clone();
                    cache.push_back(cached);
                    return Ok(page);
                }
            }
        }
        let factory = self.factories.borrow().get(&ty).cloned();
        let page = match factory {
            Some(factory) => factory()?,
            None => ty
                .create_instance()
                .ok_or_else(|| NavigationError::MissingConstructor(ty.full_name()))?
                .cast::<FramePage>()
                .ok_or_else(|| NavigationError::NotAPage(ty.full_name()))?,
        };
        if !ty.is_assignable_from(page.get_type()) {
            return Err(NavigationError::FactoryTypeMismatch {
                expected: ty.full_name(),
                actual: page.get_type().full_name(),
            });
        }
        // The cache admits pages of the modes Required and Enabled only while CacheSize is not 0;
        // a Required page already admitted stays when the size goes down to 0.
        if self.cache_size() > 0 {
            match page.navigation_cache_mode() {
                NavigationCacheMode::Required => {
                    self.permanent.borrow_mut().insert(ty, page.clone());
                }
                NavigationCacheMode::Enabled => {
                    let removed = {
                        let mut cache = self.transient.borrow_mut();
                        let removed =
                            if cache.len() >= self.cache_size() as usize { cache.pop_front() } else { None };
                        cache.push_back((ty, page.clone()));
                        removed
                    };
                    drop(removed);
                }
                NavigationCacheMode::Disabled => {}
            }
        }
        Ok(page)
    }

    pub(crate) fn remove_cached_page(&self, ty: &'static TypeInfo) {
        let permanent = self.permanent.borrow_mut().remove(&ty);
        let transient = {
            let mut cache = self.transient.borrow_mut();
            cache.iter().position(|(key, _)| *key == ty).and_then(|index| cache.remove(index))
        };
        drop((permanent, transient));
    }

    pub(crate) fn is_navigation_pending(&self) -> bool {
        self.pending.borrow().is_some()
    }

    pub(crate) fn retains_entry(&self, entry: &PageStackEntry) -> bool {
        let entry = entry.to_ref();
        self.current.borrow().as_ref() == Some(&entry)
            || self.pending.borrow().as_ref() == Some(&entry)
            || self.back_stack.snapshot().contains(&entry)
            || self.forward_stack.snapshot().contains(&entry)
    }

    pub(crate) fn history_changed(&self) {
        if !self.committing.get() {
            self.set_flags(self.back_stack.count(), self.forward_stack.count());
        }
    }

    fn set_flags(&self, back: usize, forward: usize) {
        // The whole state is set before the handlers run: a handler of a change can edit the
        // journal again, and its newer state must not be overwritten or reported as stale.
        let next_back = back > 0;
        let next_forward = forward > 0;
        let next_depth = i32::try_from(back).unwrap_or(i32::MAX);
        let old_back = self.can_go_back.replace(next_back);
        let old_forward = self.can_go_forward.replace(next_forward);
        let old_depth = self.back_stack_depth.replace(next_depth);
        if old_back != next_back {
            self.raise_property_changed(
                Self::can_go_back_property().as_property(),
                Some(&old_back),
                &next_back,
                BindingPriority::LocalValue,
                true,
            );
        }
        if old_forward != next_forward && self.can_go_forward.get() == next_forward {
            self.raise_property_changed(
                Self::can_go_forward_property().as_property(),
                Some(&old_forward),
                &next_forward,
                BindingPriority::LocalValue,
                true,
            );
        }
        if old_depth != next_depth && self.back_stack_depth.get() == next_depth {
            self.raise_property_changed(
                Self::back_stack_depth_property().as_property(),
                Some(&old_depth),
                &next_depth,
                BindingPriority::LocalValue,
                true,
            );
        }
    }

    fn set_source_type(&self, ty: Option<&'static TypeInfo>) {
        let old = self.source_page_type.replace(ty);
        if old != ty {
            self.raise_property_changed(
                Self::source_page_type_property().as_property(),
                Some(&old),
                &ty,
                BindingPriority::LocalValue,
                true,
            );
            self.raise_property_changed(
                Self::current_source_page_type_property().as_property(),
                Some(&old),
                &ty,
                BindingPriority::LocalValue,
                true,
            );
        }
    }

    /// Adds a handler of `Navigating`; the disposable removes it.
    pub fn navigating(&self, handler: impl Fn(&NavigatingCancelEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.navigating.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(frame) = weak.upgrade() {
                frame.navigating.remove(token);
            }
        })
    }

    /// Adds a handler of `Navigated`; the disposable removes it.
    pub fn navigated(&self, handler: impl Fn(&NavigationEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.navigated.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(frame) = weak.upgrade() {
                frame.navigated.remove(token);
            }
        })
    }

    /// Adds a handler of `NavigationStopped` (a cancelled navigation); the disposable removes it.
    pub fn navigation_stopped(&self, handler: impl Fn(&NavigationEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.navigation_stopped.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(frame) = weak.upgrade() {
                frame.navigation_stopped.remove(token);
            }
        })
    }

    /// Adds a handler of `NavigationFailed`; the disposable removes it.
    pub fn navigation_failed(&self, handler: impl Fn(&NavigationFailedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.navigation_failed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(frame) = weak.upgrade() {
                frame.navigation_failed.remove(token);
            }
        })
    }
}
