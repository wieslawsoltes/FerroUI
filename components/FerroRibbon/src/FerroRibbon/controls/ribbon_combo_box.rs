//! Original RibbonInputs.cs combo architecture: an editor plus owned popup/list.
use super::{
    RibbonControlBaseImpl, RibbonControlBaseImplExt, RibbonInputBase, RibbonInputBaseImpl,
    RibbonItemHelper, RibbonKeyTipResult,
};
use crate::{primitives::RibbonPopupPlacement, theming::RibbonTheme};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{RibbonItemSize, collections::RibbonArray};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, Thickness, VisualImpl,
    controls::{IResourceProvider, ResourceDictionary, ResourceKey, ResourceValue},
    data::{
        BindingMode, ReflectionBinding,
        core::{ValueType, ValueTypes},
    },
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers, NavigationMethod},
    instantiate,
    interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs, RoutingStrategies},
    layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment},
    media::{Color, FontFamily, SolidColorBrush},
    metadata::MarkupDelegate,
    reactive::IDisposable,
    styling::{ControlTheme, ThemeVariant},
    threading::{Dispatcher, DispatcherPriority},
    utilities::{CultureInfo, StringComparison},
};
use ferroui_controls::{
    Border, Button, Control, ControlImpl, ScrollViewer, SelectionChangedEventArgs, StackPanel,
    TextBlock, TextBox, TopLevel,
    presenters::ContentPresenter,
    primitives::{
        Popup, ScrollBarVisibility, TemplateAppliedEventArgs, TemplatedControlImpl,
        TemplatedControlImplExt,
    },
    templates::IDataTemplate,
};
use ferroribbon_core::collections::ObservableCollection;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub type RibbonObjectItems = ObservableCollection<Option<ferroribbon_core::model::RibbonObject>>;
/// Rust convenience for inserting an existing native value into the shared Core
/// object collection. Markup Add/Insert/Set use the same boundary conversion.
pub trait RibbonObjectItemsExt {
    fn add_native(&self, value: Option<BoxedValue>);
}
impl RibbonObjectItemsExt for RibbonObjectItems {
    fn add_native(&self, value: Option<BoxedValue>) {
        self.add(crate::metadata::model_objects::from_native(value));
    }
}
fn native_items(items: &RibbonObjectItems) -> Vec<Option<BoxedValue>> {
    items
        .iter()
        .map(|v| {
            crate::metadata::model_objects::to_native(v).expect("Core combo item native payload")
        })
        .collect()
}
pub type RibbonObjectArray = RibbonArray<Option<BoxedValue>>;

