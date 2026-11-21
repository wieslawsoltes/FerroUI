//! Shared RibbonPickers.cs: source segment state, selection and deferred overflow.
//! Copyright Wiesław Šoltés, MIT. CLR graph/hash override limitations are recorded separately.
use super::{
    RibbonControlBase, RibbonControlBaseImpl, RibbonItemLayout, RibbonOverflowMenuItems,
    RibbonToggleButton,
};
use ferroribbon_core::{RibbonItemSize, model::RibbonItemsSource};
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty, TypeInfo, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties,
    input::{InputElementImpl, Key, KeyEventArgs, KeyboardNavigation},
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::{MarkupDelegate, into_markup_value},
};
use ferroui_controls::{
    Control, ControlImpl, StackPanel,
    automation::AutomationProperties,
    primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt},
};
use ferroui_system::collections::{
    object_model::ObservableCollection, specialized::INotifyCollectionChanged,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[repr(C)]
pub struct RibbonSegment {
    base: FerroObject,
}
ferro_class!(RibbonSegment:FerroObject);
ferro_class_info!(RibbonSegment {new:RibbonSegment::new,markup:{namespace:"FerroRibbon.Controls"}});
ferro_properties! {impl RibbonSegment {
 pub fn label_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("Label",None)}
 pub fn icon_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,_>("Icon",None)}
 pub fn value_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,_>("Value",None)}
}}
impl FerroObjectImpl for RibbonSegment {}
impl RibbonSegment {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn label(&self) -> Option<String> {
        self.get_value(Self::label_property())
    }
    pub fn set_label(&self, v: Option<String>) {
        self.set_value(Self::label_property(), v)
    }
    pub fn icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_property())
    }
    pub fn set_icon(&self, v: Option<BoxedValue>) {
        self.set_value(Self::icon_property(), v)
    }
    pub fn value(&self) -> Option<BoxedValue> {
        self.get_value(Self::value_property())
    }
    pub fn set_segment_value(&self, v: Option<BoxedValue>) {
        self.set_value(Self::value_property(), v)
    }
}
// The source collection is shared, notifying and permits runtime null despite its annotation.
pub type RibbonSegments = ObservableCollection<Option<Ref<RibbonSegment>>>;
#[repr(C)]
pub struct RibbonSegmentedControl {
    base: RibbonControlBase,
    segments: RibbonSegments,
    host: RefCell<Option<Ref<StackPanel>>>,
    selection_changed: RefCell<Option<MarkupDelegate>>,
    collection_token: Cell<Option<u64>>,
}
ferro_class!(RibbonSegmentedControl:RibbonControlBase);
ferro_class_info!(RibbonSegmentedControl {new:RibbonSegmentedControl::new,markup:{namespace:"FerroRibbon.Controls",content:Segments,
 properties:[Segments:RibbonSegments{get:RibbonSegmentedControl::segments},SelectedSegment:Option<Ref<RibbonSegment>>{get:RibbonSegmentedControl::selected_segment}],
 events:[SelectionChanged(Option<BoxedValue>,Option<Ref<RibbonSegment>>){add:RibbonSegmentedControl::add_selection_changed,remove:RibbonSegmentedControl::remove_selection_changed}]
}});
ferro_impl_classes!(RibbonSegmentedControl:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_properties! {impl RibbonSegmentedControl {
 pub fn selected_index_property()->StyledProperty<i32>{FerroProperty::register::<Self,_>("SelectedIndex",0)}
}}
impl RibbonSegmentedControl {
    fn static_constructor() {
        Self::selected_index_property()
            .changed()
            .add_class_handler::<Self>(|c, e| c.on_selected_index_changed(e.get_new_value::<i32>()))
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: RibbonControlBase::construct(),
            segments: RibbonSegments::new_managed(),
            host: RefCell::new(None),
            selection_changed: RefCell::new(None),
            collection_token: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn segments(&self) -> RibbonSegments {
        self.segments.clone()
    }
    pub fn selected_index(&self) -> i32 {
        self.get_value(Self::selected_index_property())
    }
    pub fn set_selected_index(&self, value: i32) {
        self.set_value(Self::selected_index_property(), value)
    }
    pub fn selected_segment(&self) -> Option<Ref<RibbonSegment>> {
        let index = self.selected_index();
        if index >= 0 && (index as usize) < self.segments.count() {
            self.segments.get(index as usize)
        } else {
            None
        }
    }
    pub fn add_selection_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.selection_changed.borrow().as_ref(), h.as_ref());
        self.selection_changed.replace(next);
    }
    pub fn remove_selection_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.selection_changed.borrow().as_ref(), h.as_ref());
        self.selection_changed.replace(next);
    }
    fn build(&self) {
        let Some(host) = self.host.borrow().clone() else {
            return;
        };
        host.children().clear();
        // Source rereads Count, segment properties and _host around every callback.
        let mut index = 0;
        while index < self.segments.count() {
            let segment = self
                .segments
                .get(index)
                .expect("NullReference: RibbonSegment");
            let button = RibbonToggleButton::new();
            button.set_label(segment.label());
            button.set_icon(segment.icon());
            button.set_is_checked(Some(index as i32 == self.selected_index()));
            button.set_can_add_to_quick_access(false);
            KeyboardNavigation::set_is_tab_stop(&button, false);
            let mut layout = RibbonItemLayout::new(RibbonItemSize::Medium, self.metrics());
            layout.is_simplified = self.is_simplified();
            layout.show_label = Some(segment.label().is_some());
            button.apply_layout(layout);
            let owner = self.to_ref().downgrade();
            let weak_button = button.downgrade();
            let captured = i32::try_from(index).expect("Int32 collection Count");
            button.click(move |_, _| {
                if let Some(c) = owner.upgrade() {
                    c.select_from_user(captured);
                }
                if let Some(b) = weak_button.upgrade() {
                    b.set_is_checked(Some(true));
                }
            });
            AutomationProperties::set_name(&button, Some(&segment.label().unwrap_or_default()));
            let target = self
                .host
                .borrow()
                .clone()
                .expect("NullReference: PART_ItemsHost");
            target.children().add(button.upcast::<Control>());
            index += 1;
        }
    }
    fn on_selected_index_changed(&self, index: i32) {
        if self.host.borrow().is_some() {
            let mut i = 0;
            while i < self
                .host
                .borrow()
                .clone()
                .expect("NullReference: PART_ItemsHost")
                .children()
                .count()
            {
                let child = self
                    .host
                    .borrow()
                    .clone()
                    .expect("NullReference: PART_ItemsHost")
                    .children()
                    .get(i);
                if let Some(toggle) = child.cast::<RibbonToggleButton>() {
                    toggle.set_is_checked(Some(i as i32 == index));
                }
                i += 1;
            }
        }
        let h = self.selection_changed.borrow().clone();
        if let Some(h) = h {
            h.try_invoke(&[
                into_markup_value(self.to_ref()),
                into_markup_value(self.selected_segment()),
            ])
            .expect("SelectionChanged handler");
        }
    }
    fn select_from_user(&self, index: i32) {
        if index < 0 || index as usize >= self.segments.count() {
            return;
        }
        self.set_selected_index(index);
        if let Some(segment) = self.selected_segment() {
            let value = segment
                .value()
                .or_else(|| segment.label().map(|v| Rc::new(v) as BoxedValue));
            self.execute_command(value)
                .unwrap_or_else(|e| panic!("{e}"));
        }
    }
    fn on_segment_key_down(&self, e: &KeyEventArgs) {
        let count = i32::try_from(self.segments.count()).expect("Int32 collection Count");
        if count == 0 {
            return;
        }
        let current = self.selected_index().clamp(0, count - 1);
        let next = match e.key {
            Key::Left => (current - 1).max(0),
            Key::Right => (current + 1).min(count - 1),
            Key::Home => 0,
            Key::End => count - 1,
            Key::Enter | Key::Space => current,
            _ => return,
        };
        self.select_from_user(next);
        e.set_handled(true);
    }
}
impl Drop for RibbonSegmentedControl {
    fn drop(&mut self) {
        if let Some(token) = self.collection_token.take() {
            self.segments.collection_changed().remove(token);
        }
    }
}
impl FerroObjectImpl for RibbonSegmentedControl {
    fn constructed(c: &Self) {
        Self::parent_constructed(c);
        let weak = c.to_ref().downgrade();
        let token = c.segments.collection_changed().add(Rc::new(move |_| {
            if let Some(c) = weak.upgrade() {
                c.build();
            }
        }));
        c.collection_token.set(Some(token));
        c.set_can_add_to_quick_access(false);
        c.set_size(RibbonItemSize::Medium);
        c.set_focusable(true);
        KeyboardNavigation::set_is_tab_stop(c, true);
        let weak = c.to_ref().downgrade();
        c.key_down(move |_, e| {
            if let Some(c) = weak.upgrade() {
                c.on_segment_key_down(e)
            }
        });
    }
}
impl StyledElementImpl for RibbonSegmentedControl {
    fn style_key_override(_c: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
impl TemplatedControlImpl for RibbonSegmentedControl {
    fn on_apply_template(c: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(c, e);
        c.host
            .replace(e.name_scope().find_as::<StackPanel>("PART_ItemsHost"));
        c.build();
    }
}
impl RibbonControlBaseImpl for RibbonSegmentedControl {
    fn on_layout_applied_core(c: &Self, _layout: RibbonItemLayout) {
        c.build();
    }
    fn invoke_core(c: &Self) -> bool {
        let count = i32::try_from(c.segments.count()).expect("Int32 collection Count");
        if count == 0 {
            return false;
        }
        c.select_from_user(c.selected_index().max(0).wrapping_add(1) % count);
        true
    }
    fn create_overflow_menu_items_core(c: &Self) -> RibbonOverflowMenuItems {
        let c = c.to_ref();
        RibbonItemsSource::deferred(move || {
            let c = c.clone();
            let source = c.segments();
            Box::new(source.iter().enumerate().map(move |(index, segment)| {
                let segment = segment.expect("NullReference: RibbonSegment");
                let item = crate::compat::RadioMenuFlyoutItem::new();
                item.set_text(Some(segment.label().unwrap_or_default()));
                // Native identity hash is opaque, stable while this instance lives, and formatted invariantly.
                item.set_group_name(Some(
                    ((&*c as *const RibbonSegmentedControl as usize) as i32).to_string(),
                ));
                item.set_is_checked(index as i32 == c.selected_index());
                let owner = c.downgrade();
                let captured = i32::try_from(index).expect("Int32 collection Count");
                item.add_click_action(move |_, _| {
                    if let Some(c) = owner.upgrade() {
                        c.select_from_user(captured);
                    }
                });
                Some(item.upcast::<Control>())
            }))
        })
    }
}

#[cfg(test)]
#[path = "../segmented_tests.rs"]
mod tests;
