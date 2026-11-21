//! Port of the unchanged shared Controls/RibbonScreenTip.cs (control and service).
use crate::theming::RibbonTheme;
use ferroribbon_core::{RibbonString, model::RibbonScreenTip as ModelScreenTip, text::trim};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
    animation::SignedDuration,
    controls::NameScopeRef,
    data::core::ValueTypes,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, LayoutableImplExt},
    styling::ControlTheme,
    threading::DispatcherTimer,
};
use ferroui_controls::{
    Application, Control, ControlImpl, ToolTip,
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[repr(C)]
pub struct RibbonScreenTip {
    base: TemplatedControl,
    scope: RefCell<Option<NameScopeRef>>,
    extend_timer: RefCell<Option<Rc<DispatcherTimer>>>,
}
ferro_class!(RibbonScreenTip:TemplatedControl);
ferro_impl_classes!(RibbonScreenTip:VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(RibbonScreenTip {
    new:RibbonScreenTip::new,
    markup:{namespace:"FerroRibbon.Controls", properties:[
        HasExtendedContent:bool {get:RibbonScreenTip::has_extended_content},
    ]},
});
ferro_properties! {impl RibbonScreenTip {
    pub fn title_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("Title",None)}
    pub fn shortcut_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("Shortcut",None)}
    pub fn description_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("Description",None)}
    pub fn image_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,_>("Image",None)}
    pub fn help_text_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("HelpText",None)}
    pub fn disabled_reason_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("DisabledReason",None)}
    pub fn title_text_property()->StyledProperty<String> {FerroProperty::register::<Self,_>("TitleText",String::new())}
    pub fn extended_description_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("ExtendedDescription",None)}
    pub fn extended_image_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,_>("ExtendedImage",None)}
    pub fn is_extended_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("IsExtended",false)}
}}
macro_rules! property {
    ($get:ident,$set:ident,$prop:ident,$ty:ty) => {
        pub fn $get(&self) -> $ty {
            self.get_value(Self::$prop())
        }
        pub fn $set(&self, value: $ty) {
            self.set_value(Self::$prop(), value)
        }
    };
}
impl RibbonScreenTip {
    fn static_constructor() {
        macro_rules! changed {($($p:ident),*)=>{$(Self::$p().changed().add_class_handler::<Self>(|this,_|this.update()).detach();)*};}
        changed!(
            title_property,
            shortcut_property,
            description_property,
            image_property,
            help_text_property,
            disabled_reason_property,
            extended_description_property,
            extended_image_property,
            is_extended_property
        );
    }
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            scope: RefCell::default(),
            extend_timer: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    property!(title, set_title, title_property, Option<String>);
    property!(shortcut, set_shortcut, shortcut_property, Option<String>);
    property!(
        description,
        set_description,
        description_property,
        Option<String>
    );
    property!(image, set_image, image_property, Option<BoxedValue>);
    property!(help_text, set_help_text, help_text_property, Option<String>);
    property!(
        disabled_reason,
        set_disabled_reason,
        disabled_reason_property,
        Option<String>
    );
    property!(
        extended_description,
        set_extended_description,
        extended_description_property,
        Option<String>
    );
    property!(
        extended_image,
        set_extended_image,
        extended_image_property,
        Option<BoxedValue>
    );
    property!(is_extended, set_is_extended, is_extended_property, bool);
    pub fn title_text(&self) -> String {
        self.get_value(Self::title_text_property())
    }
    pub fn has_extended_content(&self) -> bool {
        !blank(self.extended_description().as_deref()) || self.extended_image().is_some()
    }
    fn start_progressive_timer(&self) {
        self.stop_progressive_timer();
        self.set_is_extended(false);
        if !self.has_extended_content() || !RibbonScreenTipService::is_extended_enabled() {
            return;
        }
        let delay = RibbonScreenTipService::extended_delay();
        if delay.ticks() <= 0 {
            self.set_is_extended(true);
            return;
        }
        let timer = DispatcherTimer::new();
        self.extend_timer.replace(Some(timer.clone()));
        timer.set_interval(delay.to_duration().expect("positive SignedDuration"));
        // A running source timer keeps its ScreenTip alive. Stop clears the one-shot
        // owner before changing IsExtended; no permanent Rust reference cycle remains.
        let this = self.to_ref();
        timer.tick(move |_| {
            this.stop_progressive_timer();
            this.set_is_extended(true);
        });
        timer.start();
    }
    fn stop_progressive_timer(&self) {
        let timer = self.extend_timer.borrow().clone();
        if let Some(timer) = timer {
            timer.stop();
        }
        self.extend_timer.replace(None);
    }
    fn part(&self, name: &str) -> Option<Ref<ferroui_base::layout::Layoutable>> {
        let scope = self.scope.borrow().clone()?;
        scope.find(name)?.cast()
    }
    fn set_visible(&self, name: &str, value: Option<&str>) {
        if let Some(part) = self.part(name) {
            part.set_is_visible(!blank(value));
        }
    }
    fn update(&self) {
        self.set_value(
            Self::title_text_property(),
            format_title(self.title(), self.shortcut()),
        );
        self.set_visible("PART_Description", self.description().as_deref());
        self.set_visible("PART_HelpText", self.help_text().as_deref());
        self.set_visible("PART_DisabledReason", self.disabled_reason().as_deref());
        if let Some(part) = self.part("PART_Image") {
            part.set_is_visible(self.image().is_some());
        }
        let extended = self.is_extended() && self.has_extended_content();
        if let Some(part) = self.part("PART_Extended") {
            part.set_is_visible(extended);
        }
        self.set_visible(
            "PART_ExtendedDescription",
            if extended {
                self.extended_description()
            } else {
                None
            }
            .as_deref(),
        );
        if let Some(part) = self.part("PART_ExtendedImage") {
            part.set_is_visible(extended && self.extended_image().is_some());
        }
        self.set_max_width(if extended { 440. } else { 320. });
    }
}
impl FerroObjectImpl for RibbonScreenTip {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        RibbonTheme::ensure_resources();
        this.loaded(|sender, _| {
            if let Some(this) = sender.downcast_ref::<RibbonScreenTip>() {
                this.start_progressive_timer();
            }
        });
        this.unloaded(|sender, _| {
            if let Some(this) = sender.downcast_ref::<RibbonScreenTip>() {
                this.stop_progressive_timer();
            }
        });
    }
}
impl StyledElementImpl for RibbonScreenTip {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
impl LayoutableImpl for RibbonScreenTip {
    fn apply_template(this: &Self) {
        Self::parent_apply_template(this);
        // Native null-template removal does not call OnApplyTemplate.
        if this.visual_children().count() == 0 {
            this.scope.replace(None);
        }
    }
}
impl TemplatedControlImpl for RibbonScreenTip {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.scope.replace(Some(e.name_scope().clone()));
        this.update();
    }
}
fn blank(value: Option<&str>) -> bool {
    value.is_none_or(|s| trim(&s.encode_utf16().collect::<Vec<_>>()).is_empty())
}
fn format_title(title: Option<String>, shortcut: Option<String>) -> String {
    ModelScreenTip::format_title(
        title.map(RibbonString::from).as_ref(),
        shortcut.map(RibbonString::from).as_ref(),
    )
    .to_utf8()
    .expect("native strings are valid UTF16")
}

