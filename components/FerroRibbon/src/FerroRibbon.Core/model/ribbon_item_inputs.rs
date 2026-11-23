// Input and leaf portions of the pinned RibbonItemModel.cs
use super::{
    RibbonItemModel, RibbonItemModelExt, RibbonItemModelImpl, RibbonNodeModel, RibbonNodeModelImpl,
    RibbonObject,
};
use crate::{
    ObservableObject, ObservableObjectImpl, RibbonItemSize, RibbonString, commands::ICommand,
};
use crate::collections::ObservableCollection;
use ferroui_system::{
    component_model::INotifyPropertyChanged,
    events::Event,
};
use std::{cell::RefCell, rc::Rc};
pub struct RibbonCheckBoxModel {
    base: RibbonItemModel,
    is_checked: RefCell<Option<bool>>,
    is_three_state: RefCell<bool>,
}
pub trait RibbonCheckBoxModelImpl: RibbonItemModelImpl {
    fn ribbon_check_box_model(&self) -> &RibbonCheckBoxModel;
}
pub trait RibbonCheckBoxModelExt: RibbonCheckBoxModelImpl {
    fn is_checked(&self) -> Option<bool> {
        self.ribbon_check_box_model().is_checked.borrow().to_owned()
    }
    fn set_is_checked(&self, value: Option<bool>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_check_box_model().is_checked,
            value,
            "IsChecked",
        );
    }
    fn is_three_state(&self) -> bool {
        self.ribbon_check_box_model()
            .is_three_state
            .borrow()
            .to_owned()
    }
    fn set_is_three_state(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_check_box_model().is_three_state,
            value,
            "IsThreeState",
        );
    }
}
impl<T: RibbonCheckBoxModelImpl + ?Sized> RibbonCheckBoxModelExt for T {}
impl RibbonCheckBoxModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
            is_checked: RefCell::new(Some(false)),
            is_three_state: RefCell::new(false),
        }
    }
    pub fn construct_with_checked(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        value: bool,
    ) -> Self {
        let model = Self::construct(id, label);
        model.is_checked.replace(Some(value));
        model
    }
    pub fn initialize<O: RibbonCheckBoxModelImpl + ?Sized>(owner: &O) {
        owner.set_size(RibbonItemSize::Medium);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_checked(id, label, false)
    }
    pub fn with_checked(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        value: bool,
    ) -> Rc<Self> {
        let model = Self::construct_with_checked(id, label, value);
        let model = Rc::new(model);
        Self::initialize(model.as_ref());
        model
    }
    pub fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonCheckBoxModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonCheckBoxModel {}
