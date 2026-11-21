//! Container and segment portions of the pinned RibbonItemModel.cs.
use super::*;
use crate::{ObservableObject, ObservableObjectImpl, RibbonItemSize, RibbonString};
use ferroui_system::{
    collections::object_model::ObservableCollection, component_model::INotifyPropertyChanged,
    events::Event,
};
use std::{cell::RefCell, rc::Rc};

pub struct RibbonButtonGroupModel {
    base: RibbonItemModel,
    items: ObservableCollection<Option<Rc<dyn RibbonItemModelImpl>>>,
}
pub trait RibbonButtonGroupModelImpl: RibbonItemModelImpl {
    fn button_group_model(&self) -> &RibbonButtonGroupModel;
}
pub trait RibbonButtonGroupModelExt: RibbonButtonGroupModelImpl {
    fn items(&self) -> ObservableCollection<Option<Rc<dyn RibbonItemModelImpl>>> {
        self.button_group_model().items.clone()
    }
}
impl<T: RibbonButtonGroupModelImpl + ?Sized> RibbonButtonGroupModelExt for T {}
impl RibbonButtonGroupModel {
    pub fn construct(id: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, None),
            items: ObservableCollection::new_managed(),
        }
    }
    pub fn initialize<O: RibbonButtonGroupModelImpl + ?Sized>(
        owner: &O,
        items: impl IntoIterator<Item = Option<Rc<dyn RibbonItemModelImpl>>>,
    ) {
        owner.set_size(RibbonItemSize::Small);
        owner.set_can_add_to_quick_access(false);
        for item in items {
            owner.items().add(item);
        }
    }
    pub fn initialize_nullable<O: RibbonButtonGroupModelImpl + ?Sized>(
        owner: &O,
        items: Option<Vec<Option<Rc<dyn RibbonItemModelImpl>>>>,
    ) {
        owner.set_size(RibbonItemSize::Small);
        owner.set_can_add_to_quick_access(false);
        for item in items.expect("NullReference: items") {
            owner.items().add(item);
        }
    }
    pub fn with_nullable_items(
        id: Option<RibbonString>,
        items: Option<Vec<Option<Rc<dyn RibbonItemModelImpl>>>>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id));
        Self::initialize_nullable(model.as_ref(), items);
        model
    }
    pub fn new(id: Option<RibbonString>) -> Rc<Self> {
        Self::with_items(id, std::iter::empty())
    }
    pub fn with_items(
        id: Option<RibbonString>,
        items: impl IntoIterator<Item = Option<Rc<dyn RibbonItemModelImpl>>>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id));
        Self::initialize(model.as_ref(), items);
        model
    }
    pub fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonButtonGroupModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonButtonGroupModel {}
impl RibbonNodeModelImpl for RibbonButtonGroupModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonButtonGroupModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonButtonGroupModelImpl for RibbonButtonGroupModel {
    fn button_group_model(&self) -> &RibbonButtonGroupModel {
        self
    }
}
impl PartialEq for RibbonButtonGroupModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}

pub struct RibbonRowModel {
    base: RibbonButtonGroupModel,
}
pub trait RibbonRowModelImpl: RibbonButtonGroupModelImpl {
    fn row_model(&self) -> &RibbonRowModel;
}
impl RibbonRowModel {
    pub fn construct(id: Option<RibbonString>) -> Self {
        Self {
            base: RibbonButtonGroupModel::construct(id),
        }
    }
    pub fn with_nullable_items(
        id: Option<RibbonString>,
        items: Option<Vec<Option<Rc<dyn RibbonItemModelImpl>>>>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id));
        RibbonButtonGroupModel::initialize_nullable(model.as_ref(), items);
        model
    }
    pub fn new(id: Option<RibbonString>) -> Rc<Self> {
        Self::with_items(id, std::iter::empty())
    }
    pub fn with_items(
        id: Option<RibbonString>,
        items: impl IntoIterator<Item = Option<Rc<dyn RibbonItemModelImpl>>>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id));
        RibbonButtonGroupModel::initialize(model.as_ref(), items);
        model
    }
    pub fn button_group_model(&self) -> &RibbonButtonGroupModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonRowModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonRowModel {}
