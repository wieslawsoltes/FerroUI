//! RibbonTitleBar.cs: RibbonStatusBar. Copyright Wiesław Šoltés, MIT.
//! Native ownership, collection identity and template integration.
use super::ribbon_quick_access_tool_bar::as_item;
use super::{
    IRibbonItemEditable, IRibbonItemOwner, IRibbonLayoutHost, Ribbon, RibbonGroup,
    RibbonItemHelper, RibbonItemInvokedEventArgs, RibbonItemLayout,
};
use crate::{
    metadata::control_items::RibbonControlItems, primitives::RibbonMetrics, theming::RibbonTheme,
};
use ferroribbon_core::{RibbonItemSize, RibbonString, commands::RibbonCommandCatalog};
use ferroui_base::{
    BoxedValue, ElementRef, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElementImpl, KeyboardNavigation},
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::MarkupDelegate,
    reactive::{Disposable, IDisposable},
    utilities::HandlerList,
};
use ferroui_controls::{
    Control, ControlImpl, Panel, StackPanel,
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
};
use ferroui_system::collections::specialized::INotifyCollectionChanged;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{Arc, OnceLock},
};

type ItemHandler = dyn Fn(&RibbonStatusBar, &Rc<RibbonItemInvokedEventArgs>);
#[repr(C)]
pub struct RibbonStatusBar {
    base: TemplatedControl,
    items: RibbonControlItems,
    end_items: RibbonControlItems,
    start: RefCell<Option<Ref<StackPanel>>>,
    end: RefCell<Option<Ref<StackPanel>>>,
    collection_tokens: [Cell<Option<u64>>; 2],
    invoked: HandlerList<ItemHandler>,
    markup_invoked: RefCell<Vec<(MarkupDelegate, u64)>>,
}
ferro_class!(RibbonStatusBar:TemplatedControl);
ferro_class_info!(RibbonStatusBar{new:RibbonStatusBar::new,markup:{namespace:"FerroRibbon.Controls",content:Items,
    properties:[Items:RibbonControlItems{get:RibbonStatusBar::items},EndItems:RibbonControlItems{get:RibbonStatusBar::end_items},Metrics:Arc<RibbonMetrics>{get:RibbonStatusBar::metrics},Ribbon:Option<Ref<Ribbon>>{get:RibbonStatusBar::ribbon,set:|c:&RibbonStatusBar,v|c.set_ribbon(v)}],
    methods:[fn OnItemInvoked(Ref<Control>,Option<String>,Option<BoxedValue>)=>|c:&RibbonStatusBar,item,id:Option<String>,p|c.on_item_invoked(item,id.map(Into::into),p),fn InvalidateItemsLayout()=>RibbonStatusBar::invalidate_items_layout],
    events:[ItemInvoked(Option<BoxedValue>,Rc<RibbonItemInvokedEventArgs>){add:RibbonStatusBar::add_item_invoked,remove:RibbonStatusBar::remove_item_invoked}]
},interfaces:[Rc<dyn IRibbonItemOwner> =>|v|Rc::new(StatusOwner(v)) as Rc<dyn IRibbonItemOwner>,Rc<dyn IRibbonLayoutHost> =>|v|Rc::new(StatusHost(v)) as Rc<dyn IRibbonLayoutHost>]});
ferro_impl_classes!(RibbonStatusBar:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_properties! {impl RibbonStatusBar{
    pub fn command_catalog_property()->StyledProperty<Option<Rc<RibbonCommandCatalog>>>{FerroProperty::register::<Self,_>("CommandCatalog",None)}
    pub fn ribbon_property()->StyledProperty<Option<ElementRef<Ribbon>>>{ValueTypes::register_element_ref::<Ribbon>();FerroProperty::register::<Self,_>("Ribbon",None)}
    pub fn show_labels_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowLabels",true)}
}}
impl RibbonStatusBar {
    fn static_constructor() {
        Self::command_catalog_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.resolve_commands())
            .detach();
        Self::ribbon_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.resolve_commands())
            .detach();
        Self::show_labels_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.sync())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            items: RibbonControlItems::new_managed(),
            end_items: RibbonControlItems::new_managed(),
            start: RefCell::new(None),
            end: RefCell::new(None),
            collection_tokens: [Cell::new(None), Cell::new(None)],
            invoked: HandlerList::new(),
            markup_invoked: RefCell::new(vec![]),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items(&self) -> RibbonControlItems {
        self.items.clone()
    }
    pub fn end_items(&self) -> RibbonControlItems {
        self.end_items.clone()
    }
    pub fn show_labels(&self) -> bool {
        self.get_value(Self::show_labels_property())
    }
    pub fn set_show_labels(&self, v: bool) {
        self.set_value(Self::show_labels_property(), v);
    }
    pub fn command_catalog(&self) -> Option<Rc<RibbonCommandCatalog>> {
        self.get_value(Self::command_catalog_property())
    }
    pub fn set_command_catalog(&self, v: Option<Rc<RibbonCommandCatalog>>) {
        self.set_value(Self::command_catalog_property(), v);
    }
    pub fn ribbon(&self) -> Option<Ref<Ribbon>> {
        ElementRef::resolve(&self.get_value(Self::ribbon_property()))
    }
    pub fn set_ribbon(&self, v: Option<Ref<Ribbon>>) {
        self.set_value(Self::ribbon_property(), ElementRef::from_nullable(v));
    }
    pub fn metrics(&self) -> Arc<RibbonMetrics> {
        static METRICS: OnceLock<Arc<RibbonMetrics>> = OnceLock::new();
        METRICS
            .get_or_init(|| {
                RibbonMetrics::compact().with_values(|v| v.simplified_item_height = 22.)
            })
            .clone()
    }
    fn owner(&self) -> Rc<dyn IRibbonItemOwner> {
        Rc::new(StatusOwner(self.to_ref()))
    }
    pub fn item_invoked(
        &self,
        handler: impl Fn(&Self, &Rc<RibbonItemInvokedEventArgs>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.invoked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(c) = weak.upgrade() {
                c.invoked.remove(token);
            }
        })
    }
    pub fn add_item_invoked(&self, handler: Option<MarkupDelegate>) {
        let Some(handler) = handler else {
            return;
        };
        let callback = handler.clone();
        let token = self.invoked.add(Rc::new(move |c, args| {
            callback
                .try_invoke(&[
                    Some(Control::boxed(c.to_ref())),
                    Some(Rc::new(args.clone())),
                ])
                .expect("StatusBar ItemInvoked XAML handler");
        }));
        self.markup_invoked.borrow_mut().push((handler, token));
    }
    pub fn remove_item_invoked(&self, handler: Option<MarkupDelegate>) {
        let removed = {
            let mut handlers = self.markup_invoked.borrow_mut();
            handlers
                .iter()
                .rposition(|(h, _)| Some(h) == handler.as_ref())
                .map(|i| handlers.remove(i))
        };
        if let Some((_, token)) = removed {
            self.invoked.remove(token);
        }
    }
    pub fn on_item_invoked(
        &self,
        item: Ref<Control>,
        command_id: Option<RibbonString>,
        parameter: Option<BoxedValue>,
    ) {
        if !self.invoked.is_empty() {
            let item_id = as_item(&item).and_then(|i| i.id()).or_else(|| {
                item.cast::<RibbonGroup>()
                    .and_then(|g| g.id().map(Into::into))
            });
            let args = Rc::new(RibbonItemInvokedEventArgs {
                item: item.clone(),
                command_id: command_id.clone(),
                item_id,
                parameter: parameter.clone(),
            });
            for (_, handler) in self.invoked.snapshot().iter() {
                handler(self, &args);
            }
        }
        if let Some(ribbon) = self.ribbon() {
            ribbon.on_item_invoked(item, command_id, parameter);
        }
    }
    fn sync(&self) {
        let start = self.start.borrow().clone();
        self.fill(start, &self.items);
        let end = self.end.borrow().clone();
        self.fill(end, &self.end_items);
        self.resolve_commands();
    }
    fn resolve_commands(&self) {
        for item in self.items.iter().chain(self.end_items.iter()).flatten() {
            for element in RibbonGroup::flatten(Some(item)).iter() {
                RibbonItemHelper::resolve_command(&element).expect("StatusBar command resolution");
            }
        }
    }
    fn fill(&self, host: Option<Ref<StackPanel>>, items: &RibbonControlItems) {
        for item in items.iter() {
            let item = item.expect("Null status-bar item");
            RibbonItemHelper::set_owner(&item, Some(self.owner())).expect("StatusBar item owner");
        }
        let Some(host) = host else {
            return;
        };
        host.children().clear();
        for item in items.iter() {
            let item = item.expect("Null status-bar item");
            if let Some(parent) = item.visual_parent().and_then(|p| p.cast::<Panel>()) {
                parent.children().remove(item.clone());
            }
            if let Some(ribbon_item) = as_item(&item) {
                let boxed = Control::boxed(item.clone());
                let editable = ValueTypes::try_convert(
                    Some(&boxed),
                    ValueType::of::<Rc<dyn IRibbonItemEditable>>(),
                )
                .flatten();
                let show_label = self.show_labels()
                    && editable.as_ref().is_none_or(|v| {
                        v.downcast_ref::<Rc<dyn IRibbonItemEditable>>()
                            .expect("Status item interface conversion")
                            .show_label()
                    });
                ribbon_item.apply_layout(RibbonItemLayout {
                    size: RibbonItemSize::Small,
                    metrics: Some(self.metrics()),
                    is_simplified: true,
                    show_label: Some(show_label),
                    ..Default::default()
                });
            }
            host.children().add(item);
        }
    }
    pub fn invalidate_items_layout(&self) {
        self.invalidate_measure();
    }
}
impl Drop for RibbonStatusBar {
    fn drop(&mut self) {
        for (items, token) in [
            (&self.items, &self.collection_tokens[0]),
            (&self.end_items, &self.collection_tokens[1]),
        ] {
            if let Some(token) = token.take() {
                items.collection_changed().remove(token);
            }
        }
    }
}
impl FerroObjectImpl for RibbonStatusBar {
    fn constructed(c: &Self) {
        Self::parent_constructed(c);
        RibbonTheme::ensure_resources();
        for (items, slot) in [
            (&c.items, &c.collection_tokens[0]),
            (&c.end_items, &c.collection_tokens[1]),
        ] {
            let weak = c.to_ref().downgrade();
            let token = items.collection_changed().add(Rc::new(move |_| {
                if let Some(c) = weak.upgrade() {
                    c.sync();
                }
            }));
            slot.set(Some(token));
        }
        KeyboardNavigation::set_is_tab_stop(c, false);
    }
}
impl StyledElementImpl for RibbonStatusBar {
    fn style_key_override(_: &Self) -> &'static TypeInfo {
        Self::TYPE
    }
}
impl TemplatedControlImpl for RibbonStatusBar {
    fn on_apply_template(c: &Self, e: &TemplateAppliedEventArgs) {
        let start = c.start.borrow().clone();
        if let Some(start) = start {
            start.children().clear();
        }
        let end = c.end.borrow().clone();
        if let Some(end) = end {
            end.children().clear();
        }
        Self::parent_on_apply_template(c, e);
        c.start
            .replace(e.name_scope().find_as::<StackPanel>("PART_StartItems"));
        c.end
            .replace(e.name_scope().find_as::<StackPanel>("PART_EndItems"));
        c.sync();
    }
}
struct StatusOwner(Ref<RibbonStatusBar>);
impl IRibbonItemOwner for StatusOwner {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn metrics(&self) -> Option<Arc<RibbonMetrics>> {
        Some(self.0.metrics())
    }
    fn owner_ribbon(&self) -> Option<Ref<Ribbon>> {
        self.0.ribbon()
    }
    fn command_catalog(&self) -> Option<Rc<RibbonCommandCatalog>> {
        self.0
            .command_catalog()
            .or_else(|| self.0.ribbon().and_then(|r| r.command_catalog()))
    }
    fn on_item_invoked(&self, item: Ref<Control>, id: Option<RibbonString>, p: Option<BoxedValue>) {
        self.0.on_item_invoked(item, id, p);
    }
}
struct StatusHost(Ref<RibbonStatusBar>);
impl IRibbonLayoutHost for StatusHost {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn invalidate_items_layout(&self) {
        self.0.invalidate_items_layout();
    }
}

#[cfg(test)]
#[path = "../status_bar_tests.rs"]
mod tests;
