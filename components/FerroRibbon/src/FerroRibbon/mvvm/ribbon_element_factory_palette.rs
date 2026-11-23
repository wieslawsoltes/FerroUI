//! Original BindPalette/Replace: materialize before equality, names before colors,
//! recent collections limited in each transfer, explicit two-way cleanup.
//! Copyright Wiesław Šoltés, MIT.
use super::{RibbonElementFactory, RibbonModelBindings};
use crate::{
    controls::RibbonColorPalette,
    theming::{to_color as native_color, to_ribbon_color as core_color},
};
use ferroribbon_core::model::{
    RibbonColorPickerModelExt, RibbonColorPickerModelImpl, RibbonItemModelExt, RibbonNodeModelExt,
};
use ferroui_base::{
    Ref,
    media::Color,
    metadata::{MarkupDelegate, from_markup_value},
};
use ferroui_controls::{Flyout, TopLevel};
use ferroribbon_core::collections::ObservableCollection;
use ferroui_system::collections::{
    specialized::INotifyCollectionChanged,
};
use std::{cell::Cell, rc::Rc};
struct Reset<'a>(&'a Cell<bool>);
impl Drop for Reset<'_> {
    fn drop(&mut self) {
        self.0.set(false)
    }
}
fn replace<T: Clone + PartialEq + Default + 'static>(
    target: &ObservableCollection<T>,
    values: impl Iterator<Item = T>,
) {
    let values = values.collect::<Vec<_>>();
    if target.iter().eq(values.iter().cloned()) {
        return;
    }
    target.clear();
    for value in values {
        target.add(value);
    }
}
impl RibbonElementFactory {
    /// Source CreateFlyoutPalette. The palette owns selection visuals; this
    /// factory path alone invokes the Core command with a Core RibbonColor.
    pub(super) fn create_flyout_palette(
        &self,
        model: Rc<dyn RibbonColorPickerModelImpl>,
        flyout: &Flyout,
        bindings: &Rc<RibbonModelBindings>,
    ) -> Ref<RibbonColorPalette> {
        let palette = RibbonColorPalette::new();
        let p = palette.clone();
        let m = model.clone();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                p.set_show_automatic(m.show_automatic());
                p.set_show_no_color(m.show_no_color());
                p.set_show_more_colors(m.show_more_colors());
                p.set_automatic_color(native_color(m.automatic_color()));
                p.set_selected_color(m.selected_color().map(native_color));
                p.set_is_visible(m.is_visible());
                p.set_is_enabled(m.is_enabled());
            })),
            Some(
                [
                    "ShowAutomatic",
                    "ShowNoColor",
                    "ShowMoreColors",
                    "AutomaticColor",
                    "SelectedColor",
                    "IsVisible",
                    "IsEnabled",
                ]
                .into_iter()
                .map(|s| Some(s.into()))
                .collect(),
            ),
        );
        self.bind_palette(Some(&palette), Some(model.clone()), Some(bindings));

        let factory = self.to_ref();
        let weak_flyout = flyout.to_ref().downgrade();
        let selected = MarkupDelegate::new(move |args| {
            let color = from_markup_value::<Option<Color>>(&args[1])
                .expect("ColorSelected color")
                .map(core_color);
            model.set_selected_color(color);
            let command = model
                .command()
                .or_else(|| factory.resolve_command(model.command_id()));
            let parameter = color.as_ref().map(|c| c as &dyn std::any::Any);
            Self::execute(command.as_deref(), parameter);
            if let Some(flyout) = weak_flyout.upgrade() {
                flyout.hide();
            }
            None
        });
        palette.add_color_selected(Some(selected.clone()));
        let weak_palette = palette.downgrade();
        let weak_flyout = flyout.to_ref().downgrade();
        let more = MarkupDelegate::new(move |_| {
            if let Some(palette) = weak_palette.upgrade() {
                // Capture the owner/theme BEFORE hiding and detaching the popup.
                let root = TopLevel::get_top_level(Some(&*palette));
                let theme = palette.actual_theme_variant();
                if let Some(flyout) = weak_flyout.upgrade() {
                    flyout.hide();
                }
                let future = crate::controls::RibbonColorDialog::show_async(
                    root,
                    palette.selected_color().unwrap_or(Color::from_rgb(0, 0, 0)),
                    theme,
                );
                // Like the source async event, retain the same palette across
                // await, including when its popup is removed in the meantime.
                crate::controls::continue_color_dialog(future, move |result| {
                    complete_flyout_color(&palette, result);
                });
            }
            None
        });
        palette.add_more_colors_requested(Some(more.clone()));
        let p = palette.clone();
        bindings.add(Some(Rc::new(move || {
            p.remove_color_selected(Some(selected.clone()));
            p.remove_more_colors_requested(Some(more.clone()));
        })));
        palette
    }

    pub fn bind_palette(
        &self,
        palette: Option<&RibbonColorPalette>,
        model: Option<Rc<dyn RibbonColorPickerModelImpl>>,
        bindings: Option<&Rc<RibbonModelBindings>>,
    ) {
        let palette = palette.expect("ArgumentNull: palette");
        let model = model.expect("ArgumentNull: model");
        let bindings = bindings.expect("ArgumentNull: bindings");
        for colors in [model.theme_colors(), model.standard_colors()] {
            let source = colors.clone();
            let p = palette.to_ref();
            let is_theme = colors == model.theme_colors();
            bindings.collection(
                Some(Rc::new(colors)),
                Some(Rc::new(move || {
                    for swatch in source.iter() {
                        let swatch = swatch.expect("NullReference: swatch");
                        p.set_color_name(
                            native_color(swatch.color),
                            swatch.name.as_ref().map(|s| {
                                s.to_utf8()
                                    .expect("Invalid UTF16 at native string boundary")
                            }),
                        );
                    }
                    let target = if is_theme {
                        p.theme_colors()
                    } else {
                        p.standard_colors()
                    };
                    replace(
                        &target,
                        source
                            .iter()
                            .map(|s| native_color(s.expect("NullReference: swatch").color)),
                    );
                })),
            );
        }
        let syncing = Rc::new(Cell::new(false));
        let sync = syncing.clone();
        let m = model.clone();
        let p = palette.to_ref();
        bindings.collection(
            Some(Rc::new(model.recent_colors())),
            Some(Rc::new(move || {
                if sync.get() {
                    return;
                }
                sync.set(true);
                let _finally = Reset(&sync);
                replace(
                    &p.recent_colors(),
                    m.recent_colors().iter().take(10).map(native_color),
                );
            })),
        );
        let weak_palette = palette.to_ref().downgrade();
        let weak_scope = Rc::downgrade(bindings);
        let recent = palette.recent_colors();
        let token = recent.collection_changed().add(Rc::new(move |_| {
            let Some(scope) = weak_scope.upgrade() else {
                return;
            };
            if syncing.get() || scope.is_suspended() {
                return;
            }
            let Some(p) = weak_palette.upgrade() else {
                return;
            };
            syncing.set(true);
            let _finally = Reset(&syncing);
            replace(
                &model.recent_colors(),
                p.recent_colors().iter().take(10).map(core_color),
            );
        }));
        bindings.add(Some(Rc::new(move || {
            recent.collection_changed().remove(token);
        })));
    }
}

fn complete_flyout_color(
    palette: &RibbonColorPalette,
    result: Result<Option<Color>, crate::controls::RibbonColorDialogError>,
) {
    use crate::controls::RibbonColorDialogError;
    match result {
        Ok(Some(color)) => palette.select(Some(color)),
        Ok(None) => {}
        // Source explicitly catches unavailable/busy ContentDialog failures.
        Err(
            RibbonColorDialogError::AlreadyOpen
            | RibbonColorDialogError::RootNotVisible
            | RibbonColorDialogError::MissingOverlay,
        ) => {}
        Err(error) => panic!("CreateFlyoutPalette More Colors: {error}"),
    }
}
