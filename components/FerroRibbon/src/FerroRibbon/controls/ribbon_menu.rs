//! RibbonButton.cs Menu helpers and RibbonItemHelper.cs menu conversion/projection.
use super::RibbonItemHelper;
use crate::compat::{
    BitmapIcon, BitmapIconSource, FontIcon, FontIconSource, ImageIcon, ImageIconSource,
    MenuFlyoutItem, MenuFlyoutSubItem, PathIconSource, SymbolIcon, SymbolIconSource,
};
use ferroribbon_core::{
    RibbonString,
    model::{RibbonIcon, RibbonIconKind, RibbonIconLayer},
};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{
    AttachedProperty, BoxedValue, FerroObject, FerroProperty, Ref, ferro_properties,
    ferro_static_type, media::Geometry, metadata::MarkupDelegate,
};
use ferroui_controls::{
    Application, Control, Flyout, IconElement, MenuFlyout, PathIcon, primitives::FlyoutBase,
};
use std::{cell::RefCell, rc::Rc};
struct RustAction(Box<dyn Fn()>);
impl PartialEq for RustAction {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class RustAction{this:Rc<RustAction>,handles:[Rc<RustAction>],methods:[fn Invoke()=>|this:&Rc<RustAction>|(this.0)()]});
pub(crate) fn action_delegate(f: impl Fn() + 'static) -> MarkupDelegate {
    ValueTypes::register_reference::<RustAction>();
    MarkupDelegate::for_method(
        Some(Rc::new(RustAction(Box::new(f)))),
        RustAction::MARKUP,
        &RustAction::MARKUP.methods[0],
    )
}
type RustClickFn = dyn Fn(Option<BoxedValue>, ferroui_base::interactivity::RoutedEventArgs);
struct RustClick(Box<RustClickFn>);
impl PartialEq for RustClick {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class RustClick{this:Rc<RustClick>,handles:[Rc<RustClick>],methods:[fn Invoke(Option<BoxedValue>,ferroui_base::interactivity::RoutedEventArgs)=>|this:&Rc<RustClick>,sender,args|(this.0)(sender,args)]});
pub(crate) fn click_delegate(
    f: impl Fn(Option<BoxedValue>, ferroui_base::interactivity::RoutedEventArgs) + 'static,
) -> MarkupDelegate {
    ValueTypes::register_reference::<RustClick>();
    MarkupDelegate::for_method(
        Some(Rc::new(RustClick(Box::new(f)))),
        RustClick::MARKUP,
        &RustClick::MARKUP.methods[0],
    )
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RibbonMenuError(pub String);
impl std::fmt::Display for RibbonMenuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for RibbonMenuError {}
ferroui_base::ferro_thread_local! {static ICON_CONVERTER:RefCell<Option<MarkupDelegate>>=const{RefCell::new(None)};}
pub struct RibbonMenu;
ferro_static_type!(RibbonMenu);
ferro_properties! {impl RibbonMenu{pub fn icon_property()->AttachedProperty<Option<BoxedValue>>{FerroProperty::register_attached::<Self,FerroObject,_>("Icon",None)}}}
impl RibbonMenu {
    fn static_constructor() {
        Self::icon_property()
            .changed()
            .add_class_handler::<FerroObject>(|target, e| {
                let converted =
                    RibbonItemHelper::create_menu_icon(e.get_new_value::<Option<BoxedValue>>())
                        .expect("attached menu icon conversion");
                if let Some(item) = target.downcast_ref::<MenuFlyoutItem>() {
                    item.set_icon(converted.map(Control::boxed));
                } else if let Some(sub) = target.downcast_ref::<MenuFlyoutSubItem>() {
                    sub.set_icon(converted.map(Control::boxed));
                }
            })
            .detach();
    }
    pub fn get_icon(target: &FerroObject) -> Option<BoxedValue> {
        target.get_value(Self::icon_property())
    }
    pub fn set_icon(target: &FerroObject, value: Option<BoxedValue>) {
        target.set_value(Self::icon_property(), value);
    }
    pub fn set_invoke(item: &MenuFlyoutItem, action: Option<MarkupDelegate>) {
        item.set_invoker(action);
    }
    pub fn get_invoke(item: &MenuFlyoutItem) -> Option<MarkupDelegate> {
        item.invoker()
    }
    pub fn item(
        text: Option<String>,
        action: Option<MarkupDelegate>,
        icon: Option<BoxedValue>,
        shortcut: Option<String>,
    ) -> Result<Ref<MenuFlyoutItem>, RibbonMenuError> {
        let item = MenuFlyoutItem::new();
        item.set_text(text);
        item.set_icon(RibbonItemHelper::create_menu_icon(icon)?.map(Control::boxed));
        if let Some(shortcut) = shortcut {
            item.set_keyboard_accelerator_text_override(Some(shortcut));
        }
        let invoke = action.clone();
        item.add_click_action(move |_, _| {
            invoke
                .as_ref()
                .expect("NullReference: menu action")
                .try_invoke(&[])
                .expect("menu action invocation");
        });
        Self::set_invoke(&item, action);
        Ok(item)
    }
    pub fn apply_theme(flyout: Option<&FlyoutBase>) {
        let Some(flyout) = flyout else { return };
        let (key, missing) = if let Some(menu) = flyout.downcast_ref::<MenuFlyout>() {
            (
                "RibbonMenuFlyoutPresenterStyle",
                menu.flyout_presenter_theme().is_none(),
            )
        } else if let Some(content) = flyout.downcast_ref::<Flyout>() {
            (
                "RibbonFlyoutPresenterStyle",
                content.flyout_presenter_theme().is_none(),
            )
        } else {
            return;
        };
        if !missing {
            return;
        }
        let style = Application::current()
            .and_then(|app| {
                app.resources()
                    .try_get_resource(&key.into(), None)
                    .flatten()
            })
            .and_then(|value| ValueTypes::as_object(&*value))
            .and_then(|o| o.cast::<ferroui_base::styling::ControlTheme>());
        if let Some(style) = style {
            if let Some(menu) = flyout.downcast_ref::<MenuFlyout>() {
                menu.set_flyout_presenter_theme(style);
            } else if let Some(content) = flyout.downcast_ref::<Flyout>() {
                content.set_flyout_presenter_theme(style);
            }
        }
    }
}
impl RibbonItemHelper {
    pub fn menu_icon_converter() -> Option<MarkupDelegate> {
        ICON_CONVERTER.with_borrow(Clone::clone)
    }
    pub fn set_menu_icon_converter(value: Option<MarkupDelegate>) {
        let old = ICON_CONVERTER.with_borrow_mut(|slot| std::mem::replace(slot, value));
        drop(old);
    }
    pub fn create_menu_icon(
        icon: Option<BoxedValue>,
    ) -> Result<Option<Ref<IconElement>>, RibbonMenuError> {
        let Some(icon) = icon else { return Ok(None) };
        if let Some(converter) = Self::menu_icon_converter() {
            let converted = converter
                .try_invoke(&[Some(icon.clone())])
                .map_err(|error| RibbonMenuError(format!("{error}")))?;
            if let Some(value) = converted {
                if let Some(o) = ValueTypes::as_object(&*value) {
                    return o.cast::<IconElement>().map(Some).ok_or(RibbonMenuError(
                        "MenuIconConverter returned a non-icon".into(),
                    ));
                }
                if ValueTypes::normalize(value).is_some() {
                    return Err(RibbonMenuError(
                        "MenuIconConverter returned a non-icon".into(),
                    ));
                }
            }
        }
        if let Some(text) = icon.downcast_ref::<String>() {
            return create_description(Some(&RibbonString::from(text.clone())), None);
        }
        if let Some(text) = icon.downcast_ref::<RibbonString>() {
            return create_description(Some(text), None);
        }
        if let Some(model) = icon.downcast_ref::<Rc<RibbonIcon>>() {
            return create_description(model.value.as_ref(), Some(model.kind));
        }
        if let Some(object) = ValueTypes::as_object(&*icon) {
            if let Some(font) = object.cast::<FontIconSource>() {
                let result = FontIcon::new();
                result.set_glyph(font.glyph());
                return Ok(Some(result.upcast()));
            }
            if let Some(path) = object.cast::<PathIconSource>() {
                let result = PathIcon::new();
                result.set_data(path.data());
                return Ok(Some(result.upcast()));
            }
            if let Some(symbol) = object.cast::<SymbolIconSource>() {
                return SymbolIcon::with_symbol(symbol.symbol())
                    .map(|s| Some(s.upcast()))
                    .map_err(|e| RibbonMenuError(e.to_string()));
            }
            if let Some(bitmap) = object.cast::<BitmapIconSource>() {
                let result = BitmapIcon::new();
                result.set_uri_source(bitmap.uri_source());
                result.set_show_as_monochrome(bitmap.show_as_monochrome());
                return Ok(Some(result.upcast()));
            }
            if let Some(image) = object.cast::<ImageIconSource>() {
                let result = ImageIcon::new();
                result.set_source(image.image_source());
                return Ok(Some(result.upcast()));
            }
            if let Some(font) = object.cast::<FontIcon>() {
                let result = FontIcon::new();
                result.set_glyph(font.glyph());
                return Ok(Some(result.upcast()));
            }
            if let Some(bitmap) = object.cast::<BitmapIcon>() {
                let result = BitmapIcon::new();
                result.set_uri_source(bitmap.uri_source());
                result.set_show_as_monochrome(bitmap.show_as_monochrome());
                return Ok(Some(result.upcast()));
            }
            if let Some(image) = object.cast::<ImageIcon>() {
                let result = ImageIcon::new();
                result.set_source(image.source());
                return Ok(Some(result.upcast()));
            }
            if let Some(symbol) = object.cast::<SymbolIcon>() {
                return SymbolIcon::with_symbol(symbol.symbol())
                    .map(|s| Some(s.upcast()))
                    .map_err(|e| RibbonMenuError(e.to_string()));
            }
        }
        Ok(None)
    }
    pub fn create_menu_item(
        item: &Control,
        label: Option<String>,
        icon: Option<BoxedValue>,
        invoke: Option<MarkupDelegate>,
        checked: Option<bool>,
    ) -> Result<Ref<Control>, RibbonMenuError> {
        create_menu_item_core(item, label, icon, checked, move || {
            invoke
                .as_ref()
                .expect("NullReference: overflow action")
                .try_invoke(&[])
                .expect("overflow action");
        })
    }
}
fn create_description(
    value: Option<&RibbonString>,
    kind: Option<RibbonIconKind>,
) -> Result<Option<Ref<IconElement>>, RibbonMenuError> {
    let kind = kind.unwrap_or_else(|| {
        value
            .map(crate::primitives::RibbonIconPresenter::classify)
            .unwrap_or(RibbonIconKind::Text)
    });
    if kind == RibbonIconKind::Glyph {
        let font = FontIcon::new();
        font.set_glyph(
            value
                .map(|v| v.to_utf8())
                .transpose()
                .map_err(|_| RibbonMenuError("glyph has unpaired UTF-16".into()))?,
        );
        return Ok(Some(font.upcast()));
    }
    if kind == RibbonIconKind::Path {
        let layers = RibbonIconLayer::parse(value);
        if layers.count() == 0
            || layers
                .iter()
                .any(|l| l.expect("source parsed layer").stroke_thickness() > 0.)
        {
            return Ok(None);
        }
        let data = layers
            .iter()
            .map(|l| {
                l.expect("source parsed layer")
                    .data()
                    .as_ref()
                    .map(|s| s.to_utf8())
                    .transpose()
                    .map(|s| s.unwrap_or_default())
            })
            .collect::<Result<Vec<_>, _>>();
        let Ok(data) = data else { return Ok(None) };
        let Ok(data) = Geometry::parse(&data.join(" ")) else {
            return Ok(None);
        };
        let path = PathIcon::new();
        path.set_data(data);
        return Ok(Some(path.upcast()));
    }
    if kind == RibbonIconKind::Image {
        let value = value.ok_or_else(|| RibbonMenuError("image URI is null".into()))?;
        let Some(source) =
            crate::primitives::RibbonIconPresenter::create_display_uri_source(value, None)
        else {
            return Ok(None);
        };
        let image = ImageIcon::new();
        image.set_source(Some(source));
        return Ok(Some(image.upcast()));
    }
    Ok(None)
}
ferroui_base::ferro_markup_type!(static RibbonMenu{type_info:RibbonMenu,namespace:"FerroRibbon.Controls",methods:[
 static fn GetIcon(Ref<FerroObject>)->Option<BoxedValue> =>|o:Ref<FerroObject>|RibbonMenu::get_icon(&o),
 static fn SetIcon(Ref<FerroObject>,Option<BoxedValue>)=>|o:Ref<FerroObject>,v|RibbonMenu::set_icon(&o,v),
 static fn GetInvoke(Ref<MenuFlyoutItem>)->Option<MarkupDelegate> =>|o:Ref<MenuFlyoutItem>|RibbonMenu::get_invoke(&o),
 static fn SetInvoke(Ref<MenuFlyoutItem>,Option<MarkupDelegate>)=>|o:Ref<MenuFlyoutItem>,v|RibbonMenu::set_invoke(&o,v),
 static fn ApplyTheme(Option<Ref<FlyoutBase>>)=>|f:Option<Ref<FlyoutBase>>|RibbonMenu::apply_theme(f.as_deref()),
 static try fn Item(Option<String>,Option<MarkupDelegate>)->Ref<MenuFlyoutItem> => |text,action|RibbonMenu::item(text,action,None,None),
 static try fn Item(Option<String>,Option<MarkupDelegate>,Option<BoxedValue>)->Ref<MenuFlyoutItem> => |text,action,icon|RibbonMenu::item(text,action,icon,None),
 static try fn Item(Option<String>,Option<MarkupDelegate>,Option<BoxedValue>,Option<String>)->Ref<MenuFlyoutItem> =>RibbonMenu::item,
]});
/// Internal Rust action carrier; the public metadata overload still uses Action/MarkupDelegate.
pub(crate) fn overflow_entry(
    item: &Control,
    label: Option<String>,
    icon: Option<BoxedValue>,
    checked: Option<bool>,
    invoke: impl Fn() + 'static,
) -> Ref<Control> {
    create_menu_item_core(item, label, icon, checked, invoke).expect("overflow menu conversion")
}
fn create_menu_item_core(
    item: &Control,
    label: Option<String>,
    icon: Option<BoxedValue>,
    checked: Option<bool>,
    invoke: impl Fn() + 'static,
) -> Result<Ref<Control>, RibbonMenuError> {
    let menu = if let Some(checked) = checked {
        let t = crate::compat::ToggleMenuFlyoutItem::new();
        t.set_is_checked(checked);
        t.upcast::<MenuFlyoutItem>()
    } else {
        MenuFlyoutItem::new()
    };
    menu.set_text(Some(label.unwrap_or_default()));
    menu.set_icon(RibbonItemHelper::create_menu_icon(icon)?.map(Control::boxed));
    menu.set_is_enabled(RibbonItemHelper::is_enabled(item));
    menu.add_click_action(move |_, _| invoke());
    let internal = ValueTypes::try_cast(
        &Control::boxed(item.to_ref()),
        ValueType::of::<Rc<dyn super::ribbon_item_helper::IRibbonItemInternal>>(),
    )
    .and_then(|v| {
        v.downcast_ref::<Rc<dyn super::ribbon_item_helper::IRibbonItemInternal>>()
            .cloned()
    });
    if let Some(shortcut) = internal
        .and_then(|i| i.shortcut())
        .filter(|s| !s.is_empty())
    {
        menu.set_keyboard_accelerator_text_override(Some(
            shortcut
                .to_utf8()
                .map_err(|_| RibbonMenuError("menu shortcut has unpaired UTF-16".into()))?,
        ));
    }
    Ok(menu.upcast())
}
