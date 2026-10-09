use crate::animation::Animatable;
use crate::collections::{FerroList, NotifyCollectionChangedAction, NotifyCollectionChangedEventArgs, ResetBehavior};
use crate::controls::{
    Classes, IPseudoClasses, IResourceHost, IResourceNode, ResourceDictionary, ResourceHostRef, ResourceKey,
    ResourceValue, ResourcesChangedEventArgs,
};
use crate::data::BindingPriority;
use crate::logical_tree::{IChildIndexProvider, LogicalTreeAttachmentEventArgs};
use crate::property_store::FrameType;
use crate::reactive::{Disposable, IDisposable};
use crate::styling::{
    DuplicateSetterError, ContainerQuery, ControlTheme, IStyle, IStyleHost, IThemeVariantHost, Style, StyleBase, StyleHostRef,
    StyleInstance, Styles, ThemeVariant,
};
use crate::utilities::HandlerList;
use crate::InitializationError;
use crate::{
    ferro_class, ferro_property, instantiate, AnyValue, BoxedValue, DirectProperty, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Nullable, Ref, StyledProperty,
    StyledPropertyOptions, TypeInfo, WeakRef,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

/// Extends an [`Animatable`] with the following features:
///
/// - An inherited `DataContext`.
/// - Implements the logical tree.
/// - A collection of class strings for custom styling.
/// - Implements styling: tracks the frames applied by styles and themes.
#[repr(C)]
pub struct StyledElement {
    base: Animatable,
    init_count: Cell<u32>,
    name: RefCell<Option<String>>,
    classes: OnceCell<Classes>,
    logical_root: RefCell<Option<WeakRef<StyledElement>>>,
    logical_children: OnceCell<FerroList<Ref<StyledElement>>>,
    parent: RefCell<Option<WeakRef<StyledElement>>>,
    resources: RefCell<Option<Ref<ResourceDictionary>>>,
    styles: OnceCell<Ref<Styles>>,
    styles_applied: Cell<bool>,
    theme_applied: Cell<bool>,
    templated_parent_theme_applied: Cell<bool>,
    templated_parent: RefCell<Option<WeakRef<FerroObject>>>,
    data_context_updating: Cell<bool>,
    implicit_theme: RefCell<ImplicitTheme>,
    last_resources_changed_event_args: Cell<ResourcesChangedEventArgs>,
    is_initialized: Cell<bool>,
    /// True when this element has been removed from a logical children
    /// collection and not added back since. Avoids a costly quadratic
    /// `contains` check when attaching to the logical tree.
    removed_from_logical_parent: Cell<bool>,
    attached_to_logical_tree: HandlerList<dyn Fn(&LogicalTreeAttachmentEventArgs)>,
    detached_from_logical_tree: HandlerList<dyn Fn(&LogicalTreeAttachmentEventArgs)>,
    data_context_changed: HandlerList<dyn Fn()>,
    initialized: HandlerList<dyn Fn()>,
    resources_changed: HandlerList<dyn Fn(&ResourcesChangedEventArgs)>,
    actual_theme_variant_changed: HandlerList<dyn Fn()>,
}

/// The control theme found for an element whose `Theme` property is not set.
enum ImplicitTheme {
    /// The theme has not been looked up yet.
    Unknown,
    /// The lookup found no theme.
    Invalid,
    Theme(Ref<ControlTheme>),
}

ferro_class! {
    StyledElement: Animatable, virtuals StyledElementImpl: FerroObjectImpl {
        /// Signals the beginning of a batch of initialization: styling is
        /// deferred until the matching `end_init`.
        fn begin_init(this);
        /// Signals the end of a batch of initialization: the element is
        /// styled (if it is attached to a rooted logical tree) and
        /// initialized. A style or control theme that cannot be applied (two
        /// setters for the same property) is an error, the counterpart of
        /// the invalid operation `EndInit` raises in the managed original;
        /// what follows the failure is skipped. This is the overridable
        /// member; [`end_init`](StyledElement::end_init) is the form that
        /// panics on the error.
        fn try_end_init(this) -> Result<(), InitializationError>;
        /// Called when the logical children collection changes.
        fn logical_children_collection_changed(this, e: &NotifyCollectionChangedEventArgs<'_, Ref<StyledElement>>);
        /// Called when the styled element is added to a rooted logical tree.
        fn on_attached_to_logical_tree(this, e: &LogicalTreeAttachmentEventArgs);
        /// Called when the styled element is removed from a rooted logical tree.
        fn on_detached_from_logical_tree(this, e: &LogicalTreeAttachmentEventArgs);
        /// Called when the data context changes.
        fn on_data_context_changed(this);
        /// Called when the data context begins updating.
        fn on_data_context_begin_update(this);
        /// Called when the data context finishes updating.
        fn on_data_context_end_update(this);
        /// Called when the control finishes initialization.
        fn on_initialized(this);
        /// The type by which the element is styled. Defaults to the element's
        /// own type; override to be styled as a base class.
        fn style_key_override(this) -> &'static TypeInfo;
        /// Called when the control theme changes.
        fn on_control_theme_changed(this);
        /// Called when the templated parent's control theme changes.
        fn on_templated_parent_control_theme_changed(this);
        /// Detaches all styles from the element (and optionally from its
        /// inheritance children) so that they are re-applied.
        fn invalidate_styles(this, recurse: bool);
        /// Whether the element is the root of a logical tree.
        fn is_logical_root(this) -> bool;
        /// The parent style host of the element: by default the inheritance
        /// parent, when it is a styled element. The root of a tree overrides
        /// this to return the host of the global styles.
        fn styling_parent(this) -> Option<StyleHostRef>;
        /// Notifies the children of the element that the resources visible to
        /// them have changed. The base method notifies the logical children.
        fn notify_child_resources_changed(this, e: ResourcesChangedEventArgs);
        /// The child index provider implemented by the element, if any. Used
        /// to match the children of the element by their position.
        fn child_index_provider(this) -> Option<Rc<dyn IChildIndexProvider>>;
        /// Whether the element is a root of theme variant resolution: at a
        /// root the default theme variant resolves to the variant of the
        /// platform instead of being inherited.
        fn is_theme_variant_root(this) -> bool;
    }
}
crate::ferro_class_info!(StyledElement {
    new: StyledElement::new,
    interfaces: [
        Rc<dyn crate::ISupportInitialize>,
        Rc<dyn crate::INamed>,
        crate::controls::ResourceHostRef,
        crate::styling::StyleHostRef,
    ],
});

impl FerroObjectImpl for StyledElement {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        if this.is_logical_root() {
            *this.logical_root.borrow_mut() = Some(this.to_ref().downgrade());
        }
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::theme_property().as_property() {
            this.on_control_theme_changed();
        } else if change.property() == ThemeVariant::actual_theme_variant_property().as_property()
            && !this.actual_theme_variant_changed.is_empty()
        {
            for (_, handler) in this.actual_theme_variant_changed.snapshot().iter() {
                handler();
            }
        }
    }
}

impl StyledElementImpl for StyledElement {
    fn begin_init(this: &Self) {
        this.init_count.set(this.init_count.get() + 1);
    }

    fn try_end_init(this: &Self) -> Result<(), InitializationError> {
        let count = this.init_count.get();
        assert!(count != 0, "BeginInit was not called.");
        this.init_count.set(count - 1);
        if count == 1 && this.logical_root.borrow().is_some() {
            this.try_apply_styling()?;
            this.initialize_if_needed();
        }
        Ok(())
    }

    fn logical_children_collection_changed(this: &Self, e: &NotifyCollectionChangedEventArgs<'_, Ref<StyledElement>>) {
        match e.action {
            NotifyCollectionChangedAction::Add => this.set_logical_parent_of(e.new_items),
            NotifyCollectionChangedAction::Remove => this.clear_logical_parent_of(e.old_items),
            NotifyCollectionChangedAction::Replace => {
                this.clear_logical_parent_of(e.old_items);
                this.set_logical_parent_of(e.new_items);
            }
            NotifyCollectionChangedAction::Reset => {
                panic!("Reset should not be signaled on LogicalChildren collection")
            }
            NotifyCollectionChangedAction::Move => {}
        }
    }

    fn on_attached_to_logical_tree(_this: &Self, _e: &LogicalTreeAttachmentEventArgs) {}

    fn on_detached_from_logical_tree(_this: &Self, _e: &LogicalTreeAttachmentEventArgs) {}

    fn on_data_context_changed(this: &Self) {
        if !this.data_context_changed.is_empty() {
            for (_, handler) in this.data_context_changed.snapshot().iter() {
                handler();
            }
        }
    }

    fn on_data_context_begin_update(_this: &Self) {}

    fn on_data_context_end_update(_this: &Self) {}

    fn on_initialized(_this: &Self) {}

    fn style_key_override(this: &Self) -> &'static TypeInfo {
        this.get_type()
    }

    fn on_control_theme_changed(this: &Self) {
        let values = this.values();
        values.begin_styling();
        values.remove_frames(this, FrameType::Theme);
        values.end_styling(this);
        this.theme_applied.set(false);
    }

    fn on_templated_parent_control_theme_changed(this: &Self) {
        let values = this.values();
        values.begin_styling();
        values.remove_frames(this, FrameType::TemplatedParentTheme);
        values.end_styling(this);
        this.templated_parent_theme_applied.set(false);
    }

    fn invalidate_styles(this: &Self, recurse: bool) {
        let values = this.values();
        values.begin_styling();
        values.remove_frames(this, FrameType::Style);
        values.end_styling(this);
        this.styles_applied.set(false);

        if recurse {
            for child in this.inheritance_children() {
                if let Some(child) = child.downcast_ref::<StyledElement>() {
                    child.invalidate_styles(recurse);
                }
            }
        }
    }

    fn is_logical_root(_this: &Self) -> bool {
        false
    }

    fn styling_parent(this: &Self) -> Option<StyleHostRef> {
        // The handle of the inheritance parent becomes the handle of the
        // host: the walks up the tree (styles, resources, the logical root)
        // ask for it at every step, and a second handle made and dropped
        // there is a count up and down per step.
        this.inheritance_parent().and_then(|p| p.downcast::<StyledElement>().ok()).map(StyleHostRef::Element)
    }

    fn is_theme_variant_root(_this: &Self) -> bool {
        false
    }

    fn notify_child_resources_changed(this: &Self, e: ResourcesChangedEventArgs) {
        if let Some(children) = this.logical_children.get() {
            if !children.is_empty() {
                for child in children.snapshot().iter() {
                    child.notify_resources_changed(e, true);
                }
            }
        }
    }

    fn child_index_provider(_this: &Self) -> Option<Rc<dyn IChildIndexProvider>> {
        None
    }
}

