//! Application event sink; editing and focus behavior stay in the library.
use ferroboard_controls::{EnterActionRequestedEventArgs, OnScreenKeyboard};
use ferroboard_core::editor::EnterAction;
use ferroui_base::{BoxedValue, FerroObject, FerroObjectImpl, Ref, metadata::MarkupAssembly};
use std::cell::{Cell, RefCell};

#[repr(C)]
pub struct Consumer {
    base: FerroObject,
    keyboard: RefCell<Option<Ref<OnScreenKeyboard>>>,
    pub enter_count: Cell<usize>,
    pub last_action: Cell<Option<EnterAction>>,
}
impl Consumer {
    pub fn new() -> Ref<Self> {
        ferroui_base::instantiate(Self {
            base: FerroObject::construct(),
            keyboard: RefCell::new(None),
            enter_count: Cell::new(0),
            last_action: Cell::new(None),
        })
    }
    pub fn keyboard(&self) -> Option<Ref<OnScreenKeyboard>> {
        self.keyboard.borrow().clone()
    }
    pub fn set_keyboard(&self, value: Option<Ref<OnScreenKeyboard>>) {
        self.keyboard.replace(value);
    }
    pub fn on_enter(&self, _sender: Option<BoxedValue>, args: EnterActionRequestedEventArgs) {
        self.enter_count.set(self.enter_count.get() + 1);
        self.last_action.set(Some(args.action()));
        args.set_handled(true);
    }
    pub fn release(&self) {
        if let Some(keyboard) = self.keyboard.borrow_mut().take() {
            keyboard
                .release_native_subscriptions()
                .expect("release keyboard subscriptions");
        }
    }
}
ferroui_base::ferro_class!(Consumer:FerroObject);
ferroui_base::ferro_impl_classes!(Consumer:FerroObjectImpl);
ferroui_base::ferro_class_info!(Consumer {new:Consumer::new,markup:{
 properties:[Keyboard:Option<Ref<OnScreenKeyboard>>{get:|t:&Ref<Consumer>|t.keyboard(),set:|t:&Ref<Consumer>,v|t.set_keyboard(v)}],
 fields:[TypingWindow:std::time::Duration => || std::time::Duration::from_nanos(123_456_789),Rectangle:ferroboard_core::layouts::KeyRect => || ferroboard_core::layouts::KeyRect::new(1.25, 2.5, 80.5, 25.75)],
 methods:[fn OnEnter(Option<BoxedValue>,EnterActionRequestedEventArgs)=>(|t:&Ref<Consumer>,s,a|t.on_enter(s,a))]
}});
ferroui_base::ferro_rust_paths! {classes:[crate::consumer::Consumer],types:[],contracts:[],generics:[],generic_contracts:[]}
static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroBoard.Native.Consumer",
    crate_name: "ferroboard_native_consumer",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};
const NAMESPACES: &[(&str, &str)] = &[(
    "ferroboard_native_consumer::consumer",
    "FerroBoard.Native.Consumer",
)];
pub fn register_types() {
    ferroui_base::TypeInfo::register_namespaces(NAMESPACES);
    ferroui_base::TypeInfo::register_all(&[Consumer::TYPE]);
    MarkupAssembly::register(&ASSEMBLY);
    register_rust_paths();
}