impl RibbonNodeModelImpl for RibbonCheckBoxModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonCheckBoxModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonCheckBoxModelImpl for RibbonCheckBoxModel {
    fn ribbon_check_box_model(&self) -> &RibbonCheckBoxModel {
        self
    }
}
impl PartialEq for RibbonCheckBoxModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonComboBoxModel {
    base: RibbonItemModel,
    items: ObservableCollection<Option<RibbonObject>>,
    selected_item: RefCell<Option<RibbonObject>>,
    text: RefCell<Option<RibbonString>>,
    is_editable: RefCell<bool>,
    input_width: RefCell<f64>,
    placeholder: RefCell<Option<RibbonString>>,
    display_member_path: RefCell<Option<RibbonString>>,
    preview_font_family: RefCell<bool>,
    max_drop_down_height: RefCell<f64>,
}
pub trait RibbonComboBoxModelImpl: RibbonItemModelImpl {
    fn ribbon_combo_box_model(&self) -> &RibbonComboBoxModel;
}
pub trait RibbonComboBoxModelExt: RibbonComboBoxModelImpl {
    fn selected_item(&self) -> Option<RibbonObject> {
        self.ribbon_combo_box_model()
            .selected_item
            .borrow()
            .to_owned()
    }
    fn set_selected_item(&self, value: Option<RibbonObject>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_combo_box_model().selected_item,
            value,
            "SelectedItem",
        );
    }
    fn text(&self) -> Option<RibbonString> {
        self.ribbon_combo_box_model().text.borrow().to_owned()
    }
    fn set_text(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(self, &self.ribbon_combo_box_model().text, value, "Text");
    }
    fn is_editable(&self) -> bool {
        self.ribbon_combo_box_model()
            .is_editable
            .borrow()
            .to_owned()
    }
    fn set_is_editable(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_combo_box_model().is_editable,
            value,
            "IsEditable",
        );
    }
    fn input_width(&self) -> f64 {
        self.ribbon_combo_box_model()
            .input_width
            .borrow()
            .to_owned()
    }
    fn set_input_width(&self, value: f64) {
        ObservableObject::set_property(
            self,
            &self.ribbon_combo_box_model().input_width,
            value,
            "InputWidth",
        );
    }
    fn placeholder(&self) -> Option<RibbonString> {
        self.ribbon_combo_box_model()
            .placeholder
            .borrow()
            .to_owned()
    }
    fn set_placeholder(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_combo_box_model().placeholder,
            value,
            "Placeholder",
        );
    }
    fn display_member_path(&self) -> Option<RibbonString> {
        self.ribbon_combo_box_model()
            .display_member_path
            .borrow()
            .to_owned()
    }
    fn set_display_member_path(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_combo_box_model().display_member_path,
            value,
            "DisplayMemberPath",
        );
    }
    fn preview_font_family(&self) -> bool {
        self.ribbon_combo_box_model()
            .preview_font_family
            .borrow()
            .to_owned()
    }
    fn set_preview_font_family(&self, value: bool) {
        ObservableObject::set_property(
            self,
            &self.ribbon_combo_box_model().preview_font_family,
            value,
            "PreviewFontFamily",
        );
    }
    fn max_drop_down_height(&self) -> f64 {
        self.ribbon_combo_box_model()
            .max_drop_down_height
            .borrow()
            .to_owned()
    }
    fn set_max_drop_down_height(&self, value: f64) {
        ObservableObject::set_property(
            self,
            &self.ribbon_combo_box_model().max_drop_down_height,
            value,
            "MaxDropDownHeight",
        );
    }
    fn items(&self) -> ObservableCollection<Option<RibbonObject>> {
        self.ribbon_combo_box_model().items.clone()
    }
}
impl<T: RibbonComboBoxModelImpl + ?Sized> RibbonComboBoxModelExt for T {}
impl RibbonComboBoxModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
            items: ObservableCollection::new_managed(),
            selected_item: RefCell::new(None),
            text: RefCell::new(None),
            is_editable: RefCell::new(false),
            input_width: RefCell::new(120.0),
            placeholder: RefCell::new(None),
            display_member_path: RefCell::new(None),
            preview_font_family: RefCell::new(false),
            max_drop_down_height: RefCell::new(420.0),
        }
    }
    pub fn initialize<O: RibbonComboBoxModelImpl + ?Sized>(
        owner: &O,
        items: impl IntoIterator<Item = Option<RibbonObject>>,
    ) {
        owner.set_size(RibbonItemSize::Small);
        for item in items {
            owner.items().add(item);
        }
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_items(id, label, std::iter::empty())
    }
    pub fn with_items(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        items: impl IntoIterator<Item = Option<RibbonObject>>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id, label));
        Self::initialize(model.as_ref(), items);
        model
    }
    pub fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonComboBoxModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonComboBoxModel {}
