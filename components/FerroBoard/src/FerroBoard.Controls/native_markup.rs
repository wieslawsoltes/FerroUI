//! Native declaration metadata, kept next to the existing implementation without a new engine.
use crate::input::ProtectedPasswordTextBox;
use crate::input::ScreenKeyboardInput;
use crate::primitives::{PaddedStackPanel, TouchButton, UniformWrapPanel};
use crate::{
    EnterActionRequestedEventArgs, KeyboardVisibilityMode, OnScreenKeyboard, ScreenKeyboardHost,
};
use ferroboard_core::editor::EnterAction;
use ferroui_base::{
    BoxedValue, Ref, TypeInfo, Visual,
    data::core::ValueTypes,
    ferro_class_info,
    metadata::{MarkupAssembly, MarkupDelegate, MarkupType, MarkupTyped, into_markup_value},
    reactive::Disposable,
    utilities::EventArgs,
};
use ferroui_system::{IDisposable, events::Event};
use std::rc::Rc;
const NAMESPACES: &[(&str, &str)] = &[
    ("ferroboard_controls", "FerroBoard.Controls"),
    (
        "ferroboard_controls::primitives",
        "FerroBoard.Controls.Primitives",
    ),
    ("ferroboard_controls::themes", "FerroBoard.Controls.Themes"),
];
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroBoard.Controls",
    crate_name: "ferroboard_controls",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[crate::OnScreenKeyboard,crate::ScreenKeyboardHost,crate::input::ScreenKeyboardInput,crate::input::ProtectedPasswordTextBox,crate::primitives::TouchButton,crate::primitives::UniformWrapPanel,crate::primitives::PaddedStackPanel],types:[crate::EnterActionRequestedEventArgs,crate::KeyboardVisibilityMode],contracts:[],generics:[],generic_contracts:[]}
