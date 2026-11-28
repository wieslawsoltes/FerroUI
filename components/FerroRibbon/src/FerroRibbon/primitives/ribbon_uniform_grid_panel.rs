use super::ribbon_hidden_focus;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::utilities::math_utilities::max;
use ferroui_base::{
    FerroObjectImpl, FerroProperty, Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
};
use ferroui_controls::{Control, ControlImpl, Panel, PanelImpl};
use std::{cell::Cell, rc::Rc};

#[repr(C)]
pub struct RibbonUniformGridPanel {
    base: Panel,
    first_row: Cell<i32>,
    max_rows: Cell<i32>,
}
ferro_class!(RibbonUniformGridPanel:Panel);
ferro_impl_classes!(RibbonUniformGridPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_class_info!(RibbonUniformGridPanel {
    new:RibbonUniformGridPanel::new,
    markup:{
        namespace:"FerroRibbon.Controls.Primitives",
        content:Children,
        properties:[
            FirstRow:i32 {get:RibbonUniformGridPanel::first_row,set:RibbonUniformGridPanel::set_first_row},
            MaxRows:i32 {get:RibbonUniformGridPanel::max_rows,set:RibbonUniformGridPanel::set_max_rows},
            RowCount:i32 {get:RibbonUniformGridPanel::row_count},
        ],
    },
});
ferro_properties! {impl RibbonUniformGridPanel {
    pub fn columns_property()->StyledProperty<i32> {FerroProperty::register::<RibbonUniformGridPanel,_>("Columns",4)}
    pub fn item_width_property()->StyledProperty<f64> {FerroProperty::register::<RibbonUniformGridPanel,_>("ItemWidth",64.)}
    pub fn item_height_property()->StyledProperty<f64> {FerroProperty::register::<RibbonUniformGridPanel,_>("ItemHeight",56.)}
    pub fn spacing_property()->StyledProperty<f64> {FerroProperty::register::<RibbonUniformGridPanel,_>("Spacing",2.)}
}}
impl RibbonUniformGridPanel {
    fn static_constructor() {
        Layoutable::affects_measure::<RibbonUniformGridPanel>(&[
            Self::columns_property().as_property(),
            Self::item_width_property().as_property(),
            Self::item_height_property().as_property(),
            Self::spacing_property().as_property(),
        ]);
    }
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            first_row: Cell::new(0),
            max_rows: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn columns(&self) -> i32 {
        self.get_value(Self::columns_property())
    }
    pub fn set_columns(&self, value: i32) {
        self.set_value(Self::columns_property(), value);
    }
    pub fn item_width(&self) -> f64 {
        self.get_value(Self::item_width_property())
    }
    pub fn set_item_width(&self, value: f64) {
        self.set_value(Self::item_width_property(), value);
    }
    pub fn item_height(&self) -> f64 {
        self.get_value(Self::item_height_property())
    }
    pub fn set_item_height(&self, value: f64) {
        self.set_value(Self::item_height_property(), value);
    }
    pub fn spacing(&self) -> f64 {
        self.get_value(Self::spacing_property())
    }
    pub fn set_spacing(&self, value: f64) {
        self.set_value(Self::spacing_property(), value);
    }
    pub fn first_row(&self) -> i32 {
        self.first_row.get()
    }
    pub fn set_first_row(&self, value: i32) {
        self.first_row.set(value);
    }
    pub fn max_rows(&self) -> i32 {
        self.max_rows.get()
    }
    pub fn set_max_rows(&self, value: i32) {
        self.max_rows.set(value);
    }
    pub fn row_count(&self) -> i32 {
        if self.columns() <= 0 {
            return 0;
        }
        (self
            .children()
            .snapshot()
            .iter()
            .filter(|c| c.is_visible())
            .count() as f64
            / self.columns() as f64)
            .ceil() as i32
    }
}
impl LayoutableImpl for RibbonUniformGridPanel {
    fn measure_override(this: &Self, _available: Size) -> Size {
        let size = Size::new(this.item_width(), this.item_height());
        each_child(this, |child| child.measure(size));
        let rows = if this.max_rows() > 0 {
            this.max_rows().min(1.max(this.row_count()))
        } else {
            this.row_count()
        };
        let columns = 1.max(this.columns());
        Size::new(
            columns as f64 * this.item_width() + (columns - 1) as f64 * this.spacing(),
            max(
                0.,
                rows as f64 * this.item_height() + (rows - 1) as f64 * this.spacing(),
            ),
        )
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let columns = 1.max(this.columns());
        let mut index = 0_i32;
        each_child(this, |child| {
            if !child.is_visible() {
                return;
            }
            let row = (index / columns).wrapping_sub(this.first_row());
            let column = index % columns;
            index = index.wrapping_add(1);
            if row < 0 || (this.max_rows() > 0 && row >= this.max_rows()) {
                child.arrange(Rect::new(0., 0., 0., 0.));
                ribbon_hidden_focus::set(&child.clone().upcast(), true);
                return;
            }
            child.arrange(Rect::new(
                column as f64 * (this.item_width() + this.spacing()),
                row as f64 * (this.item_height() + this.spacing()),
                this.item_width(),
                this.item_height(),
            ));
            ribbon_hidden_focus::set(&child.clone().upcast(), false);
        });
        final_size
    }
}
fn each_child(panel: &RibbonUniformGridPanel, mut action: impl FnMut(&Ref<Control>)) {
    // A retained snapshot lets us detect mutations, rather than silently enumerating stale children.
    let children = panel.children();
    let snapshot = children.snapshot();
    for child in snapshot.iter() {
        assert!(
            Rc::ptr_eq(&snapshot, &children.snapshot()),
            "RibbonUniformGridPanel children changed during enumeration"
        );
        action(child);
    }
    assert!(
        Rc::ptr_eq(&snapshot, &children.snapshot()),
        "RibbonUniformGridPanel children changed during enumeration"
    );
}
