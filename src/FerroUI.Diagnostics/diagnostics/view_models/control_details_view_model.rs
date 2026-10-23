//! Port of `Diagnostics/ViewModels/ControlDetailsViewModel.cs`.
//!
//! The managed original shows the properties of the selected object in a DataGrid through a
//! `DataGridCollectionView`, grouped by `PropertyViewModel.Group`, sorted by its
//! `PropertyComparer` and filtered by the properties filter of the tree page. The port shows
//! them in a TreeDataGrid: [`PropertiesView`] takes the place of the collection view. It holds
//! the properties, groups, sorts and filters them as the collection view does, and presents the
//! groups as the rows of a hierarchical source ([`PropertiesViewItem`]: a group or a property)
//! with the properties below them (docs/porting/DEVIATIONS.md, Developer tools).

use super::{
    ClrPropertyViewModel, ControlLayoutViewModel, FerroPropertyViewModel, PropertyViewModelKey, PropertyViewModelRef,
    PseudoClassViewModel, SetterViewModelRef, TreePageViewModel, ValueFrameViewModel, ViewModelBase,
};
use super::property_view_model::type_name;
use ferroui_base::collections::FerroList;
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::model::{Event, INotifyPropertyChanged, ModelTypes};
use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions;
use ferroui_base::input::platform::IClipboard;
use ferroui_base::metadata::{MarkupProperty, MarkupType, MarkupTypeKind};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_markup_type, BoxedValue, FerroObject, FerroProperty, FerroPropertyChangedEventArgs, FerroPropertyRegistry, Ref,
    StyledElement, TypeInfo, Visual,
};
use ferroui_controls::{Control, GridLength, GridUnitType, TopLevel};
use ferroui_controls_treedatagrid::models::{
    HierarchicalExpanderColumn, IViewColumn, TemplateColumn, TemplateColumnOptions, TextColumn, TextColumnOptions,
};
use ferroui_controls_treedatagrid::TreeDataGridSourceRef;
use ferroui_treedatagrid_core::models::{ColumnRef, Comparison, IColumn};
use ferroui_treedatagrid_core::selection::TreeSelectionModelSelectionChangedEventArgsOf;
use ferroui_treedatagrid_core::{Accessor, HierarchicalTreeDataGridSource, IndexPath, Models};
use std::any::TypeId;
use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::rc::{Rc, Weak};

/// The names of the pinned properties, shared by the details of every object (`ISet<string>`).
pub type PinnedProperties = Rc<RefCell<HashSet<String>>>;

/// The object whose property changes the details follow: its handler token, and how to remove
/// it.
enum EntitySubscription {
    /// `FerroObject.PropertyChanged`.
    Object(Ref<FerroObject>, u64),
    /// `INotifyPropertyChanged.PropertyChanged` of a model.
    Model(BoxedValue, u64),
}

/// The notification interface of an untyped value: its model type metadata, else its markup
/// metadata (`o as INotifyPropertyChanged`).
fn notifier_of(value: &BoxedValue) -> Option<&dyn INotifyPropertyChanged> {
    if let Some(notifier) = ModelTypes::find(&**value).and_then(|model| model.as_notify_property_changed(&**value)) {
        return Some(notifier);
    }
    let id = value_type_id(value);
    MarkupType::find_by_handle(id)
        .and_then(|markup| markup.notify_property_changed)
        .and_then(|as_notifier| as_notifier(&**value))
}

fn value_type_id(value: &BoxedValue) -> TypeId {
    (**value).value_type_id()
}

/// The details of an object: its properties, the frames of its values with their setters, its
/// pseudo-classes and its layout.
pub struct ControlDetailsViewModel {
    base: ViewModelBase,
    weak_self: Weak<ControlDetailsViewModel>,
    ferro_object: Ref<FerroObject>,
    pinned_properties: PinnedProperties,
    property_index: RefCell<Option<HashMap<PropertyViewModelKey, Vec<PropertyViewModelRef>>>>,
    selected_property: RefCell<Option<PropertyViewModelRef>>,
    properties_view: RefCell<Option<Rc<PropertiesView>>>,
    snapshot_frames: RefCell<bool>,
    show_inactive_frames: RefCell<bool>,
    frames_status: RefCell<Option<String>>,
    selected_entity: RefCell<Option<BoxedValue>>,
    selected_entities_stack: RefCell<Vec<(String, BoxedValue)>>,
    selected_entity_name: RefCell<Option<String>>,
    selected_entity_type: RefCell<Option<String>>,
    show_implemented_interfaces: Cell<bool>,
    tree_page: Weak<TreePageViewModel>,
    layout: Option<Rc<ControlLayoutViewModel>>,
    applied_frames: FerroList<Rc<ValueFrameViewModel>>,
    pseudo_classes: FerroList<Rc<PseudoClassViewModel>>,
    entity_subscription: RefCell<Option<EntitySubscription>>,
    classes_listener: Cell<Option<u64>>,
    main_view_subscription: Cell<Option<u64>>,
}