/// Conversion fails explicitly when a Core CLR string cannot enter native String.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RibbonScreenTipError {
    pub property: &'static str,
}
impl std::fmt::Display for RibbonScreenTipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ScreenTip.{} contains an unpaired UTF-16 surrogate unsupported by native String",
            self.property
        )
    }
}
impl std::error::Error for RibbonScreenTipError {}

pub struct RibbonScreenTipService;
ferroui_base::ferro_static_type!(RibbonScreenTipService);
ferroui_base::ferro_thread_local! {
    static DELAY:Cell<SignedDuration>=const {Cell::new(SignedDuration::from_ticks(15_000_000))};
    static ENABLED:Cell<bool>=const {Cell::new(true)};
}
impl RibbonScreenTipService {
    pub fn extended_delay() -> SignedDuration {
        DELAY.with(Cell::get)
    }
    pub fn set_extended_delay(value: SignedDuration) {
        DELAY.with(|v| v.set(value));
    }
    pub fn is_extended_enabled() -> bool {
        ENABLED.with(Cell::get)
    }
    pub fn set_is_extended_enabled(value: bool) {
        ENABLED.with(|v| v.set(value));
    }
    pub fn create_tool_tip(content: Option<BoxedValue>) -> Option<Ref<ToolTip>> {
        let content = content?;
        if content
            .downcast_ref::<String>()
            .is_some_and(|s| blank(Some(s)))
        {
            return None;
        }
        let tip = ToolTip::new();
        tip.set_content(Some(content));
        if let Some(style) = Application::current()
            .and_then(|app| {
                app.resources()
                    .try_get_resource(&"RibbonToolTipStyle".into(), None)
                    .flatten()
            })
            .and_then(|v| ValueTypes::as_object(&*v))
            .and_then(|v| v.cast::<ControlTheme>())
        {
            tip.set_theme(style);
        }
        Some(tip)
    }
    pub fn create(
        label: Option<String>,
        screen_tip: Option<BoxedValue>,
        shortcut: Option<String>,
    ) -> Result<Option<BoxedValue>, RibbonScreenTipError> {
        Self::create_with_enabled(label, screen_tip, shortcut, true)
    }
    pub fn create_with_enabled(
        label: Option<String>,
        screen_tip: Option<BoxedValue>,
        shortcut: Option<String>,
        is_enabled: bool,
    ) -> Result<Option<BoxedValue>, RibbonScreenTipError> {
        let title = label.map(|s| s.replace('\n', " "));
        if let Some(control) = screen_tip
            .as_ref()
            .and_then(|v| ValueTypes::as_object(&**v))
            .and_then(|v| v.cast::<RibbonScreenTip>())
        {
            let tip = RibbonScreenTip::new();
            tip.set_title(control.title().filter(|s| !s.is_empty()).or(title));
            tip.set_shortcut(control.shortcut().filter(|s| !s.is_empty()).or(shortcut));
            tip.set_description(control.description());
            tip.set_help_text(control.help_text());
            tip.set_disabled_reason(if is_enabled {
                None
            } else {
                control.disabled_reason()
            });
            tip.set_image(control.image());
            tip.set_extended_description(control.extended_description());
            tip.set_extended_image(control.extended_image());
            return Ok(Some(Control::boxed(tip)));
        }
        if let Some(model) = screen_tip.as_ref().and_then(|v| {
            v.downcast_ref::<ModelScreenTip>()
                .or_else(|| v.downcast_ref::<Rc<ModelScreenTip>>().map(Rc::as_ref))
        }) {
            fn text(
                v: Option<RibbonString>,
                property: &'static str,
            ) -> Result<Option<String>, RibbonScreenTipError> {
                v.map(|s| s.to_utf8().map_err(|_| RibbonScreenTipError { property }))
                    .transpose()
            }
            let tip = RibbonScreenTip::new();
            tip.set_title(text(model.title(), "Title")?.or(title));
            tip.set_shortcut(text(model.shortcut(), "Shortcut")?.or(shortcut));
            tip.set_description(text(model.description(), "Description")?);
            tip.set_help_text(text(model.help_text(), "HelpText")?);
            tip.set_disabled_reason(if is_enabled {
                None
            } else {
                text(model.disabled_reason(), "DisabledReason")?
            });
            tip.set_image(model.image().map(|v| v as BoxedValue));
            tip.set_extended_description(text(
                model.extended_description(),
                "ExtendedDescription",
            )?);
            tip.set_extended_image(model.extended_image().map(|v| v as BoxedValue));
            return Ok(Some(Control::boxed(tip)));
        }
        if let Some(description) = screen_tip
            .as_ref()
            .and_then(|v| v.downcast_ref::<String>())
            .filter(|s| !blank(Some(s)))
        {
            let tip = RibbonScreenTip::new();
            tip.set_title(title);
            tip.set_shortcut(shortcut);
            tip.set_description(Some(description.clone()));
            return Ok(Some(Control::boxed(tip)));
        }
        if screen_tip.is_none() && !blank(title.as_deref()) {
            return Ok(Some(Rc::new(format_title(title, shortcut))));
        }
        Ok(screen_tip)
    }
}
ferroui_base::ferro_markup_type!(static RibbonScreenTipService {
    namespace:"FerroRibbon.Controls",type_info:RibbonScreenTipService,
    static_properties:[
        ExtendedDelay:SignedDuration {get:RibbonScreenTipService::extended_delay,set:RibbonScreenTipService::set_extended_delay},
        IsExtendedEnabled:bool {get:RibbonScreenTipService::is_extended_enabled,set:RibbonScreenTipService::set_is_extended_enabled},
    ],
    methods:[
        static fn CreateToolTip(Option<BoxedValue>)->Option<Ref<ToolTip>> => RibbonScreenTipService::create_tool_tip,
        static try fn Create(Option<String>,Option<BoxedValue>,Option<String>)->Option<BoxedValue> => RibbonScreenTipService::create,
        static try fn Create(Option<String>,Option<BoxedValue>,Option<String>,bool)->Option<BoxedValue> => RibbonScreenTipService::create_with_enabled,
    ],
});

#[cfg(test)]
#[path = "../screen_tip_tests.rs"]
mod tests;