impl RibbonNodeModelImpl for RibbonComboBoxModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonComboBoxModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonComboBoxModelImpl for RibbonComboBoxModel {
    fn ribbon_combo_box_model(&self) -> &RibbonComboBoxModel {
        self
    }
}
impl PartialEq for RibbonComboBoxModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonSpinnerModel {
    base: RibbonItemModel,
    value: RefCell<f64>,
    minimum: RefCell<f64>,
    maximum: RefCell<f64>,
    increment: RefCell<f64>,
    input_width: RefCell<f64>,
    format: RefCell<Option<RibbonString>>,
    unit: RefCell<Option<RibbonString>>,
}
pub trait RibbonSpinnerModelImpl: RibbonItemModelImpl {
    fn ribbon_spinner_model(&self) -> &RibbonSpinnerModel;
}
pub trait RibbonSpinnerModelExt: RibbonSpinnerModelImpl {
    fn value(&self) -> f64 {
        self.ribbon_spinner_model().value.borrow().to_owned()
    }
    fn set_value(&self, value: f64) {
        ObservableObject::set_property(self, &self.ribbon_spinner_model().value, value, "Value");
    }
    fn minimum(&self) -> f64 {
        self.ribbon_spinner_model().minimum.borrow().to_owned()
    }
    fn set_minimum(&self, value: f64) {
        ObservableObject::set_property(
            self,
            &self.ribbon_spinner_model().minimum,
            value,
            "Minimum",
        );
    }
    fn maximum(&self) -> f64 {
        self.ribbon_spinner_model().maximum.borrow().to_owned()
    }
    fn set_maximum(&self, value: f64) {
        ObservableObject::set_property(
            self,
            &self.ribbon_spinner_model().maximum,
            value,
            "Maximum",
        );
    }
    fn increment(&self) -> f64 {
        self.ribbon_spinner_model().increment.borrow().to_owned()
    }
    fn set_increment(&self, value: f64) {
        ObservableObject::set_property(
            self,
            &self.ribbon_spinner_model().increment,
            value,
            "Increment",
        );
    }
    fn input_width(&self) -> f64 {
        self.ribbon_spinner_model().input_width.borrow().to_owned()
    }
    fn set_input_width(&self, value: f64) {
        ObservableObject::set_property(
            self,
            &self.ribbon_spinner_model().input_width,
            value,
            "InputWidth",
        );
    }
    fn format(&self) -> Option<RibbonString> {
        self.ribbon_spinner_model().format.borrow().to_owned()
    }
    fn set_format(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(self, &self.ribbon_spinner_model().format, value, "Format");
    }
    fn unit(&self) -> Option<RibbonString> {
        self.ribbon_spinner_model().unit.borrow().to_owned()
    }
    fn set_unit(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(self, &self.ribbon_spinner_model().unit, value, "Unit");
    }
}
impl<T: RibbonSpinnerModelImpl + ?Sized> RibbonSpinnerModelExt for T {}
impl RibbonSpinnerModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
            value: RefCell::new(0.0),
            minimum: RefCell::new(0.0),
            maximum: RefCell::new(100.0),
            increment: RefCell::new(1.0),
            input_width: RefCell::new(64.0),
            format: RefCell::new(Some("0.##".into())),
            unit: RefCell::new(None),
        }
    }
    pub fn construct_with_value(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        value: f64,
    ) -> Self {
        let model = Self::construct(id, label);
        model.value.replace(value);
        model
    }
    pub fn initialize<O: RibbonSpinnerModelImpl + ?Sized>(owner: &O) {
        owner.set_size(RibbonItemSize::Medium);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_value(id, label, 0.0)
    }
    pub fn with_value(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        value: f64,
    ) -> Rc<Self> {
        let model = Self::construct_with_value(id, label, value);
        let model = Rc::new(model);
        Self::initialize(model.as_ref());
        model
    }
    pub fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonSpinnerModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonSpinnerModel {}