impl PartialEq for ControlDetailsViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ControlDetailsViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ControlDetailsViewModel {
    pub fn new(
        tree_page: &Rc<TreePageViewModel>,
        ferro_object: &FerroObject,
        pinned_properties: PinnedProperties,
    ) -> Rc<Self> {
        let layout = ferro_object.downcast_ref::<Visual>().map(ControlLayoutViewModel::new);

        let this = Rc::new_cyclic(|weak_self: &Weak<Self>| Self {
            base: ViewModelBase::new(),
            weak_self: weak_self.clone(),
            ferro_object: ferro_object.to_ref(),
            pinned_properties,
            property_index: RefCell::new(None),
            selected_property: RefCell::new(None),
            properties_view: RefCell::new(None),
            snapshot_frames: RefCell::new(false),
            show_inactive_frames: RefCell::new(false),
            frames_status: RefCell::new(None),
            selected_entity: RefCell::new(None),
            selected_entities_stack: RefCell::new(Vec::new()),
            selected_entity_name: RefCell::new(None),
            selected_entity_type: RefCell::new(None),
            show_implemented_interfaces: Cell::new(false),
            tree_page: Rc::downgrade(tree_page),
            layout,
            applied_frames: FerroList::new(),
            pseudo_classes: FerroList::new(),
            entity_subscription: RefCell::new(None),
            classes_listener: Cell::new(None),
            main_view_subscription: Cell::new(None),
        });

        this.navigate_to_property(this.boxed_object(), this.object_name());

        if let Some(styled_element) = this.ferro_object.downcast_ref::<StyledElement>() {
            // `styledElement.Classes.AddListener(this)`: `IClassesChangedListener.Changed`.
            let weak = this.weak_self.clone();
            let token = styled_element.classes().add_listener(Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.classes_changed();
                }
            }));
            this.classes_listener.set(Some(token));

            // `GetCustomAttributes<PseudoClassesAttribute>(true)`: the pseudo-classes the class
            // and its base classes declare, the class first.
            let mut type_ = Some(styled_element.get_type());
            while let Some(current) = type_ {
                if let Some(markup) = current.markup() {
                    for class_attribute in markup.attributes.iter().filter(|attribute| attribute.name == "PseudoClasses") {
                        for class_name in class_attribute.arguments {
                            if let ferroui_base::metadata::MarkupAttributeValue::Str(class_name) = class_name {
                                this.pseudo_classes.add(PseudoClassViewModel::new(class_name, styled_element));
                            }
                        }
                    }
                }
                type_ = current.base_type();
            }

            let style_diagnostics = styled_element.get_value_store_diagnostic();

            let clipboard: Option<Rc<dyn IClipboard>> = this
                .ferro_object
                .downcast_ref::<Visual>()
                .and_then(|visual| TopLevel::get_top_level(Some(visual)))
                .and_then(|top_level| top_level.clipboard());

            // `OrderBy(s => s.Priority)`: a stable sort.
            let mut applied_frames = style_diagnostics.applied_frames().to_vec();
            applied_frames.sort_by_key(|frame| frame.priority());

            for applied_style in applied_frames {
                this.applied_frames.add(ValueFrameViewModel::new(styled_element, applied_style, clipboard.clone()));
            }

            this.update_styles();
        }

        this.subscribe_to_main_view();

        this
    }

    /// The object of the details, as an untyped value.
    fn boxed_object(&self) -> BoxedValue {
        Rc::new(self.ferro_object.clone())
    }

    /// `(_ferroObject as Control)?.Name ?? _ferroObject.ToString()`.
    fn object_name(&self) -> Option<String> {
        self.ferro_object
            .downcast_ref::<Control>()
            .and_then(|control| control.name())
            .or_else(|| Some(ValueTypes::to_display_string(Some(&self.boxed_object()))))
    }

    pub fn can_navigate_to_parent_property(&self) -> bool {
        !self.selected_entities_stack.borrow().is_empty()
    }

    /// The tree page the details belong to (held weakly: the page holds the details).
    pub fn tree_page(&self) -> Option<Rc<TreePageViewModel>> {
        self.tree_page.upgrade()
    }

    /// The properties of the selected entity, grouped, sorted and filtered (the
    /// `DataGridCollectionView` of the managed original).
    pub fn properties_view(&self) -> Option<Rc<PropertiesView>> {
        self.properties_view.borrow().clone()
    }

    fn set_properties_view(&self, value: Option<Rc<PropertiesView>>) {
        let changed = match (&*self.properties_view.borrow(), &value) {
            (Some(old), Some(new)) => !Rc::ptr_eq(old, new),
            (None, None) => false,
            _ => true,
        };
        if changed {
            *self.properties_view.borrow_mut() = value;
            self.raise_property_changed("PropertiesView");
        }
    }

    pub fn applied_frames(&self) -> FerroList<Rc<ValueFrameViewModel>> {
        self.applied_frames.clone()
    }

    pub fn pseudo_classes(&self) -> FerroList<Rc<PseudoClassViewModel>> {
        self.pseudo_classes.clone()
    }

    pub fn selected_entity(&self) -> Option<BoxedValue> {
        self.selected_entity.borrow().clone()
    }

    pub fn set_selected_entity(&self, value: Option<BoxedValue>) {
        let changed = !ValueTypes::identity_equals(self.selected_entity.borrow().as_ref(), value.as_ref());
        if changed {
            *self.selected_entity.borrow_mut() = value;
            self.raise_property_changed("SelectedEntity");
        }
    }

    pub fn selected_entity_name(&self) -> Option<String> {
        self.selected_entity_name.borrow().clone()
    }

    pub fn set_selected_entity_name(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.selected_entity_name, value, "SelectedEntityName");
    }

    pub fn selected_entity_type(&self) -> Option<String> {
        self.selected_entity_type.borrow().clone()
    }

    pub fn set_selected_entity_type(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.selected_entity_type, value, "SelectedEntityType");
    }

    pub fn selected_property(&self) -> Option<PropertyViewModelRef> {
        self.selected_property.borrow().clone()
    }

    /// Sets the selected property, and selects its row in the grid (the two-way binding of
    /// `SelectedItem` of the managed original).
    pub fn set_selected_property(&self, value: Option<PropertyViewModelRef>) {
        if self.raise_and_set_if_changed(&self.selected_property, value.clone(), "SelectedProperty") {
            if let Some(view) = self.properties_view() {
                view.select(value.as_ref());
            }
        }
    }

    pub fn snapshot_frames(&self) -> bool {
        *self.snapshot_frames.borrow()
    }

    pub fn set_snapshot_frames(&self, value: bool) {
        self.raise_and_set_if_changed(&self.snapshot_frames, value, "SnapshotFrames");
    }

    pub fn show_inactive_frames(&self) -> bool {
        *self.show_inactive_frames.borrow()
    }

    pub fn set_show_inactive_frames(&self, value: bool) {
        self.raise_and_set_if_changed(&self.show_inactive_frames, value, "ShowInactiveFrames");
    }

    pub fn frames_status(&self) -> Option<String> {
        self.frames_status.borrow().clone()
    }

    pub fn set_frames_status(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.frames_status, value, "FramesStatus");
    }

    pub fn layout(&self) -> Option<Rc<ControlLayoutViewModel>> {
        self.layout.clone()
    }

    /// `RaiseAndSetIfChanged` of the base class, through this class's `RaisePropertyChanged`,
    /// which runs the override of `OnPropertyChanged` first.
    fn raise_and_set_if_changed<T: PartialEq>(&self, field: &RefCell<T>, value: T, property_name: &str) -> bool {
        if *field.borrow() == value {
            return false;
        }
        *field.borrow_mut() = value;
        self.raise_property_changed(property_name);
        true
    }

    /// `RaisePropertyChanged`: the override of `OnPropertyChanged`, then the handlers.
    fn raise_property_changed(&self, property_name: &str) {
        self.on_property_changed(property_name);
        self.base.raise_property_changed(property_name);
    }

    /// The override of `OnPropertyChanged`: leaving the snapshot of the frames updates them.
    fn on_property_changed(&self, property_name: &str) {
        if property_name == "SnapshotFrames" && !self.snapshot_frames() {
            self.update_styles();
        }
    }

    pub fn update_style_filters(&self) {
        let Some(tree_page) = self.tree_page() else {
            return;
        };
        let setters_filter = tree_page.setters_filter();

        for style in self.applied_frames.to_vec() {
            let mut has_visible_setter = false;

            for setter in style.setters() {
                setter.set_is_visible(setters_filter.filter(&setter.name()));

                has_visible_setter |= setter.is_visible();
            }

            style.set_is_visible(has_visible_setter);
        }
    }

    pub fn dispose(&self) {
        self.unsubscribe_entity();

        if let (Some(styled_element), Some(token)) =
            (self.ferro_object.downcast_ref::<StyledElement>(), self.classes_listener.take())
        {
            styled_element.classes().remove_listener(token);
        }

        if let (Some(main_view), Some(token)) =
            (self.tree_page().and_then(|tree_page| tree_page.main_view()), self.main_view_subscription.take())
        {
            main_view.property_changed().remove(token);
        }
    }

    /// The registered properties of an object: the ones of its class and the attached properties
    /// registered for it; nothing for a value that is not an object.
    fn get_ferro_properties(o: &BoxedValue) -> Vec<PropertyViewModelRef> {
        let Some(ao) = ValueTypes::as_object(&**o) else {
            return Vec::new();
        };
        let registry = FerroPropertyRegistry::instance();

        // `Union`: distinct, in order.
        let mut seen: HashSet<&'static FerroProperty> = HashSet::new();
        let mut result = Vec::new();
        let registered = registry.get_registered_for(&ao);
        let attached = registry.get_registered_attached(ao.get_type());
        for property in registered.iter().chain(attached.iter()).copied() {
            if seen.insert(property) {
                result.push(PropertyViewModelRef::Ferro(FerroPropertyViewModel::new(&ao, property)));
            }
        }
        result
    }

    /// The plain properties of an object, and with `show_implemented_interfaces` the ones of the
    /// contracts its type implements.
    fn get_clr_properties(o: &BoxedValue, show_implemented_interfaces: bool) -> Vec<PropertyViewModelRef> {
        let mut result = Vec::new();

        for (property, declaring_type) in Self::get_type_properties(o) {
            result.push(PropertyViewModelRef::Clr(ClrPropertyViewModel::new(o.clone(), property, declaring_type)));
        }

        if show_implemented_interfaces {
            for i in Self::get_interfaces(o) {
                for property in i.properties {
                    result.push(PropertyViewModelRef::Clr(ClrPropertyViewModel::new(o.clone(), property, i)));
                }
            }
        }

        result
    }

    /// The markup metadata of the type of an object and of its base types, the type first: the
    /// counterpart of the reflection over a type and its base types of the managed original.
    fn get_type_chain(o: &BoxedValue) -> Vec<&'static MarkupType> {
        let mut chain = Vec::new();

        if let Some(object) = ValueTypes::as_object(&**o) {
            let mut type_: Option<&'static TypeInfo> = Some(object.get_type());
            while let Some(current) = type_ {
                if let Some(markup) = current.markup() {
                    chain.push(markup);
                }
                type_ = current.base_type();
            }
        } else {
            let id = value_type_id(o);
            let mut markup = MarkupType::find_by_handle(id);
            while let Some(current) = markup {
                if chain.iter().any(|known: &&'static MarkupType| std::ptr::eq(*known, current)) {
                    break;
                }
                chain.push(current);
                markup = current.base_type();
            }
        }

        chain
    }

    /// `GetClrProperties(o, o.GetType())`: the public properties of the type of an object, with
    /// the type that declares each (a property a derived type redeclares once).
    fn get_type_properties(o: &BoxedValue) -> Vec<(&'static MarkupProperty, &'static MarkupType)> {
        let mut seen: HashSet<&'static str> = HashSet::new();
        let mut result = Vec::new();
        for declaring_type in Self::get_type_chain(o) {
            for property in declaring_type.properties {
                if seen.insert(property.name) {
                    result.push((property, declaring_type));
                }
            }
        }
        result
    }

    /// `o.GetType().GetInterfaces()`: the contracts the type of an object and its base types
    /// implement, by their markup metadata.
    fn get_interfaces(o: &BoxedValue) -> Vec<&'static MarkupType> {
        let mut interfaces: Vec<&'static MarkupType> = Vec::new();
        let mut add = |handle: ValueType| {
            if let Some(markup) = MarkupType::find_by_handle(handle.id()) {
                if markup.kind == MarkupTypeKind::Interface && !interfaces.iter().any(|known| std::ptr::eq(*known, markup)) {
                    interfaces.push(markup);
                }
            }
        };

        if let Some(object) = ValueTypes::as_object(&**o) {
            let mut type_: Option<&'static TypeInfo> = Some(object.get_type());
            while let Some(current) = type_ {
                for interface in current.interfaces() {
                    add(interface());
                }
                type_ = current.base_type();
            }
        }
        for markup in Self::get_type_chain(o) {
            for interface in markup.interfaces {
                add(interface());
            }
        }

        interfaces
    }

    /// `FerroObject.PropertyChanged` of the selected entity.
    fn control_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let properties = self
            .property_index
            .borrow()
            .as_ref()
            .and_then(|index| index.get(&PropertyViewModelKey::Property(e.property())).cloned());
        if let Some(properties) = properties {
            for property in properties {
                property.update();
            }
        }

        if let Some(layout) = &self.layout {
            layout.control_property_changed(e);
        }
    }

    /// `INotifyPropertyChanged.PropertyChanged` of the selected entity.
    fn model_property_changed(&self, property_name: &str) {
        let properties = self
            .property_index
            .borrow()
            .as_ref()
            .and_then(|index| index.get(&PropertyViewModelKey::Name(property_name.to_string())).cloned());
        if let Some(properties) = properties {
            for property in properties {
                property.update();
            }
        }

        if !self.snapshot_frames() {
            self.post_update_styles();
        }
    }

    /// `IClassesChangedListener.Changed`.
    fn classes_changed(&self) {
        if !self.snapshot_frames() {
            self.post_update_styles();
        }
    }

    /// `Dispatcher.UIThread.Post(UpdateStyles)`.
    fn post_update_styles(&self) {
        let weak = self.weak_self.clone();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(this) = weak.upgrade() {
                    this.update_styles();
                }
            },
            DispatcherPriority::DEFAULT,
        );
    }

    fn update_styles(&self) {
        let mut active_count = 0;
        let applied_frames = self.applied_frames.to_vec();

        for style in &applied_frames {
            style.update();

            if style.is_active() {
                active_count += 1;
            }
        }

        let mut property_buckets: HashMap<&'static FerroProperty, Vec<SetterViewModelRef>> = HashMap::new();

        for style in applied_frames.iter().rev() {
            if !style.is_active() {
                continue;
            }

            for setter in style.setters() {
                match property_buckets.get_mut(setter.property()) {
                    Some(setters) => {
                        for other_setter in setters.iter() {
                            other_setter.set_is_active(false);
                        }

                        setter.set_is_active(true);

                        setters.push(setter.clone());
                    }
                    None => {
                        setter.set_is_active(true);

                        property_buckets.insert(setter.property(), vec![setter.clone()]);
                    }
                }
            }
        }

        for pseudo_class in self.pseudo_classes.to_vec() {
            pseudo_class.update();
        }

        self.set_frames_status(Some(format!("Value Frames ({}/{} active)", active_count, applied_frames.len())));
    }

    /// `FilterProperty`: whether the properties filter of the tree page shows a property.
    fn filter_property(&self, property: &PropertyViewModelRef) -> bool {
        match self.tree_page() {
            Some(tree_page) => tree_page.properties_filter().filter(&property.name()),
            None => true,
        }
    }

    /// The public properties of a type and of the contracts it implements
    /// (`GetAllPublicProperties`).
    fn get_all_public_properties(o: &BoxedValue) -> Vec<&'static MarkupProperty> {
        let mut result: Vec<&'static MarkupProperty> =
            Self::get_type_properties(o).into_iter().map(|(property, _)| property).collect();
        for i in Self::get_interfaces(o) {
            result.extend(i.properties.iter());
        }
        result
    }

    pub fn navigate_to_selected_property(&self) {
        let selected_property = self.selected_property();
        let selected_entity = self.selected_entity();
        let selected_entity_name = self.selected_entity_name();
        let (Some(selected_entity), Some(selected_property)) = (selected_entity, selected_property) else {
            return;
        };
        let property_type = selected_property.property_type();
        if property_type.is_string() || is_value_type(property_type) {
            return;
        }

        let property: Option<BoxedValue> = match &selected_property {
            PropertyViewModelRef::Ferro(ferro_property) => ValueTypes::as_object(&*selected_entity)
                .filter(|object| object.is::<Control>())
                .and_then(|control| ValueTypes::normalize(control.get_value_untyped(ferro_property.property()))),
            PropertyViewModelRef::Clr(clr_property) => Self::get_all_public_properties(&selected_entity)
                .into_iter()
                .find(|pi| std::ptr::eq(*pi, clr_property.property()))
                .and_then(|pi| pi.get)
                .and_then(|get| get(&[Some(selected_entity.clone())]).ok().flatten())
                .and_then(ValueTypes::normalize),
        };

        let Some(property) = property else {
            return;
        };

        self.selected_entities_stack
            .borrow_mut()
            .push((selected_entity_name.clone().unwrap_or_default(), selected_entity));

        let mut property_name = selected_property.name();

        // Strip out interface names.
        if let Some(p) = property_name.rfind('.') {
            property_name = property_name[p + 1..].to_string();
        }

        self.navigate_to_property(
            property,
            Some(format!("{}.{}", selected_entity_name.unwrap_or_default(), property_name)),
        );

        self.raise_property_changed("CanNavigateToParentProperty");
    }

    pub fn navigate_to_parent_property(&self) {
        let property = self.selected_entities_stack.borrow_mut().pop();
        if let Some((name, entry)) = property {
            self.navigate_to_property(entry, Some(name));

            self.raise_property_changed("CanNavigateToParentProperty");
        }
    }

    fn unsubscribe_entity(&self) {
        let subscription = self.entity_subscription.borrow_mut().take();
        match subscription {
            Some(EntitySubscription::Object(object, token)) => {
                object.remove_property_changed_handler(token);
            }
            Some(EntitySubscription::Model(model, token)) => {
                if let Some(notifier) = notifier_of(&model) {
                    notifier.property_changed().remove(token);
                }
            }
            None => {}
        }
    }

    pub fn navigate_to_property(&self, o: BoxedValue, entity_name: Option<String>) {
        self.unsubscribe_entity();

        self.set_selected_entity(Some(o.clone()));
        self.set_selected_entity_name(entity_name);
        self.set_selected_entity_type(Some(ValueTypes::to_display_string(Some(&o))));

        let mut properties = Self::get_ferro_properties(&o);
        properties.extend(Self::get_clr_properties(&o, self.show_implemented_interfaces.get()));
        for p in &properties {
            let is_pinned = self.pinned_properties.borrow().contains(&p.full_name());
            p.set_is_pinned(is_pinned);
        }

        let mut property_index: HashMap<PropertyViewModelKey, Vec<PropertyViewModelRef>> = HashMap::new();
        for property in &properties {
            property_index.entry(property.key()).or_default().push(property.clone());
        }
        *self.property_index.borrow_mut() = Some(property_index);

        let weak = self.weak_self.clone();
        let view = PropertiesView::new(
            properties,
            Rc::new(move |property: &PropertyViewModelRef| match weak.upgrade() {
                Some(this) => this.filter_property(property),
                None => true,
            }),
        );
        let weak = self.weak_self.clone();
        view.set_selection_changed(Rc::new(move |selected: Option<PropertyViewModelRef>| {
            if let Some(this) = weak.upgrade() {
                this.set_selected_property(selected);
            }
        }));
        if let Some(main_view) = self.tree_page().and_then(|tree_page| tree_page.main_view()) {
            view.set_property_type_columns(main_view.show_details_property_type());
        }
        self.set_properties_view(Some(view));

        if let Some(object) = ValueTypes::as_object(&*o) {
            let weak = self.weak_self.clone();
            let token = object.add_property_changed_handler(Rc::new(move |e: &FerroPropertyChangedEventArgs<'_>| {
                if let Some(this) = weak.upgrade() {
                    this.control_property_changed(e);
                }
            }));
            *self.entity_subscription.borrow_mut() = Some(EntitySubscription::Object(object, token));
        } else if let Some(notifier) = notifier_of(&o) {
            let weak = self.weak_self.clone();
            let token = notifier.property_changed().add(Rc::new(move |property_name: &str| {
                if let Some(this) = weak.upgrade() {
                    this.model_property_changed(property_name);
                }
            }));
            *self.entity_subscription.borrow_mut() = Some(EntitySubscription::Model(o.clone(), token));
        }
    }

    /// Selects the property `property` of the object of the details, navigating back to the
    /// object first.
    pub fn select_property(&self, property: &'static FerroProperty) {
        self.set_selected_property(None);

        let is_object = self
            .selected_entity()
            .and_then(|entity| ValueTypes::as_object(&*entity))
            .is_some_and(|entity| entity.ptr_eq(&self.ferro_object));
        if !is_object {
            self.navigate_to_property(self.boxed_object(), self.object_name());
        }

        let Some(properties_view) = self.properties_view() else {
            return;
        };

        for o in properties_view.items() {
            if let Some(property_vm) = o.as_ferro() {
                if property_vm.property() == property {
                    self.set_selected_property(Some(o.clone()));

                    break;
                }
            }
        }
    }

    pub fn update_properties_view(&self, show_implemented_interfaces: bool) {
        self.show_implemented_interfaces.set(show_implemented_interfaces);
        self.set_selected_property(None);
        self.navigate_to_property(self.boxed_object(), self.object_name());
    }

    /// Pins or unpins the property `parameter` (a property view model, or the row of one).
    pub fn toggle_pinned_property(&self, parameter: Option<BoxedValue>) {
        let model = parameter.as_ref().and_then(|parameter| {
            PropertyViewModelRef::from_boxed(parameter)
                .or_else(|| PropertiesViewItem::from_boxed(parameter).and_then(|item| item.property()))
        });
        if let Some(model) = model {
            let fullname = model.full_name();
            let contains = self.pinned_properties.borrow().contains(&fullname);
            if contains {
                self.pinned_properties.borrow_mut().remove(&fullname);
                model.set_is_pinned(false);
            } else {
                self.pinned_properties.borrow_mut().insert(fullname);
                model.set_is_pinned(true);
            }
            if let Some(properties_view) = self.properties_view() {
                properties_view.refresh();
            }
        }
    }

    /// Shows the type columns of the grid as the main view asks: the type of the property and
    /// of its value in one column, or in two (the bindings of `IsVisible` of the columns to
    /// `MainViewModel.ShowDetailsPropertyType` of the managed original).
    fn subscribe_to_main_view(&self) {
        let Some(main_view) = self.tree_page().and_then(|tree_page| tree_page.main_view()) else {
            return;
        };
        let weak = self.weak_self.clone();
        let token = main_view.property_changed().add(Rc::new(move |property_name: &str| {
            if property_name != "ShowDetailsPropertyType" {
                return;
            }
            if let Some(this) = weak.upgrade() {
                let show = this.tree_page().and_then(|tree_page| tree_page.main_view()).map(|main_view| main_view.show_details_property_type());
                if let (Some(show), Some(view)) = (show, this.properties_view()) {
                    view.set_property_type_columns(show);
                }
            }
        }));
        self.main_view_subscription.set(Some(token));
    }
}

