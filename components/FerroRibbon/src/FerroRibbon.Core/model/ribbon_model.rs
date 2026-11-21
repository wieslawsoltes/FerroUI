//! Port of the pinned root RibbonModel.cs.
use super::*;
use crate::{
    ObservableObject, ObservableObjectImpl, RibbonDensity, RibbonDisplayMode,
    RibbonMinimizeBehavior, RibbonQuickAccessPosition, RibbonString, RibbonVisibilityMode,
    commands::RibbonCommandCatalog, text::ordinal::OrdinalKey,
};
use ferroui_system::{
    collections::object_model::ObservableCollection, component_model::INotifyPropertyChanged,
    events::Event,
};
use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};
pub type RibbonItemReference = Option<Rc<dyn RibbonItemModelImpl>>;
pub type RibbonNodeReference = Option<Rc<dyn RibbonNodeModelImpl>>;
pub type RibbonTabReference = Option<Rc<dyn RibbonTabModelImpl>>;
pub type RibbonContextualGroupReference = Option<Rc<dyn RibbonContextualGroupModelImpl>>;
pub type RibbonGroupReference = Option<Rc<dyn RibbonGroupModelImpl>>;
pub type RibbonItemPath = (
    Rc<dyn RibbonTabModelImpl>,
    Rc<dyn RibbonGroupModelImpl>,
    RibbonItemReference,
);
/// Mutable fixed-size params-array input, preserving aliasing during notifications.
pub type RibbonContextualGroupIds = crate::collections::RibbonArray<Option<RibbonString>>;

pub struct RibbonModel {
    base: ObservableObject,
    tabs: ObservableCollection<RibbonTabReference>,
    contextual_groups: ObservableCollection<RibbonContextualGroupReference>,
    quick_access_items: ObservableCollection<RibbonItemReference>,
    quick_access_candidates: ObservableCollection<RibbonItemReference>,
    tab_strip_items: ObservableCollection<RibbonItemReference>,
    backstage: Rc<dyn RibbonBackstageModelImpl>,
    selected_tab_id: RefCell<Option<RibbonString>>,
    command_catalog: RefCell<Option<Rc<RibbonCommandCatalog>>>,
    display_mode: RefCell<RibbonDisplayMode>,
    visibility_mode: RefCell<RibbonVisibilityMode>,
    density: RefCell<RibbonDensity>,
    quick_access_position: RefCell<RibbonQuickAccessPosition>,
    is_quick_access_visible: RefCell<bool>,
    show_quick_access_labels: RefCell<bool>,
    minimize_behavior: RefCell<RibbonMinimizeBehavior>,
    show_group_captions: RefCell<bool>,
    application_button_label: RefCell<Option<RibbonString>>,
    is_application_button_visible: RefCell<bool>,
    title: RefCell<Option<RibbonString>>,
}
pub trait RibbonModelImpl: ObservableObjectImpl + Any {
    fn ribbon_model(&self) -> &RibbonModel;
    fn equals_model(&self, other: &dyn RibbonModelImpl) -> bool {
        std::ptr::addr_eq(
            self as *const Self as *const (),
            other as *const dyn RibbonModelImpl as *const (),
        )
    }
}
macro_rules! property {
    ($get:ident,$set:ident,$ty:ty,$name:literal) => {
        fn $get(&self) -> $ty {
            self.ribbon_model().$get.borrow().to_owned()
        }
        fn $set(&self, value: $ty) {
            ObservableObject::set_property(self, &self.ribbon_model().$get, value, $name);
        }
    };
}
pub trait RibbonModelExt: RibbonModelImpl {
    fn flatten(
        items: Option<RibbonItemsSource<RibbonItemReference>>,
    ) -> RibbonItemsSource<RibbonItemReference>
    where
        Self: Sized,
    {
        RibbonModel::flatten(items)
    }