impl RibbonNodeModelImpl for RibbonSpinnerModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonSpinnerModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonSpinnerModelImpl for RibbonSpinnerModel {
    fn ribbon_spinner_model(&self) -> &RibbonSpinnerModel {
        self
    }
}
impl PartialEq for RibbonSpinnerModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonTextBoxModel {
    base: RibbonItemModel,
    text: RefCell<Option<RibbonString>>,
    placeholder: RefCell<Option<RibbonString>>,
    input_width: RefCell<f64>,
}
pub trait RibbonTextBoxModelImpl: RibbonItemModelImpl {
    fn ribbon_text_box_model(&self) -> &RibbonTextBoxModel;
}
pub trait RibbonTextBoxModelExt: RibbonTextBoxModelImpl {
    fn text(&self) -> Option<RibbonString> {
        self.ribbon_text_box_model().text.borrow().to_owned()
    }
    fn set_text(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(self, &self.ribbon_text_box_model().text, value, "Text");
    }
    fn placeholder(&self) -> Option<RibbonString> {
        self.ribbon_text_box_model().placeholder.borrow().to_owned()
    }
    fn set_placeholder(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_text_box_model().placeholder,
            value,
            "Placeholder",
        );
    }
    fn input_width(&self) -> f64 {
        self.ribbon_text_box_model().input_width.borrow().to_owned()
    }
    fn set_input_width(&self, value: f64) {
        ObservableObject::set_property(
            self,
            &self.ribbon_text_box_model().input_width,
            value,
            "InputWidth",
        );
    }
}
impl<T: RibbonTextBoxModelImpl + ?Sized> RibbonTextBoxModelExt for T {}
impl RibbonTextBoxModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
            text: RefCell::new(None),
            placeholder: RefCell::new(None),
            input_width: RefCell::new(140.0),
        }
    }
    pub fn initialize<O: RibbonTextBoxModelImpl + ?Sized>(owner: &O) {
        owner.set_size(RibbonItemSize::Medium);
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
impl INotifyPropertyChanged for RibbonTextBoxModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonTextBoxModel {}
impl RibbonNodeModelImpl for RibbonTextBoxModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonTextBoxModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonTextBoxModelImpl for RibbonTextBoxModel {
    fn ribbon_text_box_model(&self) -> &RibbonTextBoxModel {
        self
    }
}
impl PartialEq for RibbonTextBoxModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonSliderModel {
    base: RibbonItemModel,
    value: RefCell<f64>,
    minimum: RefCell<f64>,
    maximum: RefCell<f64>,
    step_frequency: RefCell<f64>,
    slider_width: RefCell<f64>,
}
pub trait RibbonSliderModelImpl: RibbonItemModelImpl {
    fn ribbon_slider_model(&self) -> &RibbonSliderModel;
}
pub trait RibbonSliderModelExt: RibbonSliderModelImpl {
    fn value(&self) -> f64 {
        self.ribbon_slider_model().value.borrow().to_owned()
    }
    fn set_value(&self, value: f64) {
        ObservableObject::set_property(self, &self.ribbon_slider_model().value, value, "Value");
    }
    fn minimum(&self) -> f64 {
        self.ribbon_slider_model().minimum.borrow().to_owned()
    }
    fn set_minimum(&self, value: f64) {
        ObservableObject::set_property(self, &self.ribbon_slider_model().minimum, value, "Minimum");
    }
    fn maximum(&self) -> f64 {
        self.ribbon_slider_model().maximum.borrow().to_owned()
    }
    fn set_maximum(&self, value: f64) {
        ObservableObject::set_property(self, &self.ribbon_slider_model().maximum, value, "Maximum");
    }
    fn step_frequency(&self) -> f64 {
        self.ribbon_slider_model()
            .step_frequency
            .borrow()
            .to_owned()
    }
    fn set_step_frequency(&self, value: f64) {
        ObservableObject::set_property(
            self,
            &self.ribbon_slider_model().step_frequency,
            value,
            "StepFrequency",
        );
    }
    fn slider_width(&self) -> f64 {
        self.ribbon_slider_model().slider_width.borrow().to_owned()
    }
    fn set_slider_width(&self, value: f64) {
        ObservableObject::set_property(
            self,
            &self.ribbon_slider_model().slider_width,
            value,
            "SliderWidth",
        );
    }
}
impl<T: RibbonSliderModelImpl + ?Sized> RibbonSliderModelExt for T {}
impl RibbonSliderModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
            value: RefCell::new(0.0),
            minimum: RefCell::new(0.0),
            maximum: RefCell::new(100.0),
            step_frequency: RefCell::new(1.0),
            slider_width: RefCell::new(120.0),
        }
    }
    pub fn initialize<O: RibbonSliderModelImpl + ?Sized>(owner: &O) {
        owner.set_size(RibbonItemSize::Medium);
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
impl INotifyPropertyChanged for RibbonSliderModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonSliderModel {}
impl RibbonNodeModelImpl for RibbonSliderModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonSliderModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonSliderModelImpl for RibbonSliderModel {
    fn ribbon_slider_model(&self) -> &RibbonSliderModel {
        self
    }
}
impl PartialEq for RibbonSliderModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonGridPickerModel {
    base: RibbonItemModel,
    rows: RefCell<i32>,
    columns: RefCell<i32>,
}
pub trait RibbonGridPickerModelImpl: RibbonItemModelImpl {
    fn ribbon_grid_picker_model(&self) -> &RibbonGridPickerModel;
}
pub trait RibbonGridPickerModelExt: RibbonGridPickerModelImpl {
    fn rows(&self) -> i32 {
        self.ribbon_grid_picker_model().rows.borrow().to_owned()
    }
    fn set_rows(&self, value: i32) {
        ObservableObject::set_property(
            self,
            &self.ribbon_grid_picker_model().rows,
            value.max(1),
            "Rows",
        );
    }
    fn columns(&self) -> i32 {
        self.ribbon_grid_picker_model().columns.borrow().to_owned()
    }
    fn set_columns(&self, value: i32) {
        ObservableObject::set_property(
            self,
            &self.ribbon_grid_picker_model().columns,
            value.max(1),
            "Columns",
        );
    }
}
impl<T: RibbonGridPickerModelImpl + ?Sized> RibbonGridPickerModelExt for T {}
impl RibbonGridPickerModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
            rows: RefCell::new(8),
            columns: RefCell::new(10),
        }
    }
    pub fn initialize<O: RibbonGridPickerModelImpl + ?Sized>(
        owner: &O,
        command: Option<Rc<dyn ICommand>>,
    ) {
        owner.set_command(command);
    }
    pub fn new(id: Option<RibbonString>, label: Option<RibbonString>) -> Rc<Self> {
        Self::with_command(id, label, None)
    }
    pub fn with_command(
        id: Option<RibbonString>,
        label: Option<RibbonString>,
        command: Option<Rc<dyn ICommand>>,
    ) -> Rc<Self> {
        let model = Rc::new(Self::construct(id, label));
        Self::initialize(model.as_ref(), command);
        model
    }
    pub fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonGridPickerModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonGridPickerModel {}
