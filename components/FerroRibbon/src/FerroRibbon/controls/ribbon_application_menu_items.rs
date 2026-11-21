//! RibbonApplicationMenu.cs item models. Copyright Wiesław Šoltés, MIT.
//! Native properties and collections; no CLR runtime or reflection adapter.
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref,
    StyledProperty, ferro_class, ferro_class_info, ferro_properties,
    input::ICommand,
    instantiate,
    metadata::{MarkupDelegate, into_markup_value},
    reactive::{Disposable, IDisposable},
    utilities::{EventArgs, HandlerList},
};
use ferroui_system::collections::object_model::ObservableCollection;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

pub type RibbonApplicationMenuItems = ObservableCollection<Option<Ref<RibbonApplicationMenuItem>>>;
pub type RibbonApplicationMenuRecentItems =
    ObservableCollection<Option<Ref<RibbonApplicationMenuRecentItem>>>;
type Changed<T> = dyn Fn(&T);
type ItemClick = dyn Fn(&RibbonApplicationMenuItem, &Rc<RibbonApplicationMenuItemEventArgs>);

#[repr(C)]
pub struct RibbonApplicationMenuItem {
    base: FerroObject,
    id: RefCell<Option<String>>,
    items: RibbonApplicationMenuItems,
    items_token: Cell<Option<u64>>,
    changed: HandlerList<Changed<Self>>,
    clicked: HandlerList<ItemClick>,
    markup_clicked: RefCell<Vec<(MarkupDelegate, u64)>>,
}
ferro_class!(RibbonApplicationMenuItem:FerroObject);
ferro_class_info!(RibbonApplicationMenuItem{new:RibbonApplicationMenuItem::new,markup:{namespace:"FerroRibbon.Controls",content:Items,properties:[
    Id:Option<String>{get:RibbonApplicationMenuItem::id,set:|c:&RibbonApplicationMenuItem,v|c.set_id(v)},
    Items:RibbonApplicationMenuItems{get:RibbonApplicationMenuItem::items}
],events:[Click(Option<BoxedValue>,Rc<RibbonApplicationMenuItemEventArgs>){add:RibbonApplicationMenuItem::add_click,remove:RibbonApplicationMenuItem::remove_click}]}});
ferro_properties! {impl RibbonApplicationMenuItem{
    pub fn label_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("Label",None)}
    pub fn icon_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,_>("Icon",None)}
    pub fn description_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("Description",None)}
    pub fn command_property()->StyledProperty<Option<Rc<dyn ICommand>>> {FerroProperty::register::<Self,_>("Command",None)}
    pub fn command_parameter_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,_>("CommandParameter",None)}
    pub fn is_enabled_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("IsEnabled",true)}
    pub fn has_separator_before_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("HasSeparatorBefore",false)}
}}
macro_rules! property {
    ($get:ident,$set:ident,$p:ident,$ty:ty) => {
        pub fn $get(&self) -> $ty {
            self.get_value(Self::$p())
        }
        pub fn $set(&self, value: $ty) {
            self.set_value(Self::$p(), value);
        }
    };
}
impl RibbonApplicationMenuItem {
    fn static_constructor() {
        macro_rules! changed {($($p:ident),*)=>{$(Self::$p().changed().add_class_handler::<Self>(|c,_|c.raise_changed()).detach();)*};}
        changed!(
            label_property,
            icon_property,
            description_property,
            is_enabled_property,
            has_separator_before_property
        );
    }
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            id: RefCell::new(None),
            items: ObservableCollection::new_managed(),
            items_token: Cell::new(None),
            changed: HandlerList::new(),
            clicked: HandlerList::new(),
            markup_clicked: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn id(&self) -> Option<String> {
        self.id.borrow().clone()
    }
    pub fn set_id(&self, value: Option<String>) {
        self.id.replace(value);
    }
    pub fn items(&self) -> RibbonApplicationMenuItems {
        self.items.clone()
    }
    property!(label, set_label, label_property, Option<String>);
    property!(icon, set_icon, icon_property, Option<BoxedValue>);
    property!(
        description,
        set_description,
        description_property,
        Option<String>
    );
    property!(
        command,
        set_command,
        command_property,
        Option<Rc<dyn ICommand>>
    );
    property!(
        command_parameter,
        set_command_parameter,
        command_parameter_property,
        Option<BoxedValue>
    );
    property!(is_enabled, set_is_enabled, is_enabled_property, bool);
    property!(
        has_separator_before,
        set_has_separator_before,
        has_separator_before_property,
        bool
    );
    pub(crate) fn changed(&self, handler: impl Fn(&Self) + 'static) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(c) = weak.upgrade() {
                c.changed.remove(token);
            }
        })
    }
    fn raise_changed(&self) {
        for (_, handler) in self.changed.snapshot().iter() {
            handler(self);
        }
    }
    pub fn click(
        &self,
        handler: impl Fn(&Self, &Rc<RibbonApplicationMenuItemEventArgs>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.clicked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(c) = weak.upgrade() {
                c.clicked.remove(token);
            }
        })
    }
    pub(crate) fn raise_click(&self, parameter: Option<BoxedValue>) {
        let args = RibbonApplicationMenuItemEventArgs::new(self.to_ref(), parameter);
        for (_, handler) in self.clicked.snapshot().iter() {
            handler(self, &args);
        }
    }
    pub fn add_click(&self, handler: Option<MarkupDelegate>) {
        let Some(handler) = handler else {
            return;
        };
        let callback = handler.clone();
        let token = self.clicked.add(Rc::new(move |c, args| {
            callback
                .try_invoke(&[
                    into_markup_value(c.to_ref()),
                    into_markup_value(args.clone()),
                ])
                .expect("Application menu item Click handler");
        }));
        self.markup_clicked.borrow_mut().push((handler, token));
    }
    pub fn remove_click(&self, handler: Option<MarkupDelegate>) {
        let removed = {
            let mut handlers = self.markup_clicked.borrow_mut();
            handlers
                .iter()
                .rposition(|(h, _)| Some(h) == handler.as_ref())
                .map(|index| handlers.remove(index))
        };
        if let Some((_, token)) = removed {
            self.clicked.remove(token);
        }
    }
}
impl FerroObjectImpl for RibbonApplicationMenuItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let token = this.items.add_collection_changed(Rc::new(move |_| {
            if let Some(c) = weak.upgrade() {
                c.raise_changed();
            }
        }));
        this.items_token.set(Some(token));
    }
}
impl Drop for RibbonApplicationMenuItem {
    fn drop(&mut self) {
        if let Some(token) = self.items_token.take() {
            self.items.remove_collection_changed(token);
        }
    }
}