/// Whether a type is a value type (`Type.IsValueType`): a primitive, a structure or an
/// enumeration with metadata, or the nullable form of one; not text, a class or a contract.
pub(crate) fn is_value_type(type_: ValueType) -> bool {
    if type_.is_string() || type_.is_object() {
        return false;
    }
    if ValueTypes::class_of(type_).is_some() || ValueTypes::is_reference(type_.id()) {
        return false;
    }
    if let Some(markup) = MarkupType::find_by_handle(type_.id()) {
        return matches!(markup.kind, MarkupTypeKind::Struct | MarkupTypeKind::Enum);
    }
    if MarkupType::find_by_nullable_handle(type_.id()).is_some() {
        return true;
    }
    if let Some(inner) = ValueTypes::nullable_inner(type_) {
        return is_value_type(inner);
    }
    type_.is::<bool>()
        || type_.is::<char>()
        || type_.is::<i8>()
        || type_.is::<i16>()
        || type_.is::<i32>()
        || type_.is::<i64>()
        || type_.is::<u8>()
        || type_.is::<u16>()
        || type_.is::<u32>()
        || type_.is::<u64>()
        || type_.is::<f32>()
        || type_.is::<f64>()
        || type_.is::<Option<bool>>()
        || type_.is::<Option<i32>>()
        || type_.is::<Option<f64>>()
}