impl RibbonNodeModelImpl for RibbonGridPickerModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonGridPickerModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonGridPickerModelImpl for RibbonGridPickerModel {
    fn ribbon_grid_picker_model(&self) -> &RibbonGridPickerModel {
        self
    }
}
impl PartialEq for RibbonGridPickerModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonCustomItemModel {
    base: RibbonItemModel,
    content: RefCell<Option<RibbonObject>>,
    template_key: RefCell<Option<RibbonString>>,
}
pub trait RibbonCustomItemModelImpl: RibbonItemModelImpl {
    fn ribbon_custom_item_model(&self) -> &RibbonCustomItemModel;
}
pub trait RibbonCustomItemModelExt: RibbonCustomItemModelImpl {
    fn content(&self) -> Option<RibbonObject> {
        self.ribbon_custom_item_model().content.borrow().to_owned()
    }
    fn set_content(&self, value: Option<RibbonObject>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_custom_item_model().content,
            value,
            "Content",
        );
    }
    fn template_key(&self) -> Option<RibbonString> {
        self.ribbon_custom_item_model()
            .template_key
            .borrow()
            .to_owned()
    }
    fn set_template_key(&self, value: Option<RibbonString>) {
        ObservableObject::set_property(
            self,
            &self.ribbon_custom_item_model().template_key,
            value,
            "TemplateKey",
        );
    }
}
impl<T: RibbonCustomItemModelImpl + ?Sized> RibbonCustomItemModelExt for T {}
impl RibbonCustomItemModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
            content: RefCell::new(None),
            template_key: RefCell::new(None),
        }
    }
    pub fn initialize<O: RibbonCustomItemModelImpl + ?Sized>(owner: &O) {
        owner.set_can_add_to_quick_access(false);
    }
    pub fn new(id: Option<RibbonString>, content: Option<RibbonObject>) -> Rc<Self> {
        Self::with_content(id, content)
    }
    pub fn with_content(id: Option<RibbonString>, content: Option<RibbonObject>) -> Rc<Self> {
        let model = Self::construct_with_content(id, content);
        let model = Rc::new(model);
        Self::initialize(model.as_ref());
        model
    }
    pub fn construct_with_content(id: Option<RibbonString>, content: Option<RibbonObject>) -> Self {
        let model = Self::construct(id, None);
        model.content.replace(content);
        model
    }
    pub fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonCustomItemModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonCustomItemModel {}
