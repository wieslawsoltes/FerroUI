//! Port of `Embedding/NativeTextBox.cs`.

use super::{INativeTextBoxFactory, INativeTextBoxImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::platform::IPlatformHandle;
use ferroui_controls::{
    ContextMenu, Control, ControlImpl, MenuItem, NativeControlHost, NativeControlHostImpl, NativeControlHostImplExt,
    TextBlock, ToolTip,
};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static FACTORY: RefCell<Option<Rc<dyn INativeTextBoxFactory>>> = const { RefCell::new(None) };
}

/// A native control host that shows the text box of the platform.
#[repr(C)]
pub struct NativeTextBox {
    base: NativeControlHost,
    context_menu: RefCell<Option<Ref<ContextMenu>>>,
    impl_: RefCell<Option<Rc<dyn INativeTextBoxImpl>>>,
    tip_text_block: Ref<TextBlock>,
    initial_text: RefCell<String>,
}

ferro_class!(NativeTextBox: NativeControlHost);
ferro_impl_classes!(
    NativeTextBox: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(NativeTextBox {
    new: NativeTextBox::new,
    markup: {
        properties: [
            Text: String {
                get: |this: &Ref<NativeTextBox>| this.text(),
                set: |this: &Ref<NativeTextBox>, value: String| this.set_text(&value)
            },
        ],
    },
});

impl NativeControlHostImpl for NativeTextBox {
    fn create_native_control_core(this: &Self, parent: Rc<dyn IPlatformHandle>) -> Rc<dyn IPlatformHandle> {
        let Some(factory) = Self::factory() else {
            return Self::parent_create_native_control_core(this, parent);
        };

        let impl_ = factory.create_control(parent);
        impl_.set_text(&this.initial_text.borrow());
        // The control holds the platform specific part, which holds the handlers of its
        // events: the handlers hold the control weakly.
        {
            let weak = this.to_ref().downgrade();
            impl_.context_menu_requested(Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_context_menu_requested();
                }
            }));
        }
        {
            let weak = this.to_ref().downgrade();
            impl_.hovered(Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_hovered();
                }
            }));
        }
        {
            let weak = this.to_ref().downgrade();
            impl_.pointer_exited(Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.on_pointer_exited();
                }
            }));
        }
        *this.impl_.borrow_mut() = Some(impl_.clone());
        impl_.handle()
    }

    fn destroy_native_control_core(this: &Self, control: Rc<dyn IPlatformHandle>) {
        Self::parent_destroy_native_control_core(this, control);
    }
}

impl NativeTextBox {
    pub fn construct() -> Self {
        Self {
            base: NativeControlHost::construct(),
            context_menu: RefCell::new(None),
            impl_: RefCell::new(None),
            tip_text_block: TextBlock::new(),
            initial_text: RefCell::new(String::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.tip_text_block.set_text(Some("FerroUI ToolTip"));
        this.tip_text_block.set_name(Some("NativeTextBoxToolTip".to_string()));

        ToolTip::set_tip(&this, Some(Control::boxed(&this.tip_text_block)));
        ToolTip::set_show_delay(&this, 1000);
        ToolTip::set_service_enabled(&this, false);
        this
    }

    pub fn text(&self) -> String {
        let impl_ = self.impl_.borrow().clone();
        match impl_ {
            Some(impl_) => impl_.text(),
            None => self.initial_text.borrow().clone(),
        }
    }

    pub fn set_text(&self, value: &str) {
        let impl_ = self.impl_.borrow().clone();
        match impl_ {
            Some(impl_) => impl_.set_text(value),
            None => *self.initial_text.borrow_mut() = value.to_string(),
        }
    }

    /// `Factory`: what creates the platform specific part; set by the entry point.
    pub fn factory() -> Option<Rc<dyn INativeTextBoxFactory>> {
        FACTORY.with(|factory| factory.borrow().clone())
    }

    pub fn set_factory(value: Option<Rc<dyn INativeTextBoxFactory>>) {
        FACTORY.with(|factory| *factory.borrow_mut() = value);
    }

    fn on_context_menu_requested(&self) {
        if self.context_menu.borrow().is_none() {
            let menu_item = MenuItem::new();
            menu_item.set_header(Some(Rc::new(String::from("Custom Menu Item")) as BoxedValue));
            {
                // The item is held by the menu of the control: its handler holds the control
                // weakly.
                let weak = self.to_ref().downgrade();
                menu_item.click(move |_, _| {
                    let impl_ = weak.upgrade().and_then(|this| this.impl_.borrow().clone());
                    impl_.expect("the platform specific part of the text box").set_text("Context menu item clicked");
                });
            }

            let context_menu = ContextMenu::new();
            context_menu.set_name(Some("NativeTextBoxContextMenu".to_string()));
            context_menu.items().add(Some(Control::boxed(&menu_item)));
            *self.context_menu.borrow_mut() = Some(context_menu);
        }

        ToolTip::set_is_open(self, false);
        let context_menu = self.context_menu.borrow().clone().expect("the context menu of the text box");
        context_menu.open_at(Some(&self.to_ref().upcast::<Control>()));
    }

    fn on_hovered(&self) {
        ToolTip::set_is_open(self, true);
    }

    fn on_pointer_exited(&self) {
        ToolTip::set_is_open(self, false);
    }
}