    fn tabs(&self) -> ObservableCollection<RibbonTabReference> {
        self.ribbon_model().tabs.clone()
    }
    fn contextual_groups(&self) -> ObservableCollection<RibbonContextualGroupReference> {
        self.ribbon_model().contextual_groups.clone()
    }
    fn quick_access_items(&self) -> ObservableCollection<RibbonItemReference> {
        self.ribbon_model().quick_access_items.clone()
    }
    fn quick_access_candidates(&self) -> ObservableCollection<RibbonItemReference> {
        self.ribbon_model().quick_access_candidates.clone()
    }
    fn tab_strip_items(&self) -> ObservableCollection<RibbonItemReference> {
        self.ribbon_model().tab_strip_items.clone()
    }
    fn backstage(&self) -> Rc<dyn RibbonBackstageModelImpl> {
        self.ribbon_model().backstage.clone()
    }
    fn selected_tab_id(&self) -> Option<RibbonString> {
        self.ribbon_model().selected_tab_id.borrow().clone()
    }
    fn set_selected_tab_id(&self, value: Option<RibbonString>) {
        if ObservableObject::set_property(
            self,
            &self.ribbon_model().selected_tab_id,
            value,
            "SelectedTabId",
        ) {
            self.on_property_changed("SelectedTab");
        }
    }
    fn command_catalog(&self) -> Option<Rc<RibbonCommandCatalog>> {
        self.ribbon_model().command_catalog.borrow().clone()
    }
    fn set_command_catalog(&self, value: Option<Rc<RibbonCommandCatalog>>) {
        let old = self.command_catalog();
        let equal = match (&old, &value) {
            (None, None) => true,
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            _ => false,
        };
        if !equal {
            let old = self.ribbon_model().command_catalog.replace(value);
            drop(old);
            self.on_property_changed("CommandCatalog");
        }
    }
    property!(
        display_mode,
        set_display_mode,
        RibbonDisplayMode,
        "DisplayMode"
    );
    property!(
        visibility_mode,
        set_visibility_mode,
        RibbonVisibilityMode,
        "VisibilityMode"
    );
    property!(density, set_density, RibbonDensity, "Density");
    property!(
        quick_access_position,
        set_quick_access_position,
        RibbonQuickAccessPosition,
        "QuickAccessPosition"
    );
    property!(
        is_quick_access_visible,
        set_is_quick_access_visible,
        bool,
        "IsQuickAccessVisible"
    );
    property!(
        show_quick_access_labels,
        set_show_quick_access_labels,
        bool,
        "ShowQuickAccessLabels"
    );
    property!(
        minimize_behavior,
        set_minimize_behavior,
        RibbonMinimizeBehavior,
        "MinimizeBehavior"
    );
    property!(
        show_group_captions,
        set_show_group_captions,
        bool,
        "ShowGroupCaptions"
    );
    property!(
        application_button_label,
        set_application_button_label,
        Option<RibbonString>,
        "ApplicationButtonLabel"
    );
    property!(
        is_application_button_visible,
        set_is_application_button_visible,
        bool,
        "IsApplicationButtonVisible"
    );
    property!(title, set_title, Option<RibbonString>, "Title");
    fn selected_tab(&self) -> RibbonTabReference {
        self.tabs()
            .iter()
            .find(|t| t.as_ref().expect("NullReference: tab").id() == self.selected_tab_id())
            .flatten()
    }
    fn set_selected_tab(&self, value: RibbonTabReference) {
        self.set_selected_tab_id(value.and_then(|v| v.id()));
    }
    fn find_tab(&self, id: Option<RibbonString>) -> RibbonTabReference {
        self.tabs()
            .iter()
            .find(|t| t.as_ref().expect("NullReference: tab").id() == id)
            .flatten()
    }
    fn find_contextual_group(&self, id: Option<RibbonString>) -> RibbonContextualGroupReference {
        self.contextual_groups()
            .iter()
            .find(|g| g.as_ref().expect("NullReference: contextual group").id() == id)
            .flatten()
    }
    fn find_group(&self, id: Option<RibbonString>) -> RibbonGroupReference {
        for tab in self.tabs().iter() {
            for group in tab.expect("NullReference: tab").groups().iter() {
                if group.as_ref().expect("NullReference: group").id() == id {
                    return group;
                }
            }
        }
        None
    }
    fn find_item(&self, id: Option<RibbonString>) -> RibbonItemReference {
        items(self.tabs(), self.tab_strip_items())
            .iter()
            .find(|i| i.as_ref().expect("NullReference: item").id() == id)
            .flatten()
    }
    fn find_node(&self, id: Option<RibbonString>) -> RibbonNodeReference {
        if let Some(v) = self.find_tab(id.clone()) {
            return Some(v);
        }
        if let Some(v) = self.find_group(id.clone()) {
            return Some(v);
        }
        if let Some(v) = self.find_item(id.clone()) {
            return Some(v);
        }
        if let Some(v) = self.find_contextual_group(id.clone()) {
            return Some(v);
        }
        self.backstage()
            .items()
            .iter()
            .find(|v| v.as_ref().expect("NullReference: backstage item").id() == id)
            .flatten()
            .map(|v| v as Rc<dyn RibbonNodeModelImpl>)
    }
    fn enumerate_items(self: Rc<Self>) -> RibbonItemsSource<RibbonItemReference> {
        RibbonItemsSource::deferred(move || items(self.tabs(), self.tab_strip_items()).iter())
    }
    fn enumerate_items_with_path(self: Rc<Self>) -> RibbonItemsSource<RibbonItemPath> {
        RibbonItemsSource::deferred(move || {
            Box::new(self.tabs().iter().flat_map(|tab| {
                let tab = tab.expect("NullReference: tab");
                tab.groups().iter().flat_map(move |group| {
                    let group = group.expect("NullReference: group");
                    let tab = tab.clone();
                    flatten_items(RibbonItemsSource::from_collection(group.items()))
                        .iter()
                        .map(move |item| (tab.clone(), group.clone(), item))
                })
            }))
        })
    }
    fn find_items_by_command(
        self: Rc<Self>,
        command_id: Option<RibbonString>,
    ) -> RibbonItemsSource<RibbonItemReference> {
        let body = self.clone().enumerate_items();
        let qat = flatten_items(RibbonItemsSource::from_collection(
            self.quick_access_items(),
        ));
        let strip = flatten_items(RibbonItemsSource::from_collection(self.tab_strip_items()));
        distinct_items(RibbonItemsSource::deferred(move || {
            let id = command_id.clone();
            Box::new(
                body.iter()
                    .chain(qat.iter())
                    .chain(strip.iter())
                    .filter(move |item| {
                        ordinal_equals(
                            item.as_ref().expect("NullReference: item").command_id(),
                            id.clone(),
                        )
                    }),
            )
        }))
    }
    fn find_nodes_by_command(
        &self,
        command_id: Option<RibbonString>,
    ) -> RibbonItemsSource<RibbonNodeReference> {
        // The source captures its distinct item list at this call, but menu and
        // backstage traversal remain live until their turn in the iterator.
        let body = items(self.tabs(), self.tab_strip_items());
        let qat = flatten_items(RibbonItemsSource::from_collection(
            self.quick_access_items(),
        ));
        let strip = flatten_items(RibbonItemsSource::from_collection(self.tab_strip_items()));
        let captured = distinct_items(RibbonItemsSource::deferred(move || {
            Box::new(body.iter().chain(qat.iter()).chain(strip.iter()))
        }))
        .iter()
        .collect::<Vec<_>>();
        let captured = RibbonItemsSource::from_values(captured);
        let backstage = self.backstage().items();
        let nodes = RibbonItemsSource::deferred(move || {
            let menu_items = captured.clone();
            let backstage = backstage.clone();
            let menus = std::iter::once_with(move || {
                menu_items.iter().flat_map(|item| menu_entries(item).iter())
            })
            .flatten();
            let tail = std::iter::once_with(move || {
                backstage
                    .iter()
                    .map(|item| item.map(|item| item as Rc<dyn RibbonNodeModelImpl>))
            })
            .flatten();
            Box::new(
                captured
                    .iter()
                    .map(|item| item.map(|item| item as Rc<dyn RibbonNodeModelImpl>))
                    .chain(menus)
                    .chain(tail),
            )
        });
        let distinct = distinct_nodes(nodes);
        RibbonItemsSource::deferred(move || {
            let id = command_id.clone();
            Box::new(
                distinct
                    .iter()
                    .filter(move |node| ordinal_equals(command_id_of(node.clone()), id.clone())),
            )
        })
    }
    fn set_command_enabled(&self, command_id: Option<RibbonString>, enabled: bool) {
        for node in self.find_nodes_by_command(command_id) {
            node.expect("NullReference: node").set_is_enabled(enabled);
        }
    }
    fn set_command_checked(&self, command_id: Option<RibbonString>, is_checked: bool) {
        for node in self.find_nodes_by_command(command_id) {
            if let Some(v) = cast::<dyn RibbonToggleButtonModelImpl>(node.clone()) {
                v.set_is_checked(is_checked);
            } else if let Some(v) = cast::<dyn RibbonSplitButtonModelImpl>(node.clone()) {
                v.set_is_checked(is_checked);
            } else if let Some(v) = cast::<dyn RibbonCheckBoxModelImpl>(node.clone()) {
                v.set_is_checked(Some(is_checked));
            } else if let Some(v) = cast::<dyn RibbonMenuItemModelImpl>(node) {
                v.set_is_checked(is_checked);
            }
        }
    }
    fn set_contextual_group_visible(&self, id: Option<RibbonString>, visible: bool) -> bool {
        let Some(group) = self.find_contextual_group(id) else {
            return false;
        };
        group.set_is_visible(visible);
        true
    }
    fn set_active_contextual_groups(&self, ids: Option<RibbonContextualGroupIds>) {
        for group in self.contextual_groups().iter() {
            let group = group.expect("NullReference: contextual group");
            let id = group.id();
            // C# 14 binds this array call to MemoryExtensions.Contains; a null
            // array converts to an empty ReadOnlySpan, rather than throwing.
            let contains = ids.as_ref().is_some_and(|values| values.contains(&id));
            group.set_is_visible(contains);
        }
    }
    fn visible_tabs(self: Rc<Self>) -> RibbonItemsSource<RibbonTabReference> {
        RibbonItemsSource::deferred(move || {
            let owner = self.clone();
            Box::new(self.tabs().iter().filter(move |t| {
                let tab = t.as_ref().expect("NullReference: tab");
                tab.is_visible()
                    && (tab.contextual_group_id().is_none()
                        || owner
                            .find_contextual_group(tab.contextual_group_id())
                            .is_some_and(|g| g.is_visible()))
            }))
        })
    }
}
impl<T: RibbonModelImpl + ?Sized> RibbonModelExt for T {}
impl PartialEq for dyn RibbonModelImpl {
    fn eq(&self, other: &Self) -> bool {
        self.equals_model(other)
    }
}
impl RibbonModel {
    pub fn construct() -> Self {
        Self {
            base: ObservableObject::construct(),
            tabs: ObservableCollection::new_managed(),
            contextual_groups: ObservableCollection::new_managed(),
            quick_access_items: ObservableCollection::new_managed(),
            quick_access_candidates: ObservableCollection::new_managed(),
            tab_strip_items: ObservableCollection::new_managed(),
            backstage: RibbonBackstageModel::new(),
            selected_tab_id: RefCell::new(None),
            command_catalog: RefCell::new(None),
            display_mode: RefCell::new(Default::default()),
            visibility_mode: RefCell::new(Default::default()),
            density: RefCell::new(Default::default()),
            quick_access_position: RefCell::new(Default::default()),
            is_quick_access_visible: RefCell::new(true),
            show_quick_access_labels: RefCell::new(false),
            minimize_behavior: RefCell::new(Default::default()),
            show_group_captions: RefCell::new(true),
            application_button_label: RefCell::new(Some("File".into())),
            is_application_button_visible: RefCell::new(true),
            title: RefCell::new(None),
        }
    }
    pub fn new() -> Rc<Self> {
        Rc::new(Self::construct())
    }
    pub fn flatten(
        items: Option<RibbonItemsSource<RibbonItemReference>>,
    ) -> RibbonItemsSource<RibbonItemReference> {
        super::ribbon_items_source::tree(items, |item| {
            cast::<dyn RibbonButtonGroupModelImpl>(
                item.clone().map(|v| v as Rc<dyn RibbonNodeModelImpl>),
            )
            .map(|v| RibbonItemsSource::from_collection(v.items()))
        })
    }
}
impl INotifyPropertyChanged for RibbonModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}
impl ObservableObjectImpl for RibbonModel {}
impl RibbonModelImpl for RibbonModel {
    fn ribbon_model(&self) -> &RibbonModel {
        self
    }
}
impl PartialEq for RibbonModel {
    fn eq(&self, other: &Self) -> bool {
        self.equals_model(other)
    }
}
fn flatten_items(
    query: RibbonItemsSource<RibbonItemReference>,
) -> RibbonItemsSource<RibbonItemReference> {
    RibbonModel::flatten(Some(query))
}
fn items(
    tabs: ObservableCollection<RibbonTabReference>,
    strip: ObservableCollection<RibbonItemReference>,
) -> RibbonItemsSource<RibbonItemReference> {
    let body = RibbonItemsSource::deferred(move || {
        Box::new(tabs.iter().flat_map(|tab| {
            let tab = tab.expect("NullReference: tab");
            tab.groups()
                .iter()
                .flat_map(|group| {
                    let group = group.expect("NullReference: group");
                    let primary = RibbonItemsSource::from_collection(group.items());
                    let slide = RibbonItemsSource::from_collection(group.slide_out_items());
                    flatten_items(RibbonItemsSource::deferred(move || {
                        let slide = slide.clone();
                        Box::new(
                            primary
                                .iter()
                                .chain(std::iter::once_with(move || slide.iter()).flatten()),
                        )
                    }))
                    .iter()
                    .inspect(move |_| {
                        let _retain = &group;
                    })
                })
                .inspect(move |_| {
                    let _retain = &tab;
                })
        }))
    });
    let tail = flatten_items(RibbonItemsSource::from_collection(strip));
    RibbonItemsSource::deferred(move || Box::new(body.iter().chain(tail.iter())))
}
fn cast<T: ?Sized + super::RibbonModelView>(node: RibbonNodeReference) -> Option<Rc<T>> {
    try_cast_node::<T>(node).expect("Core model class metadata is undeclared")
}
fn menu_entries(item: RibbonItemReference) -> RibbonItemsSource<RibbonNodeReference> {
    let item = item.map(|v| v as Rc<dyn RibbonNodeModelImpl>);
    let roots = if let Some(v) = cast::<dyn RibbonDropDownButtonModelImpl>(item.clone()) {
        Some(v.menu_items())
    } else {
        cast::<dyn RibbonGalleryModelImpl>(item).map(|v| v.menu_items())
    };
    let Some(roots) = roots else {
        return RibbonItemsSource::from_values(vec![]);
    };
    super::ribbon_items_source::tree(Some(RibbonItemsSource::from_collection(roots)), |n| {
        cast::<dyn RibbonMenuItemModelImpl>(n.clone())
            .filter(|m| m.items().count() > 0)
            .map(|m| RibbonItemsSource::from_collection(m.items()))
    })
}
fn command_id_of(node: RibbonNodeReference) -> Option<RibbonString> {
    if let Some(v) = cast::<dyn RibbonItemModelImpl>(node.clone()) {
        return v.command_id();
    }
    if let Some(v) = cast::<dyn RibbonMenuItemModelImpl>(node.clone()) {
        return v.command_id();
    }
    cast::<dyn RibbonBackstageItemModelImpl>(node).and_then(|v| v.command_id())
}
fn ordinal_equals(a: Option<RibbonString>, b: Option<RibbonString>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => OrdinalKey(a) == OrdinalKey(b),
        _ => false,
    }
}
#[derive(Default)]
struct NodeSet {
    buckets: HashMap<i32, Vec<RibbonNodeReference>>,
}
impl NodeSet {
    fn insert(&mut self, node: RibbonNodeReference) -> bool {
        let hash = node.as_ref().map_or(0, |n| n.node_hash_code());
        let bucket = self.buckets.entry(hash).or_default();
        if bucket.iter().rev().any(|old| match (old, &node) {
            (None, None) => true,
            (Some(a), Some(b)) => a.equals_node(b.as_ref()),
            _ => false,
        }) {
            return false;
        }
        bucket.push(node);
        true
    }
}
fn distinct_nodes(
    query: RibbonItemsSource<RibbonNodeReference>,
) -> RibbonItemsSource<RibbonNodeReference> {
    RibbonItemsSource::deferred(move || {
        let mut seen = NodeSet::default();
        Box::new(query.iter().filter(move |v| seen.insert(v.clone())))
    })
}
fn distinct_items(
    query: RibbonItemsSource<RibbonItemReference>,
) -> RibbonItemsSource<RibbonItemReference> {
    RibbonItemsSource::deferred(move || {
        let mut seen = NodeSet::default();
        Box::new(
            query
                .iter()
                .filter(move |v| seen.insert(v.clone().map(|v| v as Rc<dyn RibbonNodeModelImpl>))),
        )
    })
}
