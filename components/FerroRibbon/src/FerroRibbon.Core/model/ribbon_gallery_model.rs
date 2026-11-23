// Port of the same-named pinned C# file
use super::*;
use crate::{
    ObservableObject, ObservableObjectImpl, RibbonItemSize, RibbonString,
    commands::ICommand,
    theming::{RibbonColor, RibbonColorSwatch},
};
use crate::collections::ObservableCollection;
use ferroui_system::{
    component_model::INotifyPropertyChanged,
    events::Event,
};
use std::{cell::RefCell, rc::Rc};
pub struct RibbonGalleryItemModel {
    base: RibbonNodeModel,
    category: RefCell<Option<RibbonString>>,
    content: RefCell<Option<RibbonObject>>,
    value: RefCell<Option<RibbonObject>>,
}
pub trait RibbonGalleryItemModelImpl: RibbonNodeModelImpl {
    fn ribbon_gallery_item_model(&self) -> &RibbonGalleryItemModel;
}
pub trait RibbonGalleryItemModelExt: RibbonGalleryItemModelImpl {
    fn category(&self) -> Option<RibbonString> {
        self.ribbon_gallery_item_model()
            .category
            .borrow()
            .to_owned()
    }
    fn set_category(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_item_model().category,
            value,
            "Category",
        );
    }
    fn content(&self) -> Option<RibbonObject> {
        self.ribbon_gallery_item_model().content.borrow().to_owned()
    }
    fn set_content(&self, value: Option<RibbonObject>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_item_model().content,
            value,
            "Content",
        );
    }
    fn value(&self) -> Option<RibbonObject> {
        self.ribbon_gallery_item_model().value.borrow().to_owned()
    }
    fn set_value(&self, value: Option<RibbonObject>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_item_model().value,
            value,
            "Value",
        );
    }
}
impl<T: RibbonGalleryItemModelImpl + ?Sized> RibbonGalleryItemModelExt for T {}
impl PartialEq for dyn RibbonGalleryItemModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl RibbonGalleryItemModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonNodeModel::construct(id, label),
            category: RefCell::new(None),
            content: RefCell::new(None),
            value: RefCell::new(None),
        }
    }
    pub fn initialize<O: RibbonGalleryItemModelImpl + ?Sized>(
        owner: &O,
        icon: Option<Rc<RibbonIcon>>,
        category: Option<RibbonString>,
        value: Option<RibbonObject>,
    ) {
        owner.set_icon(icon);
        owner.ribbon_gallery_item_model().category.replace(category);
        owner.ribbon_gallery_item_model().value.replace(value);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(id, label, None, None, None)
    }
    pub fn with_options(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        icon: Option<Rc<RibbonIcon>>,
        category: Option<RibbonString>,
        value: Option<RibbonObject>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id, label));
        Self::initialize(model.as_ref(), icon, category, value);
        model
    }
    pub fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonGalleryItemModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonGalleryItemModel {}