/// `PropertyComparer.GroupIndex`.
fn group_index(group: &str) -> i32 {
    match group {
        "Pinned" => -1,
        "Properties" => 0,
        "Attached Properties" => 1,
        "CLR Properties" => 2,
        _ => 3,
    }
}

/// `PropertyComparer.Compare` as a sort key: by the index of the group, then by the name
/// (ordinal).
fn property_sort_key(property: &PropertyViewModelRef) -> (i32, String) {
    (group_index(&property.group()), property.name())
}

/// A row of the property grid: a group of properties, or a property of a group.
pub struct PropertiesViewItem {
    group: Option<String>,
    property: Option<PropertyViewModelRef>,
    children: Rc<Vec<Rc<PropertiesViewItem>>>,
    is_expanded: Cell<bool>,
    property_changed: Event<str>,
}

impl PartialEq for PropertiesViewItem {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The notifications of the property of a row (the row of a group raises its own).
impl INotifyPropertyChanged for PropertiesViewItem {
    fn property_changed(&self) -> &Event<str> {
        match &self.property {
            Some(property) => property.property_changed(),
            None => &self.property_changed,
        }
    }
}

impl PropertiesViewItem {
    /// States what the source and its columns observe of the rows: their property change
    /// notifications. Once per thread.
    fn register() {
        if !ModelTypes::is_registered(TypeId::of::<PropertiesViewItem>()) {
            ModelTypes::register::<PropertiesViewItem>(|builder| builder.notify_property_changed());
        }
    }

