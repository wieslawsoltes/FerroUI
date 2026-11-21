// Copyright (c) 2026 Wiesław Šoltés. MIT. Native original ExcelPage view/adapters.
use crate::{
    demo_settings::{DemoSettings, DemoSubscription},
    excel_sheet::{ExcelAlignment, ExcelCellKey, ExcelCellSurface, ExcelSheet, ExcelSheetSurface},
    excel_view_model::ExcelViewModel,
};
use ferroribbon::controls::{
    Ribbon, RibbonColorPicker, RibbonItemInvokedEventArgs, RibbonTitleBar,
};
use ferroribbon_core::{
    RibbonString,
    model::{RibbonGalleryItemModel, RibbonGridSize, RibbonModelExt, RibbonNodeModelImpl},
    theming::RibbonColor,
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, Thickness, VisualImpl, WeakRef,
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::{InputElement, InputElementImpl},
    instantiate,
    interactivity::InteractiveImpl,
    layout::{HorizontalAlignment, LayoutableImpl},
    media::{
        Color, FontFamily, FontStyle, FontWeight, IBrush, SolidColorBrush, TextDecorationCollection,
    },
    metadata::{MarkupDelegate, from_markup_value},
};
use ferroui_controls::{
    Border, ContentControlImpl, Control, ControlImpl, Grid, TextBlock, UserControl,
    primitives::TemplatedControlImpl,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{
    any::Any,
    cell::{OnceCell, RefCell},
    collections::HashMap,
    rc::Rc,
};
#[repr(C)]
pub struct ExcelPage {
    base: UserControl,
    model: OnceCell<Rc<ExcelViewModel>>,
    sheet: OnceCell<Rc<ExcelSheet>>,
    settings: OnceCell<Rc<DemoSettings>>,
    cells: RefCell<HashMap<ExcelCellKey, Ref<TextBlock>>>,
    cleanup: RefCell<Vec<Box<dyn FnOnce()>>>,
    settings_subscription: RefCell<Option<DemoSubscription>>,
}
ferro_class!(ExcelPage:UserControl);
ferro_class_info!(ExcelPage{new:ExcelPage::new,markup:{namespace:"FerroRibbon.Sample.Pages",properties:[ViewModel:Rc<ExcelViewModel>{get:ExcelPage::view_model}]}});
ferro_impl_classes!(ExcelPage:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl ExcelPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            model: OnceCell::new(),
            sheet: OnceCell::new(),
            settings: OnceCell::new(),
            cells: RefCell::new(HashMap::new()),
            cleanup: RefCell::new(Vec::new()),
            settings_subscription: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        Self::new_with_settings(DemoSettings::current())
    }
    pub fn new_with_settings(settings: Rc<DemoSettings>) -> Ref<Self> {
        let vm = ExcelViewModel::new(Rc::new(format_parameter));
        let this = instantiate(Self::construct());
        assert!(this.model.set(vm.clone()).is_ok());
        if let Some(populate) = ferroui_markup_xaml::FerroXamlLoader::populate_override(Self::TYPE)
        {
            populate(None, &Control::boxed(this.clone())).expect("Excel populate override");
        } else {
            crate::generated::compiled_excel_page::populate(None, &this)
                .expect("compiled original Excel page");
        }
        this.named::<RibbonTitleBar>("TitleBar")
            .set_ribbon(Some(this.ribbon()));
        let grid = this.named::<Grid>("Sheet");
        assert_eq!(
            grid.children().count(),
            325,
            "complete original worksheet visual loop"
        );
        for row in 1..=24 {
            for column in 1..=12 {
                let b = grid
                    .children()
                    .get((row * 13 + column) as usize)
                    .cast::<Border>()
                    .expect("sheet Border");
                let text = b
                    .child()
                    .unwrap()
                    .cast::<TextBlock>()
                    .expect("sheet TextBlock");
                this.cells.borrow_mut().insert((row, column), text);
                let weak = this.downgrade();
                let token = b.add_handler(InputElement::tapped_event(), move |_, _| {
                    if let Some(page) = weak.upgrade() {
                        page.sheet().select_command().execute(Some(&(row, column)));
                    }
                });
                let weak = b.downgrade();
                this.cleanup.borrow_mut().push(Box::new(move || {
                    if let Some(b) = weak.upgrade() {
                        b.remove_handler(InputElement::tapped_event(), token);
                    }
                }));
            }
        }
        let sheet = ExcelSheet::new(vm.clone(), Rc::new(NativeSheet(this.downgrade())));
        assert!(this.sheet.set(sheet).is_ok());
        let weak_settings = Rc::downgrade(&settings);
        let weak_vm = Rc::downgrade(&vm);
        let token = vm.property_changed().add(Rc::new(move |name| {
            if name == "Status"
                && let Some(vm) = weak_vm.upgrade()
                && let Some(settings) = weak_settings.upgrade()
            {
                settings
                    .write(Some(&format!(
                        "Excel · {}",
                        vm.status().unwrap_or_default()
                    )))
                    .expect("Excel source log");
            }
        }));
        let weak_vm = Rc::downgrade(&vm);
        this.cleanup.borrow_mut().push(Box::new(move || {
            if let Some(vm) = weak_vm.upgrade() {
                vm.property_changed().remove(token);
            }
        }));
        let weak = this.downgrade();
        let invoked = MarkupDelegate::new(move |args| {
            if let Some(page) = weak.upgrade() {
                let e = from_markup_value::<Rc<RibbonItemInvokedEventArgs>>(&args[1])
                    .expect("Excel invocation");
                page.view_model().item_invoked(
                    e.item_id
                        .as_ref()
                        .map(|s| s.to_utf8().expect("native Excel ID")),
                    e.command_id
                        .as_ref()
                        .map(|s| s.to_utf8().expect("native Excel command ID")),
                );
            }
            None
        });
        this.ribbon().add_item_invoked(Some(invoked.clone()));
        let weak = this.ribbon().downgrade();
        this.cleanup.borrow_mut().push(Box::new(move || {
            if let Some(r) = weak.upgrade() {
                r.remove_item_invoked(Some(invoked));
            }
        }));
        assert!(this.settings.set(settings.clone()).is_ok());
        let weak = this.downgrade();
        this.settings_subscription
            .replace(Some(settings.changed(move || {
                if let Some(page) = weak.upgrade() {
                    page.apply_settings();
                }
            })));
        this.apply_settings();
        this
    }
    fn apply_settings(&self) {
        let settings = self.settings.get().unwrap();
        self.view_model().ribbon().set_density(settings.density());
        if let Some(mode) = settings.display_mode() {
            self.view_model().ribbon().set_display_mode(mode);
        }
    }
    pub fn view_model(&self) -> Rc<ExcelViewModel> {
        self.model.get().expect("Excel model initialized").clone()
    }
    pub fn sheet(&self) -> Rc<ExcelSheet> {
        self.sheet.get().expect("Excel sheet initialized").clone()
    }
    pub fn ribbon(&self) -> Ref<Ribbon> {
        self.named("Ribbon")
    }
    pub(crate) fn named<T: ferroui_base::ObjectType>(&self, name: &str) -> Ref<T> {
        ferroui_base::controls::NameScopeExtensions::find(self, name)
            .unwrap_or_else(|| panic!("missing Excel element {name}"))
    }
    pub(crate) fn cell_text(&self, key: ExcelCellKey) -> Option<Ref<TextBlock>> {
        self.cells.borrow().get(&key).cloned()
    }
}
impl Drop for ExcelPage {
    fn drop(&mut self) {
        self.settings_subscription.get_mut().take();
        for remove in self.cleanup.get_mut().drain(..) {
            remove();
        }
    }
}
struct NativeSheet(WeakRef<ExcelPage>);
impl ExcelSheetSurface for NativeSheet {
    fn cell(&self, key: ExcelCellKey) -> Option<Rc<dyn ExcelCellSurface>> {
        let text = self.0.upgrade()?.cell_text(key)?;
        let border = text.parent()?.cast::<Border>()?;
        Some(Rc::new(NativeCell { text, border }))
    }
}
struct NativeCell {
    text: Ref<TextBlock>,
    border: Ref<Border>,
}
fn brush(c: RibbonColor) -> Rc<dyn IBrush> {
    SolidColorBrush::with_color(Color::from_argb(c.a, c.r, c.g, c.b)).into()
}
impl ExcelCellSurface for NativeCell {
    fn font_weight(&self) -> u16 {
        self.text.font_weight().0 as u16
    }
    fn italic(&self) -> bool {
        self.text.font_style() == FontStyle::Italic
    }
    fn set_border_color(&self, c: RibbonColor) {
        self.border.set_border_brush(Some(brush(c)));
    }
    fn set_border_widths(&self, l: f64, t: f64, r: f64, b: f64) {
        self.border.set_border_thickness(Thickness::new(l, t, r, b));
    }
    fn set_font_weight(&self, w: u16) {
        self.text.set_font_weight(FontWeight(w as i32));
    }
    fn set_italic(&self, b: bool) {
        self.text.set_font_style(if b {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        });
    }
    fn set_underline(&self, b: bool) {
        self.text.set_text_decorations(if b {
            Some(TextDecorationCollection::parse("Underline").unwrap())
        } else {
            None
        });
    }
    fn set_foreground(&self, c: RibbonColor) {
        self.text.set_foreground(Some(brush(c)));
    }
    fn set_background(&self, c: RibbonColor) {
        self.border.set_background(Some(brush(c)));
    }
    fn set_font_family(&self, f: RibbonString) -> Result<(), String> {
        let f = f.to_utf8().map_err(|_| {
            "Native FontFamily boundary cannot represent an unpaired UTF-16 surrogate".to_owned()
        })?;
        self.text.set_font_family(FontFamily::new(&f));
        Ok(())
    }
    fn set_font_size(&self, n: f64) {
        self.text.set_font_size(n);
    }
    fn set_alignment(&self, a: ExcelAlignment) {
        self.text.set_horizontal_alignment(match a {
            ExcelAlignment::Left => HorizontalAlignment::Left,
            ExcelAlignment::Center => HorizontalAlignment::Center,
            ExcelAlignment::Right => HorizontalAlignment::Right,
        });
    }
}
// Concrete native example payloads; applications can supply their own formatter
// through ExcelViewModel::new for custom Rust payloads. No object/CLR runtime.
fn format_parameter(p: &dyn Any) -> Result<String, String> {
    let sentinel = RibbonColorPicker::no_color_parameter();
    let reference: &dyn Any = sentinel.as_ref();
    if std::ptr::eq(p, reference) {
        return Ok("System.Object".into());
    }
    if let Some(s) = p.downcast_ref::<RibbonString>() {
        return s.to_utf8().map_err(|e| e.to_string());
    }
    if let Some(s) = p.downcast_ref::<RibbonGridSize>() {
        return Ok(s.to_string());
    }
    if let Some(s) = p.downcast_ref::<RibbonColor>() {
        return Ok(s.to_string());
    }
    if let Some(s) = p.downcast_ref::<RibbonGalleryItemModel>() {
        return s.to_ribbon_string().to_utf8().map_err(|e| e.to_string());
    }
    macro_rules! native {($($ty:ty),*)=>{$(if let Some(v)=p.downcast_ref::<$ty>(){let value:BoxedValue=Rc::new(v.clone());return super::word_page::word_parameter_text(&value);})*};}
    native!(
        String, bool, i8, u8, i16, u16, i32, u32, i64, u64, f32, f64, Color
    );
    Err("Unregistered Excel command parameter display; supply a formatter for the application payload".into())
}