impl RibbonNodeModelImpl for RibbonGalleryItemModel {
    fn node_model(&self) -> &RibbonNodeModel {
        &self.base
    }
}
impl RibbonGalleryItemModelImpl for RibbonGalleryItemModel {
    fn ribbon_gallery_item_model(&self) -> &RibbonGalleryItemModel {
        self
    }
}
impl PartialEq for RibbonGalleryItemModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonGalleryModel {
    base: RibbonItemModel,
    selected_item: RefCell<Option<Rc<dyn RibbonGalleryItemModelImpl>>>,
    min_columns: RefCell<i32>,
    max_columns: RefCell<i32>,
    drop_down_columns: RefCell<i32>,
    item_width: RefCell<f64>,
    item_height: RefCell<f64>,
    is_filter_enabled: RefCell<bool>,
    show_labels: RefCell<bool>,
    rows: RefCell<i32>,
    items: ObservableCollection<Option<Rc<dyn RibbonGalleryItemModelImpl>>>,
    menu_items: ObservableCollection<Option<Rc<dyn RibbonNodeModelImpl>>>,
    preview_command: RefCell<Option<Rc<dyn ICommand>>>,
}
pub trait RibbonGalleryModelImpl: RibbonItemModelImpl {
    fn ribbon_gallery_model(&self) -> &RibbonGalleryModel;
}
pub trait RibbonGalleryModelExt: RibbonGalleryModelImpl {
    fn selected_item(&self) -> Option<Rc<dyn RibbonGalleryItemModelImpl>> {
        self.ribbon_gallery_model()
            .selected_item
            .borrow()
            .to_owned()
    }
    fn set_selected_item(&self, value: Option<Rc<dyn RibbonGalleryItemModelImpl>>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_model().selected_item,
            value,
            "SelectedItem",
        );
    }
    fn min_columns(&self) -> i32 {
        self.ribbon_gallery_model().min_columns.borrow().to_owned()
    }
    fn set_min_columns(&self, value: i32) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_model().min_columns,
            value.max(1),
            "MinColumns",
        );
    }
    fn max_columns(&self) -> i32 {
        self.ribbon_gallery_model().max_columns.borrow().to_owned()
    }
    fn set_max_columns(&self, value: i32) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_model().max_columns,
            value.max(1),
            "MaxColumns",
        );
    }
    fn drop_down_columns(&self) -> i32 {
        self.ribbon_gallery_model()
            .drop_down_columns
            .borrow()
            .to_owned()
    }
    fn set_drop_down_columns(&self, value: i32) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_model().drop_down_columns,
            value.max(1),
            "DropDownColumns",
        );
    }
    fn item_width(&self) -> f64 {
        self.ribbon_gallery_model().item_width.borrow().to_owned()
    }
    fn set_item_width(&self, value: f64) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_model().item_width,
            value,
            "ItemWidth",
        );
    }
    fn item_height(&self) -> f64 {
        self.ribbon_gallery_model().item_height.borrow().to_owned()
    }
    fn set_item_height(&self, value: f64) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_model().item_height,
            value,
            "ItemHeight",
        );
    }
    fn is_filter_enabled(&self) -> bool {
        self.ribbon_gallery_model()
            .is_filter_enabled
            .borrow()
            .to_owned()
    }
    fn set_is_filter_enabled(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_model().is_filter_enabled,
            value,
            "IsFilterEnabled",
        );
    }
    fn show_labels(&self) -> bool {
        self.ribbon_gallery_model().show_labels.borrow().to_owned()
    }
    fn set_show_labels(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_model().show_labels,
            value,
            "ShowLabels",
        );
    }
    fn rows(&self) -> i32 {
        self.ribbon_gallery_model().rows.borrow().to_owned()
    }
    fn set_rows(&self, value: i32) {
        ObservableObject::set_property(
            self,
            &self.ribbon_gallery_model().rows,
            value.max(1),
            "Rows",
        );
    }
    fn items(&self) -> ObservableCollection<Option<Rc<dyn RibbonGalleryItemModelImpl>>> {
        self.ribbon_gallery_model().items.clone()
    }
    fn menu_items(&self) -> ObservableCollection<Option<Rc<dyn RibbonNodeModelImpl>>> {
        self.ribbon_gallery_model().menu_items.clone()
    }
    fn preview_command(&self) -> Option<Rc<dyn ICommand>> {
        self.ribbon_gallery_model().preview_command.borrow().clone()
    }
    fn set_preview_command(&self, value: Option<Rc<dyn ICommand>>) {
        let equal = match (
            &*self.ribbon_gallery_model().preview_command.borrow(),
            &value,
        ) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        if !equal {
            let old = self.ribbon_gallery_model().preview_command.replace(value);
            drop(old);
            self.on_property_changed("PreviewCommand");
        }
    }
}
impl<T: RibbonGalleryModelImpl + ?Sized> RibbonGalleryModelExt for T {}
impl PartialEq for dyn RibbonGalleryModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl RibbonGalleryModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
            selected_item: RefCell::new(None),
            min_columns: RefCell::new(3),
            max_columns: RefCell::new(7),
            drop_down_columns: RefCell::new(7),
            item_width: RefCell::new(72.0),
            item_height: RefCell::new(60.0),
            is_filter_enabled: RefCell::new(false),
            show_labels: RefCell::new(true),
            rows: RefCell::new(1),
            items: ObservableCollection::new_managed(),
            menu_items: ObservableCollection::new_managed(),
            preview_command: RefCell::default(),
        }
    }
    pub fn initialize<O: RibbonGalleryModelImpl + ?Sized>(owner: &O) {
        owner.set_size(RibbonItemSize::Large);
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
impl INotifyPropertyChanged for RibbonGalleryModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonGalleryModel {}
impl RibbonNodeModelImpl for RibbonGalleryModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonGalleryModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonGalleryModelImpl for RibbonGalleryModel {
    fn ribbon_gallery_model(&self) -> &RibbonGalleryModel {
        self
    }
}
impl PartialEq for RibbonGalleryModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonColorPickerModel {
    base: RibbonItemModel,
    selected_color: RefCell<Option<RibbonColor>>,
    show_automatic: RefCell<bool>,
    show_no_color: RefCell<bool>,
    show_more_colors: RefCell<bool>,
    is_split: RefCell<bool>,
    automatic_color: RefCell<RibbonColor>,
    theme_colors: ObservableCollection<Option<Rc<RibbonColorSwatch>>>,
    standard_colors: ObservableCollection<Option<Rc<RibbonColorSwatch>>>,
    recent_colors: ObservableCollection<RibbonColor>,
}
pub trait RibbonColorPickerModelImpl: RibbonItemModelImpl {
    fn ribbon_color_picker_model(&self) -> &RibbonColorPickerModel;
}
pub trait RibbonColorPickerModelExt: RibbonColorPickerModelImpl {
    fn selected_color(&self) -> Option<RibbonColor> {
        self.ribbon_color_picker_model()
            .selected_color
            .borrow()
            .to_owned()
    }
    fn set_selected_color(&self, value: Option<RibbonColor>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_color_picker_model().selected_color,
            value,
            "SelectedColor",
        );
    }
    fn show_automatic(&self) -> bool {
        self.ribbon_color_picker_model()
            .show_automatic
            .borrow()
            .to_owned()
    }
    fn set_show_automatic(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_color_picker_model().show_automatic,
            value,
            "ShowAutomatic",
        );
    }
    fn show_no_color(&self) -> bool {
        self.ribbon_color_picker_model()
            .show_no_color
            .borrow()
            .to_owned()
    }
    fn set_show_no_color(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_color_picker_model().show_no_color,
            value,
            "ShowNoColor",
        );
    }
    fn show_more_colors(&self) -> bool {
        self.ribbon_color_picker_model()
            .show_more_colors
            .borrow()
            .to_owned()
    }
    fn set_show_more_colors(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_color_picker_model().show_more_colors,
            value,
            "ShowMoreColors",
        );
    }
    fn is_split(&self) -> bool {
        self.ribbon_color_picker_model()
            .is_split
            .borrow()
            .to_owned()
    }
    fn set_is_split(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_color_picker_model().is_split,
            value,
            "IsSplit",
        );
    }
    fn automatic_color(&self) -> RibbonColor {
        self.ribbon_color_picker_model()
            .automatic_color
            .borrow()
            .to_owned()
    }
    fn set_automatic_color(&self, value: RibbonColor) {
        ObservableObject::set_property(
            self,
            &self.ribbon_color_picker_model().automatic_color,
            value,
            "AutomaticColor",
        );
    }
    fn theme_colors(&self) -> ObservableCollection<Option<Rc<RibbonColorSwatch>>> {
        self.ribbon_color_picker_model().theme_colors.clone()
    }
    fn standard_colors(&self) -> ObservableCollection<Option<Rc<RibbonColorSwatch>>> {
        self.ribbon_color_picker_model().standard_colors.clone()
    }
    fn recent_colors(&self) -> ObservableCollection<RibbonColor> {
        self.ribbon_color_picker_model().recent_colors.clone()
    }
}
impl<T: RibbonColorPickerModelImpl + ?Sized> RibbonColorPickerModelExt for T {}
impl PartialEq for dyn RibbonColorPickerModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
impl RibbonColorPickerModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
            selected_color: RefCell::new(Some(RibbonColor::from_rgb(255, 0, 0))),
            show_automatic: RefCell::new(true),
            show_no_color: RefCell::new(false),
            show_more_colors: RefCell::new(true),
            is_split: RefCell::new(true),
            automatic_color: RefCell::new(RibbonColor::BLACK),
            theme_colors: ObservableCollection::new_managed(),
            standard_colors: ObservableCollection::new_managed(),
            recent_colors: ObservableCollection::new_unmanaged(),
        }
    }
    pub fn initialize<O: RibbonColorPickerModelImpl + ?Sized>(
        owner: &O,
        icon: Option<Rc<RibbonIcon>>,
        command: Option<Rc<dyn ICommand>>,
    ) {
        owner.set_icon(icon);
        owner.set_command(command);
        owner.set_size(RibbonItemSize::Small);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_options(id, label, None, None)
    }
    pub fn with_options(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        icon: Option<Rc<RibbonIcon>>,
        command: Option<Rc<dyn ICommand>>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id, label));
        Self::initialize(model.as_ref(), icon, command);
        model
    }
    pub fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonColorPickerModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonColorPickerModel {}
impl RibbonNodeModelImpl for RibbonColorPickerModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonColorPickerModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonColorPickerModelImpl for RibbonColorPickerModel {
    fn ribbon_color_picker_model(&self) -> &RibbonColorPickerModel {
        self
    }
}
impl PartialEq for RibbonColorPickerModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