    /// The row of the group `name` with the properties `children`.
    pub fn group(name: String, children: Vec<Rc<PropertiesViewItem>>, is_expanded: bool) -> Rc<Self> {
        Self::register();
        Rc::new(Self {
            group: Some(name),
            property: None,
            children: Rc::new(children),
            is_expanded: Cell::new(is_expanded),
            property_changed: Event::new(),
        })
    }

    /// The row of a property.
    pub fn property_item(property: PropertyViewModelRef) -> Rc<Self> {
        Self::register();
        Rc::new(Self {
            group: None,
            property: Some(property),
            children: Rc::new(Vec::new()),
            is_expanded: Cell::new(false),
            property_changed: Event::new(),
        })
    }

    /// The row of an untyped value (a data context of a cell).
    pub fn from_boxed(value: &BoxedValue) -> Option<Rc<Self>> {
        if let Some(item) = value.downcast_ref::<Rc<PropertiesViewItem>>() {
            return Some(item.clone());
        }
        let any: Rc<dyn std::any::Any> = value.clone();
        any.downcast::<PropertiesViewItem>().ok()
    }

    pub fn is_group(&self) -> bool {
        self.group.is_some()
    }

    /// The name of the group, or of the property.
    pub fn name(&self) -> String {
        match (&self.group, &self.property) {
            (Some(group), _) => group.clone(),
            (None, Some(property)) => property.name(),
            (None, None) => String::new(),
        }
    }

    /// The group of the row: the group itself, or the group of the property.
    pub fn group_name(&self) -> String {
        match (&self.group, &self.property) {
            (Some(group), _) => group.clone(),
            (None, Some(property)) => property.group(),
            (None, None) => String::new(),
        }
    }

    /// The property of the row; `None` for a group.
    pub fn property(&self) -> Option<PropertyViewModelRef> {
        self.property.clone()
    }

    /// The property as a value of its class; `None` for a group.
    pub fn boxed_property(&self) -> Option<BoxedValue> {
        self.property.as_ref().map(PropertyViewModelRef::boxed)
    }

    pub fn children(&self) -> Rc<Vec<Rc<PropertiesViewItem>>> {
        self.children.clone()
    }

    /// The number of properties of a group (the item count of a group header).
    pub fn item_count(&self) -> i32 {
        self.children.len() as i32
    }

    pub fn is_pinned(&self) -> bool {
        self.property.as_ref().is_some_and(|property| property.is_pinned())
    }

    pub fn set_is_pinned(&self, value: bool) {
        if let Some(property) = &self.property {
            property.set_is_pinned(value);
        }
    }

    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    pub fn set_is_expanded(&self, value: bool) {
        if self.is_expanded.replace(value) != value && self.property.is_none() {
            self.property_changed.raise("IsExpanded");
        }
    }

    pub fn type_(&self) -> String {
        self.property.as_ref().map(PropertyViewModelRef::type_).unwrap_or_default()
    }

