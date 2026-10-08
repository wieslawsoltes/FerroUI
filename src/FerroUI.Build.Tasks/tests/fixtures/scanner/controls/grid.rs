use ferroui_base::*;

pub struct Grid {
    base: Control,
}

ferro_class! {
    Grid: Control, virtuals GridImpl: ferroui_base::ControlImpl {
        /// Arranges the cells.
        fn arrange_cells(this, size: Size) -> Size;
    }
}

ferro_properties! {
    impl Grid {
        pub fn row_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<Grid, Control, _>("Row", 0)
        }
    }
}

pub struct Layout;

ferro_static_type!(Layout);

ferro_properties! { impl Layout, fn register_layout {
    pub fn spacing_property() -> AttachedProperty<f64> {
        let property = FerroProperty::register_attached_with::<Layout, Grid, _>("Spacing", options());
        property.set_assign_binding(true);
        property
    }

    pub fn odd_property() -> AttachedProperty<f64> {
        build_property()
    }

    pub fn wrong_property() -> Rc<Something> {
        make()
    }
} }
