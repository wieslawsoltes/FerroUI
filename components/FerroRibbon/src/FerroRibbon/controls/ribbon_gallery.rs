//! Port of RibbonGallery.cs at RibbonSpace 9755e281. Copyright Wiesław Šoltés, MIT.
use super::{
    RibbonButton, RibbonControlBase, RibbonControlBaseImpl, RibbonControlBaseImplExt,
    RibbonItemHelper, RibbonItemLayout, RibbonKeyTipResult, RibbonObjectArray, RibbonObjectItems,
    RibbonOverflowMenuItems,
};
use crate::{
    metadata::control_items::RibbonControlItems,
    primitives::{RibbonIconPresenter, RibbonPopupPlacement, RibbonUniformGridPanel},
    theming::RibbonTheme,
};
use ferroribbon_core::{
    RibbonItemSize, RibbonString,
    localization::RibbonStrings,
    model::{
        RibbonGalleryItemModelExt, RibbonGalleryItemModelImpl, RibbonItemsSource,
        RibbonNodeModelExt, RibbonNodeModelImpl,
    },
};
use ferroui_base::{
    AttachedProperty, BoxedValue, CornerRadius, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, Thickness, VisualImpl,
    data::{
        core::{ValueType, ValueTypes},
        model::ModelTypes,
    },
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        FocusManager, ICommand, InputElementImpl, Key, KeyModifiers, KeyboardNavigation,
        KeyboardNavigationMode, NavigationMethod,
        navigation::{XYFocus, XYFocusNavigationModes},
    },
    instantiate,
    interactivity::InteractiveImpl,
    layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment},
    media::{FontFamily, FontStyle, FontWeight, IBrush, TextAlignment, TextTrimming, TextWrapping},
    metadata::{MarkupDelegate, into_markup_value},
    reactive::{Disposable, IDisposable},
    styling::{ControlTheme, ThemeVariant},
    threading::{Dispatcher, DispatcherPriority},
    utilities::CompareOptions,
};
use ferroui_controls::{
    Application, Border, Button, ContentControl, Control, ControlImpl, Grid, Panel, RowDefinitions,
    ScrollViewer, StackPanel, TextBlock, TextBox, ToolTip, TopLevel, Viewbox,
    automation::AutomationProperties,
    presenters::ContentPresenter,
    primitives::{
        Popup, ScrollBarVisibility, TemplateAppliedEventArgs, TemplatedControlImpl,
        TemplatedControlImplExt,
    },
    shapes::{Rectangle, Shape},
    templates::IDataTemplate,
};
use ferroui_system::collections::specialized::INotifyCollectionChanged;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
macro_rules! property {
    ($get:ident,$set:ident,$p:ident,$t:ty) => {
        pub fn $get(&self) -> $t {
            self.get_value(Self::$p())
        }
        pub fn $set(&self, v: $t) {
            self.set_value(Self::$p(), v)
        }
    };
}
#[repr(C)]
pub struct RibbonGalleryItem {
    base: FerroObject,
}
ferro_class!(RibbonGalleryItem:FerroObject);
impl FerroObjectImpl for RibbonGalleryItem {}
ferro_class_info!(RibbonGalleryItem{new:RibbonGalleryItem::new,markup:{namespace:"FerroRibbon.Controls"}});
ferro_properties! {impl RibbonGalleryItem {
pub fn label_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Label",None)}
pub fn icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("Icon",None)}
pub fn category_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Category",None)}
pub fn value_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("Value",None)}
pub fn preview_text_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("PreviewText",None)}
pub fn preview_foreground_property()->StyledProperty<Option<Rc<dyn IBrush>>>{FerroProperty::register::<Self,_>("PreviewForeground",None)}
pub fn preview_background_property()->StyledProperty<Option<Rc<dyn IBrush>>>{FerroProperty::register::<Self,_>("PreviewBackground",None)}
pub fn preview_font_family_property()->StyledProperty<Option<FontFamily>>{FerroProperty::register::<Self,_>("PreviewFontFamily",None)}
pub fn preview_font_size_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("PreviewFontSize",14.)}
pub fn preview_font_weight_property()->StyledProperty<FontWeight>{FerroProperty::register::<Self,_>("PreviewFontWeight",FontWeight::Normal)}
pub fn preview_font_style_property()->StyledProperty<FontStyle>{FerroProperty::register::<Self,_>("PreviewFontStyle",FontStyle::Normal)}
}}
impl RibbonGalleryItem {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    property!(label, set_label, label_property, Option<String>);
    property!(icon, set_icon, icon_property, Option<BoxedValue>);
    property!(category, set_category, category_property, Option<String>);
    property!(value, set_item_value, value_property, Option<BoxedValue>);
    property!(
        preview_text,
        set_preview_text,
        preview_text_property,
        Option<String>
    );
    property!(
        preview_foreground,
        set_preview_foreground,
        preview_foreground_property,
        Option<Rc<dyn IBrush>>
    );
    property!(
        preview_background,
        set_preview_background,
        preview_background_property,
        Option<Rc<dyn IBrush>>
    );
    property!(
        preview_font_family,
        set_preview_font_family,
        preview_font_family_property,
        Option<FontFamily>
    );
    property!(
        preview_font_size,
        set_preview_font_size,
        preview_font_size_property,
        f64
    );
    property!(
        preview_font_weight,
        set_preview_font_weight,
        preview_font_weight_property,
        FontWeight
    );
    property!(
        preview_font_style,
        set_preview_font_style,
        preview_font_style_property,
        FontStyle
    );
}
pub struct RibbonGalleryItemEventArgs {
    item: Option<BoxedValue>,
    value: Option<BoxedValue>,
}
impl PartialEq for RibbonGalleryItemEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl RibbonGalleryItemEventArgs {
    pub fn new(item: Option<BoxedValue>, value: Option<BoxedValue>) -> Rc<Self> {
        Rc::new(Self { item, value })
    }
    pub fn item(&self) -> Option<BoxedValue> {
        self.item.clone()
    }
    pub fn value(&self) -> Option<BoxedValue> {
        self.value.clone()
    }
}
ferroui_base::ferro_markup_type!(class RibbonGalleryItemEventArgs{namespace:"FerroRibbon.Controls",this:Rc<RibbonGalleryItemEventArgs>,handles:[RibbonGalleryItemEventArgs,Rc<RibbonGalleryItemEventArgs>,Option<Rc<RibbonGalleryItemEventArgs>>],constructors:[(Option<BoxedValue>,Option<BoxedValue>)=>RibbonGalleryItemEventArgs::new],properties:[Item:Option<BoxedValue>{get:|v:&Rc<RibbonGalleryItemEventArgs>|v.item()},Value:Option<BoxedValue>{get:|v:&Rc<RibbonGalleryItemEventArgs>|v.value()}]});
#[derive(Default)]
struct Parts {
    host: Option<Ref<Border>>,
    root: Option<Ref<Control>>,
    dropdown: Option<Ref<RibbonButton>>,
    up: Option<Ref<Button>>,
    down: Option<Ref<Button>>,
    more: Option<Ref<Button>>,
    subscriptions: Vec<Rc<dyn IDisposable>>,
}
struct Observation {
    value: BoxedValue,
    subscription: Rc<dyn IDisposable>,
}
#[repr(C)]
pub struct RibbonGallery {
    base: RibbonControlBase,
    items: RibbonObjectItems,
    footer: RibbonControlItems,
    inline: Ref<RibbonUniformGridPanel>,
    parts: RefCell<Parts>,
    popup: RefCell<Option<Ref<Popup>>>,
    opener: RefCell<Option<Ref<Control>>>,
    observed: RefCell<Option<Observation>>,
    built: RefCell<Vec<Option<BoxedValue>>>,
    columns: Cell<i32>,
    previewing: Cell<bool>,
    preview_end_pending: Cell<bool>,
    item_click: RefCell<Option<MarkupDelegate>>,
    item_preview: RefCell<Option<MarkupDelegate>>,
    category_selector: RefCell<Option<MarkupDelegate>>,
    items_token: Cell<Option<u64>>,
}
ferro_class!(RibbonGallery:RibbonControlBase);
ferro_impl_classes!(RibbonGallery:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(RibbonGallery{new:RibbonGallery::new,markup:{namespace:"FerroRibbon.Controls",content:Items,properties:[Items:RibbonObjectItems{get:RibbonGallery::items},FooterItems:RibbonControlItems{get:RibbonGallery::footer_items},EffectiveItems:RibbonObjectArray{get:RibbonGallery::effective_items},IsInline:bool{get:RibbonGallery::is_inline},IsDropDownOpen:bool{get:RibbonGallery::is_drop_down_open},FirstRow:i32{get:RibbonGallery::first_row},Columns:i32{get:RibbonGallery::columns},CategorySelector:Option<MarkupDelegate>{get:RibbonGallery::category_selector,set:RibbonGallery::set_category_selector}],methods:[static fn GetSupportsSelectedState(Ref<FerroObject>)->bool=>|e:Ref<FerroObject>|RibbonGallery::get_supports_selected_state(&e),static fn SetSupportsSelectedState(Ref<FerroObject>,bool)=>|e:Ref<FerroObject>,v:bool|RibbonGallery::set_supports_selected_state(&e,v),static fn GetSupportsUnselectedState(Ref<FerroObject>)->bool=>|e:Ref<FerroObject>|RibbonGallery::get_supports_unselected_state(&e),static fn SetSupportsUnselectedState(Ref<FerroObject>,bool)=>|e:Ref<FerroObject>,v:bool|RibbonGallery::set_supports_unselected_state(&e,v),fn ScrollRows(i32)=>RibbonGallery::scroll_rows,fn Pick(Option<BoxedValue>)=>RibbonGallery::pick,fn OpenDropDown()=>RibbonGallery::open_drop_down,fn CloseDropDown()=>RibbonGallery::close_drop_down],events:[ItemClick(Option<BoxedValue>,Rc<RibbonGalleryItemEventArgs>){add:RibbonGallery::add_item_click,remove:RibbonGallery::remove_item_click},ItemPreview(Option<BoxedValue>,Rc<RibbonGalleryItemEventArgs>){add:RibbonGallery::add_item_preview,remove:RibbonGallery::remove_item_preview}]}});
ferro_properties! {impl RibbonGallery {
pub fn supports_selected_state_property()->AttachedProperty<bool>{FerroProperty::register_attached::<Self,FerroObject,_>("SupportsSelectedState",false)}
pub fn supports_unselected_state_property()->AttachedProperty<bool>{FerroProperty::register_attached::<Self,FerroObject,_>("SupportsUnselectedState",false)}
pub fn items_source_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("ItemsSource",None)}
pub fn selected_item_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("SelectedItem",None)}
pub fn item_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<Self,_>("ItemTemplate",None)}
pub fn item_width_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("ItemWidth",72.)}
pub fn item_height_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("ItemHeight",62.)}
pub fn min_columns_property()->StyledProperty<i32>{FerroProperty::register::<Self,_>("MinColumns",3)}
pub fn max_columns_property()->StyledProperty<i32>{FerroProperty::register::<Self,_>("MaxColumns",6)}
pub fn rows_property()->StyledProperty<i32>{FerroProperty::register::<Self,_>("Rows",1)}
pub fn drop_down_columns_property()->StyledProperty<i32>{FerroProperty::register::<Self,_>("DropDownColumns",0)}
pub fn preview_command_property()->StyledProperty<Option<Rc<dyn ICommand>>>{FerroProperty::register::<Self,_>("PreviewCommand",None)}
pub fn is_filter_enabled_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsFilterEnabled",false)}
pub fn show_item_labels_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowItemLabels",true)}
pub fn is_drop_down_only_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsDropDownOnly",false)}
pub fn max_drop_down_height_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("MaxDropDownHeight",460.)}
}}
impl RibbonGallery {
    /// Template-root declaration for the native Selected state. The attached
    /// properties describe these two Gallery states, not a general state system.
    pub fn get_supports_selected_state(element: &FerroObject) -> bool {
        element.get_value(Self::supports_selected_state_property())
    }
    pub fn set_supports_selected_state(element: &FerroObject, value: bool) {
        element.set_value(Self::supports_selected_state_property(), value);
    }
    pub fn get_supports_unselected_state(element: &FerroObject) -> bool {
        element.get_value(Self::supports_unselected_state_property())
    }
    pub fn set_supports_unselected_state(element: &FerroObject, value: bool) {
        element.set_value(Self::supports_unselected_state_property(), value);
    }
    property!(
        items_source,
        set_items_source,
        items_source_property,
        Option<BoxedValue>
    );
    property!(
        selected_item,
        set_selected_item,
        selected_item_property,
        Option<BoxedValue>
    );
    property!(
        item_template,
        set_item_template,
        item_template_property,
        Option<Rc<dyn IDataTemplate>>
    );
    property!(item_width, set_item_width, item_width_property, f64);
    property!(item_height, set_item_height, item_height_property, f64);
    property!(min_columns, set_min_columns, min_columns_property, i32);
    property!(max_columns, set_max_columns, max_columns_property, i32);
    property!(rows, set_rows, rows_property, i32);
    property!(
        drop_down_columns,
        set_drop_down_columns,
        drop_down_columns_property,
        i32
    );
    property!(
        preview_command,
        set_preview_command,
        preview_command_property,
        Option<Rc<dyn ICommand>>
    );
    property!(
        is_filter_enabled,
        set_is_filter_enabled,
        is_filter_enabled_property,
        bool
    );
    property!(
        show_item_labels,
        set_show_item_labels,
        show_item_labels_property,
        bool
    );
    property!(
        is_drop_down_only,
        set_is_drop_down_only,
        is_drop_down_only_property,
        bool
    );
    property!(
        max_drop_down_height,
        set_max_drop_down_height,
        max_drop_down_height_property,
        f64
    );
    pub fn construct() -> Self {
        Self {
            base: RibbonControlBase::construct(),
            items: RibbonObjectItems::new_managed(),
            footer: RibbonControlItems::new_managed(),
            inline: RibbonUniformGridPanel::new(),
            parts: RefCell::default(),
            popup: RefCell::default(),
            opener: RefCell::default(),
            observed: RefCell::default(),
            built: RefCell::default(),
            columns: Cell::new(6),
            previewing: Cell::new(false),
            preview_end_pending: Cell::new(false),
            item_click: RefCell::default(),
            item_preview: RefCell::default(),
            category_selector: RefCell::default(),
            items_token: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items(&self) -> RibbonObjectItems {
        self.items.clone()
    }
    pub fn footer_items(&self) -> RibbonControlItems {
        self.footer.clone()
    }
    pub fn is_inline(&self) -> bool {
        !self.is_drop_down_only()
            && self.current_size() != RibbonItemSize::Small
            && !self.is_simplified()
    }
    pub fn is_drop_down_open(&self) -> bool {
        self.popup.borrow().as_ref().is_some_and(|p| p.is_open())
    }
    pub fn first_row(&self) -> i32 {
        self.inline.first_row()
    }
    pub fn columns(&self) -> i32 {
        self.columns.get()
    }
    pub fn category_selector(&self) -> Option<MarkupDelegate> {
        self.category_selector.borrow().clone()
    }
    pub fn set_category_selector(&self, value: Option<MarkupDelegate>) {
        self.category_selector.replace(value);
    }
    fn raise(&self, event: &RefCell<Option<MarkupDelegate>>, e: Rc<RibbonGalleryItemEventArgs>) {
        let h = event.borrow().clone();
        if let Some(h) = h {
            h.try_invoke(&[into_markup_value(self.to_ref()), Some(Rc::new(e))])
                .expect("gallery event handler");
        }
    }
    pub fn add_item_click(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.item_click.borrow().as_ref(), h.as_ref());
        self.item_click.replace(next);
    }
    pub fn remove_item_click(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.item_click.borrow().as_ref(), h.as_ref());
        self.item_click.replace(next);
    }
    pub fn add_item_preview(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.item_preview.borrow().as_ref(), h.as_ref());
        self.item_preview.replace(next);
    }
    pub fn remove_item_preview(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.item_preview.borrow().as_ref(), h.as_ref());
        self.item_preview.replace(next);
    }
    fn own_items(&self) -> Vec<Option<BoxedValue>> {
        self.items
            .iter()
            .map(|v| {
                crate::metadata::model_objects::to_native(v).expect("Gallery item native payload")
            })
            .collect()
    }
    pub fn effective_items(&self) -> RibbonObjectArray {
        let values = match self.items_source() {
            None => self.own_items(),
            Some(value) => {
                if value.downcast_ref::<String>().is_some()
                    || value.downcast_ref::<&'static str>().is_some()
                {
                    self.own_items()
                } else if let Some(items) = value.downcast_ref::<RibbonObjectItems>() {
                    items
                        .iter()
                        .map(|v| {
                            crate::metadata::model_objects::to_native(v)
                                .expect("Gallery native item")
                        })
                        .collect()
                } else if let Some(items) = value.downcast_ref::<RibbonObjectArray>() {
                    items.to_vec()
                } else if let Some(items) = ValueTypes::try_cast(
                    &value,
                    ValueType::of::<crate::metadata::combo_items::RibbonObjectEnumerable>(),
                )
                .and_then(|v| {
                    v.downcast_ref::<crate::metadata::combo_items::RibbonObjectEnumerable>()
                        .cloned()
                }) {
                    items.iter().collect()
                } else if let Some(source) = ValueTypes::try_convert(
                    Some(&value),
                    ValueType::of::<ferroui_controls::ItemsSource>(),
                )
                .flatten()
                {
                    source
                        .downcast_ref::<ferroui_controls::ItemsSource>()
                        .expect("IEnumerable conversion")
                        .to_vec()
                } else {
                    self.own_items()
                }
            }
        };
        super::ribbon_combo_box::object_array(values)
    }
    fn observe_source(&self, detach: bool) {
        let source = if detach || !self.is_loaded() {
            None
        } else {
            self.items_source()
        };
        if source
            .as_ref()
            .zip(self.observed.borrow().as_ref())
            .is_some_and(|(a, b)| Rc::ptr_eq(a, &b.value))
        {
            return;
        }
        let old = self.observed.take();
        if let Some(old) = old {
            old.subscription.dispose();
        }
        let Some(value) = source else { return };
        if let Some(model) = ModelTypes::find(&*value)
            && let Some(notify) = model.as_notify_collection_changed(&*value)
        {
            let weak = self.to_ref().downgrade();
            let token = notify.collection_changed().add(Rc::new(move |_| {
                if let Some(g) = weak.upgrade() {
                    g.rebuild();
                }
            }));
            let retained = value.clone();
            let sub = Disposable::create(move || {
                model
                    .as_notify_collection_changed(&*retained)
                    .expect("retained notification interface")
                    .collection_changed()
                    .remove(token);
            });
            self.observed.replace(Some(Observation {
                value,
                subscription: sub,
            }));
        } else if let Some(source) = ValueTypes::try_convert(
            Some(&value),
            ValueType::of::<ferroui_controls::ItemsSource>(),
        )
        .flatten()
        {
            let source = source
                .downcast_ref::<ferroui_controls::ItemsSource>()
                .expect("registered source")
                .clone();
            let weak = self.to_ref().downgrade();
            if let Some(token) = source.list().add_collection_changed(Rc::new(move |_| {
                if let Some(g) = weak.upgrade() {
                    g.rebuild();
                }
            })) {
                let sub =
                    Disposable::create(move || source.list().remove_collection_changed(token));
                self.observed.replace(Some(Observation {
                    value,
                    subscription: sub,
                }));
            }
        }
    }
    fn apply_presentation(&self) {
        self.columns
            .set(1.max(if self.current_size() == RibbonItemSize::Medium {
                self.min_columns()
            } else {
                self.max_columns()
            }));
        let inline = self.is_inline();
        let (root, dropdown) = {
            let p = self.parts.borrow();
            (p.root.clone(), p.dropdown.clone())
        };
        if let Some(root) = root {
            root.set_is_visible(inline);
        }
        if let Some(button) = dropdown {
            button.set_is_visible(!inline);
            button.set_label(self.label());
            button.set_icon(self.icon().or_else(|| Some(Rc::new("\u{e790}".to_owned()))));
            let size = if self.is_simplified() {
                RibbonItemSize::Small
            } else if self.current_size() == RibbonItemSize::Medium && self.is_drop_down_only() {
                RibbonItemSize::Medium
            } else {
                RibbonItemSize::Large
            };
            let mut layout = RibbonItemLayout::new(size, self.metrics());
            layout.is_simplified = self.is_simplified();
            layout.show_label = if self.is_simplified() {
                Some(self.show_label_in_simplified_mode())
            } else if size == RibbonItemSize::Medium {
                Some(true)
            } else {
                None
            };
            button.apply_layout(layout);
        }
        let rows = 1.max(self.rows());
        let available = ferroui_base::utilities::math_utilities::max(
            24.,
            self.metrics()
                .expect("NullReference: Gallery Metrics")
                .group_content_height()
                - 4.,
        );
        let height = ferroui_base::utilities::math_utilities::min(
            self.item_height(),
            (available - ((rows - 1) as f64 * 2.)) / rows as f64,
        );
        self.inline.set_columns(self.columns());
        self.inline.set_item_width(self.item_width());
        self.inline.set_item_height(height);
        self.inline.set_max_rows(rows);
        self.clamp_first_row();
        self.update_scroll_buttons();
    }
    fn release_elements(panel: &Panel) {
        for child in panel.children().to_vec() {
            if let Some(content) = child.cast::<ContentControl>()
                && let Some(visual) = content
                    .content()
                    .as_ref()
                    .and_then(Control::from_boxed)
                    .and_then(|c| c.cast::<Panel>())
            {
                for child in visual.children().to_vec() {
                    if let Some(host) = child.cast::<Viewbox>()
                        && host
                            .tag()
                            .as_ref()
                            .is_some_and(|t| Rc::ptr_eq(t, &element_host_tag()))
                    {
                        host.set_child(None);
                    }
                }
            }
        }
    }
    fn rebuild(&self) {
        Self::release_elements(&self.inline);
        self.inline.children().clear();
        let built = self.effective_items().to_vec();
        self.built.replace(built.clone());
        for item in built {
            self.inline
                .children()
                .add(self.create_container(item, false));
        }
        self.apply_presentation();
        self.update_selection();
        RibbonItemHelper::invalidate_host_layout(self);
    }
    fn clamp_first_row(&self) {
        let max_first = 0.max(self.inline.row_count().wrapping_sub(1.max(self.rows())));
        self.inline
            .set_first_row(self.inline.first_row().clamp(0, max_first));
        self.inline.invalidate_arrange();
    }
    fn update_scroll_buttons(&self) {
        let max_first = 0.max(self.inline.row_count().wrapping_sub(1.max(self.rows())));
        let (up, down) = {
            let p = self.parts.borrow();
            (p.up.clone(), p.down.clone())
        };
        if let Some(up) = up {
            up.set_is_enabled(self.inline.first_row() > 0);
        }
        if let Some(down) = down {
            down.set_is_enabled(self.inline.first_row() < max_first);
        }
    }
    pub fn scroll_rows(&self, delta: i32) {
        self.inline
            .set_first_row(self.inline.first_row().wrapping_add(delta));
        self.clamp_first_row();
        self.update_scroll_buttons();
    }
    fn item_value(item: &Option<BoxedValue>) -> Option<BoxedValue> {
        gallery_item(item)
            .and_then(|g| g.value())
            .or_else(|| item.clone())
    }
    pub fn pick(&self, item: Option<BoxedValue>) {
        self.preview(None);
        let value = Self::item_value(&item);
        self.set_selected_item(value.clone());
        self.raise(
            &self.item_click,
            RibbonGalleryItemEventArgs::new(item, value.clone()),
        );
        self.execute_command(value).expect("Gallery ExecuteCommand");
        if self.is_drop_down_open() {
            self.close_with_focus(true);
        }
    }
    fn preview(&self, item: Option<BoxedValue>) {
        if item.is_none() && !self.previewing.get() {
            return;
        }
        self.previewing.set(item.is_some());
        let value = Self::item_value(&item);
        self.raise(
            &self.item_preview,
            RibbonGalleryItemEventArgs::new(item, value.clone()),
        );
        if let Some(command) = self.preview_command()
            && command.can_execute(value.as_ref())
        {
            command.execute(value.as_ref());
        }
    }
    fn is_selected(&self, item: &Option<BoxedValue>) -> bool {
        if item.is_none() {
            return false;
        }
        let selected = self.selected_item();
        selected_equals(item, &selected)
            || gallery_item(item)
                .and_then(|g| g.value())
                .is_some_and(|v| selected_equals(&Some(v), &selected))
    }
    fn update_selection(&self) {
        // Pinned Uno crossruntime UIElementCollection uses List<T>.Enumerator:
        // a reentrant visual/equality callback must not continue stale children.
        let children = self.inline.children();
        let snapshot = children.snapshot();
        for child in snapshot.iter() {
            assert!(
                Rc::ptr_eq(&snapshot, &children.snapshot()),
                "InvalidOperation: Gallery children changed during enumeration"
            );
            if let Some(b) = child.cast::<Button>() {
                self.apply_selection(&b);
            }
        }
        assert!(
            Rc::ptr_eq(&snapshot, &children.snapshot()),
            "InvalidOperation: Gallery children changed during enumeration"
        );
        let popup = self.popup.borrow().clone();
        if let Some(popup) = popup
            && popup.is_open()
            && let Some(root) = popup.child()
        {
            for child in descendants(root) {
                if let Some(b) = child.cast::<Button>()
                    && b.tag().is_some()
                {
                    self.apply_selection(&b);
                }
            }
        }
    }
    fn apply_selection(&self, button: &Button) {
        let selected = self.is_selected(&button.tag());
        // Resolve state support before activation: native style callbacks can
        // replace the template, just as source state callbacks can reenter.
        let root = button.visual_children().snapshot().first().cloned();
        let supported = root.as_ref().is_some_and(|root| {
            if selected {
                Self::get_supports_selected_state(root)
            } else {
                Self::get_supports_unselected_state(root)
            }
        });
        if supported {
            let root = root.as_ref().unwrap();
            // The two native Gallery states belong to this template instance.
            // A failed request must leave its prior state intact. A new root
            // starts with neither state, independently of model selection.
            let (previous, next) = if selected {
                (":ribbon-gallery-unselected", ":ribbon-gallery-selected")
            } else {
                (":ribbon-gallery-selected", ":ribbon-gallery-unselected")
            };
            root.pseudo_classes().set(previous, false);
            root.pseudo_classes().set(next, true);
        }
        // Existing native extension styles may also observe the data signal.
        button.pseudo_classes().set(":ribbon-selected", selected);
        if supported || !button.is_loaded() {
            return;
        }
        let brush = if selected {
            RibbonTheme::get_brush(Some(button), Some("RibbonGalleryItemSelectedBorderBrush"))
                .expect("Gallery selection border")
                .map(Rc::<dyn IBrush>::from)
        } else {
            Some(
                ferroui_base::media::SolidColorBrush::with_color(
                    ferroui_base::media::Colors::TRANSPARENT,
                )
                .into(),
            )
        };
        button.set_border_brush(brush);
    }
    fn create_container(&self, item: Option<BoxedValue>, popup: bool) -> Ref<Button> {
        let host_elements = !popup && RibbonItemHelper::get_source_item(self).is_none();
        let button = Button::new();
        button.set_tag(item.clone());
        button.set_content(Some(Control::boxed(
            self.create_item_visual(item.clone(), host_elements),
        )));
        button.set_horizontal_alignment(HorizontalAlignment::Stretch);
        button.set_vertical_alignment(VerticalAlignment::Stretch);
        let app = Application::current().expect("NullReference: Application.Current");
        if let Some(style) = super::ribbon_combo_box::source_dictionary_resource(
            &app.resources(),
            &"RibbonGalleryItemStyle".into(),
            app.actual_theme_variant().as_ref(),
        ) {
            button.set_theme(style.map(|s| {
                ValueTypes::as_object(&*s)
                    .and_then(|o| o.cast::<ControlTheme>())
                    .expect("GalleryItemStyle must be ControlTheme")
            }));
        }
        if popup {
            button.set_width(self.item_width());
            button.set_height(self.item_height());
        }
        let label = item_label(&item);
        AutomationProperties::set_name(&button, Some(label.as_deref().unwrap_or("")));
        if label.as_ref().is_some_and(|s| !s.is_empty()) {
            ToolTip::set_tip(&button, label.map(|s| Rc::new(s) as BoxedValue));
        }
        let weak = self.to_ref().downgrade();
        let data = item.clone();
        button.click(move |_, _| {
            if let Some(g) = weak.upgrade() {
                g.pick(data.clone());
            }
        });
        let weak = self.to_ref().downgrade();
        let data = item.clone();
        button.pointer_entered(move |_, _| {
            if let Some(g) = weak.upgrade() {
                g.preview(data.clone());
            }
        });
        let weak = self.to_ref().downgrade();
        button.pointer_exited(move |_, _| {
            if let Some(g) = weak.upgrade() {
                g.preview(None);
            }
        });
        let weak = self.to_ref().downgrade();
        let data = item;
        button.got_focus(move |_, e| {
            if let Some(g) = weak.upgrade() {
                g.preview_end_pending.set(false);
                if matches!(
                    e.navigation_method,
                    NavigationMethod::Tab | NavigationMethod::Directional
                ) {
                    g.preview(data.clone());
                }
            }
        });
        let weak = self.to_ref().downgrade();
        button.lost_focus(move |_, _| {
            if let Some(g) = weak.upgrade() {
                g.preview_end_pending.set(true);
                Dispatcher::ui_thread().post_local(
                    move || {
                        if g.preview_end_pending.replace(false) {
                            g.preview(None);
                        }
                    },
                    DispatcherPriority::NORMAL,
                );
            }
        });
        let weak = self.to_ref().downgrade();
        button.loaded(move |sender, _| {
            if let Some(g) = weak.upgrade() {
                g.apply_selection(&sender.to_ref().cast::<Button>().expect("loaded button"));
            }
        });
        button
    }
    fn create_item_visual(&self, item: Option<BoxedValue>, host_elements: bool) -> Ref<Control> {
        if let Some(element) = item.as_ref().and_then(Control::from_boxed) {
            if host_elements && element.visual_parent().is_none() && element.parent().is_none() {
                let view = Viewbox::new();
                view.set_child(element);
                view.set_tag(Some(element_host_tag()));
                let grid = Grid::new();
                grid.children().add(view);
                return grid.upcast();
            }
            let text = TextBlock::new();
            text.set_text(
                Some(item_label(&item).unwrap_or_else(|| element.get_type().name().to_owned()))
                    .as_deref(),
            );
            center_text(&text);
            text.set_text_wrapping(TextWrapping::Wrap);
            text.set_text_alignment(TextAlignment::Center);
            text.set_margin(Thickness::uniform(2.));
            return text.upcast();
        }
        if let Some(template) = self.item_template() {
            let presenter = ContentPresenter::new();
            presenter.set_content(Self::item_value(&item));
            presenter.set_content_template(Some(template));
            presenter.set_horizontal_alignment(HorizontalAlignment::Stretch);
            presenter.set_vertical_alignment(VerticalAlignment::Stretch);
            return presenter.upcast();
        }
        let grid = Grid::new();
        grid.set_row_definitions(RowDefinitions::parse("*,Auto").expect("source rows"));
        let preview = if let Some(g) = gallery_item(&item) {
            self.add_label(&grid, g.label());
            create_preview(
                g.preview_text(),
                g.icon(),
                g.preview_background(),
                g.preview_foreground(),
                g.preview_font_family(),
                g.preview_font_size(),
                g.preview_font_weight(),
                g.preview_font_style(),
            )
        } else if let Some(g) = gallery_model(&item) {
            // Source `Content as string` accepts string content only. Inspect
            // the existing payload without requiring unrelated object types to
            // be registered with the native value boxer.
            let text = g.content().and_then(|v| {
                v.downcast_ref::<String>()
                    .cloned()
                    .or_else(|| core_text(v.downcast_ref::<RibbonString>().cloned()))
            });
            self.add_label(&grid, core_text(g.label()));
            create_preview(
                text,
                g.icon().map(|i| Rc::new(i) as BoxedValue),
                None,
                None,
                None,
                14.,
                FontWeight::Normal,
                FontStyle::Normal,
            )
        } else {
            let text = TextBlock::new();
            text.set_text(item_label(&item).as_deref());
            center_text(&text);
            text.set_text_wrapping(TextWrapping::Wrap);
            text.upcast()
        };
        grid.children().insert(0, preview);
        grid.upcast()
    }
    fn add_label(&self, grid: &Grid, label: Option<String>) {
        if !self.show_item_labels() || label.as_ref().is_none_or(String::is_empty) {
            return;
        }
        let text = TextBlock::new();
        text.set_text(label.as_deref());
        text.set_font_size(11.);
        text.set_horizontal_alignment(HorizontalAlignment::Center);
        text.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        text.set_margin(Thickness::new(2., 0., 2., 2.));
        Grid::set_row(&text, 1);
        grid.children().add(text);
    }
    fn category(&self, item: &Option<BoxedValue>) -> Option<String> {
        if let Some(selector) = self.category_selector() {
            let value = selector
                .try_invoke(std::slice::from_ref(item))
                .expect("CategorySelector");
            if let Some(v) = value {
                if let Some(text) = v.downcast_ref::<String>() {
                    return Some(text.clone());
                }
                if let Some(text) = v.downcast_ref::<Option<String>>() {
                    if text.is_some() {
                        return text.clone();
                    }
                } else {
                    panic!("CategorySelector returns string or null");
                }
            }
        }
        gallery_item(item)
            .and_then(|g| g.category())
            .or_else(|| gallery_model(item).and_then(|g| core_text(g.category())))
    }
    fn fill_popup(&self, items_host: &StackPanel, query: Option<&str>, columns: i32) {
        items_host.children().clear();
        let blank = query.is_none_or(|q| {
            ferroribbon_core::text::trim(&q.encode_utf16().collect::<Vec<_>>()).is_empty()
        });
        let mut groups: Vec<(Option<String>, Vec<Option<BoxedValue>>)> = Vec::new();
        for item in self.effective_items().iter() {
            if !blank && !filter_matches(&item, query.unwrap()) {
                continue;
            }
            let category = self.category(&item);
            if let Some(group) = groups.iter_mut().find(|(c, _)| c == &category) {
                group.1.push(item);
            } else {
                groups.push((category, vec![item]));
            }
        }
        for (category, items) in groups {
            if category.as_ref().is_some_and(|s| !s.is_empty()) {
                let text = TextBlock::new();
                text.set_text(category.as_deref());
                text.set_font_weight(FontWeight::SemiBold);
                text.set_font_size(12.);
                text.set_margin(Thickness::new(4., 6., 4., 2.));
                items_host.children().add(text);
            }
            let panel = RibbonUniformGridPanel::new();
            panel.set_columns(columns);
            panel.set_item_width(self.item_width());
            panel.set_item_height(self.item_height());
            panel.set_spacing(2.);
            for item in items {
                panel.children().add(self.create_container(item, true));
            }
            items_host.children().add(panel);
        }
    }
    pub fn open_drop_down(&self) {
        if TopLevel::get_top_level(Some(self)).is_none() || self.is_drop_down_open() {
            return;
        }
        self.opener.replace(None);
        let columns = if self.drop_down_columns() > 0 {
            self.drop_down_columns()
        } else {
            self.max_columns().max(5)
        };
        let root = StackPanel::new();
        root.set_spacing(4.);
        let padding = Border::new();
        padding.set_padding(Thickness::uniform(4.));
        padding.set_child(root.clone());
        let items_host = StackPanel::new();
        items_host.set_spacing(2.);
        XYFocus::set_navigation_modes(&items_host, XYFocusNavigationModes::ENABLED);
        KeyboardNavigation::set_tab_navigation(&items_host, KeyboardNavigationMode::Once);
        let filter = if self.is_filter_enabled() {
            let filter = TextBox::new();
            filter.set_placeholder_text(
                core_text(RibbonStrings::current().gallery_filter()).as_deref(),
            );
            filter.set_margin(Thickness::new(2., 2., 2., 4.));
            root.children().add(filter.clone());
            Some(filter)
        } else {
            None
        };
        self.fill_popup(&items_host, None, columns);
        if let Some(filter) = &filter {
            let weak = self.to_ref().downgrade();
            let host = items_host.downgrade();
            let edit = filter.downgrade();
            filter.text_changed(move |_, _| {
                if let (Some(g), Some(host), Some(edit)) =
                    (weak.upgrade(), host.upgrade(), edit.upgrade())
                {
                    g.fill_popup(&host, edit.text().as_deref(), columns);
                }
            });
        }
        let scroll = ScrollViewer::new();
        scroll.set_content(Some(Control::boxed(items_host.clone())));
        scroll.set_max_height(self.max_drop_down_height());
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        root.children().add(scroll);
        if self.footer.count() > 0 {
            let separator = Rectangle::new();
            separator.set_height(1.);
            separator.set_margin(Thickness::new(0., 2., 0., 2.));
            RibbonTheme::set_theme_brush(
                Some(&separator),
                Some(Shape::fill_property().as_property()),
                Some("RibbonSeparatorBrush"),
            )
            .expect("Gallery separator");
            root.children().add(separator);
            for footer in self.footer.iter() {
                let footer = footer.expect("NullReference: Gallery footer");
                if let Some(parent) = footer.visual_parent().and_then(|p| p.cast::<Panel>()) {
                    parent.children().remove(&footer);
                }
                if let Some(item) = ValueTypes::try_cast(
                    &Control::boxed(footer.clone()),
                    ValueType::of::<Rc<dyn super::IRibbonItem>>(),
                )
                .and_then(|v| v.downcast_ref::<Rc<dyn super::IRibbonItem>>().cloned())
                {
                    let mut layout = RibbonItemLayout::new(RibbonItemSize::Medium, self.metrics());
                    layout.show_label = Some(true);
                    item.apply_layout(layout);
                }
                footer.set_horizontal_alignment(HorizontalAlignment::Stretch);
                RibbonItemHelper::set_owner(&footer, RibbonItemHelper::find_owner(Some(self)))
                    .expect("Gallery footer owner");
                root.children().add(footer);
            }
        }
        let chrome = Border::new();
        chrome.set_child(padding);
        chrome.set_border_thickness(Thickness::uniform(1.));
        chrome.set_corner_radius(CornerRadius::uniform(6.));
        chrome.set_value(
            ThemeVariant::requested_theme_variant_property(),
            self.actual_theme_variant(),
        );
        RibbonTheme::set_theme_brush(
            Some(&chrome),
            Some(Border::background_property().as_property()),
            Some("RibbonPopupBackgroundBrush"),
        )
        .expect("Gallery popup background");
        RibbonTheme::set_theme_brush(
            Some(&chrome),
            Some(Border::border_brush_property().as_property()),
            Some("RibbonPopupBorderBrush"),
        )
        .expect("Gallery popup border");
        let weak = self.to_ref().downgrade();
        chrome.key_down(move |_, e| {
            if e.key == Key::Escape
                && let Some(g) = weak.upgrade()
            {
                g.close_with_focus(true);
                e.set_handled(true);
            }
        });
        let popup = Popup::new();
        popup.set_child(chrome);
        popup.set_is_light_dismiss_enabled(true);
        popup.set_popup_parent(Some(&self.to_ref().upcast::<Control>()));
        let weak = self.to_ref().downgrade();
        let weak_root = root.downgrade();
        popup
            .closed(move || {
                if let Some(g) = weak.upgrade() {
                    g.preview_end_pending.set(false);
                    g.preview(None);
                }
                if let Some(root) = weak_root.upgrade() {
                    root.children().clear();
                }
            })
            .detach();
        self.popup.replace(Some(popup.clone()));
        let anchor = {
            let p = self.parts.borrow();
            if self.is_inline() {
                p.root.clone()
            } else {
                p.dropdown.clone().map(|b| b.upcast())
            }
        }
        .unwrap_or_else(|| self.to_ref().upcast());
        RibbonPopupPlacement::place_below_with_options(&popup, &anchor, self.is_inline(), 0.);
        popup.set_is_open(true);
        let buttons = descendants(items_host.upcast())
            .filter_map(|c| c.cast::<Button>())
            .filter(|b| b.tag().is_some())
            .collect::<Vec<_>>();
        let initial = buttons
            .iter()
            .find(|b| self.is_selected(&b.tag()))
            .or_else(|| buttons.first())
            .cloned();
        let g = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                if g.is_drop_down_open()
                    && !initial.is_some_and(|b| {
                        b.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE)
                    })
                    && let Some(filter) = filter
                {
                    filter.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE);
                }
            },
            DispatcherPriority::NORMAL,
        );
    }
    pub fn close_drop_down(&self) {
        self.close_with_focus(false);
    }
    fn close_with_focus(&self, restore: bool) {
        let popup = self.popup.borrow().clone();
        let Some(popup) = popup.filter(|p| p.is_open()) else {
            return;
        };
        let navigation = FocusManager::get_focus_manager(self)
            .and_then(|f| f.get_focused_element())
            .filter(|e| e.pseudo_classes().contains(":focus-visible"))
            .map_or(NavigationMethod::Unspecified, |_| NavigationMethod::Tab);
        popup.set_is_open(false);
        if !restore {
            return;
        }
        let (more, dropdown) = {
            let p = self.parts.borrow();
            (p.more.clone(), p.dropdown.clone())
        };
        let fallback = if self.is_inline() {
            self.inline
                .children()
                .to_vec()
                .into_iter()
                .filter_map(|c| c.cast::<Button>())
                .find(|b| self.is_selected(&b.tag()))
                .or_else(|| {
                    self.inline
                        .children()
                        .to_vec()
                        .first()
                        .and_then(|c| c.cast::<Button>())
                })
        } else {
            None
        };
        let targets = [
            self.opener.borrow().clone(),
            if self.is_inline() {
                more.map(|b| b.upcast())
            } else {
                dropdown.map(|b| b.upcast())
            },
            fallback.map(|b| b.upcast()),
        ];
        for target in targets.into_iter().flatten() {
            if target.is_tab_stop()
                && target.is_enabled()
                && target.focus_with(navigation, KeyModifiers::NONE)
            {
                break;
            }
        }
    }
    fn defer_open(&self, opener: Ref<Control>) {
        let g = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                g.open_drop_down();
                g.opener.replace(Some(opener));
            },
            DispatcherPriority::NORMAL,
        );
    }
}
struct ElementHostTag;
impl PartialEq for ElementHostTag {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
fn element_host_tag() -> BoxedValue {
    ferroui_base::ferro_thread_local! {static TAG:BoxedValue=Rc::new(ElementHostTag);}
    TAG.with(Clone::clone)
}
fn selected_equals(a: &Option<BoxedValue>, b: &Option<BoxedValue>) -> bool {
    if let (Some(a), Some(b)) = (
        crate::commands::command_parameter(a.as_ref()),
        crate::commands::command_parameter(b.as_ref()),
    ) && Rc::ptr_eq(&a, &b)
    {
        return true;
    }
    if let (Some(a), Some(b)) = (a, b)
        && let (Some(a), Some(b)) = (ValueTypes::as_object(&**a), ValueTypes::as_object(&**b))
        && a.ptr_eq(&b)
    {
        return true;
    }
    object_equals(a, b)
}
fn object_equals(a: &Option<BoxedValue>, b: &Option<BoxedValue>) -> bool {
    crate::metadata::model_objects::from_native(a.clone())
        == crate::metadata::model_objects::from_native(b.clone())
}
fn filter_matches(item: &Option<BoxedValue>, query: &str) -> bool {
    // The source evaluates the virtual item label before resolving current
    // culture. Do not substitute the framework's provider-less ordinal fallback.
    let label = item_label(item).unwrap_or_default();
    if let Some(rules) = crate::search::current_compare_rules() {
        return rules.index_of(&label, query, CompareOptions::IGNORE_CASE) >= 0;
    }
    #[cfg(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "emscripten",
        target_os = "linux"
    ))]
    {
        let collator = ferroribbon_platform::SearchLabelCollator::new(
            ferroui_base::utilities::CultureInfo::current_formatting().name(),
        )
        .expect("native gallery filter locale");
        collator
            .contains(
                &label.encode_utf16().collect::<Vec<_>>(),
                &query.encode_utf16().collect::<Vec<_>>(),
            )
            .expect("native gallery label matching")
    }
    #[cfg(not(any(
        target_os = "macos",
        target_os = "windows",
        target_os = "emscripten",
        target_os = "linux"
    )))]
    panic!(
        "Unavailable: faithful Gallery current-culture filtering requires framework culture data on this platform"
    )
}
fn core_text(value: Option<RibbonString>) -> Option<String> {
    value.map(|s| s.to_utf8().expect("Gallery native UTF16 boundary"))
}
fn gallery_item(item: &Option<BoxedValue>) -> Option<Ref<RibbonGalleryItem>> {
    item.as_ref()
        .and_then(|v| ValueTypes::as_object(&**v))
        .and_then(|v| v.cast())
}
fn core_model<T: ?Sized + ferroribbon_core::model::RibbonModelView>(
    item: &Option<BoxedValue>,
) -> Option<Rc<T>> {
    let value = crate::commands::command_parameter(item.as_ref())?;
    if !ferroribbon_core::model::is_registered_model_type(value.as_ref().type_id()) {
        return None;
    }
    ferroribbon_core::model::try_cast_model::<T>(Some(value)).expect("declared model hierarchy")
}
fn gallery_model(item: &Option<BoxedValue>) -> Option<Rc<dyn RibbonGalleryItemModelImpl>> {
    core_model::<dyn RibbonGalleryItemModelImpl>(item)
}
fn item_label(item: &Option<BoxedValue>) -> Option<String> {
    if let Some(g) = gallery_item(item) {
        return g.label();
    }
    let value = item.as_ref().expect("NullReference: gallery item");
    if let Some(node) = core_model::<dyn RibbonNodeModelImpl>(item) {
        return core_text(node.label());
    }
    if let Some(element) = Control::from_boxed(value) {
        return AutomationProperties::get_name(&element)
            .filter(|s| !s.is_empty())
            .or_else(|| element.name().filter(|s| !s.is_empty()));
    }
    ValueTypes::try_convert_with_culture(
        Some(value),
        ValueType::of::<String>(),
        &ferroui_base::utilities::CultureInfo::current_formatting(),
    )
    .flatten()
    .and_then(|v| v.downcast_ref::<String>().cloned())
    .or_else(|| Some(ValueTypes::to_display_string(Some(value))))
}
fn center_text(text: &TextBlock) {
    text.set_horizontal_alignment(HorizontalAlignment::Center);
    text.set_vertical_alignment(VerticalAlignment::Center);
}
#[allow(clippy::too_many_arguments)]
fn create_preview(
    text: Option<String>,
    icon: Option<BoxedValue>,
    background: Option<Rc<dyn IBrush>>,
    foreground: Option<Rc<dyn IBrush>>,
    family: Option<FontFamily>,
    size: f64,
    weight: FontWeight,
    style: FontStyle,
) -> Ref<Control> {
    let border = Border::new();
    border.set_background(background);
    border.set_corner_radius(CornerRadius::uniform(3.));
    border.set_margin(Thickness::uniform(3.));
    if let Some(text) = text.filter(|s| !s.is_empty()) {
        let tb = TextBlock::new();
        tb.set_text(Some(&text));
        tb.set_font_size(size);
        tb.set_font_weight(weight);
        tb.set_font_style(style);
        center_text(&tb);
        // Source Clip is a local preview request, so application text styles
        // must not replace it with ellipses or disable the visual clipping.
        tb.set_text_trimming(<dyn TextTrimming>::none());
        tb.set_clip_to_bounds(true);
        if let Some(family) = family {
            tb.set_font_family(family);
        }
        if let Some(foreground) = foreground {
            tb.set_foreground(Some(foreground));
        }
        border.set_child(tb);
    } else if let Some(icon) = icon {
        let presenter = RibbonIconPresenter::new();
        presenter.set_icon(Some(icon));
        presenter.set_icon_size(28.);
        if let Some(foreground) = foreground {
            presenter.set_foreground(Some(foreground));
        } else {
            RibbonTheme::set_theme_brush(
                Some(&presenter),
                Some(RibbonIconPresenter::foreground_property().as_property()),
                Some("RibbonIconBrush"),
            )
            .expect("Gallery icon foreground");
        }
        border.set_child(presenter);
    }
    border.upcast()
}
// Preserve the source iterator's suspension before visiting each node's children.
// Materializing the complete tree here would hide mutations made after a yield.
struct Descendants {
    stack: Vec<Ref<Control>>,
    pending: Option<Ref<Control>>,
}
impl Iterator for Descendants {
    type Item = Ref<Control>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current) = self.pending.take() {
            if let Some(panel) = current.cast::<Panel>() {
                self.stack.extend(panel.children().to_vec());
            } else if let Some(border) = current.cast::<Border>() {
                if let Some(child) = border.child() {
                    self.stack.push(child);
                }
            } else if let Some(scroll) = current.cast::<ScrollViewer>()
                && let Some(content) = scroll.content().as_ref().and_then(Control::from_boxed)
            {
                self.stack.push(content);
            }
        }
        let current = self.stack.pop()?;
        self.pending = Some(current.clone());
        Some(current)
    }
}
fn descendants(root: Ref<Control>) -> Descendants {
    Descendants {
        stack: vec![root],
        pending: None,
    }
}
impl FerroObjectImpl for RibbonGallery {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_size(RibbonItemSize::Large);
        this.set_is_tab_stop(false);
        let weak = this.to_ref().downgrade();
        let token = this.items.collection_changed().add(Rc::new(move |_| {
            if let Some(g) = weak.upgrade() {
                g.rebuild();
            }
        }));
        this.items_token.set(Some(token));
        let weak = this.to_ref().downgrade();
        this.loaded(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.observe_source(false);
                let built = this.built.borrow().clone();
                let values = this.effective_items().to_vec();
                if built.len() != values.len()
                    || !built
                        .iter()
                        .zip(values.iter())
                        .all(|(a, b)| object_equals(a, b))
                {
                    this.rebuild();
                }
            }
        });
        let weak = this.to_ref().downgrade();
        this.unloaded(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.close_drop_down();
                this.observe_source(true);
            }
        });
    }
    fn on_property_changed(this: &Self, e: &ferroui_base::FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, e);
        let p = e.property();
        if p == Self::items_source_property().as_property() {
            this.observe_source(false);
            this.rebuild();
        } else if p == Self::selected_item_property().as_property() {
            this.update_selection();
        } else if p == Self::item_template_property().as_property()
            || p == Self::show_item_labels_property().as_property()
        {
            this.rebuild();
        } else if [
            Self::item_width_property().as_property(),
            Self::item_height_property().as_property(),
            Self::min_columns_property().as_property(),
            Self::max_columns_property().as_property(),
            Self::rows_property().as_property(),
            Self::is_drop_down_only_property().as_property(),
        ]
        .contains(&p)
        {
            this.apply_presentation();
            RibbonItemHelper::invalidate_host_layout(this);
        }
    }
}
impl StyledElementImpl for RibbonGallery {
    fn style_key_override(_: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl TemplatedControlImpl for RibbonGallery {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let old = this.parts.take();
        if let Some(host) = old.host {
            host.set_child(None);
        }
        for subscription in old.subscriptions {
            subscription.dispose();
        }
        Self::release_elements(&this.inline);
        Self::parent_on_apply_template(this, e);
        let scope = e.name_scope();
        let mut parts = Parts {
            host: scope.find_as("PART_InlineHost"),
            root: scope.find_as("PART_InlineRoot"),
            dropdown: scope.find_as("PART_DropDownButton"),
            up: scope.find_as("PART_UpButton"),
            down: scope.find_as("PART_DownButton"),
            more: scope.find_as("PART_MoreButton"),
            subscriptions: vec![],
        };
        if let Some(host) = &parts.host {
            host.set_child(this.inline.clone());
        }
        if let Some(button) = &parts.dropdown {
            button.set_is_chrome_button(true);
            let weak = this.to_ref().downgrade();
            parts.subscriptions.push(
                button
                    .add_disposable_handler(Button::click_event(), move |sender, _| {
                        if let Some(g) = weak.upgrade() {
                            g.defer_open(
                                sender.to_ref().cast::<Control>().expect("Gallery opener"),
                            );
                        }
                    })
                    .detach(),
            );
        }
        for (button, kind) in [(&parts.more, 0), (&parts.up, -1), (&parts.down, 1)] {
            if let Some(button) = button {
                let strings = RibbonStrings::current();
                let label = core_text(match kind {
                    0 => strings.gallery_more(),
                    -1 => strings.gallery_up(),
                    _ => strings.gallery_down(),
                });
                AutomationProperties::set_name(button, label.as_deref());
                if kind == 0 {
                    ToolTip::set_tip(button, label.map(|s| Rc::new(s) as BoxedValue));
                }
                let weak = this.to_ref().downgrade();
                parts.subscriptions.push(
                    button
                        .add_disposable_handler(Button::click_event(), move |sender, _| {
                            if let Some(g) = weak.upgrade() {
                                if kind == 0 {
                                    g.defer_open(
                                        sender.to_ref().cast::<Control>().expect("Gallery opener"),
                                    );
                                } else {
                                    g.scroll_rows(kind);
                                }
                            }
                        })
                        .detach(),
                );
            }
        }
        this.parts.replace(parts);
        this.rebuild();
    }
}
impl RibbonControlBaseImpl for RibbonGallery {
    fn on_layout_applied_core(this: &Self, _: RibbonItemLayout) {
        this.apply_presentation();
    }
    fn on_ribbon_property_changed(
        this: &Self,
        _: &ferroui_base::FerroPropertyChangedEventArgs<'_>,
    ) {
        this.apply_presentation();
    }
    fn create_linked_copy_core(this: &Self) -> Option<Ref<Control>> {
        let copy = Self::new();
        copy.set_is_drop_down_only(true);
        RibbonItemHelper::link_common(this, &copy);
        for p in [
            Self::item_template_property().as_property(),
            Self::item_width_property().as_property(),
            Self::item_height_property().as_property(),
            Self::max_columns_property().as_property(),
            Self::min_columns_property().as_property(),
            Self::drop_down_columns_property().as_property(),
            Self::is_filter_enabled_property().as_property(),
            Self::show_item_labels_property().as_property(),
            Self::items_source_property().as_property(),
        ] {
            RibbonItemHelper::link_one_way(this, &copy, p);
        }
        if this.items_source().is_none() {
            copy.set_items_source(Some(Rc::new(this.items())));
        }
        RibbonItemHelper::link(
            this,
            &copy,
            Self::selected_item_property().as_property(),
            true,
        );
        copy.set_category_selector(this.category_selector());
        let source = this.to_ref();
        copy.add_item_click(Some(MarkupDelegate::new(move |a| {
            let e = a[1]
                .as_ref()
                .expect("GalleryItemClick")
                .downcast_ref::<Rc<RibbonGalleryItemEventArgs>>()
                .expect("Gallery event args")
                .clone();
            source.raise(&source.item_click, e.clone());
            source
                .execute_command(e.value())
                .expect("Gallery source ExecuteCommand");
            None
        })));
        let source = this.to_ref();
        copy.add_item_preview(Some(MarkupDelegate::new(move |a| {
            let e = a[1]
                .as_ref()
                .expect("Gallery preview")
                .downcast_ref::<Rc<RibbonGalleryItemEventArgs>>()
                .expect("Gallery event args");
            source.preview(e.item());
            None
        })));
        Some(copy.upcast())
    }
    fn on_unlinked_core(this: &Self) {
        Self::parent_on_unlinked_core(this);
        this.close_drop_down();
        this.set_items_source(None);
    }
    fn create_overflow_menu_items_core(this: &Self) -> RibbonOverflowMenuItems {
        let source = this.to_ref();
        RibbonItemsSource::deferred(move || {
            let source = source.clone();
            let invoke = source.clone();
            let menu = RibbonItemHelper::create_menu_item(
                &source,
                source.label(),
                source.icon(),
                Some(MarkupDelegate::new(move |_| {
                    invoke.open_drop_down();
                    None
                })),
                None,
            )
            .expect("Gallery overflow");
            Box::new(std::iter::once(Some(menu)))
        })
    }
    fn on_key_tip_core(this: &Self) -> Rc<RibbonKeyTipResult> {
        this.open_drop_down();
        RibbonKeyTipResult::close()
    }
    fn invoke_core(this: &Self) -> bool {
        this.open_drop_down();
        true
    }
}
impl Drop for RibbonGallery {
    fn drop(&mut self) {
        if let Some(token) = self.items_token.take() {
            self.items.collection_changed().remove(token);
        }
        if let Some(observed) = self.observed.get_mut().take() {
            observed.subscription.dispose();
        }
        for subscription in self.parts.get_mut().subscriptions.drain(..) {
            subscription.dispose();
        }
    }
}
#[cfg(test)]
#[path = "../gallery_tests.rs"]
mod tests;
