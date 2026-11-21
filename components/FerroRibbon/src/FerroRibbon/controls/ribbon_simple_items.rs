//! Native controls from the shared RibbonSimpleItems.cs.
use super::{RibbonControlBase, RibbonControlBaseImpl, RibbonItemLayout, RibbonOverflowMenuItems};
use ferroribbon_core::{RibbonItemSize, model::RibbonItemsSource};
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StaticType, StyledElementImpl,
    StyledProperty, Thickness, TypeInfo, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_base::{
    controls::NameScopeRef,
    input::{InputElementImpl, KeyboardNavigation},
    interactivity::InteractiveImpl,
    layout::{
        HorizontalAlignment, LayoutableImpl, LayoutableImplExt, Orientation, VerticalAlignment,
    },
};
use ferroui_controls::{
    Control, ControlImpl, Separator,
    primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt},
};
use std::cell::RefCell;
#[repr(C)]
pub struct RibbonSeparator {
    base: RibbonControlBase,
    scope: RefCell<Option<NameScopeRef>>,
}
ferro_class!(RibbonSeparator:RibbonControlBase);
ferro_class_info!(RibbonSeparator{new:RibbonSeparator::new,markup:{namespace:"FerroRibbon.Controls"}});
ferro_impl_classes!(RibbonSeparator:VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_properties! {impl RibbonSeparator {
    pub fn orientation_property()->StyledProperty<Orientation>{FerroProperty::register::<Self,_>("Orientation",Orientation::Vertical)}
}}
impl RibbonSeparator {
    fn static_constructor() {
        Self::orientation_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_orientation())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: RibbonControlBase::construct(),
            scope: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }
    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }
    fn update_orientation(&self) {
        let vertical = self.orientation() == Orientation::Vertical;
        // Source VisualStateManager orientation states map to native pseudoclasses.
        self.pseudo_classes().set(":vertical", vertical);
        self.pseudo_classes().set(":horizontal", !vertical);
        self.set_horizontal_alignment(if vertical {
            HorizontalAlignment::Left
        } else {
            HorizontalAlignment::Stretch
        });
        self.set_vertical_alignment(if vertical {
            VerticalAlignment::Center
        } else {
            VerticalAlignment::Top
        });
        let scope = self.scope.borrow().clone();
        if let Some(line) = scope
            .and_then(|scope| scope.find("PART_Line"))
            .and_then(|line| line.cast::<Control>())
        {
            line.set_width(if vertical { 1. } else { f64::NAN });
            line.set_height(if vertical {
                let metrics = self.metrics().expect("RibbonSeparator.Metrics is null");
                if self.is_simplified() {
                    metrics.simplified_item_height() - 8.
                } else {
                    metrics.group_content_height() - 6.
                }
            } else {
                1.
            });
            line.set_margin(if vertical {
                Thickness::new(3., 3., 3., 3.)
            } else {
                Thickness::new(4., 3., 4., 3.)
            });
        }
    }
}
impl FerroObjectImpl for RibbonSeparator {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_can_add_to_quick_access(false);
        KeyboardNavigation::set_is_tab_stop(this, false);
        this.set_size(RibbonItemSize::Large);
    }
}
impl StyledElementImpl for RibbonSeparator {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
impl LayoutableImpl for RibbonSeparator {
    fn apply_template(this: &Self) {
        Self::parent_apply_template(this);
        if this.visual_children().count() == 0 {
            this.scope.replace(None);
        }
    }
}
impl TemplatedControlImpl for RibbonSeparator {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.scope.replace(Some(e.name_scope().clone()));
        this.update_orientation();
    }
}
impl RibbonControlBaseImpl for RibbonSeparator {
    fn on_layout_applied_core(this: &Self, _layout: RibbonItemLayout) {
        this.update_orientation()
    }
    fn create_overflow_menu_items_core(_this: &Self) -> RibbonOverflowMenuItems {
        // Materialized once per method call, as the source collection expression.
        let item = Separator::new().upcast::<Control>();
        RibbonItemsSource::from_values(vec![Some(item)])
    }
    fn invoke_core(_this: &Self) -> bool {
        false
    }
}
#[repr(C)]
pub struct RibbonLabel {
    base: RibbonControlBase,
}
ferro_class!(RibbonLabel:RibbonControlBase);
ferro_class_info!(RibbonLabel{new:RibbonLabel::new,markup:{namespace:"FerroRibbon.Controls"}});
ferro_impl_classes!(RibbonLabel:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl RibbonLabel {
    pub fn construct() -> Self {
        Self {
            base: RibbonControlBase::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
impl FerroObjectImpl for RibbonLabel {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_can_add_to_quick_access(false);
        KeyboardNavigation::set_is_tab_stop(this, false);
    }
}
impl StyledElementImpl for RibbonLabel {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
// The source empty collection expression lowers to the shared Array.Empty,
// unlike the one-element separator expression's fresh array.
ferroui_base::ferro_thread_local! {static EMPTY_LABEL_OVERFLOW:RibbonOverflowMenuItems=RibbonItemsSource::from_values(vec![]);}
impl RibbonControlBaseImpl for RibbonLabel {
    fn create_overflow_menu_items_core(_this: &Self) -> RibbonOverflowMenuItems {
        EMPTY_LABEL_OVERFLOW.with(Clone::clone)
    }
    fn invoke_core(_this: &Self) -> bool {
        false
    }
}

use super::{IRibbonItem, IRibbonLayoutHost, RibbonKeyTipResult};
use crate::metadata::control_items::RibbonControlItems;
use ferroui_base::{
    FerroObject,
    data::core::{ValueType, ValueTypes},
};
use ferroui_controls::Panel;
use std::rc::Rc;
#[repr(C)]
pub struct RibbonItemsContainer {
    base: RibbonControlBase,
    items: RibbonControlItems,
    host: RefCell<Option<Ref<Panel>>>,
    scope: RefCell<Option<NameScopeRef>>,
    last_layout: RefCell<RibbonItemLayout>,
}
ferro_class! {RibbonItemsContainer:RibbonControlBase,virtuals RibbonItemsContainerImpl:RibbonControlBaseImpl {
 fn create_container_copy(this)->Ref<RibbonItemsContainer>;
 fn get_child_layout(this,child:Rc<dyn IRibbonItem>,layout:RibbonItemLayout)->RibbonItemLayout;
}}
ferro_class_info!(RibbonItemsContainer {markup:{namespace:"FerroRibbon.Controls",content:Items,properties:[Items:RibbonControlItems{get:RibbonItemsContainer::items}],methods:[fn InvalidateItemsLayout()=>|this:&RibbonItemsContainer|this.invalidate_items_layout()]},interfaces:[Rc<dyn IRibbonLayoutHost> =>|value|Rc::new(ContainerHost(value)) as Rc<dyn IRibbonLayoutHost>]});
ferro_impl_classes!(RibbonItemsContainer:StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl RibbonItemsContainer {
    pub fn construct() -> Self {
        Self {
            base: RibbonControlBase::construct(),
            items: RibbonControlItems::new_managed(),
            host: RefCell::new(None),
            scope: RefCell::new(None),
            last_layout: RefCell::new(RibbonItemLayout::new(
                RibbonItemSize::Small,
                Some(super::super::primitives::RibbonMetrics::comfortable()),
            )),
        }
    }
    pub fn items(&self) -> RibbonControlItems {
        self.items.clone()
    }
    pub fn host(&self) -> Option<Ref<Panel>> {
        self.host.borrow().clone()
    }
    pub fn invalidate_items_layout(&self) {
        super::RibbonItemHelper::invalidate_host_layout(self)
    }
    fn as_item(child: &Control) -> Option<Rc<dyn IRibbonItem>> {
        let value = Control::boxed(child.to_ref());
        ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IRibbonItem>>())
            .and_then(|value| value.downcast_ref::<Rc<dyn IRibbonItem>>().cloned())
    }
    fn detach(item: Option<&Control>) {
        let item = item.expect("ArgumentNull: child");
        if let Some(panel) = item
            .visual_parent()
            .and_then(|parent| parent.cast::<Panel>())
        {
            panel.children().remove(item.to_ref());
        }
    }
    fn populate_host(&self) {
        for item in self.items.iter() {
            Self::detach(item.as_deref());
            self.host()
                .expect("ItemsHost became null during collection enumeration")
                .children()
                .add(item.expect("ArgumentNull: child"));
        }
    }
    fn on_items_changed(&self) {
        if let Some(host) = self.host() {
            host.children().clear();
            self.populate_host();
        }
        self.apply_child_layouts();
        super::RibbonItemHelper::invalidate_host_layout(self);
    }
    fn apply_child_layouts(&self) {
        for child in self.items.iter().flatten() {
            if let Some(item) = Self::as_item(&child) {
                let layout = self.last_layout.borrow().clone();
                let layout = self.get_child_layout(item.clone(), layout);
                item.apply_layout(layout);
            }
        }
    }
}
impl FerroObjectImpl for RibbonItemsContainer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        this.items.add_collection_changed(Rc::new(move |_| {
            if let Some(this) = weak.upgrade() {
                this.on_items_changed()
            }
        }));
        this.set_can_add_to_quick_access(false);
        KeyboardNavigation::set_is_tab_stop(this, false);
    }
}
impl TemplatedControlImpl for RibbonItemsContainer {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        if let Some(host) = this.host() {
            host.children().clear();
        }
        Self::parent_on_apply_template(this, e);
        this.scope.replace(Some(e.name_scope().clone()));
        let host = e
            .name_scope()
            .find("PART_ItemsHost")
            .and_then(|host| host.cast::<Panel>());
        let previous = this.host.replace(host);
        drop(previous);
        if this.host().is_some() {
            this.populate_host();
        }
        this.apply_child_layouts();
    }
}
impl LayoutableImpl for RibbonItemsContainer {
    fn apply_template(this: &Self) {
        Self::parent_apply_template(this);
        if this.visual_children().count() == 0 {
            let old = this.host.replace(None);
            if let Some(old) = old {
                old.children().clear();
            }
            this.scope.replace(None);
        }
    }
}
impl RibbonControlBaseImpl for RibbonItemsContainer {
    fn on_layout_applied_core(this: &Self, layout: RibbonItemLayout) {
        let old = this.last_layout.replace(layout);
        drop(old);
        this.apply_child_layouts();
    }
    fn create_overflow_menu_items_core(this: &Self) -> RibbonOverflowMenuItems {
        let items = this.items();
        RibbonItemsSource::deferred(move || {
            Box::new(
                items
                    .iter()
                    .filter_map(|child| {
                        child.and_then(|child| RibbonItemsContainer::as_item(&child))
                    })
                    .flat_map(|item| item.create_overflow_menu_items().iter()),
            )
        })
    }
    fn create_linked_copy_core(this: &Self) -> Option<Ref<Control>> {
        let copy = this.create_container_copy();
        if let Some(stack) = copy.downcast_ref::<RibbonStackPanel>()
            && let Some(source) = this.downcast_ref::<RibbonStackPanel>()
        {
            stack.set_orientation(source.orientation());
            stack.set_spacing(source.spacing());
        }
        for item in this.items.iter().flatten() {
            if let Some(ribbon) = Self::as_item(&item)
                && let Some(child) = ribbon.create_linked_copy()
            {
                super::RibbonItemHelper::set_source_item(&child, item);
                copy.items.add(Some(child));
            }
        }
        (copy.items.count() > 0).then(|| copy.upcast())
    }
    fn on_key_tip_core(_this: &Self) -> Rc<RibbonKeyTipResult> {
        RibbonKeyTipResult::close()
    }
    fn invoke_core(_this: &Self) -> bool {
        false
    }
}
impl RibbonItemsContainerImpl for RibbonItemsContainer {
    fn create_container_copy(_this: &Self) -> Ref<RibbonItemsContainer> {
        panic!(
            "RibbonItemsContainer is abstract: CreateContainerCopy must be implemented by the deriving class"
        )
    }
    fn get_child_layout(
        _this: &Self,
        _child: Rc<dyn IRibbonItem>,
        _layout: RibbonItemLayout,
    ) -> RibbonItemLayout {
        panic!(
            "RibbonItemsContainer is abstract: GetChildLayout must be implemented by the deriving class"
        )
    }
}
// Source abstract methods have no native default success or public constructor.
// Concrete classes below provide both required implementations.
struct ContainerHost(Ref<RibbonItemsContainer>);
impl IRibbonLayoutHost for ContainerHost {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn invalidate_items_layout(&self) {
        self.0.invalidate_items_layout()
    }
}
#[repr(C)]
pub struct RibbonButtonGroup {
    base: RibbonItemsContainer,
}
ferro_class!(RibbonButtonGroup:RibbonItemsContainer);
ferro_class_info!(RibbonButtonGroup{new:RibbonButtonGroup::new,markup:{namespace:"FerroRibbon.Controls",content:Items}});
ferro_impl_classes!(RibbonButtonGroup:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonControlBaseImpl);
impl RibbonButtonGroup {
    pub fn construct() -> Self {
        Self {
            base: RibbonItemsContainer::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
impl FerroObjectImpl for RibbonButtonGroup {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_size(RibbonItemSize::Small);
    }
}
impl StyledElementImpl for RibbonButtonGroup {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
impl RibbonItemsContainerImpl for RibbonButtonGroup {
    fn create_container_copy(_this: &Self) -> Ref<RibbonItemsContainer> {
        RibbonButtonGroup::new().upcast()
    }
    fn get_child_layout(
        _this: &Self,
        _child: Rc<dyn IRibbonItem>,
        layout: RibbonItemLayout,
    ) -> RibbonItemLayout {
        RibbonItemLayout {
            size: RibbonItemSize::Small,
            show_label: Some(false),
            ..layout
        }
    }
}
#[repr(C)]
pub struct RibbonStackPanel {
    base: RibbonItemsContainer,
}
ferro_class!(RibbonStackPanel:RibbonItemsContainer);
ferro_class_info!(RibbonStackPanel{new:RibbonStackPanel::new,markup:{namespace:"FerroRibbon.Controls",content:Items}});
ferro_impl_classes!(RibbonStackPanel:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,RibbonControlBaseImpl);
ferro_properties! {impl RibbonStackPanel {
 pub fn orientation_property()->StyledProperty<Orientation>{FerroProperty::register::<Self,_>("Orientation",Orientation::Horizontal)}
 pub fn spacing_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("Spacing",3.)}
}}
impl RibbonStackPanel {
    pub fn construct() -> Self {
        Self {
            base: RibbonItemsContainer::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }
    pub fn set_orientation(&self, value: Orientation) {
        self.set_value(Self::orientation_property(), value)
    }
    pub fn spacing(&self) -> f64 {
        self.get_value(Self::spacing_property())
    }
    pub fn set_spacing(&self, value: f64) {
        self.set_value(Self::spacing_property(), value)
    }
}
impl FerroObjectImpl for RibbonStackPanel {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_size(RibbonItemSize::Small);
    }
}
impl StyledElementImpl for RibbonStackPanel {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
impl RibbonItemsContainerImpl for RibbonStackPanel {
    fn create_container_copy(_this: &Self) -> Ref<RibbonItemsContainer> {
        RibbonStackPanel::new().upcast()
    }
    fn get_child_layout(
        _this: &Self,
        child: Rc<dyn IRibbonItem>,
        layout: RibbonItemLayout,
    ) -> RibbonItemLayout {
        RibbonItemLayout {
            size: if layout.is_simplified {
                RibbonItemSize::Small
            } else {
                child
                    .effective_size_definition()
                    .get_size(layout.group_state)
            },
            show_label: None,
            ..layout
        }
    }
}