// Shared CLR Array.Empty<object> identity for native object-array producers.
pub(super) fn object_array(values: Vec<Option<BoxedValue>>) -> RibbonObjectArray {
    if values.is_empty() {
        ferroui_base::ferro_thread_local! {static EMPTY:RibbonObjectArray=RibbonFixedItems::new(vec![]);}
        EMPTY.with(Clone::clone)
    } else {
        RibbonFixedItems::new(values)
    }
}
// Uno ResourceDictionary.TryGetValue uses own, reverse merged, then ONE active
// theme dictionary. Native try_get_resource uses theme before merged and falls
// back by key, so it is not an equivalent call for the source's direct lookup.
// Opaque native providers (including ResourceInclude) expose only public lookup;
// precedence *inside* those providers remains a separately recorded gap.
pub(super) fn source_dictionary_resource(
    dictionary: &ResourceDictionary,
    key: &ResourceKey,
    theme: Option<&ThemeVariant>,
) -> Option<ResourceValue> {
    if let Some(value) = dictionary.try_get_value(key) {
        return Some(value);
    }
    for provider in dictionary.merged_dictionaries().snapshot().iter().rev() {
        if let Some(value) = source_provider_resource(&**provider, key, theme) {
            return Some(value);
        }
    }
    let active = theme
        .filter(|t| **t != ThemeVariant::default())
        .and_then(|t| dictionary.theme_dictionary(t))
        .or_else(|| dictionary.theme_dictionary(&ThemeVariant::default()))?;
    source_provider_resource(&*active, key, theme)
}
fn source_provider_resource(
    provider: &dyn IResourceProvider,
    key: &ResourceKey,
    theme: Option<&ThemeVariant>,
) -> Option<ResourceValue> {
    if let Some(dictionary) = provider
        .as_object()
        .and_then(|o| o.downcast_ref::<ResourceDictionary>())
    {
        source_dictionary_resource(dictionary, key, theme)
    } else {
        provider.try_get_resource(key, theme)
    }
}
#[repr(C)]
pub struct RibbonComboBox {
    base: RibbonInputBase,
    items: RibbonObjectItems,
    text_box: RefCell<Option<Ref<TextBox>>>,
    drop_down_button: RefCell<Option<Ref<Button>>>,
    template_scope: RefCell<Option<ferroui_base::controls::NameScopeRef>>,
    selection_box: RefCell<Option<Ref<ContentPresenter>>>,
    member_evaluator: RefCell<Option<Ref<TextBlock>>>,
    popup: RefCell<Option<Ref<Popup>>>,
    list: RefCell<Option<Ref<StackPanel>>>,
    scroll: RefCell<Option<Ref<ScrollViewer>>>,
    highlight: Cell<i32>,
    syncing: Cell<bool>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    selector: RefCell<Option<MarkupDelegate>>,
    committed: RefCell<Option<MarkupDelegate>>,
    selection_changed: RefCell<Option<MarkupDelegate>>,
}
ferro_class!(RibbonComboBox:RibbonInputBase,virtuals RibbonComboBoxImpl:RibbonInputBaseImpl {fn create_copy_instance(this)->Ref<RibbonComboBox>;});
ferro_class_info!(RibbonComboBox{new:RibbonComboBox::new,markup:{namespace:"FerroRibbon.Controls",content:Items,properties:[Items:RibbonObjectItems{get:RibbonComboBox::items},EffectiveItems:RibbonObjectArray{get:RibbonComboBox::effective_items},IsDropDownOpen:bool{get:RibbonComboBox::is_drop_down_open},ItemTextSelector:Option<MarkupDelegate>{get:RibbonComboBox::item_text_selector,set:RibbonComboBox::set_item_text_selector}],events:[Committed(Option<BoxedValue>,Rc<RibbonComboBoxCommittedEventArgs>){add:RibbonComboBox::add_committed,remove:RibbonComboBox::remove_committed},SelectionChanged(Option<BoxedValue>,Rc<dyn IRoutedEventArgs>){add:RibbonComboBox::add_selection_changed,remove:RibbonComboBox::remove_selection_changed}],methods:[fn GetItemText(Option<BoxedValue>)->Option<String> =>RibbonComboBox::get_item_text,fn Commit(Option<BoxedValue>)=>RibbonComboBox::commit,fn CommitText(Option<String>)=>RibbonComboBox::commit_text,fn OpenDropDown()=>RibbonComboBox::open_drop_down,fn CloseDropDown()=>RibbonComboBox::close_drop_down]}});
ferro_impl_classes!(RibbonComboBox:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_properties! {impl RibbonComboBox{
 pub fn items_source_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("ItemsSource",None)}
 pub fn selected_item_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("SelectedItem",None)}
 pub fn text_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Text",None)}
 pub fn is_editable_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsEditable",false)}
 pub fn item_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<Self,_>("ItemTemplate",None)}
 pub fn selection_box_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>>{FerroProperty::register::<Self,_>("SelectionBoxTemplate",None)}
 pub fn display_member_path_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("DisplayMemberPath",None)}
 pub fn max_drop_down_height_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("MaxDropDownHeight",420.)}
 pub fn is_font_preview_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsFontPreview",false)}
}}
impl RibbonComboBox {
    fn static_constructor() {
        Self::items_source_property()
            .changed()
            .add_class_handler::<Self>(|c, _| {
                if c.is_drop_down_open() {
                    c.build_list();
                }
            })
            .detach();
        Self::selected_item_property()
            .changed()
            .add_class_handler::<Self>(|c, e| {
                let (old, new) = e.get_old_and_new_value::<Option<BoxedValue>>();
                c.on_selected_item_changed(old, new);
            })
            .detach();
        Self::text_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.on_text_changed())
            .detach();
        Self::is_editable_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.update_editable())
            .detach();
        Self::selection_box_template_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.update_selection_box())
            .detach();
        Self::display_member_path_property()
            .changed()
            .add_class_handler::<Self>(|c, _| {
                if let Some(i) = c.selected_item() {
                    c.set_text(c.get_item_text(Some(i)));
                }
                if c.is_drop_down_open() {
                    c.build_list();
                }
            })
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: RibbonInputBase::construct(),
            items: ObservableCollection::new_managed(),
            text_box: RefCell::new(None),
            drop_down_button: RefCell::new(None),
            template_scope: RefCell::new(None),
            selection_box: RefCell::new(None),
            member_evaluator: RefCell::new(None),
            popup: RefCell::new(None),
            list: RefCell::new(None),
            scroll: RefCell::new(None),
            highlight: Cell::new(-1),
            syncing: Cell::new(false),
            subscriptions: RefCell::new(vec![]),
            selector: RefCell::new(None),
            committed: RefCell::new(None),
            selection_changed: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items(&self) -> RibbonObjectItems {
        self.items.clone()
    }
    pub fn items_source(&self) -> Option<BoxedValue> {
        self.get_value(Self::items_source_property())
    }
    pub fn set_items_source(&self, v: Option<BoxedValue>) {
        self.set_value(Self::items_source_property(), v)
    }
    pub fn selected_item(&self) -> Option<BoxedValue> {
        self.get_value(Self::selected_item_property())
    }
    pub fn set_selected_item(&self, v: Option<BoxedValue>) {
        self.set_value(Self::selected_item_property(), v)
    }
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }
    pub fn set_text(&self, v: Option<String>) {
        self.set_value(Self::text_property(), v)
    }
    pub fn is_editable(&self) -> bool {
        self.get_value(Self::is_editable_property())
    }
    pub fn set_is_editable(&self, v: bool) {
        self.set_value(Self::is_editable_property(), v)
    }
    pub fn item_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::item_template_property())
    }
    pub fn set_item_template(&self, v: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::item_template_property(), v)
    }
    pub fn selection_box_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::selection_box_template_property())
    }
    pub fn set_selection_box_template(&self, v: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::selection_box_template_property(), v)
    }
    pub fn display_member_path(&self) -> Option<String> {
        self.get_value(Self::display_member_path_property())
    }
    pub fn set_display_member_path(&self, v: Option<String>) {
        self.set_value(Self::display_member_path_property(), v)
    }
    pub fn max_drop_down_height(&self) -> f64 {
        self.get_value(Self::max_drop_down_height_property())
    }
    pub fn set_max_drop_down_height(&self, v: f64) {
        self.set_value(Self::max_drop_down_height_property(), v)
    }
    pub fn is_font_preview(&self) -> bool {
        self.get_value(Self::is_font_preview_property())
    }
    pub fn set_is_font_preview(&self, v: bool) {
        self.set_value(Self::is_font_preview_property(), v)
    }
    pub fn item_text_selector(&self) -> Option<MarkupDelegate> {
        self.selector.borrow().clone()
    }
    pub fn set_item_text_selector(&self, v: Option<MarkupDelegate>) {
        self.selector.replace(v);
    }
    pub fn add_committed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.committed.borrow().as_ref(), h.as_ref());
        self.committed.replace(next);
    }
    pub fn remove_committed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.committed.borrow().as_ref(), h.as_ref());
        self.committed.replace(next);
    }
    pub fn add_selection_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.selection_changed.borrow().as_ref(), h.as_ref());
        self.selection_changed.replace(next);
    }
    pub fn remove_selection_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.selection_changed.borrow().as_ref(), h.as_ref());
        self.selection_changed.replace(next);
    }
    fn raise_committed(&self, e: Rc<RibbonComboBoxCommittedEventArgs>) {
        let h = self.committed.borrow().clone();
        if let Some(h) = h {
            h.try_invoke(&[Some(Control::boxed(self.to_ref())), Some(Rc::new(e))])
                .expect("Committed handler");
        }
    }
    fn on_selected_item_changed(&self, old: Option<BoxedValue>, new: Option<BoxedValue>) {
        if !self.syncing.get() {
            self.syncing.set(true);
            self.set_text(if new.is_some() {
                self.get_item_text(new.clone())
            } else {
                self.text()
            });
            self.syncing.set(false);
        }
        self.update_selection_box();
        let handler = self.selection_changed.borrow().clone();
        if let Some(h) = handler {
            let e: Rc<dyn IRoutedEventArgs> =
                Rc::new(SelectionChangedEventArgs::new::<RoutedEventArgs>(
                    None,
                    old.into_iter().map(Some).collect(),
                    new.into_iter().map(Some).collect(),
                ));
            h.try_invoke(&[Some(Control::boxed(self.to_ref())), Some(Rc::new(e))])
                .expect("SelectionChanged handler");
        }
    }
    fn on_text_changed(&self) {
        let part = self.text_box.borrow().clone();
        if let Some(p) = part {
            let text = self.text().unwrap_or_default();
            if p.text().as_deref() != Some(text.as_str()) {
                p.set_text(Some(&text));
            }
        }
    }
    fn update_editable(&self) {
        let p = self.text_box.borrow().clone();
        if let Some(p) = p {
            p.set_is_read_only(!self.is_editable());
        }
        self.update_selection_box();
    }
    fn update_selection_box(&self) {
        let templated = self.selection_box_template().is_some()
            && !self.is_editable()
            && self.selected_item().is_some();
        let selection = self.selection_box.borrow().clone();
        if let Some(p) = selection {
            p.set_content_template(self.selection_box_template());
            p.set_content(if templated {
                self.selected_item()
            } else {
                None
            });
            p.set_is_visible(templated);
        }
        let text = self.text_box.borrow().clone();
        if let Some(p) = text {
            p.set_opacity(if templated { 0. } else { 1. });
        }
    }
    pub fn effective_items(&self) -> RibbonObjectArray {
        let source = self.items_source();
        let values = match source.as_ref() {
            Some(s)
                if (s.downcast_ref::<String>().is_some()
                    || s.downcast_ref::<&'static str>().is_some()) =>
            {
                native_items(&self.items)
            }
            Some(s) if s.downcast_ref::<RibbonObjectItems>().is_some() => {
                native_items(s.downcast_ref::<RibbonObjectItems>().unwrap())
            }
            Some(s) if s.downcast_ref::<RibbonObjectArray>().is_some() => {
                s.downcast_ref::<RibbonObjectArray>().unwrap().to_vec()
            }
            Some(s) if s.downcast_ref::<RibbonArray<String>>().is_some() => s
                .downcast_ref::<RibbonArray<String>>()
                .unwrap()
                .iter()
                .map(|v| Some(Rc::new(v) as BoxedValue))
                .collect(),
            Some(s) if s.downcast_ref::<RibbonArray<f64>>().is_some() => s
                .downcast_ref::<RibbonArray<f64>>()
                .unwrap()
                .iter()
                .map(|v| Some(Rc::new(v) as BoxedValue))
                .collect(),
            Some(s)
                if s.downcast_ref::<ferroribbon_core::collections::RibbonReadOnlyList<String>>(
                )
                .is_some() =>
            {
                s.downcast_ref::<ferroribbon_core::collections::RibbonReadOnlyList<String>>()
                    .unwrap()
                    .iter()
                    .map(|v| Some(Rc::new(v) as BoxedValue))
                    .collect()
            }
            Some(s)
                if s.downcast_ref::<ferroribbon_core::collections::RibbonReadOnlyList<f64>>()
                    .is_some() =>
            {
                s.downcast_ref::<ferroribbon_core::collections::RibbonReadOnlyList<f64>>()
                    .unwrap()
                    .iter()
                    .map(|v| Some(Rc::new(v) as BoxedValue))
                    .collect()
            }
            Some(s) => {
                let converted = ValueTypes::try_convert(
                    Some(s),
                    ValueType::of::<ferroui_controls::ItemsSource>(),
                )
                .flatten();
                if let Some(v) = converted {
                    v.downcast_ref::<ferroui_controls::ItemsSource>()
                        .expect("registered IEnumerable conversion")
                        .to_vec()
                } else {
                    native_items(&self.items)
                }
            }
            None => native_items(&self.items),
        };
        object_array(values)
    }
    fn evaluate_display_member(&self, item: BoxedValue) -> Option<String> {
        let path = self.display_member_path()?;
        if path.is_empty()
            || (item.downcast_ref::<String>().is_some()
                || item.downcast_ref::<&'static str>().is_some())
        {
            return None;
        }
        let existing = self.member_evaluator.borrow().clone();
        let p = if let Some(p) = existing {
            p
        } else {
            let p = TextBlock::new();
            self.member_evaluator.replace(Some(p.clone()));
            p
        };
        let b = ReflectionBinding::new(&path);
        b.set_source(Some(item));
        b.set_mode(BindingMode::OneTime);
        let _binding = p.bind_binding(TextBlock::text_property().as_property(), &*b);
        // Pinned Uno TextBlock has empty default and coerces null to empty.
        let text = Some(p.text().unwrap_or_default());
        p.clear_value(TextBlock::text_property());
        text
    }
    pub fn get_item_text(&self, item: Option<BoxedValue>) -> Option<String> {
        if let Some(f) = self.item_text_selector()
            && let Some(v) = f
                .try_invoke(std::slice::from_ref(&item))
                .expect("ItemTextSelector callback")
        {
            if let Some(s) = v.downcast_ref::<String>() {
                return Some(s.clone());
            }
            if let Some(s) = v.downcast_ref::<Option<String>>() {
                if let Some(s) = s {
                    return Some(s.clone());
                }
            } else {
                panic!("ItemTextSelector must return string or null");
            }
        }
        let Some(item) = item else {
            return Some(String::new());
        };
        if let Some(text) = self.evaluate_display_member(item.clone()) {
            return Some(text);
        }
        // Native model values are normalized to their actual concrete Core
        // allocation. Dispatch the declared Node protocol before wrapper views.
        if ferroribbon_core::model::is_registered_model_type(item.as_ref().value_type_id())
            && let Some(node) = ferroribbon_core::model::try_cast_model::<
                dyn ferroribbon_core::model::RibbonNodeModelImpl,
            >(Some(item.clone()))
            .expect("declared Core model hierarchy")
        {
            use ferroribbon_core::model::RibbonNodeModelExt;
            return node.label().or_else(|| node.id()).map(|s| {
                s.to_utf8()
                    .expect("Native GetItemText cannot represent invalid UTF16")
            });
        }
        // Explicit source type-pattern dispatch over the declared Core hierarchy.
        // A downstream Core node can travel boxed as Rc<dyn RibbonNodeModelImpl>.
        use ferroribbon_core::model::RibbonNodeModelExt;
        macro_rules! node {
            ($ty:ty) => {
                if let Some(n) = item.downcast_ref::<Rc<$ty>>() {
                    return n.label().or_else(|| n.id()).map(|s| {
                        s.to_utf8()
                            .expect("Native GetItemText cannot represent invalid UTF16")
                    });
                }
            };
        }
        node!(dyn ferroribbon_core::model::RibbonNodeModelImpl);
        node!(ferroribbon_core::model::RibbonBackstageItemModel);
        node!(ferroribbon_core::model::RibbonButtonGroupModel);
        node!(ferroribbon_core::model::RibbonButtonModel);
        node!(ferroribbon_core::model::RibbonCheckBoxModel);
        node!(ferroribbon_core::model::RibbonColorPickerModel);
        node!(ferroribbon_core::model::RibbonComboBoxModel);
        node!(ferroribbon_core::model::RibbonContextualGroupModel);
        node!(ferroribbon_core::model::RibbonCustomItemModel);
        node!(ferroribbon_core::model::RibbonDropDownButtonModel);
        node!(ferroribbon_core::model::RibbonGalleryItemModel);
        node!(ferroribbon_core::model::RibbonGalleryModel);
        node!(ferroribbon_core::model::RibbonGridPickerModel);
        node!(ferroribbon_core::model::RibbonGroupModel);
        node!(ferroribbon_core::model::RibbonLabelModel);
        node!(ferroribbon_core::model::RibbonMenuItemModel);
        node!(ferroribbon_core::model::RibbonRowModel);
        node!(ferroribbon_core::model::RibbonSegmentModel);
        node!(ferroribbon_core::model::RibbonSegmentedModel);
        node!(ferroribbon_core::model::RibbonSeparatorModel);
        node!(ferroribbon_core::model::RibbonSliderModel);
        node!(ferroribbon_core::model::RibbonSpinnerModel);
        node!(ferroribbon_core::model::RibbonSplitButtonModel);
        node!(ferroribbon_core::model::RibbonTabModel);
        node!(ferroribbon_core::model::RibbonTextBoxModel);
        node!(ferroribbon_core::model::RibbonToggleButtonModel);
        // Gallery-item support is added with its actual native public class; there is no
        // unrelated stand-in object or successful empty implementation here.
        if let Some(v) = ValueTypes::try_convert_with_culture(
            Some(&item),
            ValueType::of::<String>(),
            &CultureInfo::current_formatting(),
        )
        .flatten()
            && let Some(s) = v.downcast_ref::<String>()
        {
            return Some(s.clone());
        }
        Some(ValueTypes::to_display_string(Some(&item)))
    }
    pub fn commit(&self, item: Option<BoxedValue>) {
        self.close_drop_down();
        self.set_selected_item(item.clone());
        self.syncing.set(true);
        self.set_text(self.get_item_text(item.clone()));
        self.syncing.set(false);
        let p = self.text_box.borrow().clone();
        if let Some(p) = p {
            p.set_text(Some(self.text().as_deref().unwrap_or("")));
        }
        self.raise_committed(RibbonComboBoxCommittedEventArgs::new(
            item.clone(),
            self.text(),
        ));
        self.execute_command(item)
            .expect("RibbonComboBox.ExecuteCommand");
    }
    pub fn commit_text(&self, text: Option<String>) {
        let matched = self
            .effective_items()
            .iter()
            .find(|i| {
                StringComparison::CurrentCultureIgnoreCase
                    .equals(self.get_item_text(i.clone()).as_deref(), text.as_deref())
            })
            .flatten();
        if let Some(i) = matched {
            self.commit(Some(i));
            return;
        }
        self.syncing.set(true);
        self.set_text(text.clone());
        self.set_selected_item(None);
        self.syncing.set(false);
        self.raise_committed(RibbonComboBoxCommittedEventArgs::new(None, text.clone()));
        self.execute_command(text.map(|v| Rc::new(v) as BoxedValue))
            .expect("RibbonComboBox.ExecuteCommand");
    }
    pub fn is_drop_down_open(&self) -> bool {
        self.popup.borrow().as_ref().is_some_and(|p| p.is_open())
    }
    pub fn close_drop_down(&self) {
        let p = self.popup.borrow().clone();
        if let Some(p) = p
            && p.is_open()
        {
            p.set_is_open(false);
        }
    }
    pub fn open_drop_down(&self) {
        if TopLevel::get_top_level(Some(self)).is_none() || !self.is_effectively_enabled() {
            return;
        }
        if self.popup.borrow().is_none() {
            let list = StackPanel::new();
            let padding = Border::new();
            padding.set_padding(Thickness::uniform(2.));
            padding.set_child(list.clone());
            let scroll = ScrollViewer::new();
            scroll.set_content(Some(Control::boxed(padding)));
            scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
            scroll.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
            let chrome = Border::new();
            chrome.set_child(scroll.clone());
            chrome.set_border_thickness(Thickness::uniform(1.));
            chrome.set_corner_radius(RibbonTheme::get_corner_radius(
                Some(self),
                "RibbonPopupCornerRadius",
                6.,
            ));
            chrome.set_padding(Thickness::new(0., 2., 0., 2.));
            chrome.set_value(
                ferroui_base::styling::ThemeVariant::requested_theme_variant_property(),
                self.actual_theme_variant(),
            );
            RibbonTheme::set_theme_brush(
                Some(&chrome),
                Some(Border::background_property().as_property()),
                Some("RibbonPopupBackgroundBrush"),
            )
            .expect("combo popup background");
            RibbonTheme::set_theme_brush(
                Some(&chrome),
                Some(Border::border_brush_property().as_property()),
                Some("RibbonPopupBorderBrush"),
            )
            .expect("combo popup border");
            let popup = Popup::new();
            popup.set_child(chrome);
            popup.set_popup_parent(Some(&self.to_ref().upcast::<Control>()));
            popup.set_is_light_dismiss_enabled(true);
            let weak = self.to_ref().downgrade();
            popup
                .closed(move || {
                    if let Some(c) = weak.upgrade() {
                        c.highlight.set(-1);
                    }
                })
                .detach();
            self.list.replace(Some(list));
            self.scroll.replace(Some(scroll));
            self.popup.replace(Some(popup));
        }
        self.build_list();
        let anchor = self
            .template_scope
            .borrow()
            .as_ref()
            .and_then(|s| s.find_as::<Control>("PART_InputBorder"))
            .unwrap_or_else(|| self.to_ref().upcast());
        let popup = self.popup.borrow().clone().unwrap();
        if let Some(child) = popup.child() {
            child.set_min_width(anchor.bounds().width);
            child.set_max_height(self.max_drop_down_height());
            child.set_value(
                ferroui_base::styling::ThemeVariant::requested_theme_variant_property(),
                self.actual_theme_variant(),
            );
        }
        RibbonPopupPlacement::place_below_with_options(&popup, &anchor, false, 1.);
        popup.set_is_open(true);
        self.highlight(Self::index_of(
            &self.effective_items(),
            &self.selected_item(),
        ));
        let p = self.text_box.borrow().clone();
        if let Some(p) = p {
            p.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE);
        }
    }
    fn build_list(&self) {
        let Some(list) = self.list.borrow().clone() else {
            return;
        };
        list.children().clear();
        let items = self.effective_items();
        for item in items.iter() {
            let content: BoxedValue = if let Some(template) = self.item_template() {
                let p = ContentPresenter::new();
                p.set_content(item.clone());
                p.set_content_template(Some(template));
                Control::boxed(p)
            } else {
                let text = TextBlock::new();
                text.set_text(self.get_item_text(item.clone()).as_deref());
                text.set_vertical_alignment(VerticalAlignment::Center);
                if self.is_font_preview() && !text.text().as_deref().unwrap_or("").is_empty() {
                    text.set_font_family(FontFamily::new(&text.text().unwrap()));
                    text.set_font_size(14.);
                }
                Control::boxed(text)
            };
            let button = Button::new();
            button.set_content(Some(content));
            button.set_tag(item.clone());
            button.set_horizontal_alignment(HorizontalAlignment::Stretch);
            button.set_horizontal_content_alignment(HorizontalAlignment::Left);
            if let Some(style) = ferroui_controls::Application::current().and_then(|a| {
                source_dictionary_resource(
                    &a.resources(),
                    &"RibbonMenuItemButtonStyle".into(),
                    a.actual_theme_variant().as_ref(),
                )
            }) {
                button.set_theme(style.map(|style| {
                    ValueTypes::as_object(&*style)
                        .and_then(|v| v.cast::<ControlTheme>())
                        .expect("RibbonMenuItemButtonStyle must be a ControlTheme")
                }));
            }
            let weak = self.to_ref().downgrade();
            button.click(move |_, _| {
                if let Some(c) = weak.upgrade() {
                    c.commit(item.clone());
                }
            });
            list.children().add(button);
        }
    }
    fn highlight(&self, index: i32) {
        self.highlight.set(index);
        let Some(list) = self.list.borrow().clone() else {
            return;
        };
        for i in 0..list.children().count() {
            if let Some(b) = list.children().get(i).cast::<Button>() {
                let selected = i as i32 == index;
                b.pseudo_classes().set(":highlighted", selected);
                b.pseudo_classes().set(":not-highlighted", !selected);
                b.set_background(if selected {
                    RibbonTheme::get_brush(Some(&b), Some("RibbonItemHoverBrush"))
                        .expect("combo highlight brush")
                        .map(Into::into)
                } else {
                    Some(SolidColorBrush::with_color(Color::from_argb(0, 255, 255, 255)).into())
                });
                if selected {
                    b.bring_into_view();
                }
            }
        }
    }
    fn index_of(items: &RibbonObjectArray, item: &Option<BoxedValue>) -> i32 {
        items
            .iter()
            .position(|v| match (&v, item) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b) || a.value_eq(&**b),
                _ => false,
            })
            .map(|i| i as i32)
            .unwrap_or(-1)
    }
    fn on_text_key_down(&self, e: &KeyEventArgs) {
        let items = self.effective_items();
        match e.key {
            Key::Enter => {
                if self.is_drop_down_open()
                    && self.highlight.get() >= 0
                    && (self.highlight.get() as usize) < items.len()
                {
                    self.commit(items.get(self.highlight.get() as usize));
                } else if self.is_editable() {
                    let p = self.text_box.borrow().clone();
                    if let Some(p) = p {
                        self.commit_text(p.text());
                    }
                }
                e.set_handled(true);
            }
            Key::Escape => {
                if self.is_drop_down_open() {
                    self.close_drop_down();
                    e.set_handled(true);
                } else {
                    let p = self.text_box.borrow().clone();
                    if let Some(p) = p {
                        p.set_text(Some(self.text().as_deref().unwrap_or("")));
                    }
                }
            }
            Key::F4 => {
                self.open_drop_down();
                e.set_handled(true);
            }
            Key::Up | Key::Down => {
                if items.is_empty() {
                    return;
                }
                if e.key_modifiers.contains(KeyModifiers::ALT) {
                    self.open_drop_down();
                    e.set_handled(true);
                    return;
                }
                let current = if self.is_drop_down_open() {
                    self.highlight.get()
                } else {
                    Self::index_of(&items, &self.selected_item())
                };
                let next = (current + if e.key == Key::Down { 1 } else { -1 })
                    .clamp(0, items.len() as i32 - 1);
                if self.is_drop_down_open() {
                    self.highlight(next);
                } else {
                    self.commit(items.get(next as usize));
                }
                e.set_handled(true);
            }
            _ => {}
        }
    }
}
impl StyledElementImpl for RibbonComboBox {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}
impl FerroObjectImpl for RibbonComboBox {
    fn constructed(c: &Self) {
        Self::parent_constructed(c);
        c.set_size(RibbonItemSize::Small);
        c.set_show_label(false);
    }
}
impl RibbonInputBaseImpl for RibbonComboBox {
    fn shows_label_by_default(c: &Self) -> bool {
        c.show_label()
    }
}
impl RibbonComboBoxImpl for RibbonComboBox {
    fn create_copy_instance(c: &Self) -> Ref<RibbonComboBox> {
        if c.is::<super::RibbonFontComboBox>() {
            super::RibbonFontComboBox::new().upcast()
        } else if c.is::<super::RibbonFontSizeComboBox>() {
            super::RibbonFontSizeComboBox::new().upcast()
        } else {
            Self::new()
        }
    }
}
impl TemplatedControlImpl for RibbonComboBox {
    fn on_apply_template(c: &Self, e: &TemplateAppliedEventArgs) {
        for s in std::mem::take(&mut *c.subscriptions.borrow_mut()) {
            s.dispose();
        }
        Self::parent_on_apply_template(c, e);
        c.template_scope.replace(Some(e.name_scope().clone()));
        let text = e.name_scope().find_as::<TextBox>("PART_TextBox");
        c.text_box.replace(text.clone());
        c.drop_down_button
            .replace(e.name_scope().find_as::<Button>("PART_DropDownButton"));
        c.selection_box.replace(
            e.name_scope()
                .find_as::<ContentPresenter>("PART_SelectionBox"),
        );
        if let Some(p) = text {
            let weak = c.to_ref().downgrade();
            let key = p.add_disposable_handler_with(
                InputElement::key_down_event(),
                move |_, e| {
                    if let Some(c) = weak.upgrade() {
                        c.on_text_key_down(e);
                    }
                },
                RoutingStrategies::TUNNEL,
                false,
            );
            let weak = c.to_ref().downgrade();
            let got = p.add_disposable_handler(InputElement::got_focus_event(), move |_, _| {
                if let Some(c) = weak.upgrade() {
                    let p = c.text_box.borrow().clone();
                    if let Some(p) = p {
                        p.select_all();
                    }
                }
            });
            let weak = c.to_ref().downgrade();
            let lost = p.add_disposable_handler(InputElement::lost_focus_event(), move |_, _| {
                if let Some(c) = weak.upgrade() {
                    let p = c.text_box.borrow().clone();
                    if c.is_editable()
                        && !c.is_drop_down_open()
                        && let Some(p) = p
                        && p.text().as_deref() != Some(c.text().as_deref().unwrap_or(""))
                    {
                        c.commit_text(p.text());
                    }
                }
            });
            c.subscriptions
                .borrow_mut()
                .extend([key.detach(), lost.detach(), got.detach()]);
            p.set_text(Some(c.text().as_deref().unwrap_or("")));
            ferroui_controls::automation::AutomationProperties::set_name(
                &p,
                Some(c.label().as_deref().unwrap_or("")),
            );
        }
        let button = c.drop_down_button.borrow().clone();
        if let Some(b) = button {
            let weak = c.to_ref().downgrade();
            let click = b.add_disposable_handler(Button::click_event(), move |_, _| {
                if let Some(c) = weak.upgrade() {
                    if c.is_drop_down_open() {
                        c.close_drop_down();
                    } else {
                        let c = c.to_ref();
                        Dispatcher::ui_thread()
                            .post_local(move || c.open_drop_down(), DispatcherPriority::NORMAL);
                    }
                }
            });
            c.subscriptions.borrow_mut().push(click.detach());
        }
        c.update_editable();
    }
}
impl RibbonControlBaseImpl for RibbonComboBox {
    fn create_linked_copy_core(c: &Self) -> Option<Ref<Control>> {
        let copy = c.create_copy_instance();
        RibbonItemHelper::link_common(c, &copy);
        RibbonItemHelper::link(c, &copy, Self::items_source_property().as_property(), false);
        RibbonItemHelper::link(c, &copy, Self::selected_item_property().as_property(), true);
        RibbonItemHelper::link(c, &copy, Self::text_property().as_property(), true);
        for p in [
            Self::is_editable_property().as_property(),
            RibbonInputBase::input_width_property().as_property(),
            Self::item_template_property().as_property(),
            Self::display_member_path_property().as_property(),
            Self::selection_box_template_property().as_property(),
            Self::is_font_preview_property().as_property(),
        ] {
            RibbonItemHelper::link(c, &copy, p, false);
        }
        if c.items_source().is_none() {
            copy.set_items_source(Some(Rc::new(c.items())));
        }
        copy.set_item_text_selector(c.item_text_selector());
        let source = c.to_ref();
        copy.add_committed(Some(MarkupDelegate::new(move |args| {
            let e = args[1]
                .as_ref()
                .unwrap()
                .downcast_ref::<Rc<RibbonComboBoxCommittedEventArgs>>()
                .unwrap()
                .clone();
            source.raise_committed(e.clone());
            source
                .execute_command(
                    e.item()
                        .or_else(|| e.text().map(|v| Rc::new(v) as BoxedValue)),
                )
                .expect("linked combo ExecuteCommand");
            None
        })));
        Some(copy.upcast())
    }
    fn on_unlinked_core(c: &Self) {
        Self::parent_on_unlinked_core(c);
        c.close_drop_down();
        c.set_items_source(None);
        c.set_item_text_selector(None);
    }
    fn on_key_tip_core(c: &Self) -> Rc<RibbonKeyTipResult> {
        let p = c.text_box.borrow().clone();
        if let Some(p) = p {
            p.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
        }
        if !c.is_editable() {
            c.open_drop_down();
        }
        RibbonKeyTipResult::close()
    }
    fn invoke_core(c: &Self) -> bool {
        c.open_drop_down();
        true
    }
}
pub struct RibbonComboBoxCommittedEventArgs {
    item: Option<BoxedValue>,
    text: Option<String>,
}
impl PartialEq for RibbonComboBoxCommittedEventArgs {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
impl RibbonComboBoxCommittedEventArgs {
    pub fn new(item: Option<BoxedValue>, text: Option<String>) -> Rc<Self> {
        Rc::new(Self { item, text })
    }
    pub fn item(&self) -> Option<BoxedValue> {
        self.item.clone()
    }
    pub fn text(&self) -> Option<String> {
        self.text.clone()
    }
}
ferroui_base::ferro_markup_type!(class RibbonComboBoxCommittedEventArgs{namespace:"FerroRibbon.Controls",this:Rc<RibbonComboBoxCommittedEventArgs>,handles:[RibbonComboBoxCommittedEventArgs,Rc<RibbonComboBoxCommittedEventArgs>,Option<Rc<RibbonComboBoxCommittedEventArgs>>],constructors:[(Option<BoxedValue>,Option<String>)=>RibbonComboBoxCommittedEventArgs::new],properties:[Item:Option<BoxedValue>{get:|v:&Rc<RibbonComboBoxCommittedEventArgs>|v.item()},Text:Option<String>{get:|v:&Rc<RibbonComboBoxCommittedEventArgs>|v.text()}]});
#[cfg(test)]
#[path = "../combo_box_tests.rs"]
mod tests;