    pub fn assigned_type(&self) -> String {
        self.property.as_ref().map(|property| type_name(property.assigned_type())).unwrap_or_default()
    }

    pub fn property_type(&self) -> String {
        self.property.as_ref().map(|property| type_name(property.property_type())).unwrap_or_default()
    }

    pub fn priority(&self) -> String {
        self.property.as_ref().map(PropertyViewModelRef::priority).unwrap_or_default()
    }
}

ferro_markup_type!(class PropertiesViewItem {
    this: Rc<PropertiesViewItem>,
    handles: [PropertiesViewItem, Rc<PropertiesViewItem>, Option<Rc<PropertiesViewItem>>],
    properties: [
        Name: String { get: PropertiesViewItem::name },
        IsGroup: bool { get: PropertiesViewItem::is_group },
        ItemCount: i32 { get: PropertiesViewItem::item_count },
        Property: Option<BoxedValue> { get: PropertiesViewItem::boxed_property },
        IsPinned: bool { get: PropertiesViewItem::is_pinned, set: PropertiesViewItem::set_is_pinned },
        IsExpanded: bool { get: PropertiesViewItem::is_expanded, set: PropertiesViewItem::set_is_expanded },
        Type: String { get: PropertiesViewItem::type_ },
        AssignedType: String { get: PropertiesViewItem::assigned_type },
        PropertyType: String { get: PropertiesViewItem::property_type },
        Priority: String { get: PropertiesViewItem::priority },
    ],
    notify_property_changed: PropertiesViewItem,
});

/// The key of the data template of the cells of the name column, a resource of the view.
pub const PROPERTY_NAME_CELL_TEMPLATE: &str = "PropertyNameCell";
/// The key of the data template of the cells of the value column, a resource of the view.
pub const PROPERTY_VALUE_CELL_TEMPLATE: &str = "PropertyValueCell";

/// A comparison of the rows of the grid for a sort by a column: the groups keep their order
/// (`PropertyComparer`), the properties of a group follow `compare` in the direction asked.
fn rows_comparison(
    ascending: bool,
    compare: fn(&PropertyViewModelRef, &PropertyViewModelRef) -> Ordering,
) -> Comparison<PropertiesViewItem> {
    Rc::new(move |x: &Rc<PropertiesViewItem>, y: &Rc<PropertiesViewItem>| match (&x.property, &y.property) {
        (Some(x), Some(y)) => {
            let ordering = compare(x, y);
            if ascending {
                ordering
            } else {
                ordering.reverse()
            }
        }
        _ => {
            let group_x = group_index(&x.group_name());
            let group_y = group_index(&y.group_name());
            group_x.cmp(&group_y).then_with(|| x.group_name().cmp(&y.group_name()))
        }
    })
}

/// The properties of an object for the property grid: the counterpart of the
/// `DataGridCollectionView` of the managed original, with its group description (by
/// `Group`), its sort description (`PropertyComparer`) and its filter, presented as a
/// hierarchical TreeDataGrid source whose root rows are the groups.
///
/// Deviation (DEVIATIONS.md, Developer tools): the property grid is a TreeDataGrid, not a DataGrid.
pub struct PropertiesView {
    weak_self: Weak<PropertiesView>,
    properties: Vec<PropertyViewModelRef>,
    filter: Rc<dyn Fn(&PropertyViewModelRef) -> bool>,
    items: RefCell<Rc<Vec<Rc<PropertiesViewItem>>>>,
    collapsed_groups: RefCell<HashSet<String>>,
    source: Rc<HierarchicalTreeDataGridSource<PropertiesViewItem>>,
    type_column: RefCell<Option<ColumnRef<PropertiesViewItem>>>,
    assigned_type_column: RefCell<Option<ColumnRef<PropertiesViewItem>>>,
    property_type_column: RefCell<Option<ColumnRef<PropertiesViewItem>>>,
    selection_changed: RefCell<Option<Rc<dyn Fn(Option<PropertyViewModelRef>)>>>,
    updating_selection: Cell<bool>,
}

impl PartialEq for PropertiesView {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

fn header(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

impl PropertiesView {
    pub fn new(properties: Vec<PropertyViewModelRef>, filter: Rc<dyn Fn(&PropertyViewModelRef) -> bool>) -> Rc<Self> {
        let source = HierarchicalTreeDataGridSource::new(
            Rc::new(Vec::<Rc<PropertiesViewItem>>::new()) as Models<PropertiesViewItem>
        );

        let this = Rc::new_cyclic(|weak_self: &Weak<Self>| Self {
            weak_self: weak_self.clone(),
            properties,
            filter,
            items: RefCell::new(Rc::new(Vec::new())),
            collapsed_groups: RefCell::new(HashSet::new()),
            source,
            type_column: RefCell::new(None),
            assigned_type_column: RefCell::new(None),
            property_type_column: RefCell::new(None),
            selection_changed: RefCell::new(None),
            updating_selection: Cell::new(false),
        });

        this.create_columns();
        this.refresh();

        if let Some(row_selection) = this.source.row_selection() {
            let weak = this.weak_self.clone();
            row_selection.add_selection_changed(Rc::new(
                move |_: &TreeSelectionModelSelectionChangedEventArgsOf<PropertiesViewItem>| {
                    if let Some(this) = weak.upgrade() {
                        this.on_selection_changed();
                    }
                },
            ));
        }

        this
    }

    /// The columns of the grid, as upstream's columns: the name (with the pin of a property, and
    /// the expander of a group), the value (an editor), the type, the assigned type and the type
    /// of the property (shown instead of the type), and the priority.
    fn create_columns(&self) {
        let columns = self.source.columns();

        // `DataGridTemplateColumn Header="Property"`, under the expander of the groups.
        let name_options = TemplateColumnOptions::<PropertiesViewItem>::new();
        name_options.set_can_user_resize_column(Some(true));
        name_options.set_can_user_sort_column(Some(true));
        name_options.set_compare_ascending(Some(rows_comparison(true, |x, y| x.name().cmp(&y.name()))));
        name_options.set_compare_descending(Some(rows_comparison(false, |x, y| x.name().cmp(&y.name()))));
        let name_column = TemplateColumn::from_resource_keys(
            header("Property"),
            ResourceKey::from(PROPERTY_NAME_CELL_TEMPLATE),
            None,
            // Deviation (DEVIATIONS.md, Developer tools): as wide as the names, so that the
            // columns of types after it do not take their room.
            Some(GridLength::new(1.0, GridUnitType::Auto)),
            Some(Rc::new(name_options)),
        );
        let weak = self.weak_self.clone();
        columns.add(HierarchicalExpanderColumn::new(
            name_column as Rc<dyn IViewColumn<PropertiesViewItem>>,
            |x: &Rc<PropertiesViewItem>| {
                if x.is_group() {
                    Some(x.children() as Models<PropertiesViewItem>)
                } else {
                    None
                }
            },
            Some(Accessor::member("IsGroup", |x: &Rc<PropertiesViewItem>| x.is_group())),
            Some(
                Accessor::member("IsExpanded", |x: &Rc<PropertiesViewItem>| x.is_expanded()).with_setter(
                    move |x: &Rc<PropertiesViewItem>, value: bool| {
                        x.set_is_expanded(value);
                        if let Some(this) = weak.upgrade() {
                            this.group_expanded_changed(x, value);
                        }
                    },
                ),
            ),
        ) as ColumnRef<PropertiesViewItem>);

        // `DataGridTemplateColumn Header="Value" Width="100"`: the editor of the value.
        let value_options = TemplateColumnOptions::<PropertiesViewItem>::new();
        value_options.set_can_user_resize_column(Some(true));
        value_options.set_can_user_sort_column(Some(false));
        columns.add(TemplateColumn::from_resource_keys(
            header("Value"),
            ResourceKey::from(PROPERTY_VALUE_CELL_TEMPLATE),
            None,
            // Deviation (DEVIATIONS.md, Developer tools): 200 px, wide enough for the editors of the
            // compact form (upstream: 100).
            Some(GridLength::new(200.0, GridUnitType::Pixel)),
            Some(Rc::new(value_options)),
        ) as ColumnRef<PropertiesViewItem>);

        let type_column =
            Self::text_column("Type", "Type", |x: &Rc<PropertiesViewItem>| x.type_(), |x, y| x.type_().cmp(&y.type_()));
        let assigned_type_column = Self::text_column(
            "Assigned Type",
            "AssignedType",
            |x: &Rc<PropertiesViewItem>| x.assigned_type(),
            |x, y| type_name(x.assigned_type()).cmp(&type_name(y.assigned_type())),
        );
        let property_type_column = Self::text_column(
            "Property Type",
            "PropertyType",
            |x: &Rc<PropertiesViewItem>| x.property_type(),
            |x, y| type_name(x.property_type()).cmp(&type_name(y.property_type())),
        );
        let priority_column = Self::text_column(
            "Priority",
            "Priority",
            |x: &Rc<PropertiesViewItem>| x.priority(),
            |x, y| x.priority().cmp(&y.priority()),
        );

        columns.add(type_column.clone());
        columns.add(assigned_type_column.clone());
        columns.add(property_type_column.clone());
        columns.add(priority_column);

        *self.type_column.borrow_mut() = Some(type_column);
        *self.assigned_type_column.borrow_mut() = Some(assigned_type_column);
        *self.property_type_column.borrow_mut() = Some(property_type_column);

        self.set_property_type_columns(false);
    }

    /// A read-only text column of a value of the rows (`DataGridTextColumn IsReadOnly="True"`).
    fn text_column(
        header_text: &str,
        member: &'static str,
        getter: impl Fn(&Rc<PropertiesViewItem>) -> String + 'static,
        compare: fn(&PropertyViewModelRef, &PropertyViewModelRef) -> Ordering,
    ) -> ColumnRef<PropertiesViewItem> {
        let options = TextColumnOptions::<PropertiesViewItem>::new();
        options.set_can_user_resize_column(Some(true));
        options.set_can_user_sort_column(Some(true));
        options.set_compare_ascending(Some(rows_comparison(true, compare)));
        options.set_compare_descending(Some(rows_comparison(false, compare)));
        TextColumn::new(header(header_text), Accessor::member(member, getter), None, Some(Rc::new(options)))
            as ColumnRef<PropertiesViewItem>
    }

    /// Shows the type of a property and of its value in one column, or (with `show_property_type`)
    /// in two.
    pub fn set_property_type_columns(&self, show_property_type: bool) {
        if let Some(column) = self.type_column.borrow().as_ref() {
            IColumn::set_is_visible(&**column, !show_property_type);
        }
        if let Some(column) = self.assigned_type_column.borrow().as_ref() {
            IColumn::set_is_visible(&**column, show_property_type);
        }
        if let Some(column) = self.property_type_column.borrow().as_ref() {
            IColumn::set_is_visible(&**column, show_property_type);
        }
    }

    /// The source of the grid.
    pub fn source(&self) -> Rc<HierarchicalTreeDataGridSource<PropertiesViewItem>> {
        self.source.clone()
    }

    /// The source of the grid, as the grid takes it.
    pub fn source_ref(&self) -> TreeDataGridSourceRef {
        TreeDataGridSourceRef::of::<PropertiesViewItem, _>(self.source.clone())
    }

    /// The rows of the groups.
    pub fn groups(&self) -> Rc<Vec<Rc<PropertiesViewItem>>> {
        self.items.borrow().clone()
    }

    /// The properties the view shows, in the order it shows them (`foreach (object o in
    /// PropertiesView)`).
    pub fn items(&self) -> Vec<PropertyViewModelRef> {
        self.groups().iter().flat_map(|group| group.children().iter().filter_map(|item| item.property()).collect::<Vec<_>>()).collect()
    }

    /// Sets the handler of a change of the selected row (the selected property, `None` for a
    /// group or no row).
    pub fn set_selection_changed(&self, handler: Rc<dyn Fn(Option<PropertyViewModelRef>)>) {
        *self.selection_changed.borrow_mut() = Some(handler);
    }

    /// Groups, sorts and filters the properties again (`Refresh()`), keeping the selected
    /// property selected while it is shown.
    pub fn refresh(&self) {
        let selected = self.selected();

        let mut properties: Vec<PropertyViewModelRef> =
            self.properties.iter().filter(|property| (self.filter)(*property)).cloned().collect();
        // `PropertyComparer`, with the keys of each property read once (a stable sort, as
        // `sort_by`): the group and the name are new strings at every read.
        properties.sort_by_cached_key(property_sort_key);

        let mut groups: Vec<(String, Vec<Rc<PropertiesViewItem>>)> = Vec::new();
        for property in properties {
            let group = property.group();
            let item = PropertiesViewItem::property_item(property);
            match groups.iter_mut().find(|(name, _)| *name == group) {
                Some((_, items)) => items.push(item),
                None => groups.push((group, vec![item])),
            }
        }

        let collapsed_groups = self.collapsed_groups.borrow().clone();
        let items: Rc<Vec<Rc<PropertiesViewItem>>> = Rc::new(
            groups
                .into_iter()
                .map(|(name, children)| {
                    let is_expanded = !collapsed_groups.contains(&name);
                    PropertiesViewItem::group(name, children, is_expanded)
                })
                .collect(),
        );
        *self.items.borrow_mut() = items.clone();

        self.updating_selection.set(true);
        self.source.set_items(items as Models<PropertiesViewItem>);
        let reselected = selected.as_ref().is_some_and(|selected| self.select_row(Some(selected)));
        self.updating_selection.set(false);

        if selected.is_some() && !reselected {
            self.on_selection_changed();
        }
    }

    /// The index path of the row of a property.
    pub fn index_path_of(&self, property: &PropertyViewModelRef) -> Option<IndexPath> {
        for (group_index, group) in self.groups().iter().enumerate() {
            for (index, item) in group.children().iter().enumerate() {
                if item.property().as_ref() == Some(property) {
                    return Some(IndexPath::new(group_index as i32).append(index as i32));
                }
            }
        }
        None
    }

    /// The index of the row of a property among the rows the grid shows; `-1` when it shows
    /// none.
    pub fn row_index_of(&self, property: &PropertyViewModelRef) -> i32 {
        match self.index_path_of(property) {
            Some(index) => self.source.get_row_index(&index, 0),
            None => -1,
        }
    }

    /// The selected property; `None` when no row or a group is selected.
    pub fn selected(&self) -> Option<PropertyViewModelRef> {
        self.source
            .row_selection()
            .and_then(|selection| selection.base().selected_item())
            .and_then(|item| item.property())
    }

    /// Selects the row of a property, or clears the selection.
    pub fn select(&self, property: Option<&PropertyViewModelRef>) {
        if self.updating_selection.get() || self.selected().as_ref() == property {
            return;
        }
        self.updating_selection.set(true);
        self.select_row(property);
        self.updating_selection.set(false);
    }

    /// Selects the row of a property, expanding its group; `false` when the view does not show
    /// it (the selection is cleared then).
    fn select_row(&self, property: Option<&PropertyViewModelRef>) -> bool {
        let Some(selection) = self.source.row_selection() else {
            return false;
        };
        let index = property.and_then(|property| self.index_path_of(property));
        match index {
            Some(index) => {
                self.source.expand(&index.parent());
                selection.base().set_selected_index(index);
                true
            }
            None => {
                selection.base().clear();
                false
            }
        }
    }

    fn on_selection_changed(&self) {
        if self.updating_selection.get() {
            return;
        }
        let handler = self.selection_changed.borrow().clone();
        if let Some(handler) = handler {
            handler(self.selected());
        }
    }

    /// Remembers the groups the user collapsed, for the next refresh.
    fn group_expanded_changed(&self, item: &PropertiesViewItem, is_expanded: bool) {
        if !item.is_group() {
            return;
        }
        let mut collapsed_groups = self.collapsed_groups.borrow_mut();
        if is_expanded {
            collapsed_groups.remove(&item.name());
        } else {
            collapsed_groups.insert(item.name());
        }
    }
}

ferro_markup_type!(class PropertiesView {
    this: Rc<PropertiesView>,
    handles: [PropertiesView, Rc<PropertiesView>, Option<Rc<PropertiesView>>],
    properties: [
        Source: TreeDataGridSourceRef { get: PropertiesView::source_ref },
    ],
    methods: [
        fn Refresh() => PropertiesView::refresh,
    ],
});

ferro_markup_type!(class ControlDetailsViewModel {
    this: Rc<ControlDetailsViewModel>,
    handles: [ControlDetailsViewModel, Rc<ControlDetailsViewModel>, Option<Rc<ControlDetailsViewModel>>],
    properties: [
        CanNavigateToParentProperty: bool { get: ControlDetailsViewModel::can_navigate_to_parent_property },
        TreePage: Option<Rc<TreePageViewModel>> { get: ControlDetailsViewModel::tree_page },
        PropertiesView: Option<Rc<PropertiesView>> { get: ControlDetailsViewModel::properties_view },
        AppliedFrames: FerroList<Rc<ValueFrameViewModel>> { get: ControlDetailsViewModel::applied_frames },
        PseudoClasses: FerroList<Rc<PseudoClassViewModel>> { get: ControlDetailsViewModel::pseudo_classes },
        SelectedEntity: Option<BoxedValue> {
            get: ControlDetailsViewModel::selected_entity,
            set: ControlDetailsViewModel::set_selected_entity
        },
        SelectedEntityName: Option<String> {
            get: ControlDetailsViewModel::selected_entity_name,
            set: ControlDetailsViewModel::set_selected_entity_name
        },
        SelectedEntityType: Option<String> {
            get: ControlDetailsViewModel::selected_entity_type,
            set: ControlDetailsViewModel::set_selected_entity_type
        },
        SelectedProperty: Option<BoxedValue> {
            get: |this: &Rc<ControlDetailsViewModel>| this.selected_property().map(|property| property.boxed()),
            set: |this: &Rc<ControlDetailsViewModel>, value: Option<BoxedValue>| {
                this.set_selected_property(value.as_ref().and_then(PropertyViewModelRef::from_boxed))
            }
        },
        SnapshotFrames: bool {
            get: ControlDetailsViewModel::snapshot_frames,
            set: ControlDetailsViewModel::set_snapshot_frames
        },
        ShowInactiveFrames: bool {
            get: ControlDetailsViewModel::show_inactive_frames,
            set: ControlDetailsViewModel::set_show_inactive_frames
        },
        FramesStatus: Option<String> {
            get: ControlDetailsViewModel::frames_status,
            set: ControlDetailsViewModel::set_frames_status
        },
        Layout: Option<Rc<ControlLayoutViewModel>> { get: ControlDetailsViewModel::layout },
    ],
    methods: [
        fn UpdateStyleFilters() => ControlDetailsViewModel::update_style_filters,
        fn NavigateToSelectedProperty() => ControlDetailsViewModel::navigate_to_selected_property,
        fn NavigateToParentProperty() => ControlDetailsViewModel::navigate_to_parent_property,
        fn TogglePinnedProperty(Option<BoxedValue>) => ControlDetailsViewModel::toggle_pinned_property,
        fn Dispose() => ControlDetailsViewModel::dispose,
    ],
    notify_property_changed: ControlDetailsViewModel,
});

/// What the untyped value conversions must know about the view models of the control details:
/// they are reference types, held by bindings as the shared objects they are. Registered once
/// with `ValueTypes::register_global` from the `register_types()` of the crate.
pub fn register_details_value_types() {
    use super::{BindingSetterViewModel, ResourceSetterViewModel, SetterViewModel};

    ValueTypes::register_reference::<ControlDetailsViewModel>();
    ValueTypes::register_reference::<PropertiesView>();
    ValueTypes::register_reference::<PropertiesViewItem>();
    ValueTypes::register_reference::<ControlLayoutViewModel>();
    ValueTypes::register_reference::<ValueFrameViewModel>();
    ValueTypes::register_reference::<SetterViewModel>();
    ValueTypes::register_reference::<BindingSetterViewModel>();
    ValueTypes::register_reference::<ResourceSetterViewModel>();
    ValueTypes::register_reference::<PseudoClassViewModel>();
    ValueTypes::register_reference::<FerroPropertyViewModel>();
    ValueTypes::register_reference::<ClrPropertyViewModel>();
}

// The lists of the control details that items controls show: their markup metadata gives the
// compiler the type of their items (the data type of the item templates of the documents).
ferroui_controls::ferro_markup_list!(pub ValueFrameViewModelList: Rc<ValueFrameViewModel>);
ferroui_controls::ferro_markup_list!(pub PseudoClassViewModelList: Rc<PseudoClassViewModel>);