#[repr(C)]
pub struct RibbonApplicationMenuRecentItem {
    base: FerroObject,
    tag: RefCell<Option<BoxedValue>>,
    changed: HandlerList<Changed<Self>>,
    clicked: HandlerList<Changed<Self>>,
    markup_clicked: RefCell<Vec<(MarkupDelegate, u64)>>,
}
ferro_class!(RibbonApplicationMenuRecentItem:FerroObject);
ferro_class_info!(RibbonApplicationMenuRecentItem{new:RibbonApplicationMenuRecentItem::new,markup:{namespace:"FerroRibbon.Controls",properties:[Tag:Option<BoxedValue>{get:RibbonApplicationMenuRecentItem::tag,set:|c:&RibbonApplicationMenuRecentItem,v|c.set_tag(v)}],events:[Click(Option<BoxedValue>,EventArgs){add:RibbonApplicationMenuRecentItem::add_click,remove:RibbonApplicationMenuRecentItem::remove_click}]}});
ferro_properties! {impl RibbonApplicationMenuRecentItem{
    pub fn title_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("Title",None)}
    pub fn path_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("Path",None)}
    pub fn icon_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,_>("Icon",None)}
    pub fn is_pinned_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("IsPinned",false)}
    pub fn command_property()->StyledProperty<Option<Rc<dyn ICommand>>> {FerroProperty::register::<Self,_>("Command",None)}
}}
impl RibbonApplicationMenuRecentItem {
    fn static_constructor() {
        macro_rules! changed {($($p:ident),*)=>{$(Self::$p().changed().add_class_handler::<Self>(|c,_|c.raise_changed()).detach();)*};}
        changed!(
            title_property,
            path_property,
            icon_property,
            is_pinned_property
        );
    }
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            tag: RefCell::new(None),
            changed: HandlerList::new(),
            clicked: HandlerList::new(),
            markup_clicked: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    property!(title, set_title, title_property, Option<String>);
    property!(path, set_path, path_property, Option<String>);
    property!(icon, set_icon, icon_property, Option<BoxedValue>);
    property!(is_pinned, set_is_pinned, is_pinned_property, bool);
    property!(
        command,
        set_command,
        command_property,
        Option<Rc<dyn ICommand>>
    );
    pub fn tag(&self) -> Option<BoxedValue> {
        self.tag.borrow().clone()
    }
    pub fn set_tag(&self, value: Option<BoxedValue>) {
        self.tag.replace(value);
    }
    pub(crate) fn changed(&self, handler: impl Fn(&Self) + 'static) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(c) = weak.upgrade() {
                c.changed.remove(token);
            }
        })
    }
    fn raise_changed(&self) {
        for (_, handler) in self.changed.snapshot().iter() {
            handler(self);
        }
    }
    pub fn click(&self, handler: impl Fn(&Self) + 'static) -> Rc<dyn IDisposable> {
        let token = self.clicked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(c) = weak.upgrade() {
                c.clicked.remove(token);
            }
        })
    }
    pub(crate) fn raise_click(&self) {
        for (_, handler) in self.clicked.snapshot().iter() {
            handler(self);
        }
    }
    pub fn add_click(&self, handler: Option<MarkupDelegate>) {
        let Some(handler) = handler else {
            return;
        };
        let callback = handler.clone();
        let token = self.clicked.add(Rc::new(move |c| {
            callback
                .try_invoke(&[
                    into_markup_value(c.to_ref()),
                    into_markup_value(EventArgs::EMPTY),
                ])
                .expect("Application menu recent Click handler");
        }));
        self.markup_clicked.borrow_mut().push((handler, token));
    }
    pub fn remove_click(&self, handler: Option<MarkupDelegate>) {
        let removed = {
            let mut handlers = self.markup_clicked.borrow_mut();
            handlers
                .iter()
                .rposition(|(h, _)| Some(h) == handler.as_ref())
                .map(|index| handlers.remove(index))
        };
        if let Some((_, token)) = removed {
            self.clicked.remove(token);
        }
    }
}
impl FerroObjectImpl for RibbonApplicationMenuRecentItem {}