// The theme variant properties are declared with the theme variant type and
// owned by this class.
crate::ferro_properties! { impl StyledElement, also [
    crate::styling::ThemeVariant::actual_theme_variant_property,
    crate::styling::ThemeVariant::requested_theme_variant_property,
] {
    ferro_property!(
        /// Defines the `DataContext` property.
        pub fn data_context_property() -> StyledProperty<Option<BoxedValue>> {
            let property = FerroProperty::register_with::<StyledElement, _>(
                "DataContext",
                StyledPropertyOptions::new(None).inherits(true).notifying(Self::data_context_notifying),
            );
            property.changed().add_class_handler::<StyledElement>(|x, _| x.on_data_context_changed());
            property
        }
    );

    ferro_property!(
        /// Defines the `Name` property.
        pub fn name_property() -> DirectProperty<StyledElement, Option<String>> {
            FerroProperty::register_direct::<StyledElement, _>("Name", |o| o.name(), Some(|o, v| o.set_name(v)), None)
        }
    );

    ferro_property!(
        /// Defines the `Parent` property.
        pub fn parent_property() -> DirectProperty<StyledElement, Option<Ref<StyledElement>>> {
            FerroProperty::register_direct::<StyledElement, _>("Parent", |o| o.parent(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `TemplatedParent` property.
        pub fn templated_parent_property() -> DirectProperty<StyledElement, Option<Ref<FerroObject>>> {
            FerroProperty::register_direct::<StyledElement, _>(
                "TemplatedParent",
                |o| o.templated_parent(),
                None,
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `Theme` property.
        pub fn theme_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<StyledElement, _>("Theme", None)
        }
    );
} }

impl StyledElement {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Animatable::construct(),
            init_count: Cell::new(0),
            name: RefCell::new(None),
            classes: OnceCell::new(),
            logical_root: RefCell::new(None),
            logical_children: OnceCell::new(),
            parent: RefCell::new(None),
            resources: RefCell::new(None),
            styles: OnceCell::new(),
            styles_applied: Cell::new(false),
            theme_applied: Cell::new(false),
            templated_parent_theme_applied: Cell::new(false),
            templated_parent: RefCell::new(None),
            data_context_updating: Cell::new(false),
            implicit_theme: RefCell::new(ImplicitTheme::Unknown),
            last_resources_changed_event_args: Cell::new(ResourcesChangedEventArgs::default()),
            is_initialized: Cell::new(false),
            removed_from_logical_parent: Cell::new(false),
            attached_to_logical_tree: HandlerList::new(),
            detached_from_logical_tree: HandlerList::new(),
            data_context_changed: HandlerList::new(),
            initialized: HandlerList::new(),
            resources_changed: HandlerList::new(),
            actual_theme_variant_changed: HandlerList::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    // --- events -----------------------------------------------------------

    /// Raised when the styled element is attached to a rooted logical tree.
    pub fn attached_to_logical_tree(
        &self,
        handler: impl Fn(&LogicalTreeAttachmentEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.attached_to_logical_tree.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.attached_to_logical_tree.remove(token);
            }
        })
    }

    /// Raised when the styled element is detached from a rooted logical tree.
    pub fn detached_from_logical_tree(
        &self,
        handler: impl Fn(&LogicalTreeAttachmentEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.detached_from_logical_tree.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.detached_from_logical_tree.remove(token);
            }
        })
    }

    /// Occurs when the data context property changes.
    pub fn data_context_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.data_context_changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.data_context_changed.remove(token);
            }
        })
    }

    /// Occurs when the styled element has finished initialization.
    pub fn initialized(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.initialized.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.initialized.remove(token);
            }
        })
    }

    /// Raised when the resources change on the element or an ancestor of the
    /// element.
    pub fn resources_changed(
        &self,
        handler: impl Fn(&ResourcesChangedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.subscribe_resources_changed(Rc::new(handler))
    }

    fn subscribe_resources_changed(&self, handler: Rc<dyn Fn(&ResourcesChangedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.resources_changed.add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.resources_changed.remove(token);
            }
        })
    }

    /// Raised when the theme variant in effect for the element changes.
    pub fn actual_theme_variant_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_actual_theme_variant_changed(Rc::new(handler))
    }

    fn subscribe_actual_theme_variant_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.actual_theme_variant_changed.add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.actual_theme_variant_changed.remove(token);
            }
        })
    }

    // --- properties -------------------------------------------------------

    /// The name of the styled element.
    ///
    /// An element's name is used to uniquely identify it within its name
    /// scope. Once the element has been styled its name may not be changed.
    pub fn name(&self) -> Option<String> {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, value: Option<String>) {
        assert!(!self.styles_applied.get(), "Cannot set Name : styled element already styled.");
        *self.name.borrow_mut() = value;
    }

    /// The styled element's classes.
    ///
    /// The collection is returned by handle: clones refer to the same
    /// collection.
    pub fn classes(&self) -> Classes {
        self.classes_ref().clone()
    }

    #[inline]
    fn classes_ref(&self) -> &Classes {
        self.classes.get_or_init(Classes::new)
    }

    /// The pseudoclasses of the element, settable by the control itself.
    pub fn pseudo_classes(&self) -> &dyn IPseudoClasses {
        self.classes_ref()
    }

    /// The control's data context.
    ///
    /// The data context is an inherited property that specifies the default
    /// object that will be used for data binding.
    pub fn data_context(&self) -> Option<BoxedValue> {
        self.get_value(Self::data_context_property())
    }

    pub fn set_data_context(&self, value: Option<BoxedValue>) {
        self.set_value(Self::data_context_property(), value)
    }

    /// Whether the element has finished initialization.
    ///
    /// For more information about when this happens, see the `initialized`
    /// event.
    pub fn is_initialized(&self) -> bool {
        self.is_initialized.get()
    }

    /// The styles for the styled element.
    ///
    /// Styles for the entire application are added to the application's
    /// styles collection.
    pub fn styles(&self) -> Ref<Styles> {
        self.styles.get_or_init(|| Styles::with_owner(self.to_ref())).clone()
    }

    /// Whether the styles collection of the element has been created.
    pub fn is_styles_initialized(&self) -> bool {
        self.styles.get().is_some()
    }

    /// The type by which the element is styled.
    pub fn style_key(&self) -> &'static TypeInfo {
        self.style_key_override()
    }

    /// The styled element's resource dictionary.
    pub fn resources(&self) -> Ref<ResourceDictionary> {
        if let Some(resources) = &*self.resources.borrow() {
            return resources.clone();
        }
        let resources = ResourceDictionary::with_owner(self.to_ref());
        *self.resources.borrow_mut() = Some(resources.clone());
        resources
    }

    pub fn set_resources(&self, value: Ref<ResourceDictionary>) {
        let this = ResourceHostRef::Element(self.to_ref());
        let old = self.resources.borrow_mut().take();
        if let Some(old) = old {
            old.remove_owner(&this);
        }
        *self.resources.borrow_mut() = Some(value.clone());
        value.add_owner(&this);
    }

    /// The theme to be applied to the element.
    ///
    /// By default the theme is looked up as a resource keyed by the element's
    /// style key; setting this property takes precedence over that lookup.
    pub fn theme(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::theme_property())
    }

    pub fn set_theme(&self, value: impl Into<Nullable<ControlTheme>>) {
        self.set_value(Self::theme_property(), value.into().0)
    }

    /// The UI theme variant that is currently used by the element, which
    /// might be different than the requested theme variant. `None` until a
    /// theme variant has been established for the element or an ancestor.
    pub fn actual_theme_variant(&self) -> Option<ThemeVariant> {
        self.get_value(ThemeVariant::actual_theme_variant_property())
    }

    /// Whether the element or its styles have resources.
    pub fn has_resources(&self) -> bool {
        self.resources.borrow().as_ref().is_some_and(|r| r.has_resources())
            || self.styles.get().is_some_and(|s| s.has_resources())
    }

    /// Tries to find a resource within the element: in its resource
    /// dictionary, then in its styles.
    pub fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let resources = self.resources.borrow().clone();
        if let Some(resources) = resources {
            if let Some(value) = resources.try_get_resource(key, theme) {
                return Some(value);
            }
        }
        match self.styles.get() {
            Some(styles) => styles.try_get_resource(key, theme),
            None => None,
        }
    }

    /// Tries to find a resource by searching up the logical tree and then
    /// the global styles.
    pub fn try_find_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let host: &dyn IResourceHost = self;
        host.try_find_resource(key, theme)
    }

    /// Finds a resource by searching up the logical tree and then the global
    /// styles. Returns the unset value marker if it is not found.
    pub fn find_resource(&self, key: &ResourceKey) -> ResourceValue {
        let host: &dyn IResourceHost = self;
        host.find_resource(key)
    }

    /// The styled element whose lookless template this element is part of.
    pub fn templated_parent(&self) -> Option<Ref<FerroObject>> {
        self.templated_parent.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// Sets the templated parent. Called by the template that creates the
    /// element.
    pub fn set_templated_parent(&self, value: impl Into<Nullable<FerroObject>>) {
        let value = value.into().0;
        let old = self.templated_parent();
        if old != value {
            *self.templated_parent.borrow_mut() = value.as_ref().map(Ref::downgrade);
            self.raise_direct_property_changed(Self::templated_parent_property(), &old, &value);
        }
    }

    /// The styled element's logical children.
    pub fn logical_children(&self) -> &FerroList<Ref<StyledElement>> {
        self.logical_children.get_or_init(|| {
            let list = FerroList::new();
            list.set_reset_behavior(ResetBehavior::Remove);
            let weak = self.to_ref().downgrade();
            list.add_collection_changed(Rc::new(move |e: &NotifyCollectionChangedEventArgs<'_, Ref<StyledElement>>| {
                if let Some(this) = weak.upgrade() {
                    this.logical_children_collection_changed(e);
                }
            }));
            list
        })
    }

    /// Whether the element is attached to a rooted logical tree.
    pub fn is_attached_to_logical_tree(&self) -> bool {
        self.logical_root.borrow().is_some()
    }

    /// The styled element's logical parent.
    pub fn parent(&self) -> Option<Ref<StyledElement>> {
        self.parent.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    // --- initialization and styling ---------------------------------------

    /// Applies styling to the control if the control is initialized and
    /// styling is not already applied.
    ///
    /// The styling system will automatically apply styling when required, so
    /// this method need only be called by tests or in rare special cases.
    ///
    /// Returns true if styling is applied to the control, false if the
    /// control is still initializing.
    pub fn apply_styling(&self) -> bool {
        match self.try_apply_styling() {
            Ok(applied) => applied,
            Err(error) => panic!("{error}"),
        }
    }

    /// As [`apply_styling`](Self::apply_styling), reporting a style or
    /// control theme that cannot be applied (two setters for the same
    /// property: the invalid operation of the managed original) instead of
    /// panicking. After an error the styling pass is ended and the element
    /// is not marked as styled, as when the managed original throws.
    pub fn try_apply_styling(&self) -> Result<bool, DuplicateSetterError> {
        if self.init_count.get() == 0
            && (!self.styles_applied.get()
                || !self.theme_applied.get()
                || !self.templated_parent_theme_applied.get())
        {
            let values = self.values();
            values.begin_styling();
            let _end_styling = EndStyling(self);
            if !self.theme_applied.get() {
                self.apply_control_theme()?;
                self.theme_applied.set(true);
            }
            if !self.templated_parent_theme_applied.get() {
                self.apply_templated_parent_control_theme()?;
                self.templated_parent_theme_applied.set(true);
            }
            if !self.styles_applied.get() {
                self.apply_styles(&StyleHostRef::Element(self.to_ref()))?;
                self.styles_applied.set(true);
            }
        }
        Ok(self.styles_applied.get())
    }

    /// Signals the end of a batch of initialization
    /// ([`try_end_init`](Self::try_end_init)). A style or control theme that
    /// cannot be applied panics with the message of the error.
    pub fn end_init(&self) {
        if let Err(error) = self.try_end_init() {
            panic!("{error}");
        }
    }

    /// Marks the element as initialized and raises the initialization
    /// notifications, if initialization is complete.
    pub fn initialize_if_needed(&self) {
        if self.init_count.get() == 0 && !self.is_initialized.get() {
            self.is_initialized.set(true);
            self.on_initialized();
            if !self.initialized.is_empty() {
                for (_, handler) in self.initialized.snapshot().iter() {
                    handler();
                }
            }
        }
    }

    // --- logical tree -----------------------------------------------------

    /// Sets the element's logical parent.
    ///
    /// You should not usually need to call this method: it is called when an
    /// element is added to or removed from a logical children collection.
    pub fn set_parent(&self, parent: impl Into<Nullable<StyledElement>>) {
        let parent = parent.into().0;
        let old = self.parent();
        if parent == old {
            return;
        }
        assert!(old.is_none() || parent.is_none(), "The Control already has a parent.");

        if self.inheritance_parent().is_none() || parent.is_none() {
            self.set_inheritance_parent(parent.clone().map(Ref::upcast::<FerroObject>));
        }

        *self.parent.borrow_mut() = parent.as_ref().map(Ref::downgrade);

        let this = self.to_ref();
        let old_root = self.logical_root();
        if let Some(root) = old_root {
            let e = LogicalTreeAttachmentEventArgs::new(root, this.clone(), old.clone());
            self.on_detached_from_logical_tree_core(&e);
        }

        if let Some(new_root) = self.find_logical_root() {
            let e = LogicalTreeAttachmentEventArgs::new(new_root, this, parent.clone());
            self.on_attached_to_logical_tree_core(&e);
        } else if parent.is_none() {
            // If we were attached to the logical tree, we piggyback on the
            // tree traversal there to raise resources changed notifications.
            // If we're being removed from the logical tree, then traverse the
            // tree raising notifications now.
            //
            // We don't raise resources changed notifications if we're being
            // attached to a non-rooted control because it's unlikely that
            // dynamic resources need to be correct until the control is added
            // to the tree, and it causes a *lot* of notifications.
            self.notify_resources_changed(ResourcesChangedEventArgs::create(), true);
        }

        self.raise_direct_property_changed(Self::parent_property(), &old, &parent);
    }

    /// Notifies the element that it has been attached to a rooted logical
    /// tree. Called by a parent for children that are part of its logical
    /// tree without being in its logical children collection (the template
    /// child of a templated control).
    pub fn notify_attached_to_logical_tree(&self, e: &LogicalTreeAttachmentEventArgs) {
        self.on_attached_to_logical_tree_core(e);
    }

    /// Notifies the element that it has been detached from a rooted logical
    /// tree. See [`notify_attached_to_logical_tree`](Self::notify_attached_to_logical_tree).
    pub fn notify_detached_from_logical_tree(&self, e: &LogicalTreeAttachmentEventArgs) {
        self.on_detached_from_logical_tree_core(e);
    }

    fn logical_root(&self) -> Option<Ref<StyledElement>> {
        self.logical_root.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    fn find_logical_root(&self) -> Option<Ref<StyledElement>> {
        if self.is_logical_root() {
            return Some(self.to_ref());
        }
        let mut current = self.styling_parent();
        while let Some(host) = current {
            if let StyleHostRef::Element(e) = &host {
                if e.is_logical_root() {
                    return Some(e.clone());
                }
            }
            current = host.styling_parent();
        }
        None
    }

    fn on_attached_to_logical_tree_core(&self, e: &LogicalTreeAttachmentEventArgs) {
        if self.parent().is_none() && !self.is_logical_root() {
            panic!(
                "AttachedToLogicalTreeCore called for '{}' but control has no logical parent.",
                self.get_type().name()
            );
        }

        // This method can be called when a control is already attached to the
        // logical tree in the following scenario:
        // - ListBox gets assigned Items containing ListBoxItem
        // - ListBox makes ListBoxItem a logical child
        // - ListBox template gets applied; making its Panel get attached to
        //   the logical tree
        // - That AttachedToLogicalTree signal travels down to the ListBoxItem
        if self.logical_root.borrow().is_none() {
            crate::perf_count!(LogicalTreeAttachments);
            *self.logical_root.borrow_mut() = Some(e.root().downgrade());
            self.reevaluate_implicit_theme();
            self.apply_styling();
            self.notify_resources_changed(ResourcesChangedEventArgs::create(), false);
            self.on_attached_to_logical_tree(e);
            if !self.attached_to_logical_tree.is_empty() {
                for (_, handler) in self.attached_to_logical_tree.snapshot().iter() {
                    handler(e);
                }
            }
        }

        if let Some(children) = self.logical_children.get() {
            // An event handler may have modified the children while we're
            // enumerating a snapshot of the collection.
            for child in children.snapshot().iter() {
                let attached_to_root = child.logical_root().is_some_and(|r| r == *e.root());
                if !attached_to_root && (child.parent().is_some() || !child.removed_from_logical_parent.get()) {
                    child.on_attached_to_logical_tree_core(e);
                }
            }
        }
    }

    fn on_detached_from_logical_tree_core(&self, e: &LogicalTreeAttachmentEventArgs) {
        if self.logical_root.borrow().is_some() {
            crate::perf_count!(LogicalTreeDetachments);
            *self.logical_root.borrow_mut() = None;
            self.invalidate_styles(false);
            self.on_detached_from_logical_tree(e);
            if !self.detached_from_logical_tree.is_empty() {
                for (_, handler) in self.detached_from_logical_tree.snapshot().iter() {
                    handler(e);
                }
            }

            if let Some(children) = self.logical_children.get() {
                for child in children.snapshot().iter() {
                    child.on_detached_from_logical_tree_core(e);
                }
            }
        }
    }

    fn set_logical_parent_of(&self, children: &[Ref<StyledElement>]) {
        for child in children {
            child.removed_from_logical_parent.set(false);
            if child.parent().is_none() {
                child.set_parent(self.to_ref());
            }
        }
    }

    fn clear_logical_parent_of(&self, children: &[Ref<StyledElement>]) {
        let this = self.to_ref();
        for child in children {
            child.removed_from_logical_parent.set(true);
            if child.parent().is_some_and(|p| p == this) {
                child.set_parent(None);
            }
        }
    }

    // --- styling ----------------------------------------------------------

    /// The theme in effect for the element: the `Theme` property if set,
    /// otherwise the control theme resource keyed by the element's style key
    /// that is visible to the element (a resource holding a
    /// `Ref<ControlTheme>`).
    pub fn get_effective_theme(&self) -> Option<Ref<ControlTheme>> {
        // Explicitly set Theme property takes precedence.
        if let Some(theme) = self.theme() {
            return Some(theme);
        }

        // If the Theme property is not set, try to find a ControlTheme
        // resource with our StyleKey.
        if matches!(*self.implicit_theme.borrow(), ImplicitTheme::Unknown) {
            crate::perf_count!(ImplicitThemeLookups);
            let key = ResourceKey::Type(self.style_key());
            let found = self.try_find_resource(&key, None).flatten().and_then(|value| {
                let value: &dyn AnyValue = &*value;
                value.downcast_ref::<Ref<ControlTheme>>().cloned()
            });
            *self.implicit_theme.borrow_mut() = match found {
                Some(theme) => ImplicitTheme::Theme(theme),
                None => ImplicitTheme::Invalid,
            };
        }

        match &*self.implicit_theme.borrow() {
            ImplicitTheme::Theme(theme) => Some(theme.clone()),
            _ => None,
        }
    }

    fn apply_control_theme(&self) -> Result<(), DuplicateSetterError> {
        if let Some(theme) = self.get_effective_theme() {
            self.apply_control_theme_core(&theme, FrameType::Theme)?;
        }
        Ok(())
    }

    fn apply_templated_parent_control_theme(&self) -> Result<(), DuplicateSetterError> {
        // Most elements have no templated parent: avoid upgrading the weak
        // reference for them.
        if self.templated_parent.borrow().is_none() {
            return Ok(());
        }
        let parent_theme = self
            .templated_parent()
            .and_then(|p| p.downcast_ref::<StyledElement>().and_then(StyledElement::get_effective_theme));
        if let Some(parent_theme) = parent_theme {
            self.apply_control_theme_core(&parent_theme, FrameType::TemplatedParentTheme)?;
        }
        Ok(())
    }

    fn apply_control_theme_core(&self, theme: &ControlTheme, type_: FrameType) -> Result<(), DuplicateSetterError> {
        debug_assert!(matches!(type_, FrameType::Theme | FrameType::TemplatedParentTheme));

        if let Some(based_on) = theme.based_on() {
            self.apply_control_theme_core(&based_on, type_)?;
        }

        theme.try_attach_checked(self, type_)?;

        if theme.has_children() {
            let children = theme.children().snapshot();
            for child in children.iter() {
                self.apply_style(&**child, None, type_)?;
            }
        }
        Ok(())
    }

    fn apply_styles(&self, host: &StyleHostRef) -> Result<(), DuplicateSetterError> {
        if let Some(parent) = host.styling_parent() {
            self.apply_styles(&parent)?;
        }
        crate::perf_count!(StyleHostsWalked);

        if host.is_styles_initialized() {
            let styles = host.styles().snapshot();
            for style in styles.iter() {
                self.apply_style(&**style, Some(host), FrameType::Style)?;
            }
        }
        Ok(())
    }

    fn apply_style(
        &self,
        style: &dyn IStyle,
        host: Option<&StyleHostRef>,
        type_: FrameType,
    ) -> Result<(), DuplicateSetterError> {
        if let Some(base) = style.as_object().and_then(|o| o.downcast_ref::<StyleBase>()) {
            if let Some(s) = base.downcast_ref::<Style>() {
                s.try_attach_checked(self, host, type_)?;
            } else if let Some(m) = base.downcast_ref::<ContainerQuery>() {
                m.try_attach_checked(self, host, type_)?;
            }

            // The common case of a style without nested styles.
            if !base.has_children() {
                return Ok(());
            }
        }

        let children = style.children();
        for child in children.iter() {
            self.apply_style(&**child, host, type_)?;
        }
        Ok(())
    }

    fn reevaluate_implicit_theme(&self) {
        // We only need to check if the theme has changed when Theme isn't set
        // (i.e. when we have an implicit theme).
        if self.theme().is_some() {
            return;
        }

        // Refetch the implicit theme.
        let old_implicit_theme = match self.implicit_theme.replace(ImplicitTheme::Unknown) {
            ImplicitTheme::Theme(theme) => Some(theme),
            _ => None,
        };
        let new_implicit_theme = self.get_effective_theme();

        // If the implicit theme has changed, detach the existing theme.
        if new_implicit_theme != old_implicit_theme {
            self.on_control_theme_changed();
            self.theme_applied.set(false);
        }
    }

    /// Called when styles are added to the styles collection of the element
    /// or a nested styles collection.
    pub fn styles_added(&self, styles: &[Rc<dyn IStyle>]) {
        if Self::has_setters_or_animations(styles) {
            self.invalidate_styles(true);
        }
    }

    /// Called when styles are removed from the styles collection of the
    /// element or a nested styles collection.
    pub fn styles_removed(&self, styles: &[Rc<dyn IStyle>]) {
        let mut all_styles = Vec::new();
        Self::flatten_styles(styles, &mut all_styles);
        if !all_styles.is_empty() {
            self.detach_styles(&all_styles);
        }
    }

    fn detach_styles(&self, styles: &[Ref<StyleBase>]) {
        let values = self.values();
        values.begin_styling();
        values.remove_frames_where(self, |frame| {
            frame
                .as_any()
                .downcast_ref::<StyleInstance>()
                .is_some_and(|instance| styles.iter().any(|style| instance.is_source(style)))
        });
        values.end_styling(self);

        if let Some(children) = self.logical_children.get() {
            if !children.is_empty() {
                for child in children.snapshot().iter() {
                    child.detach_styles(styles);
                }
            }
        }
    }

    /// Notifies the element, and with `propagate` its children, that the
    /// resources visible to it have changed.
    pub fn notify_resources_changed(&self, e: ResourcesChangedEventArgs, propagate: bool) {
        // We already got a notification for this element, ignore.
        if e == self.last_resources_changed_event_args.get() {
            return;
        }

        self.last_resources_changed_event_args.set(e);

        if !self.resources_changed.is_empty() {
            for (_, handler) in self.resources_changed.snapshot().iter() {
                handler(&e);
            }
        }

        if propagate {
            self.notify_child_resources_changed(e);
        }
    }

    fn flatten_styles(styles: &[Rc<dyn IStyle>], result: &mut Vec<Ref<StyleBase>>) {
        for style in styles {
            if let Some(s) = style.as_object().and_then(|o| o.downcast_ref::<Style>()) {
                result.push(s.to_ref().upcast());
            }
            let children = style.children();
            if !children.is_empty() {
                Self::flatten_styles(&children, result);
            }
        }
    }

    fn has_setters_or_animations(styles: &[Rc<dyn IStyle>]) -> bool {
        styles.iter().any(|style| {
            if style
                .as_object()
                .and_then(|o| o.downcast_ref::<StyleBase>())
                .is_some_and(StyleBase::has_setters_or_animations)
            {
                return true;
            }
            let children = style.children();
            !children.is_empty() && Self::has_setters_or_animations(&children)
        })
    }

    // --- data context -----------------------------------------------------

    fn data_context_notifying(o: &FerroObject, update_started: bool) {
        if let Some(element) = o.downcast_ref::<StyledElement>() {
            element.data_context_notifying_core(update_started);
        }
    }

    fn data_context_notifying_core(&self, update_started: bool) {
        if update_started {
            if !self.data_context_updating.get() {
                self.data_context_updating.set(true);
                self.on_data_context_begin_update();

                if let Some(children) = self.logical_children.get() {
                    let this: Ref<FerroObject> = self.to_ref().upcast();
                    for child in children.snapshot().iter() {
                        if child.inheritance_parent().is_some_and(|p| p == this)
                            && !child.is_set(Self::data_context_property())
                        {
                            child.data_context_notifying_core(update_started);
                        }
                    }
                }
            }
        } else if self.data_context_updating.get() {
            self.on_data_context_end_update();
            self.data_context_updating.set(false);
        }
    }

    /// The priority-aware setter used by templates.
    pub fn set_data_context_with_priority(
        &self,
        value: Option<BoxedValue>,
        priority: BindingPriority,
    ) -> Option<Rc<dyn IDisposable>> {
        self.set_value_with_priority(Self::data_context_property(), value, priority)
    }
}

/// Ends a styling pass when dropped, so that the pass also ends when applying
/// a style panics.
struct EndStyling<'a>(&'a StyledElement);