impl RibbonNodeModelImpl for RibbonCustomItemModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonCustomItemModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonCustomItemModelImpl for RibbonCustomItemModel {
    fn ribbon_custom_item_model(&self) -> &RibbonCustomItemModel {
        self
    }
}
impl PartialEq for RibbonCustomItemModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonLabelModel {
    base: RibbonItemModel,
}
pub trait RibbonLabelModelImpl: RibbonItemModelImpl {
    fn ribbon_label_model(&self) -> &RibbonLabelModel;
}
pub trait RibbonLabelModelExt: RibbonLabelModelImpl {}
impl<T: RibbonLabelModelImpl + ?Sized> RibbonLabelModelExt for T {}
impl RibbonLabelModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
        }
    }
    pub fn initialize<O: RibbonLabelModelImpl + ?Sized>(owner: &O) {
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
impl INotifyPropertyChanged for RibbonLabelModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonLabelModel {}
impl RibbonNodeModelImpl for RibbonLabelModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonLabelModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonLabelModelImpl for RibbonLabelModel {
    fn ribbon_label_model(&self) -> &RibbonLabelModel {
        self
    }
}
impl PartialEq for RibbonLabelModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
pub struct RibbonSeparatorModel {
    base: RibbonItemModel,
}
pub trait RibbonSeparatorModelImpl: RibbonItemModelImpl {
    fn ribbon_separator_model(&self) -> &RibbonSeparatorModel;
}
pub trait RibbonSeparatorModelExt: RibbonSeparatorModelImpl {}
impl<T: RibbonSeparatorModelImpl + ?Sized> RibbonSeparatorModelExt for T {}
impl RibbonSeparatorModel {
    pub fn construct(id: Option<RibbonString>, label: Option<RibbonString>) -> Self {
        Self {
            base: RibbonItemModel::construct(id, label),
        }
    }
    pub fn initialize<O: RibbonSeparatorModelImpl + ?Sized>(owner: &O) {
        owner.set_can_add_to_quick_access(false);
    }
    pub fn new() -> Rc<Self> {
        let model = Rc::new(Self::construct(None, None));
        Self::initialize(model.as_ref());
        model
    }
    pub fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl INotifyPropertyChanged for RibbonSeparatorModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonSeparatorModel {}
impl RibbonNodeModelImpl for RibbonSeparatorModel {
    fn node_model(&self) -> &RibbonNodeModel {
        self.base.node_model()
    }
}
impl RibbonItemModelImpl for RibbonSeparatorModel {
    fn item_model(&self) -> &RibbonItemModel {
        &self.base
    }
}
impl RibbonSeparatorModelImpl for RibbonSeparatorModel {
    fn ribbon_separator_model(&self) -> &RibbonSeparatorModel {
        self
    }
}
impl PartialEq for RibbonSeparatorModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_node(other)
    }
}
