//! RibbonColorPicker.cs palette, source algorithms and native tree/input services.
//! Copyright Wiesław Šoltés, MIT. Platform/CLR differences are tracked separately.
use crate::{
    compat::FontIcon,
    theming::{RibbonTheme, to_color as native_color, to_ribbon_color as core_color},
};
use ferroribbon_core::collections::RibbonItems;
use ferroribbon_core::{
    RibbonString,
    collections::RibbonList,
    localization::RibbonStrings,
    theming::{RibbonColorPalette as Palette, RibbonColorSwatch},
};
use ferroui_base::{
    BoxedValue, CornerRadius, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, Thickness, TypeInfo, VisualImpl,
    data::core::ValueTypes,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        InputElementImpl, KeyModifiers, KeyboardNavigation, KeyboardNavigationMode,
        NavigationMethod,
        navigation::{XYFocus, XYFocusNavigationModes},
    },
    instantiate,
    interactivity::InteractiveImpl,
    layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment},
    media::{Color, FontWeight, SolidColorBrush},
    metadata::{MarkupDelegate, into_markup_value},
    styling::ControlTheme,
    utilities::EventArgs,
};
use ferroui_controls::{
    Application, Border, Button, Control, ControlImpl, StackPanel, TextBlock, ToolTip,
    automation::AutomationProperties,
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
};
use ferroui_system::collections::{
    object_model::ObservableCollection, specialized::INotifyCollectionChanged,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
pub type RibbonColors = ObservableCollection<Color>;
/// Live, read-only display names belonging to one palette. Obtain an `Rc` from
/// [`RibbonColorPalette::color_names`] and change names through `set_color_name`.
/// Names have no collection notifications. Snapshot iteration is unordered.
///
/// ```compile_fail
/// use ferroribbon::controls::RibbonColorPalette;
/// use ferroui_base::media::Color;
/// let palette = RibbonColorPalette::new();
/// palette.color_names().insert(Color::from_rgb(1, 2, 3), "Name".into());
/// ```
#[derive(Default)]
pub struct RibbonColorNames {
    values: RefCell<HashMap<Color, String>>,
}
impl PartialEq for RibbonColorNames {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl RibbonColorNames {
    pub fn len(&self) -> usize {
        self.values.borrow().len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.borrow().is_empty()
    }
    pub fn contains_key(&self, color: &Color) -> bool {
        self.values.borrow().contains_key(color)
    }
    pub fn get(&self, color: &Color) -> Option<String> {
        self.values.borrow().get(color).cloned()
    }
    /// An owned native map. Changing it cannot change the palette's live names.
    pub fn snapshot(&self) -> HashMap<Color, String> {
        self.values.borrow().clone()
    }
}
fn text(s: Option<RibbonString>) -> Option<String> {
    s.map(|s| {
        s.to_utf8()
            .expect("Invalid UTF16 at native string boundary")
    })
}
#[repr(C)]
pub struct RibbonColorPalette {
    base: TemplatedControl,
    root: Ref<Border>,
    stack: Ref<StackPanel>,
    swatches: RibbonList<Option<Ref<Button>>>,
    previewing: Cell<bool>,
    theme_colors: RibbonColors,
    standard_colors: RibbonColors,
    recent_colors: RibbonColors,
    names: Rc<RibbonColorNames>,
    selected: RefCell<Option<MarkupDelegate>>,
    preview: RefCell<Option<MarkupDelegate>>,
    more: RefCell<Option<MarkupDelegate>>,
    tokens: RefCell<Vec<(RibbonColors, u64)>>,
}
ferro_class!(RibbonColorPalette:TemplatedControl);
ferro_class_info!(RibbonColorPalette{new:RibbonColorPalette::new,markup:{namespace:"FerroRibbon.Controls",properties:[ThemeColors:RibbonColors{get:RibbonColorPalette::theme_colors},StandardColors:RibbonColors{get:RibbonColorPalette::standard_colors},RecentColors:RibbonColors{get:RibbonColorPalette::recent_colors},ColorNames:Rc<RibbonColorNames>{get:RibbonColorPalette::color_names}],methods:[fn SetColorName(Color,Option<String>)=>RibbonColorPalette::set_color_name,fn EndPreview()=>RibbonColorPalette::end_preview,fn FocusSelected()->bool=>RibbonColorPalette::focus_selected,fn Select(Option<Color>)=>RibbonColorPalette::select],events:[ColorSelected(Option<BoxedValue>,Option<Color>){add:RibbonColorPalette::add_color_selected,remove:RibbonColorPalette::remove_color_selected},ColorPreview(Option<BoxedValue>,Option<Color>){add:RibbonColorPalette::add_color_preview,remove:RibbonColorPalette::remove_color_preview},MoreColorsRequested(Option<BoxedValue>,EventArgs){add:RibbonColorPalette::add_more_colors_requested,remove:RibbonColorPalette::remove_more_colors_requested}]}});
ferro_impl_classes!(RibbonColorPalette:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_properties! {impl RibbonColorPalette{
 pub fn selected_color_property()->StyledProperty<Option<Color>>{FerroProperty::register::<Self,_>("SelectedColor",None)}
 pub fn show_automatic_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowAutomatic",true)}
 pub fn automatic_color_property()->StyledProperty<Color>{FerroProperty::register::<Self,_>("AutomaticColor",Color::from_rgb(0,0,0))}
 pub fn show_no_color_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowNoColor",false)}
 pub fn show_more_colors_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowMoreColors",true)}
 pub fn show_theme_colors_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowThemeColors",true)}
}}
macro_rules! prop {
    ($get:ident,$set:ident,$p:ident,$t:ty) => {
        pub fn $get(&self) -> $t {
            self.get_value(Self::$p())
        }
        pub fn $set(&self, v: $t) {
            self.set_value(Self::$p(), v)
        }
    };
}
macro_rules! event {
    ($add:ident,$remove:ident,$field:ident) => {
        pub fn $add(&self, h: Option<MarkupDelegate>) {
            let n = MarkupDelegate::combine(self.$field.borrow().as_ref(), h.as_ref());
            self.$field.replace(n);
        }
        pub fn $remove(&self, h: Option<MarkupDelegate>) {
            let n = MarkupDelegate::remove(self.$field.borrow().as_ref(), h.as_ref());
            self.$field.replace(n);
        }
    };
}
impl RibbonColorPalette {
    fn static_constructor() {
        Self::selected_color_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.update_selection_visuals())
            .detach();
        Self::automatic_color_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.build())
            .detach();
        for p in [
            Self::show_automatic_property(),
            Self::show_no_color_property(),
            Self::show_more_colors_property(),
            Self::show_theme_colors_property(),
        ] {
            p.changed()
                .add_class_handler::<Self>(|c, _| c.build())
                .detach();
        }
    }
    pub fn construct() -> Self {
        let stack = StackPanel::new();
        stack.set_spacing(4.);
        XYFocus::set_navigation_modes(&stack, XYFocusNavigationModes::ENABLED);
        KeyboardNavigation::set_tab_navigation(&stack, KeyboardNavigationMode::Once);
        let root = Border::new();
        root.set_padding(Thickness::uniform(4.));
        root.set_child(Some(stack.clone().upcast::<Control>()));
        Self {
            base: TemplatedControl::construct(),
            root,
            stack,
            swatches: RibbonItems::new(),
            previewing: Cell::new(false),
            theme_colors: RibbonColors::new_managed(),
            standard_colors: RibbonColors::new_managed(),
            recent_colors: RibbonColors::new_managed(),
            names: Rc::new(RibbonColorNames::default()),
            selected: RefCell::new(None),
            preview: RefCell::new(None),
            more: RefCell::new(None),
            tokens: RefCell::new(vec![]),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    prop!(
        selected_color,
        set_selected_color,
        selected_color_property,
        Option<Color>
    );
    prop!(
        show_automatic,
        set_show_automatic,
        show_automatic_property,
        bool
    );
    prop!(
        automatic_color,
        set_automatic_color,
        automatic_color_property,
        Color
    );
    prop!(
        show_no_color,
        set_show_no_color,
        show_no_color_property,
        bool
    );
    prop!(
        show_more_colors,
        set_show_more_colors,
        show_more_colors_property,
        bool
    );
    prop!(
        show_theme_colors,
        set_show_theme_colors,
        show_theme_colors_property,
        bool
    );
    event!(add_color_selected, remove_color_selected, selected);
    event!(add_color_preview, remove_color_preview, preview);
    event!(
        add_more_colors_requested,
        remove_more_colors_requested,
        more
    );
    pub fn theme_colors(&self) -> RibbonColors {
        self.theme_colors.clone()
    }
    pub fn standard_colors(&self) -> RibbonColors {
        self.standard_colors.clone()
    }
    pub fn recent_colors(&self) -> RibbonColors {
        self.recent_colors.clone()
    }
    pub fn color_names(&self) -> Rc<RibbonColorNames> {
        self.names.clone()
    }
    fn raise_color(&self, event: &RefCell<Option<MarkupDelegate>>, color: Option<Color>) {
        let h = event.borrow().clone();
        if let Some(h) = h {
            h.try_invoke(&[into_markup_value(self.to_ref()), into_markup_value(color)])
                .expect("palette event handler");
        }
    }
    pub fn set_color_name(&self, color: Color, name: Option<String>) {
        if let Some(name) = name.filter(|n| !n.is_empty()) {
            if self.names.get(&color).as_ref() == Some(&name) {
                return;
            }
            self.names.values.borrow_mut().insert(color, name);
        } else if self.names.values.borrow_mut().remove(&color).is_none() {
            return;
        }
        self.build();
    }
    fn swatch(&self, color: Color) -> Option<Rc<RibbonColorSwatch>> {
        Some(RibbonColorSwatch::new(
            core_color(color),
            Some(
                self.names
                    .get(&color)
                    .unwrap_or_else(|| core_color(color).to_hex(false))
                    .into(),
            ),
        ))
    }
    pub fn end_preview(&self) {
        if self.previewing.get() {
            self.previewing.set(false);
            self.raise_color(&self.preview, None);
        }
    }
    fn preview(&self, color: Option<Color>) {
        if color.is_none() && !self.previewing.get() {
            return;
        }
        self.previewing.set(color.is_some());
        self.raise_color(&self.preview, color);
    }
    pub fn select(&self, color: Option<Color>) {
        self.end_preview();
        self.set_selected_color(color);
        if let Some(c) = color
            && !self.recent_colors.contains(&c)
            && !Self::is_palette_color(c)
        {
            self.recent_colors.insert(0, c);
            while self.recent_colors.count() > 10 {
                self.recent_colors.remove_at(self.recent_colors.count() - 1);
            }
        }
        self.raise_color(&self.selected, color);
    }
    fn is_palette_color(color: Color) -> bool {
        let c = core_color(color);
        Palette::standard_colors()
            .iter()
            .any(|s| s.as_ref().expect("null swatch").color == c)
            || Palette::build_theme_grid(None)
                .iter()
                .flatten()
                .any(|s| s.as_ref().expect("null swatch").color == c)
    }
    pub fn focus_selected(&self) -> bool {
        let target = self
            .swatches
            .iter()
            .find(|b| {
                b.as_ref()
                    .unwrap()
                    .tag()
                    .as_ref()
                    .and_then(|v| v.downcast_ref::<Color>())
                    .is_some_and(|c| Some(*c) == self.selected_color())
            })
            .or_else(|| self.swatches.iter().next())
            .flatten();
        target.is_some_and(|b| b.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE))
    }
    fn update_selection_visuals(&self) {
        for b in self.swatches.iter() {
            self.update_selection_visual(&b.expect("NullReference: swatch"));
        }
    }
    fn update_selection_visual(&self, b: &Button) {
        let selected = b
            .tag()
            .as_ref()
            .and_then(|v| v.downcast_ref::<Color>())
            .is_some_and(|c| self.selected_color() == Some(*c));
        b.pseudo_classes().set(":ribbon-selected", selected);
    }
    fn header(value: Option<String>) -> Ref<TextBlock> {
        let t = TextBlock::new();
        t.set_text(value.as_deref());
        t.set_font_size(12.);
        t.set_font_weight(FontWeight::SemiBold);
        t.set_margin(Thickness::new(2., 4., 2., 0.));
        t
    }
    fn apply_button_style(button: &Button, key: &str) -> bool {
        let app = Application::current().expect("NullReference: Application.Current");
        let Some(value) = app.resources().try_get_resource(&key.into(), None) else {
            return false;
        };
        let theme = value.map(|value| {
            ValueTypes::as_object(&*value)
                .and_then(|o| o.cast::<ControlTheme>())
                .expect("InvalidCast: palette button style")
        });
        button.set_theme(theme);
        true
    }
    fn command_row(
        &self,
        label: Option<String>,
        color: Option<Color>,
        action: impl Fn(&Self) + 'static,
        glyph: Option<&str>,
    ) -> Ref<Button> {
        let content = StackPanel::new();
        content.set_orientation(Orientation::Horizontal);
        content.set_spacing(8.);
        if let Some(c) = color {
            let b = Border::new();
            b.set_width(16.);
            b.set_height(16.);
            b.set_corner_radius(CornerRadius::uniform(2.));
            b.set_background(Some(SolidColorBrush::with_color(c).into()));
            b.set_border_thickness(Thickness::uniform(1.));
            RibbonTheme::set_theme_brush(
                Some(&b),
                Some(Border::border_brush_property().as_property()),
                Some("RibbonSwatchBorderBrush"),
            )
            .expect("swatch border resource");
            content.children().add(b.upcast::<Control>());
        } else if let Some(glyph) = glyph {
            let icon = FontIcon::new();
            icon.set_glyph(Some(glyph.into()));
            icon.set_font_size(14.);
            content.children().add(icon.upcast::<Control>());
        }
        let text = TextBlock::new();
        text.set_text(label.as_deref());
        text.set_vertical_alignment(VerticalAlignment::Center);
        content.children().add(text.upcast::<Control>());
        let b = Button::new();
        b.set_content(Some(Control::boxed(content)));
        b.set_horizontal_alignment(HorizontalAlignment::Stretch);
        b.set_horizontal_content_alignment(HorizontalAlignment::Left);
        Self::apply_button_style(&b, "RibbonMenuItemButtonStyle");
        let weak = self.to_ref().downgrade();
        b.click(move |_, _| {
            if let Some(c) = weak.upgrade() {
                action(&c)
            }
        });
        b
    }
    fn swatch_row(
        &self,
        swatches: &[Option<Rc<RibbonColorSwatch>>],
        spacing: f64,
    ) -> Ref<StackPanel> {
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        row.set_spacing(if spacing == 0. { 3. } else { spacing });
        row.set_margin(Thickness::new(2., 0., 2., 0.));
        for swatch in swatches {
            let swatch = swatch.as_ref().expect("NullReference: swatch");
            let color = native_color(swatch.color);
            let b = Button::new();
            b.set_width(18.);
            b.set_height(18.);
            b.set_padding(Thickness::uniform(0.));
            b.set_min_width(0.);
            b.set_min_height(0.);
            b.set_background(Some(SolidColorBrush::with_color(color).into()));
            b.set_border_thickness(Thickness::uniform(1.));
            b.set_corner_radius(CornerRadius::uniform(2.));
            b.set_tag(Some(Rc::new(color)));
            if Self::apply_button_style(&b, "RibbonSwatchButtonStyle") {
                b.set_background(Some(SolidColorBrush::with_color(color).into()));
            }
            let name = text(swatch.name.clone());
            ToolTip::set_tip(&b, name.clone().map(|s| Rc::new(s) as BoxedValue));
            AutomationProperties::set_name(&b, name.as_deref());
            let weak = self.to_ref().downgrade();
            b.click(move |_, _| {
                if let Some(c) = weak.upgrade() {
                    c.select(Some(color));
                }
            });
            let weak = self.to_ref().downgrade();
            b.pointer_entered(move |_, _| {
                if let Some(c) = weak.upgrade() {
                    c.preview(Some(color));
                }
            });
            let weak = self.to_ref().downgrade();
            b.pointer_exited(move |_, _| {
                if let Some(c) = weak.upgrade() {
                    c.preview(None);
                }
            });
            let weak = self.to_ref().downgrade();
            b.loaded(move |sender, _| {
                if let Some(c) = weak.upgrade() {
                    c.update_selection_visual(&sender.to_ref().cast::<Button>().unwrap());
                }
            });
            self.swatches.add(Some(b.clone()));
            row.children().add(b.upcast::<Control>());
        }
        row
    }
    fn build(&self) {
        let strings = RibbonStrings::current();
        self.stack.children().clear();
        self.swatches.clear();
        if self.show_automatic() {
            self.stack.children().add(
                self.command_row(
                    text(strings.automatic()),
                    Some(self.automatic_color()),
                    |c| c.select(Some(c.automatic_color())),
                    None,
                )
                .upcast::<Control>(),
            );
        }
        if self.show_theme_colors() {
            self.stack
                .children()
                .add(Self::header(text(strings.theme_colors())).upcast::<Control>());
            let base = if self.theme_colors.count() > 0 {
                self.theme_colors
                    .iter()
                    .map(|c| self.swatch(c))
                    .collect::<Vec<_>>()
            } else {
                Palette::office_theme_colors().to_vec()
            };
            let grid = Palette::build_theme_grid(Some(&base));
            self.stack
                .children()
                .add(self.swatch_row(&grid[0], 3.).upcast::<Control>());
            let shades = StackPanel::new();
            shades.set_spacing(0.);
            for r in grid.iter().skip(1) {
                shades
                    .children()
                    .add(self.swatch_row(r, 0.).upcast::<Control>());
            }
            self.stack.children().add(shades.upcast::<Control>());
        }
        self.stack
            .children()
            .add(Self::header(text(strings.standard_colors())).upcast::<Control>());
        let standard = if self.standard_colors.count() > 0 {
            self.standard_colors
                .iter()
                .map(|c| self.swatch(c))
                .collect::<Vec<_>>()
        } else {
            Palette::standard_colors().to_vec()
        };
        self.stack
            .children()
            .add(self.swatch_row(&standard, 3.).upcast::<Control>());
        if self.recent_colors.count() > 0 {
            self.stack
                .children()
                .add(Self::header(text(strings.recent_colors())).upcast::<Control>());
            let recent = self
                .recent_colors
                .iter()
                .take(10)
                .map(|c| self.swatch(c))
                .collect::<Vec<_>>();
            self.stack
                .children()
                .add(self.swatch_row(&recent, 3.).upcast::<Control>());
        }
        if self.show_no_color() {
            self.stack.children().add(
                self.command_row(
                    text(strings.no_color()),
                    None,
                    |c| c.select(None),
                    Some("\u{e711}"),
                )
                .upcast::<Control>(),
            );
        }
        if self.show_more_colors() {
            self.stack.children().add(
                self.command_row(
                    text(strings.more_colors()),
                    None,
                    |c| {
                        let h = c.more.borrow().clone();
                        if let Some(h) = h {
                            h.try_invoke(&[
                                into_markup_value(c.to_ref()),
                                Some(crate::compat::empty_event_args()),
                            ])
                            .expect("MoreColorsRequested handler");
                        }
                    },
                    Some("\u{e790}"),
                )
                .upcast::<Control>(),
            );
        }
    }
}
impl Drop for RibbonColorPalette {
    fn drop(&mut self) {
        for (c, t) in self.tokens.get_mut().drain(..) {
            c.collection_changed().remove(t);
        }
    }
}
impl FerroObjectImpl for RibbonColorPalette {
    fn constructed(c: &Self) {
        Self::parent_constructed(c);
        RibbonTheme::ensure_resources();
        for collection in [c.theme_colors(), c.standard_colors(), c.recent_colors()] {
            let weak = c.to_ref().downgrade();
            let token = collection.collection_changed().add(Rc::new(move |_| {
                if let Some(c) = weak.upgrade() {
                    c.build();
                }
            }));
            c.tokens.borrow_mut().push((collection, token));
        }
        let weak = c.to_ref().downgrade();
        c.unloaded(move |_, _| {
            if let Some(c) = weak.upgrade() {
                c.end_preview();
            }
        });
        c.build();
    }
}
impl StyledElementImpl for RibbonColorPalette {
    fn style_key_override(_: &Self) -> &'static TypeInfo {
        Self::TYPE
    }
}
impl TemplatedControlImpl for RibbonColorPalette {
    fn on_apply_template(c: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(c, e);
        if let Some(host) = e.name_scope().find_as::<Border>("PART_Host") {
            if let Some(old) = c.root.visual_parent().and_then(|v| v.cast::<Border>()) {
                old.set_child(None);
            }
            host.set_child(Some(c.root.clone().upcast::<Control>()));
        }
    }
}

#[cfg(test)]
#[path = "../color_palette_tests.rs"]
mod tests;
