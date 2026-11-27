//! Independent consumer: native subclass, properties and source-style event handlers.
use ferroribbon::controls::{
    RibbonApplicationMenu, RibbonApplicationMenuItem, RibbonApplicationMenuItemEventArgs,
    RibbonApplicationMenuRecentItem,
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl, utilities::EventArgs,
};
use ferroui_controls::{
    ContentControlImpl, ControlImpl, PanelImpl, StackPanel, StackPanelImpl,
    primitives::TemplatedControlImpl,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct ConsumerApplicationMenuItem {
    base: RibbonApplicationMenuItem,
    constructed: Cell<usize>,
}
ferro_class!(ConsumerApplicationMenuItem:RibbonApplicationMenuItem);
ferro_class_info!(ConsumerApplicationMenuItem{new:ConsumerApplicationMenuItem::new,markup:{namespace:"FerroRibbon.Consumer"}});
impl ConsumerApplicationMenuItem {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonApplicationMenuItem::construct(),
            constructed: Cell::new(0),
        })
    }
    pub fn constructions(&self) -> usize {
        self.constructed.get()
    }
}
impl FerroObjectImpl for ConsumerApplicationMenuItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.constructed.set(this.constructed.get() + 1);
    }
}
#[repr(C)]
pub struct ApplicationMenuView {
    base: StackPanel,
    item: RefCell<Option<Ref<RibbonApplicationMenuItem>>>,
    recent: RefCell<Option<Ref<RibbonApplicationMenuRecentItem>>>,
    clicks: Cell<usize>,
    command_runs: Rc<Cell<usize>>,
    command: Rc<dyn ferroui_base::input::ICommand>,
}
ferro_class!(ApplicationMenuView:StackPanel);
ferro_class_info!(ApplicationMenuView{new:ApplicationMenuView::new,markup:{namespace:"FerroRibbon.Consumer",properties:[Command:Rc<dyn ferroui_base::input::ICommand>{get:ApplicationMenuView::command},Parameter:i32{get:|_:&ApplicationMenuView|7},MenuItem:Option<Ref<RibbonApplicationMenuItem>>{get:ApplicationMenuView::menu_item,set:|v:&ApplicationMenuView,x|{v.item.replace(x);}},RecentItem:Option<Ref<RibbonApplicationMenuRecentItem>>{get:ApplicationMenuView::recent_item,set:|v:&ApplicationMenuView,x|{v.recent.replace(x);}}],methods:[fn ItemClick(Option<BoxedValue>,Rc<RibbonApplicationMenuItemEventArgs>)=>ApplicationMenuView::item_click,fn RecentClick(Option<BoxedValue>,EventArgs)=>ApplicationMenuView::recent_click,fn RecentInvoked(Option<BoxedValue>,Ref<RibbonApplicationMenuRecentItem>)=>ApplicationMenuView::recent_invoked]}});
ferro_impl_classes!(ApplicationMenuView:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl,StackPanelImpl);
ferro_properties! {impl ApplicationMenuView{pub fn header_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Header",Some("Open".into()))}}}
impl ApplicationMenuView {
    pub fn new() -> Ref<Self> {
        let command_runs = Rc::new(Cell::new(0));
        let command = Rc::new(NativeMenuCommand(command_runs.clone()));
        instantiate(Self {
            command_runs,
            command,
            base: StackPanel::construct(),
            item: RefCell::default(),
            recent: RefCell::default(),
            clicks: Cell::new(0),
        })
    }
    pub fn menu_item(&self) -> Option<Ref<RibbonApplicationMenuItem>> {
        self.item.borrow().clone()
    }
    pub fn recent_item(&self) -> Option<Ref<RibbonApplicationMenuRecentItem>> {
        self.recent.borrow().clone()
    }
    pub fn header(&self) -> Option<String> {
        self.get_value(Self::header_property())
    }
    pub fn set_header(&self, value: Option<String>) {
        self.set_value(Self::header_property(), value);
    }
    pub fn item_click(&self, _: Option<BoxedValue>, _: Rc<RibbonApplicationMenuItemEventArgs>) {
        self.clicks.set(self.clicks.get() + 1);
    }
    pub fn recent_click(&self, _: Option<BoxedValue>, _: EventArgs) {
        self.clicks.set(self.clicks.get() + 1);
    }
    pub fn command(&self) -> Rc<dyn ferroui_base::input::ICommand> {
        self.command.clone()
    }
    pub fn command_runs(&self) -> usize {
        self.command_runs.get()
    }
    pub fn recent_invoked(&self, _: Option<BoxedValue>, _: Ref<RibbonApplicationMenuRecentItem>) {
        self.clicks.set(self.clicks.get() + 1);
    }
    pub fn calls(&self) -> usize {
        self.clicks.get()
    }
}

// Immutable test action, exposed through the actual native command interface.
struct NativeMenuCommand(Rc<Cell<usize>>);
impl ferroui_base::input::ICommand for NativeMenuCommand {
    fn can_execute(&self, p: Option<&BoxedValue>) -> bool {
        p.and_then(|v| v.downcast_ref::<i32>()) == Some(&7)
    }
    fn execute(&self, p: Option<&BoxedValue>) {
        assert!(self.can_execute(p));
        self.0.set(self.0.get() + 1);
    }
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn ferroui_base::reactive::IDisposable> {
        ferroui_base::reactive::Disposable::empty()
    }
}
#[repr(C)]
pub struct ConsumerApplicationMenu {
    base: RibbonApplicationMenu,
    constructed: Cell<usize>,
}
ferro_class!(ConsumerApplicationMenu:RibbonApplicationMenu);
ferro_class_info!(ConsumerApplicationMenu{new:ConsumerApplicationMenu::new,markup:{namespace:"FerroRibbon.Consumer"}});
ferro_impl_classes!(ConsumerApplicationMenu:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl ConsumerApplicationMenu {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: RibbonApplicationMenu::construct(),
            constructed: Cell::new(0),
        })
    }
    pub fn constructions(&self) -> usize {
        self.constructed.get()
    }
}
impl FerroObjectImpl for ConsumerApplicationMenu {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.constructed.set(this.constructed.get() + 1);
    }
}