pub fn register_types() {
    ferroui_controls::register_types();
    ferroboard_core_ferro::register_types();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[
        OnScreenKeyboard::TYPE,
        ScreenKeyboardHost::TYPE,
        ScreenKeyboardInput::TYPE,
        TouchButton::TYPE,
        UniformWrapPanel::TYPE,
        PaddedStackPanel::TYPE,
        ProtectedPasswordTextBox::TYPE,
    ]);
    MarkupAssembly::register(&ASSEMBLY);
    register_rust_paths();
    MarkupType::register_all(&[
        KeyboardVisibilityMode::MARKUP,
        EnterActionRequestedEventArgs::MARKUP,
    ]);
    crate::services::register_emoji_font_metadata();
    crate::themes::register_markup_types();
    crate::icons::register_markup_types();
    crate::input::register_markup_types();
    crate::smartbar_action_metadata::register();
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_nullable::<KeyboardVisibilityMode>();
    crate::items_source::register_conversions();
}
// A native event is synchronous and may throw. The convenience invoke() discards a failed
// bound method at this framework revision; preserve failure through the callback boundary.
fn invoke_handler(handler: &MarkupDelegate, args: &[ferroui_base::metadata::MarkupValue]) {
    if let Err(error) = handler.try_invoke(args) {
        std::panic::panic_any(error);
    }
}
fn keyboard_event(
    this: &Ref<OnScreenKeyboard>,
    event: fn(&OnScreenKeyboard) -> &Event<OnScreenKeyboard>,
    handler: MarkupDelegate,
) -> Rc<dyn IDisposable> {
    let sender = this.downgrade();
    let token = event(this).add(Rc::new(move |_| {
        invoke_handler(
            &handler,
            &[
                into_markup_value(sender.upgrade()),
                into_markup_value(EventArgs::EMPTY),
            ],
        );
    }));
    let source = this.downgrade();
    let cleanup = Disposable::create(move || {
        if let Some(source) = source.upgrade() {
            event(&source).remove(token);
        }
    });
    this.retain_markup_subscription(cleanup.clone());
    cleanup
}
fn host_event(this: &Ref<ScreenKeyboardHost>, handler: MarkupDelegate) -> Rc<dyn IDisposable> {
    let sender = this.downgrade();
    let token = this.keyboard_open_changed().add(Rc::new(move |_| {
        invoke_handler(
            &handler,
            &[
                into_markup_value(sender.upgrade()),
                into_markup_value(EventArgs::EMPTY),
            ],
        );
    }));
    let source = this.downgrade();
    let cleanup = Disposable::create(move || {
        if let Some(source) = source.upgrade() {
            source.keyboard_open_changed().remove(token);
        }
    });
    this.retain_markup_subscription(cleanup.clone());
    cleanup
}
ferro_class_info!(ScreenKeyboardHost {markup:{
 content:Content,constructors:[try ()=>ScreenKeyboardHost::new,try (Ref<OnScreenKeyboard>)=>(|keyboard:Ref<OnScreenKeyboard>|ScreenKeyboardHost::with_keyboard(keyboard,None))],
 properties:[Keyboard:Ref<OnScreenKeyboard>{get:ScreenKeyboardHost::keyboard},IsFloating:bool{get:ScreenKeyboardHost::is_floating}],
 events:[KeyboardOpenChanged(Option<BoxedValue>,EventArgs)=>host_event],
}});
ferro_class_info!(OnScreenKeyboard {markup:{
 constructors:[try ()=> (||OnScreenKeyboard::new(None)),try (Option<Rc<ferroboard_core::engine::KeyboardEngine>>)=>OnScreenKeyboard::new],
 properties:[Engine:Rc<ferroboard_core::engine::KeyboardEngine>{get:OnScreenKeyboard::engine},TouchProcessor:ferroboard_core_ferro::KeyboardTouchProcessorRef{get:|k:&Ref<OnScreenKeyboard>|ferroboard_core_ferro::KeyboardTouchProcessorRef::from_original(k.touch_processor())},SmartbarActions:Option<crate::themes::ThemeList<Option<Rc<crate::SmartbarAction>>>>{get:OnScreenKeyboard::smartbar_actions},TargetElement:Option<Ref<Visual>>{get:OnScreenKeyboard::target_element},EffectiveTheme:std::sync::Arc<crate::themes::KeyboardTheme>{get:OnScreenKeyboard::effective_theme}],
 methods:[try fn Attach(Rc<dyn ferroboard_core::editor::ITextInputTarget>)=>OnScreenKeyboard::attach,try fn AttachTo(Option<Ref<Visual>>)->bool=>OnScreenKeyboard::attach_to,try fn Detach()=>OnScreenKeyboard::detach,static fn IsTextInput(Option<Ref<Visual>>)->bool=>(|element:Option<Ref<Visual>>|OnScreenKeyboard::is_text_input(element.as_deref())),try fn ApplyTheme()=>OnScreenKeyboard::apply_theme,try fn ShowResizeOverlay()=>OnScreenKeyboard::show_resize_overlay],
 events:[
 SettingsRequested(Option<BoxedValue>,EventArgs)=>(|this:&Ref<OnScreenKeyboard>,handler:MarkupDelegate|keyboard_event(this,OnScreenKeyboard::settings_requested,handler)),
 HideRequested(Option<BoxedValue>,EventArgs)=>(|this:&Ref<OnScreenKeyboard>,handler:MarkupDelegate|keyboard_event(this,OnScreenKeyboard::hide_requested,handler)),
 VoiceInputRequested(Option<BoxedValue>,EventArgs)=>(|this:&Ref<OnScreenKeyboard>,handler:MarkupDelegate|keyboard_event(this,OnScreenKeyboard::voice_input_requested,handler)),
 TargetChanged(Option<BoxedValue>,EventArgs)=>(|this:&Ref<OnScreenKeyboard>,handler:MarkupDelegate|keyboard_event(this,OnScreenKeyboard::target_changed,handler)),

 FeedbackRequested(Option<BoxedValue>,ferroboard_core::settings::FeedbackKind)=>(|this:&Ref<OnScreenKeyboard>,handler:MarkupDelegate|{
  let sender=this.downgrade();let token=this.feedback_requested().add(Rc::new(move|kind|invoke_handler(&handler,&[into_markup_value(sender.upgrade()),into_markup_value(*kind)])));let source=this.downgrade();let cleanup=Disposable::create(move||{if let Some(source)=source.upgrade(){source.feedback_requested().remove(token);}});this.retain_markup_subscription(cleanup.clone());cleanup
 }),
 EnterActionRequested(Option<BoxedValue>,EnterActionRequestedEventArgs)=>(|this:&Ref<OnScreenKeyboard>,handler:MarkupDelegate|{
  let sender=this.downgrade();let token=this.enter_action_requested().add(Rc::new(move |args|{invoke_handler(&handler,&[into_markup_value(sender.upgrade()),into_markup_value(args.clone())]);}));let source=this.downgrade();let cleanup=Disposable::create(move||{if let Some(source)=source.upgrade(){source.enter_action_requested().remove(token);}});this.retain_markup_subscription(cleanup.clone());cleanup
 }),
 ],
}});
ferroui_base::ferro_markup_type!(class EnterActionRequestedEventArgs {
 namespace:"FerroBoard.Controls",handles:[EnterActionRequestedEventArgs],this:EnterActionRequestedEventArgs,
 constructors:[(Option<Ref<Visual>>,EnterAction)=>EnterActionRequestedEventArgs::new],
 properties:[Element:Option<Ref<Visual>>{get:|args:&EnterActionRequestedEventArgs|args.element()},Action:EnterAction{get:|args:&EnterActionRequestedEventArgs|args.action()},Handled:bool{get:|args:&EnterActionRequestedEventArgs|args.handled(),set:|args:&EnterActionRequestedEventArgs,value:bool|args.set_handled(value)}],
});
ferro_class_info!(UniformWrapPanel {markup:{properties:[
 ItemWidth:f64{get:UniformWrapPanel::item_width,set:UniformWrapPanel::set_item_width},
 ItemHeight:f64{get:UniformWrapPanel::item_height,set:UniformWrapPanel::set_item_height},
 MaxColumns:i32{get:UniformWrapPanel::max_columns,set:UniformWrapPanel::set_max_columns},
]}});
impl MarkupTyped for KeyboardVisibilityMode {
    const MARKUP: &'static MarkupType = {
        static M: MarkupType = {
            let mut m = MarkupType::new(
                "KeyboardVisibilityMode",
                ferroui_base::metadata::MarkupTypeKind::Enum,
                module_path!(),
            );
            m.explicit_namespace = Some("FerroBoard.Controls");
            m.handles = &[ferroui_base::data::core::ValueType::of::<KeyboardVisibilityMode>];
            m.nullable =
                Some(ferroui_base::data::core::ValueType::of::<Option<KeyboardVisibilityMode>>);
            m.enum_members = &[
                ferroui_base::metadata::MarkupEnumMember {
                    name: "Auto",
                    value: 0,
                    get: || Rc::new(KeyboardVisibilityMode::Auto),
                    rust_variant: Some("Auto"),
                },
                ferroui_base::metadata::MarkupEnumMember {
                    name: "AlwaysVisible",
                    value: 1,
                    get: || Rc::new(KeyboardVisibilityMode::AlwaysVisible),
                    rust_variant: Some("AlwaysVisible"),
                },
                ferroui_base::metadata::MarkupEnumMember {
                    name: "Manual",
                    value: 2,
                    get: || Rc::new(KeyboardVisibilityMode::Manual),
                    rust_variant: Some("Manual"),
                },
            ];
            m.enum_from_value = Some(|v| {
                i32::try_from(v)
                    .ok()
                    .map(|v| Rc::new(KeyboardVisibilityMode(v)) as BoxedValue)
            });
            m.enum_to_value = Some(|v| {
                v.downcast_ref::<KeyboardVisibilityMode>()
                    .map(|v| i64::from(v.0))
            });
            m
        };
        &M
    };
}