pub struct RibbonApplicationMenuItemEventArgs {
    item: Ref<RibbonApplicationMenuItem>,
    parameter: Option<BoxedValue>,
}
impl RibbonApplicationMenuItemEventArgs {
    pub fn new(item: Ref<RibbonApplicationMenuItem>, parameter: Option<BoxedValue>) -> Rc<Self> {
        ferroui_base::data::core::ValueTypes::register_reference::<Self>();
        Rc::new(Self { item, parameter })
    }
    pub fn item(&self) -> Ref<RibbonApplicationMenuItem> {
        self.item.clone()
    }
    pub fn parameter(&self) -> Option<BoxedValue> {
        self.parameter.clone()
    }
}
impl PartialEq for RibbonApplicationMenuItemEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class RibbonApplicationMenuItemEventArgs{namespace:"FerroRibbon.Controls",this:Rc<RibbonApplicationMenuItemEventArgs>,handles:[RibbonApplicationMenuItemEventArgs,Rc<RibbonApplicationMenuItemEventArgs>],properties:[Item:Ref<RibbonApplicationMenuItem>{get:|c:&Rc<RibbonApplicationMenuItemEventArgs>|c.item()},Parameter:Option<BoxedValue>{get:|c:&Rc<RibbonApplicationMenuItemEventArgs>|c.parameter()}]});
