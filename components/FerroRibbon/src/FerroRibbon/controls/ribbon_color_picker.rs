//! RibbonColorPicker.cs: owned palette/flyout, source split action and exact copy forwarding.
//! Copyright Wiesław Šoltés, MIT.
use super::{
    RibbonColorDialog, RibbonColorPalette, RibbonControlBaseImpl, RibbonItemHelper,
    RibbonSplitButton, RibbonSplitButtonImpl, RibbonSplitButtonImplExt,
};
use ferroribbon_core::RibbonItemSize;
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, TypeInfo, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{Color, SolidColorBrush},
    metadata::{MarkupDelegate, from_markup_value, into_markup_value},
    reactive::IDisposable,
    styling::ControlTheme,
};
use ferroui_controls::{
    Application, Control, ControlImpl, Flyout, PlacementMode, primitives::TemplatedControlImpl,
};
use std::{cell::RefCell, rc::Rc};
ferroui_base::ferro_thread_local! {static NO_COLOR:BoxedValue=Rc::new(NoColor);}
struct NoColor;
impl PartialEq for NoColor {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
#[repr(C)]
pub struct RibbonColorPicker {
    base: RibbonSplitButton,
    palette: Ref<RibbonColorPalette>,
    private_flyout: Ref<Flyout>,
    selected: RefCell<Option<MarkupDelegate>>,
    preview: RefCell<Option<MarkupDelegate>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
}
ferro_class!(RibbonColorPicker:RibbonSplitButton);
ferro_class_info!(RibbonColorPicker{new:RibbonColorPicker::new,markup:{namespace:"FerroRibbon.Controls",static_properties:[NoColorParameter:BoxedValue{get:RibbonColorPicker::no_color_parameter}],properties:[Palette:Ref<RibbonColorPalette>{get:RibbonColorPicker::palette},ShowAutomatic:bool{get:RibbonColorPicker::show_automatic,set:RibbonColorPicker::set_show_automatic},ShowNoColor:bool{get:RibbonColorPicker::show_no_color,set:RibbonColorPicker::set_show_no_color},ShowMoreColors:bool{get:RibbonColorPicker::show_more_colors,set:RibbonColorPicker::set_show_more_colors},AutomaticColor:Color{get:RibbonColorPicker::automatic_color,set:RibbonColorPicker::set_automatic_color}],events:[ColorSelected(Option<BoxedValue>,Option<Color>){add:RibbonColorPicker::add_color_selected,remove:RibbonColorPicker::remove_color_selected},ColorPreview(Option<BoxedValue>,Option<Color>){add:RibbonColorPicker::add_color_preview,remove:RibbonColorPicker::remove_color_preview}]}});
ferro_impl_classes!(RibbonColorPicker:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
ferro_properties! {impl RibbonColorPicker{
 pub fn selected_color_property()->StyledProperty<Option<Color>>{FerroProperty::register::<Self,_>("SelectedColor",Some(Color::from_rgb(255,0,0)))}
 pub fn is_split_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsSplit",true)}
}}
macro_rules! proxy {
    ($get:ident,$set:ident,$t:ty) => {
        pub fn $get(&self) -> $t {
            self.palette.$get()
        }
        pub fn $set(&self, value: $t) {
            self.palette.$set(value)
        }
    };
}
impl RibbonColorPicker {
    fn static_constructor() {
        Self::selected_color_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.update_color_bar())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: RibbonSplitButton::construct(),
            palette: RibbonColorPalette::new(),
            private_flyout: Flyout::new(),
            selected: RefCell::default(),
            preview: RefCell::default(),
            subscriptions: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn selected_color(&self) -> Option<Color> {
        self.get_value(Self::selected_color_property())
    }
    pub fn set_selected_color(&self, v: Option<Color>) {
        self.set_value(Self::selected_color_property(), v)
    }
    pub fn is_split(&self) -> bool {
        self.get_value(Self::is_split_property())
    }
    pub fn set_is_split(&self, v: bool) {
        self.set_value(Self::is_split_property(), v)
    }
    pub fn palette(&self) -> Ref<RibbonColorPalette> {
        self.palette.clone()
    }
    pub fn no_color_parameter() -> BoxedValue {
        NO_COLOR.with(Clone::clone)
    }
    proxy!(show_automatic, set_show_automatic, bool);
    proxy!(show_no_color, set_show_no_color, bool);
    proxy!(show_more_colors, set_show_more_colors, bool);
    proxy!(automatic_color, set_automatic_color, Color);
    pub fn add_color_selected(&self, h: Option<MarkupDelegate>) {
        let n = MarkupDelegate::combine(self.selected.borrow().as_ref(), h.as_ref());
        self.selected.replace(n);
    }
    pub fn remove_color_selected(&self, h: Option<MarkupDelegate>) {
        let n = MarkupDelegate::remove(self.selected.borrow().as_ref(), h.as_ref());
        self.selected.replace(n);
    }
    pub fn add_color_preview(&self, h: Option<MarkupDelegate>) {
        let n = MarkupDelegate::combine(self.preview.borrow().as_ref(), h.as_ref());
        self.preview.replace(n);
    }
    pub fn remove_color_preview(&self, h: Option<MarkupDelegate>) {
        let n = MarkupDelegate::remove(self.preview.borrow().as_ref(), h.as_ref());
        self.preview.replace(n);
    }
    fn selected_event(&self, color: Option<Color>) {
        let h = self.selected.borrow().clone();
        self.color_event(h, color);
    }
    fn preview_event(&self, color: Option<Color>) {
        let h = self.preview.borrow().clone();
        self.color_event(h, color);
    }
    fn color_event(&self, h: Option<MarkupDelegate>, color: Option<Color>) {
        if let Some(h) = h {
            h.try_invoke(&[
                Some(Control::boxed(self.to_ref())),
                into_markup_value(color),
            ])
            .expect("picker color handler");
        }
    }
    fn update_color_bar(&self) {
        let color = self.selected_color();
        self.palette.set_selected_color(color);
        self.set_color_bar(Some(
            SolidColorBrush::with_color(color.unwrap_or(Color::from_argb(0, 255, 255, 255))).into(),
        ));
        self.set_command_parameter(color.map(|c| Rc::new(c) as BoxedValue));
    }
    fn execute_color(&self, color: Option<Color>) {
        self.execute_command(Some(
            color.map_or_else(Self::no_color_parameter, |c| Rc::new(c) as BoxedValue),
        ))
        .expect("picker color command");
    }
    fn show_more_colors_dialog(&self) {
        self.private_flyout.hide();
        let root = ferroui_controls::TopLevel::get_top_level(Some(self));
        let initial = self.selected_color().unwrap_or(Color::from_rgb(0, 0, 0));
        let theme = self.actual_theme_variant();
        let source = self.to_ref();
        let request = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            RibbonColorDialog::show_async(root, initial, theme)
        }));
        match request {
            Ok(future) => super::continue_color_dialog(future, move |result| {
                source.complete_more_colors(result)
            }),
            Err(e) => eprintln!("RibbonColorPicker: More Colors dialog failed: {e:?}"),
        }
    }
    fn complete_more_colors(&self, result: Result<Option<Color>, super::RibbonColorDialogError>) {
        match result {
            Ok(Some(color)) => {
                let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.palette.select(Some(color))
                }));
                if let Err(e) = r {
                    eprintln!("RibbonColorPicker: More Colors dialog failed: {e:?}");
                }
            }
            Ok(None) => {}
            Err(e) => eprintln!("RibbonColorPicker: More Colors dialog failed: {e}"),
        }
    }
}
impl FerroObjectImpl for RibbonColorPicker {
    fn constructed(c: &Self) {
        Self::parent_constructed(c);
        c.set_size(RibbonItemSize::Small);
        if c.icon().is_none() {
            c.set_icon(Some(Rc::new("\u{e8d3}".to_owned())));
        }
        c.private_flyout
            .set_content(Some(Control::boxed(c.palette.clone())));
        c.private_flyout
            .set_placement(PlacementMode::BottomEdgeAlignedLeft);
        if let Some(app) = Application::current()
            && let Some(style) = app
                .resources()
                .try_get_resource(&"RibbonFlyoutPresenterStyle".into(), None)
        {
            c.private_flyout.set_flyout_presenter_theme(style.map(|v| {
                ferroui_base::data::core::ValueTypes::as_object(&*v)
                    .and_then(|o| o.cast::<ControlTheme>())
                    .expect("InvalidCast: flyout presenter style")
            }));
        }
        c.set_flyout(Some(c.private_flyout.clone().upcast()));
        let weak = c.to_ref().downgrade();
        c.subscriptions.borrow_mut().push(
            c.private_flyout
                .opening(move |_| {
                    if let Some(c) = weak.upgrade() {
                        crate::theming::RibbonTheme::set_theme(
                            Some(&c.palette),
                            c.actual_theme_variant(),
                        )
                        .expect("palette theme");
                    }
                })
                .detach(),
        );
        let weak = c.to_ref().downgrade();
        c.subscriptions.borrow_mut().push(
            c.private_flyout
                .opened(move || {
                    if let Some(c) = weak.upgrade() {
                        c.palette.focus_selected();
                    }
                })
                .detach(),
        );
        let weak = c.to_ref().downgrade();
        c.subscriptions.borrow_mut().push(
            c.private_flyout
                .closed(move || {
                    if let Some(c) = weak.upgrade() {
                        c.palette.end_preview();
                    }
                })
                .detach(),
        );
        let weak = c.to_ref().downgrade();
        c.palette
            .add_color_selected(Some(MarkupDelegate::new(move |a| {
                if let Some(c) = weak.upgrade() {
                    let color =
                        from_markup_value::<Option<Color>>(&a[1]).expect("palette ColorSelected");
                    c.private_flyout.hide();
                    c.set_selected_color(color);
                    c.selected_event(color);
                    c.execute_color(color);
                }
                None
            })));
        let weak = c.to_ref().downgrade();
        c.palette
            .add_color_preview(Some(MarkupDelegate::new(move |a| {
                if let Some(c) = weak.upgrade() {
                    c.preview_event(
                        from_markup_value::<Option<Color>>(&a[1]).expect("palette ColorPreview"),
                    );
                }
                None
            })));
        let weak = c.to_ref().downgrade();
        c.palette
            .add_more_colors_requested(Some(MarkupDelegate::new(move |_| {
                if let Some(c) = weak.upgrade() {
                    c.show_more_colors_dialog();
                }
                None
            })));
        c.update_color_bar();
    }
}
impl StyledElementImpl for RibbonColorPicker {
    fn style_key_override(_: &Self) -> &'static TypeInfo {
        Self::TYPE
    }
}
impl RibbonSplitButtonImpl for RibbonColorPicker {
    fn invoke_primary(c: &Self) {
        if !c.is_split() {
            c.open_drop_down();
            return;
        }
        Self::parent_invoke_primary(c);
    }
}
impl RibbonControlBaseImpl for RibbonColorPicker {
    fn create_linked_copy_core(c: &Self) -> Option<Ref<Control>> {
        let copy = Self::new();
        RibbonItemHelper::link_common(c, &copy);
        RibbonItemHelper::link(
            c,
            &copy,
            Self::selected_color_property().as_property(),
            true,
        );
        RibbonItemHelper::link_one_way(c, &copy, Self::is_split_property().as_property());
        copy.set_show_automatic(c.show_automatic());
        copy.set_show_no_color(c.show_no_color());
        copy.set_show_more_colors(c.show_more_colors());
        let source = c.to_ref();
        copy.add_color_selected(Some(MarkupDelegate::new(move |a| {
            let color = from_markup_value::<Option<Color>>(&a[1]).expect("copy color");
            source.selected_event(color);
            source.execute_color(color);
            None
        })));
        let source = c.to_ref();
        copy.add_color_preview(Some(MarkupDelegate::new(move |a| {
            source.preview_event(from_markup_value::<Option<Color>>(&a[1]).expect("copy preview"));
            None
        })));
        let source = c.to_ref();
        let weak = copy.downgrade();
        copy.add_click(Some(super::ribbon_menu::click_delegate(move |_, _| {
            if let Some(copy) = weak.upgrade()
                && copy.is_split()
            {
                source
                    .execute_command(source.selected_color().map(|c| Rc::new(c) as BoxedValue))
                    .expect("copied picker primary");
            }
        })));
        Some(copy.upcast())
    }
}
impl Drop for RibbonColorPicker {
    fn drop(&mut self) {
        for s in self.subscriptions.get_mut().drain(..) {
            s.dispose();
        }
    }
}
#[cfg(test)]
#[path = "../color_picker_tests.rs"]
mod tests;