impl RibbonNodeModelImpl for RibbonRowModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.item_model().node_model()
    }
}
impl RibbonItemModelImpl for RibbonRowModel {
    fn item_model(&self) -> &RibbonItemModel {
        self.base.item_model()
    }
}
impl RibbonButtonGroupModelImpl for RibbonRowModel {
    fn button_group_model(&self) -> &RibbonButtonGroupModel {
        &self.base
    }
}
impl RibbonRowModelImpl for RibbonRowModel {
    fn row_model(&self) -> &RibbonRowModel {
        self
    }
}
impl PartialEq for RibbonRowModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RibbonGridSize {
    pub rows: i32,
    pub columns: i32,
}
impl RibbonGridSize {
    pub const fn new(rows: i32, columns: i32) -> Self {
        Self { rows, columns }
    }
}
impl RibbonGridSize {
    pub fn to_ribbon_string(&self) -> RibbonString {
        let format = crate::commands::current_number_format();
        let mut units = Vec::new();
        for (index, value) in [self.columns, self.rows].into_iter().enumerate() {
            if index == 1 {
                units.push(120);
            }
            if value < 0 {
                units.extend_from_slice(format.negative_sign.as_utf16());
            }
            units.extend(value.unsigned_abs().to_string().encode_utf16());
        }
        RibbonString::from_utf16(units)
    }
}
impl std::fmt::Display for RibbonGridSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            &self
                .to_ribbon_string()
                .to_utf8()
                .map_err(|_| std::fmt::Error)?,
        )
    }
}

pub struct RibbonSegmentModel {
    base: RibbonNodeModel,
    value: RefCell<Option<RibbonObject>>,
}
pub trait RibbonSegmentModelImpl: RibbonNodeModelImpl {
    fn segment_model(&self) -> &RibbonSegmentModel;
}
pub trait RibbonSegmentModelExt: RibbonSegmentModelImpl {
    fn value(&self) -> Option<RibbonObject> {
        self.segment_model().value.borrow().clone()
    }
    fn set_value(&self, value: Option<RibbonObject>) {
        ObservableObject::set_property(self, &self.segment_model().value, value, "Value");
    }
}
impl<T: RibbonSegmentModelImpl + ?Sized> RibbonSegmentModelExt for T {}
impl PartialEq for dyn RibbonSegmentModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl RibbonSegmentModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonNodeModel::construct(id, label),
            value: RefCell::default(),
        }
    }
    pub fn initialize<O: RibbonSegmentModelImpl + ?Sized>(
        owner: &O,
        original_id: Option<RibbonString>,
        icon: Option<Rc<RibbonIcon>>,
        value: Option<RibbonObject>,
    ) {
        owner.set_icon(icon);
        owner
            .segment_model()
            .value
            .replace(value.or_else(|| original_id.map(RibbonObject::value)));
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(id, label, None, None)
    }
    pub fn with_options(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        icon: Option<Rc<RibbonIcon>>,
        value: Option<RibbonObject>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id.clone(), label));
        Self::initialize(model.as_ref(), id, icon, value);
        model
    }
    pub fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonSegmentModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonSegmentModel {}
impl RibbonNodeModelImpl for RibbonSegmentModel {
    fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl RibbonSegmentModelImpl for RibbonSegmentModel {
    fn segment_model(&self) -> &RibbonSegmentModel {
        self
    }
}
impl PartialEq for RibbonSegmentModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}

pub struct RibbonSegmentedModel {
    base: RibbonItemModel,
    segments: ObservableCollection<Option<Rc<dyn RibbonSegmentModelImpl>>>,
    selected_segment: RefCell<Option<Rc<dyn RibbonSegmentModelImpl>>>,
}
pub trait RibbonSegmentedModelImpl: RibbonItemModelImpl {
    fn segmented_model(&self) -> &RibbonSegmentedModel;
}
pub trait RibbonSegmentedModelExt: RibbonSegmentedModelImpl {
    fn segments(&self) -> ObservableCollection<Option<Rc<dyn RibbonSegmentModelImpl>>> {
        self.segmented_model().segments.clone()
    }
    fn selected_segment(&self) -> Option<Rc<dyn RibbonSegmentModelImpl>> {
        self.segmented_model().selected_segment.borrow().clone()
    }
    fn set_selected_segment(&self, value: Option<Rc<dyn RibbonSegmentModelImpl>>) {
        ObservableObject::set_property(
            self,
            &self.segmented_model().selected_segment,
            value,
            "SelectedSegment",
        );
    }
}
impl<T: RibbonSegmentedModelImpl + ?Sized> RibbonSegmentedModelExt for T {}
impl RibbonSegmentedModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
            segments: ObservableCollection::new_managed(),
            selected_segment: RefCell::default(),
        }
    }
    pub fn initialize<O: RibbonSegmentedModelImpl + ?Sized>(owner: &O) {
        owner.set_can_add_to_quick_access(false);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        let model = Rc::new(Self::construct(id, label));
        Self::initialize(model.as_ref());
        model
    }
    pub fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonSegmentedModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonSegmentedModel {}
impl RibbonNodeModelImpl for RibbonSegmentedModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonSegmentedModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonSegmentedModelImpl for RibbonSegmentedModel {
    fn segmented_model(&self) -> &RibbonSegmentedModel {
        self
    }
}
impl PartialEq for RibbonSegmentedModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
