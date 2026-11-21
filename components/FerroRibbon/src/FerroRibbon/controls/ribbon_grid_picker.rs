//! RibbonPickers.cs: original grid selection, command order and retained template content.
//! Copyright Wiesław Šoltés, MIT. Native ownership/platform differences are recorded separately.
use super::RibbonItemHelper;
use crate::theming::RibbonTheme;
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{
    localization::{RibbonFormatArgument, RibbonStrings},
    model::RibbonGridSize,
};
use ferroui_base::{
    BoxedValue, CornerRadius, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, Thickness, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties,
    input::{
        FocusChangedEventArgs, ICommand, InputElement, InputElementImpl, InputElementImplExt, Key,
        KeyEventArgs, NavigationMethod,
    },
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{Color, SolidColorBrush},
    metadata::MarkupDelegate,
};
use ferroui_controls::{
    Border, ColumnDefinition, Control, ControlImpl, Grid, GridLength, GridUnitType, RowDefinition,
    StackPanel, TextBlock, TopLevel,
    automation::{
        AutomationProperties,
        peers::{AutomationControlType, AutomationPeer, ControlAutomationPeer},
    },
    primitives::{
        Popup, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
        TemplatedControlImplExt,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[repr(C)]
pub struct RibbonGridPicker {
    base: TemplatedControl,
    grid: Ref<Grid>,
    caption: Ref<TextBlock>,
    // Native StackPanel has no padding. This retained Border supplies source _root.Padding.
    root: Ref<Border>,
    hover: Cell<RibbonGridSize>,
    picked: RefCell<Option<MarkupDelegate>>,
}
ferro_class!(RibbonGridPicker:TemplatedControl);
ferro_class_info!(RibbonGridPicker{new:RibbonGridPicker::new,markup:{namespace:"FerroRibbon.Controls",
 properties:[HighlightedSize:RibbonGridSize{get:RibbonGridPicker::highlighted_size}],
 methods:[fn Highlight(RibbonGridSize)=>RibbonGridPicker::highlight,fn Pick(RibbonGridSize)=>RibbonGridPicker::pick],
 events:[SizePicked(Option<BoxedValue>,RibbonGridSize){add:RibbonGridPicker::add_size_picked,remove:RibbonGridPicker::remove_size_picked}]
}});
ferro_impl_classes!(RibbonGridPicker:VisualImpl,LayoutableImpl,InteractiveImpl);

impl ControlImpl for RibbonGridPicker {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        ControlAutomationPeer::new(this).upcast()
    }
}

ferro_properties! {impl RibbonGridPicker {
 pub fn rows_property()->StyledProperty<i32>{FerroProperty::register::<Self,_>("Rows",8)}
 pub fn columns_property()->StyledProperty<i32>{FerroProperty::register::<Self,_>("Columns",10)}
 pub fn command_property()->StyledProperty<Option<Rc<dyn ICommand>>>{FerroProperty::register::<Self,_>("Command",None)}
}}
impl RibbonGridPicker {
    fn static_constructor() {
        AutomationProperties::control_type_override_property()
            .override_default_value::<Self>(Some(AutomationControlType::Group));
        Self::rows_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.build())
            .detach();
        Self::columns_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.build())
            .detach();
    }
    pub fn construct() -> Self {
        let grid = Grid::new();
        grid.set_row_spacing(2.);
        grid.set_column_spacing(2.);
        let caption = TextBlock::new();
        caption.set_margin(Thickness::new(2., 0., 2., 6.));
        let stack = StackPanel::new();
        stack.children().add(caption.clone().upcast::<Control>());
        stack.children().add(grid.clone().upcast::<Control>());
        let root = Border::new();
        root.set_padding(Thickness::uniform(4.));
        root.set_child(Some(stack.upcast()));
        Self {
            base: TemplatedControl::construct(),
            grid,
            caption,
            root,
            hover: Cell::new(RibbonGridSize::new(0, 0)),
            picked: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn rows(&self) -> i32 {
        self.get_value(Self::rows_property())
    }
    pub fn set_rows(&self, v: i32) {
        self.set_value(Self::rows_property(), v)
    }
    pub fn columns(&self) -> i32 {
        self.get_value(Self::columns_property())
    }
    pub fn set_columns(&self, v: i32) {
        self.set_value(Self::columns_property(), v)
    }
    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }
    pub fn set_command(&self, v: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), v)
    }
    pub fn highlighted_size(&self) -> RibbonGridSize {
        self.hover.get()
    }
    pub fn add_size_picked(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.picked.borrow().as_ref(), h.as_ref());
        self.picked.replace(next);
    }
    pub fn remove_size_picked(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.picked.borrow().as_ref(), h.as_ref());
        self.picked.replace(next);
    }
    fn build(&self) {
        self.grid.children().clear();
        self.grid.row_definitions().clear();
        self.grid.column_definitions().clear();
        for _ in 0..self.rows() {
            let d = RowDefinition::new();
            d.set_height(GridLength::new(16., GridUnitType::Pixel));
            self.grid.row_definitions().add(d);
        }
        for _ in 0..self.columns() {
            let d = ColumnDefinition::new();
            d.set_width(GridLength::new(16., GridUnitType::Pixel));
            self.grid.column_definitions().add(d);
        }
        for r in 0..self.rows() {
            for c in 0..self.columns() {
                let cell = Border::new();
                cell.set_border_thickness(Thickness::uniform(1.));
                cell.set_corner_radius(CornerRadius::uniform(1.));
                cell.set_tag(Some(Rc::new(RibbonGridSize::new(r + 1, c + 1))));
                cell.set_background(Some(
                    SolidColorBrush::with_color(Color::from_argb(0, 255, 255, 255)).into(),
                ));
                Grid::set_row(&cell, r);
                Grid::set_column(&cell, c);
                // Cell backedges follow native ownership; no control/root/cell strong cycle.
                let weak = self.to_ref().downgrade();
                cell.pointer_entered(move |sender, _| {
                    if let Some(p) = weak.upgrade() {
                        let cell = sender.to_ref().cast::<Border>().expect("grid cell");
                        let tag = cell.tag().expect("grid cell Tag");
                        p.highlight(
                            *tag.downcast_ref::<RibbonGridSize>()
                                .expect("RibbonGridSize Tag"),
                        );
                    }
                });
                let weak = self.to_ref().downgrade();
                cell.add_handler(InputElement::tapped_event(), move |sender, _| {
                    if let Some(p) = weak.upgrade() {
                        let cell = sender.to_ref().cast::<Border>().expect("grid cell");
                        let tag = cell.tag().expect("grid cell Tag");
                        p.pick(
                            *tag.downcast_ref::<RibbonGridSize>()
                                .expect("RibbonGridSize Tag"),
                        );
                    }
                });
                self.grid.children().add(cell.upcast::<Control>());
            }
        }
        self.highlight(RibbonGridSize::new(0, 0));
    }
    pub fn highlight(&self, size: RibbonGridSize) {
        self.hover.set(size);
        let strings = RibbonStrings::current();
        let text = if size.rows == 0 {
            strings.insert_table()
        } else {
            Some(
                strings
                    .format(
                        Some(&"TablePickerFormat".into()),
                        Some(RibbonFixedItems::new(vec![
                            RibbonFormatArgument::I32(size.columns),
                            RibbonFormatArgument::I32(size.rows),
                        ])),
                    )
                    .expect("TablePickerFormat"),
            )
        };
        let text = text.map(|s| {
            s.to_utf8()
                .expect("native grid caption cannot represent unpaired UTF16")
        });
        self.caption.set_text(text.as_deref());
        let on = RibbonTheme::get_brush(Some(self), Some("RibbonAccentSubtleStrongBrush"))
            .expect("grid brush");
        let border =
            RibbonTheme::get_brush(Some(self), Some("RibbonAccentBrush")).expect("grid brush");
        let off =
            RibbonTheme::get_brush(Some(self), Some("RibbonInputBorderBrush")).expect("grid brush");
        for child in self.grid.children().to_vec() {
            if let Some(cell) = child.cast::<Border>() {
                let tag = cell.tag().expect("grid cell Tag");
                let s = tag
                    .downcast_ref::<RibbonGridSize>()
                    .expect("RibbonGridSize Tag");
                let active = s.rows <= size.rows && s.columns <= size.columns;
                cell.set_background(if active {
                    on.clone().map(Into::into)
                } else {
                    Some(SolidColorBrush::with_color(Color::from_argb(0, 255, 255, 255)).into())
                });
                cell.set_border_brush(if active {
                    border.clone().map(Into::into)
                } else {
                    off.clone().map(Into::into)
                });
            }
        }
        AutomationProperties::set_name(self, text.as_deref());
    }
    pub fn pick(&self, size: RibbonGridSize) {
        let owner = RibbonItemHelper::find_owner(Some(self));
        let callback = self.picked.borrow().clone();
        if let Some(h) = callback {
            h.try_invoke(&[Some(Control::boxed(self.to_ref())), Some(Rc::new(size))])
                .expect("SizePicked handler");
        }
        if let Some(command) = self.command() {
            let parameter = Some(Rc::new(size) as BoxedValue);
            if command.can_execute(parameter.as_ref()) {
                command.execute(parameter.as_ref());
            }
        }
        if let Some(owner) = owner {
            owner.on_item_invoked(self.to_ref().upcast(), None, Some(Rc::new(size)));
        }
        self.close_host_popup();
        self.highlight(RibbonGridSize::new(0, 0));
    }
    fn close_host_popup(&self) {
        // PopupRoot is a separate native visual tree. Follow its hosting Popup to
        // the owning top level, then enumerate open descendants in opening order.
        let mut current = Some(self.to_ref().upcast::<FerroObject>());
        let mut top = None;
        while let Some(node) = current {
            if let Some(root) = node.downcast_ref::<TopLevel>() {
                top = Some(root.to_ref());
                break;
            }
            current = Self::parent_of(&node);
        }
        let Some(mut top) = top else { return };
        while let Some(hosted) = top.as_hosted_visual_tree_root() {
            let Some(host) = hosted.host() else { break };
            let Some(parent) = TopLevel::get_top_level(Some(&host)) else {
                break;
            };
            if parent.ptr_eq(&top) {
                break;
            }
            top = parent;
        }
        fn find(popups: Vec<Ref<Popup>>, picker: &RibbonGridPicker) -> Option<Ref<Popup>> {
            for p in popups {
                if p.child().is_some_and(|child| picker.is_within(&child)) {
                    return Some(p);
                }
                if let Some(found) = find(p.opened_popups(), picker) {
                    return Some(found);
                }
            }
            None
        }
        if let Some(p) = find(top.opened_popups(), self) {
            p.set_is_open(false);
        }
    }
    // Exact source IsWithin walk: a logical parent is consulted only when
    // there is no visual parent. This matters before template materialization.
    fn parent_of(node: &FerroObject) -> Option<Ref<FerroObject>> {
        node.downcast_ref::<ferroui_base::Visual>()
            .and_then(|v| v.visual_parent())
            .map(|p| p.upcast())
            .or_else(|| {
                node.downcast_ref::<Control>()
                    .and_then(|c| c.parent())
                    .map(|p| p.upcast())
            })
    }
    fn is_within(&self, ancestor: &FerroObject) -> bool {
        let mut current = Some(self.to_ref().upcast::<FerroObject>());
        while let Some(node) = current {
            if node.ptr_eq(&ancestor.to_ref()) {
                return true;
            }
            current = Self::parent_of(&node);
        }
        false
    }
    fn on_picker_key_down(&self, e: &KeyEventArgs) {
        let mut s = if self.hover.get().rows == 0 {
            RibbonGridSize::new(1, 1)
        } else {
            self.hover.get()
        };
        match e.key {
            Key::Right => s.columns = self.columns().min(s.columns.wrapping_add(1)),
            Key::Left => s.columns = 1.max(s.columns.wrapping_sub(1)),
            Key::Down => s.rows = self.rows().min(s.rows.wrapping_add(1)),
            Key::Up => s.rows = 1.max(s.rows.wrapping_sub(1)),
            Key::Home => s = RibbonGridSize::new(1, 1),
            Key::Enter | Key::Space => {
                self.pick(s);
                e.set_handled(true);
                return;
            }
            _ => return,
        }
        self.highlight(s);
        e.set_handled(true);
    }
}
impl StyledElementImpl for RibbonGridPicker {
    fn style_key_override(_: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}
impl FerroObjectImpl for RibbonGridPicker {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        RibbonTheme::ensure_resources();
        this.set_focusable(true);
        let weak = this.to_ref().downgrade();
        this.key_down(move |_, e| {
            if let Some(p) = weak.upgrade() {
                p.on_picker_key_down(e)
            }
        });
        let weak = this.to_ref().downgrade();
        this.actual_theme_variant_changed(move || {
            if let Some(p) = weak.upgrade() {
                p.highlight(p.hover.get())
            }
        })
        .detach();
        this.build();
    }
}
impl InputElementImpl for RibbonGridPicker {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        if matches!(
            e.navigation_method,
            NavigationMethod::Tab | NavigationMethod::Directional
        ) && this.hover.get().rows == 0
        {
            this.highlight(RibbonGridSize::new(1, 1));
        }
    }
}
impl TemplatedControlImpl for RibbonGridPicker {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        if let Some(host) = e.name_scope().find_as::<Border>("PART_Host") {
            if let Some(old) = this.root.visual_parent().and_then(|v| v.cast::<Border>()) {
                old.set_child(None);
            }
            host.set_child(Some(this.root.clone().upcast()));
        }
    }
}
#[cfg(test)]
#[path = "../grid_picker_tests.rs"]
mod tests;
