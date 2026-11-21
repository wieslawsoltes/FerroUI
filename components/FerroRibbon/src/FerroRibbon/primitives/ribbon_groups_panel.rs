//! The real shared RibbonGroupsPanel.cs: cache-backed adaptive group composition.
use super::{IRibbonScrollContent, RibbonMetrics, ribbon_hidden_focus};
use crate::controls::{IRibbonItem, RibbonGroup, RibbonPanelPresentation};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{
    RibbonGroupState, RibbonSimplifiedVisibility,
    collections::RibbonReadOnlyList,
    layout::{
        RibbonAdaptiveLayout, RibbonGroupLayoutInfo, RibbonReductionStrategy,
        RibbonSimplifiedGroup, RibbonSimplifiedItem, RibbonSimplifiedLayout,
    },
    model::RibbonItemsSource,
};
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, VisualImpl,
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::{MarkupDelegate, MarkupInvokeError},
    threading::{Dispatcher, DispatcherPriority},
    utilities::EventArgs,
};
use ferroui_controls::{Control, ControlImpl, Controls, Panel, PanelImpl};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::Arc,
};

#[derive(Clone)]
struct Cached {
    group: Ref<RibbonGroup>,
    version: i32,
    metrics: Arc<RibbonMetrics>,
    widths: Rc<RefCell<Vec<f64>>>,
}
#[derive(Default)]
struct Cache {
    entries: HashMap<usize, Cached>,
    order: Vec<usize>,
}
fn key(group: &RibbonGroup) -> usize {
    group as *const RibbonGroup as usize
}
impl Cache {
    fn get(&self, group: &RibbonGroup) -> Option<Cached> {
        self.entries.get(&key(group)).cloned()
    }
    fn insert(&mut self, value: Cached) -> Option<Cached> {
        let key = key(&value.group);
        if !self.entries.contains_key(&key) {
            self.order.push(key);
        }
        self.entries.insert(key, value)
    }
    fn remove(&mut self, group: &RibbonGroup) -> Option<Cached> {
        let key = key(group);
        self.order.retain(|entry| *entry != key);
        self.entries.remove(&key)
    }
    fn clear(&mut self) -> Vec<Cached> {
        self.order.clear();
        self.entries.drain().map(|(_, value)| value).collect()
    }
}
// Native Controls exposes COW storage rather than the source enumerator version. Keeping its
// storage identity detects mutation before every next step, including after the last yielded
// child.
struct ChildrenCursor {
    children: Controls,
    storage: Rc<Vec<Ref<Control>>>,
    index: usize,
}
impl ChildrenCursor {
    fn new(children: Controls) -> Self {
        let storage = children.snapshot();
        Self {
            children,
            storage,
            index: 0,
        }
    }
}
impl Iterator for ChildrenCursor {
    type Item = Ref<Control>;
    fn next(&mut self) -> Option<Self::Item> {
        assert!(
            Rc::ptr_eq(&self.storage, &self.children.snapshot()),
            "Collection was modified; enumeration operation may not execute."
        );
        let value = self.storage.get(self.index).cloned();
        if value.is_some() {
            self.index += 1;
        }
        value
    }
}
#[repr(C)]
pub struct RibbonGroupsPanel {
    base: Panel,
    classic_cache: RefCell<Cache>,
    simplified_cache: RefCell<Cache>,
    overflow_button: RefCell<Option<Ref<Control>>>,
    extent: Cell<f64>,
    is_simplified: Cell<bool>,
    metrics: RefCell<Option<Arc<RibbonMetrics>>>,
    is_adaptive: Cell<bool>,
    panel_presentation: Cell<RibbonPanelPresentation>,
    reduction_strategy: Cell<RibbonReductionStrategy>,
    show_captions: Cell<bool>,
    group_spacing: Cell<f64>,
    has_overflow: Cell<bool>,
    overflow_changed: RefCell<Option<MarkupDelegate>>,
}
ferro_class!(RibbonGroupsPanel:Panel);
ferro_impl_classes!(RibbonGroupsPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_class_info!(RibbonGroupsPanel{new:RibbonGroupsPanel::new,
    markup:{namespace:"FerroRibbon.Controls.Primitives",content:Children,properties:[
        IsSimplified:bool{get:RibbonGroupsPanel::is_simplified,set:RibbonGroupsPanel::set_is_simplified},
        Metrics:Option<Arc<RibbonMetrics>>{get:RibbonGroupsPanel::metrics,set:RibbonGroupsPanel::set_metrics},
        IsAdaptive:bool{get:RibbonGroupsPanel::is_adaptive,set:RibbonGroupsPanel::set_is_adaptive},
        PanelPresentation:RibbonPanelPresentation{get:RibbonGroupsPanel::panel_presentation,set:RibbonGroupsPanel::set_panel_presentation},
        ReductionStrategy:RibbonReductionStrategy{get:RibbonGroupsPanel::reduction_strategy,set:RibbonGroupsPanel::set_reduction_strategy},
        ShowCaptions:bool{get:RibbonGroupsPanel::show_captions,set:RibbonGroupsPanel::set_show_captions},
        GroupSpacing:f64{get:RibbonGroupsPanel::group_spacing,set:RibbonGroupsPanel::set_group_spacing},
        HasOverflow:bool{get:RibbonGroupsPanel::has_overflow},
        OverflowButton:Option<Ref<Control>>{get:RibbonGroupsPanel::overflow_button,set:RibbonGroupsPanel::set_overflow_button},
        ExtentWidth:f64{get:RibbonGroupsPanel::extent_width},
        Groups:RibbonItemsSource<Ref<RibbonGroup>>{get:RibbonGroupsPanel::groups},
    ],methods:[fn InvalidateCache()=>RibbonGroupsPanel::invalidate_cache],
    events:[OverflowChanged(Option<BoxedValue>,EventArgs){add:RibbonGroupsPanel::add_overflow_changed,remove:RibbonGroupsPanel::remove_overflow_changed}]},
    interfaces:[Rc<dyn IRibbonScrollContent> =>|value|Rc::new(ScrollView(value)) as Rc<dyn IRibbonScrollContent>],
});
macro_rules! cell_property {
    ($get:ident,$set:ident,$field:ident,$ty:ty) => {
        pub fn $get(&self) -> $ty {
            self.$field.get()
        }
        pub fn $set(&self, value: $ty) {
            self.$field.set(value)
        }
    };
}
impl RibbonGroupsPanel {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            classic_cache: RefCell::default(),
            simplified_cache: RefCell::default(),
            overflow_button: RefCell::default(),
            extent: Cell::new(0.),
            is_simplified: Cell::new(false),
            metrics: RefCell::new(Some(RibbonMetrics::comfortable())),
            is_adaptive: Cell::new(true),
            panel_presentation: Cell::new(RibbonPanelPresentation::Full),
            reduction_strategy: Cell::new(RibbonReductionStrategy::Stepwise),
            show_captions: Cell::new(true),
            group_spacing: Cell::new(0.),
            has_overflow: Cell::new(false),
            overflow_changed: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    cell_property!(is_simplified, set_is_simplified, is_simplified, bool);
    cell_property!(is_adaptive, set_is_adaptive, is_adaptive, bool);
    cell_property!(
        panel_presentation,
        set_panel_presentation,
        panel_presentation,
        RibbonPanelPresentation
    );
    cell_property!(
        reduction_strategy,
        set_reduction_strategy,
        reduction_strategy,
        RibbonReductionStrategy
    );
    cell_property!(show_captions, set_show_captions, show_captions, bool);
    cell_property!(group_spacing, set_group_spacing, group_spacing, f64);
    pub fn metrics(&self) -> Option<Arc<RibbonMetrics>> {
        self.metrics.borrow().clone()
    }
    pub fn set_metrics(&self, value: Option<Arc<RibbonMetrics>>) {
        drop(self.metrics.replace(value));
    }
    fn required_metrics(&self) -> Arc<RibbonMetrics> {
        self.metrics().expect("RibbonGroupsPanel.Metrics is null")
    }
    pub fn extent_width(&self) -> f64 {
        self.extent.get()
    }
    pub fn has_overflow(&self) -> bool {
        self.has_overflow.get()
    }
    pub fn overflow_button(&self) -> Option<Ref<Control>> {
        self.overflow_button.borrow().clone()
    }
    pub fn set_overflow_button(&self, value: Option<Ref<Control>>) {
        if let Some(old) = self.overflow_button() {
            self.children().remove(old);
        }
        drop(self.overflow_button.replace(value.clone()));
        if let Some(value) = value {
            self.children().add(value);
        }
    }
    pub fn groups(&self) -> RibbonItemsSource<Ref<RibbonGroup>> {
        let children = self.children();
        RibbonItemsSource::deferred(move || {
            Box::new(
                ChildrenCursor::new(children.clone())
                    .filter_map(|child| child.cast::<RibbonGroup>()),
            )
        })
    }
    pub fn invalidate_cache(&self) {
        let removed = self.classic_cache.borrow_mut().clear();
        drop(removed);
        let removed = self.simplified_cache.borrow_mut().clear();
        drop(removed);
        self.invalidate_measure();
    }
    pub fn add_overflow_changed(&self, handler: Option<MarkupDelegate>) {
        let old = {
            let mut current = self.overflow_changed.borrow_mut();
            let next = MarkupDelegate::combine(current.as_ref(), handler.as_ref());
            std::mem::replace(&mut *current, next)
        };
        drop(old);
    }
    pub fn remove_overflow_changed(&self, handler: Option<MarkupDelegate>) {
        let old = {
            let mut current = self.overflow_changed.borrow_mut();
            let next = MarkupDelegate::remove(current.as_ref(), handler.as_ref());
            std::mem::replace(&mut *current, next)
        };
        drop(old);
    }
    fn raise_overflow_changed(&self) -> Result<(), MarkupInvokeError> {
        let event = self.overflow_changed.borrow().clone();
        if let Some(event) = event {
            OVERFLOW_ARGS.with(|args| {
                event.try_invoke(&[Some(Control::boxed(self.to_ref())), Some(args.clone())])
            })?;
        }
        Ok(())
    }
    pub(crate) fn set_overflow(&self, value: bool) {
        if self.has_overflow() == value {
            return;
        }
        self.has_overflow.set(value);
        // Capture this, but read the invocation list when the queued callback runs.
        let this = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                this.raise_overflow_changed()
                    .expect("OverflowChanged handler")
            },
            DispatcherPriority::NORMAL,
        );
    }
    fn measure_classic(&self, groups: &[Ref<RibbonGroup>], available: f64, height: f64) -> Size {
        self.set_overflow(false);
        if let Some(button) = self.overflow_button() {
            button.measure(Size::new(0., 0.));
        }
        let mut states = vec![RibbonGroupState::Large; groups.len()];
        if self.panel_presentation() != RibbonPanelPresentation::Full {
            states.fill(RibbonGroupState::Collapsed);
        } else if self.is_adaptive() && !available.is_infinite() {
            let mut infos = Vec::with_capacity(groups.len());
            for group in groups {
                let cached = self.classic_cache.borrow().get(group);
                let cached = if let Some(cached) = cached.filter(|cached| {
                    cached.version == group.layout_version()
                        && Arc::ptr_eq(&cached.metrics, &self.required_metrics())
                }) {
                    cached
                } else {
                    let mut widths = vec![0.; 4];
                    for (state, width) in widths.iter_mut().enumerate() {
                        *width = group.measure_width(
                            RibbonGroupState::from_raw(state as i32),
                            self.required_metrics(),
                            height,
                        );
                    }
                    let cached = Cached {
                        group: group.clone(),
                        version: group.layout_version(),
                        metrics: self.required_metrics(),
                        widths: Rc::new(RefCell::new(widths)),
                    };
                    let previous = self.classic_cache.borrow_mut().insert(cached.clone());
                    drop(previous);
                    cached
                };
                infos.push(RibbonGroupLayoutInfo {
                    widths: Some(cached.widths),
                    reduction_order: group.reduction_order(),
                    can_collapse: group.can_collapse(),
                });
            }
            states = RibbonAdaptiveLayout::compute_with_strategy(
                &infos,
                available,
                self.group_spacing(),
                self.reduction_strategy(),
            )
            .states;
        }
        let mut total = 0.;
        for (index, group) in groups.iter().enumerate() {
            group.apply_state(states[index], false, self.required_metrics(), None);
            group.measure(Size::new(f64::INFINITY, height));
            total += group.desired_size().width + if index > 0 { self.group_spacing() } else { 0. };
        }
        for group in self.groups().iter().filter(|group| !group.is_visible()) {
            group.measure(Size::new(0., 0.));
        }
        Size::new(total, height)
    }
    fn measure_simplified(&self, groups: &[Ref<RibbonGroup>], available: f64, height: f64) -> Size {
        let mut overflow_width = 0.;
        if let Some(button) = self.overflow_button() {
            button.measure(Size::new(f64::INFINITY, height));
            overflow_width = button.desired_size().width + 4.;
        }
        let mut model = Vec::with_capacity(groups.len());
        let mut chrome = Vec::with_capacity(groups.len());
        for group in groups {
            let cached = self.simplified_cache.borrow().get(group);
            let cached = if let Some(cached) = cached.filter(|cached| {
                cached.version == group.layout_version()
                    && Arc::ptr_eq(&cached.metrics, &self.required_metrics())
            }) {
                cached
            } else {
                // C# tuple evaluates Version and Metrics before the measurement call.
                let version = group.layout_version();
                let metrics = self.required_metrics();
                let widths = group.measure_simplified_item_widths(self.required_metrics());
                let cached = Cached {
                    group: group.clone(),
                    version,
                    metrics,
                    widths: Rc::new(RefCell::new(widths)),
                };
                let previous = self.simplified_cache.borrow_mut().insert(cached.clone());
                drop(previous);
                cached
            };
            let children = group.items_panel().children();
            let widths = cached.widths.borrow();
            let mut items = Vec::with_capacity(widths.len());
            for (index, width) in widths.iter().enumerate() {
                let child = if index < children.count() {
                    Some(children.get(index))
                } else {
                    None
                };
                let mut visibility =
                    if group.simplified_visibility() == RibbonSimplifiedVisibility::Auto {
                        child
                            .as_ref()
                            .and_then(|child| as_item(child))
                            .map_or(RibbonSimplifiedVisibility::Auto, |item| {
                                item.simplified_visibility()
                            })
                    } else {
                        group.simplified_visibility()
                    };
                if child.as_ref().is_none_or(|child| !child.is_visible()) {
                    visibility = RibbonSimplifiedVisibility::Hidden;
                }
                items.push(RibbonSimplifiedItem {
                    width: *width,
                    visibility,
                });
            }
            model.push(RibbonSimplifiedGroup {
                items,
                reduction_order: group.reduction_order(),
            });
            chrome.push(9.);
        }
        let chrome_total = chrome.into_iter().sum::<f64>();
        let result = RibbonSimplifiedLayout::compute(
            &model,
            if available.is_infinite() {
                f64::MAX
            } else {
                ferroui_base::utilities::math_utilities::max(0., available - chrome_total)
            },
            overflow_width,
        );
        let mut total = 0.;
        for (index, group) in groups.iter().enumerate() {
            let mask = &result.in_line[index];
            group.apply_state(
                RibbonGroupState::Large,
                true,
                self.required_metrics(),
                Some(RibbonFixedItems::new(mask.clone()) as RibbonReadOnlyList<_>),
            );
            group.measure(Size::new(f64::INFINITY, height));
            if !mask.iter().any(|value| *value) {
                continue;
            }
            total += group.desired_size().width;
        }
        self.set_overflow(result.has_overflow);
        if self.has_overflow() {
            total += overflow_width;
        }
        Size::new(total, height)
    }
}
fn as_item(child: &Control) -> Option<Rc<dyn IRibbonItem>> {
    let value = Control::boxed(child.to_ref());
    ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IRibbonItem>>())
        .and_then(|value| value.downcast_ref::<Rc<dyn IRibbonItem>>().cloned())
}
ferroui_base::ferro_thread_local! {static OVERFLOW_ARGS:BoxedValue=Rc::new(EventArgs::EMPTY);}
impl LayoutableImpl for RibbonGroupsPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        let groups = this
            .groups()
            .iter()
            .filter(|group| group.is_visible())
            .collect::<Vec<_>>();
        let stale = {
            let cache = this.classic_cache.borrow();
            cache
                .order
                .iter()
                .filter_map(|key| cache.entries.get(key))
                .filter(|entry| !groups.iter().any(|group| group.ptr_eq(&entry.group)))
                .map(|entry| entry.group.clone())
                .collect::<Vec<_>>()
        };
        for group in stale {
            let removed = this.classic_cache.borrow_mut().remove(&group);
            drop(removed);
            let removed = this.simplified_cache.borrow_mut().remove(&group);
            drop(removed);
        }
        let height = if this.is_simplified() {
            this.required_metrics().simplified_height()
        } else if this.panel_presentation() == RibbonPanelPresentation::Titles {
            this.required_metrics().group_caption_height() + 12.
        } else {
            this.required_metrics().group_content_height()
                + if this.show_captions() {
                    this.required_metrics().group_caption_height()
                } else {
                    0.
                }
                + 4.
        };
        for group in &groups {
            group.set_panel_presentation(if this.is_simplified() {
                RibbonPanelPresentation::Full
            } else {
                this.panel_presentation()
            });
        }
        let result = if this.is_simplified() {
            this.measure_simplified(&groups, available.width, height)
        } else {
            this.measure_classic(&groups, available.width, height)
        };
        this.extent.set(result.width);
        result
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let mut x = 0.;
        for child in ChildrenCursor::new(this.children()) {
            if this
                .overflow_button()
                .is_some_and(|button| button.ptr_eq(&child))
            {
                continue;
            }
            if let Some(group) = child.cast::<RibbonGroup>()
                && (!group.is_visible()
                    || (this.is_simplified()
                        && group
                            .items_panel()
                            .in_line_mask()
                            .is_some_and(|mask| !mask.iter().any(|value| value))))
            {
                child.arrange(Rect::default());
                ribbon_hidden_focus::set(&child.clone().upcast(), true);
                continue;
            }
            child.arrange(Rect::new(
                x,
                0.,
                child.desired_size().width,
                final_size.height,
            ));
            ribbon_hidden_focus::set(&child.clone().upcast(), false);
            x += child.desired_size().width + this.group_spacing();
        }
        if let Some(button) = this.overflow_button() {
            if this.is_simplified() && this.has_overflow() {
                let w = button.desired_size().width;
                let h = button.desired_size().height;
                let ox = ferroui_base::utilities::math_utilities::max(x, final_size.width - w - 2.);
                button.set_is_visible(true);
                // Source re-reads the field after Visibility can invoke user code.
                this.overflow_button()
                    .expect("OverflowButton became null during Visibility notification")
                    .arrange(Rect::new(ox, (final_size.height - h) / 2., w, h));
            } else {
                button.arrange(Rect::default());
            }
        }
        final_size
    }
}
struct ScrollView(Ref<RibbonGroupsPanel>);
impl IRibbonScrollContent for ScrollView {
    fn extent_width(&self) -> f64 {
        self.0.extent_width()
    }
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
}