impl Drop for EndStyling<'_> {
    fn drop(&mut self) {
        self.0.values().end_styling(self.0);
    }
}

impl IResourceNode for StyledElement {
    fn has_resources(&self) -> bool {
        StyledElement::has_resources(self)
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        StyledElement::try_get_resource(self, key, theme)
    }
}

impl IResourceHost for StyledElement {
    fn resources_changed(&self, handler: Rc<dyn Fn(&ResourcesChangedEventArgs)>) -> Rc<dyn IDisposable> {
        self.subscribe_resources_changed(handler)
    }

    fn notify_hosted_resources_changed(&self, e: ResourcesChangedEventArgs) {
        self.notify_resources_changed(e, true)
    }

    fn as_style_host(&self) -> Option<&dyn IStyleHost> {
        Some(self)
    }

    fn as_theme_variant_host(&self) -> Option<&dyn IThemeVariantHost> {
        Some(self)
    }
}

impl IStyleHost for StyledElement {
    fn is_styles_initialized(&self) -> bool {
        StyledElement::is_styles_initialized(self)
    }

    fn styles(&self) -> Ref<Styles> {
        StyledElement::styles(self)
    }

    fn styling_parent(&self) -> Option<StyleHostRef> {
        StyledElement::styling_parent(self)
    }

    fn styles_added(&self, styles: &[Rc<dyn IStyle>]) {
        StyledElement::styles_added(self, styles)
    }

    fn styles_removed(&self, styles: &[Rc<dyn IStyle>]) {
        StyledElement::styles_removed(self, styles)
    }
}

impl IThemeVariantHost for StyledElement {
    fn actual_theme_variant(&self) -> Option<ThemeVariant> {
        StyledElement::actual_theme_variant(self)
    }

    fn actual_theme_variant_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.subscribe_actual_theme_variant_changed(handler)
    }
}
