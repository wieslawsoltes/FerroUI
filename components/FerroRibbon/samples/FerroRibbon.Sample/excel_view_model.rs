// Copyright (c) 2026 Wiesław Šoltés. MIT. ExcelViewModel.cs application model.
use ferroribbon_core::{
    RibbonContextualActivation, RibbonString,
    commands::{ICommand, RibbonCommandCatalog, RibbonCommandMetadata},
    model::*,
};
use ferroui_reactiveui::{IReactiveObjectExt, ReactiveObject, reactive_object};
use std::{
    any::Any,
    cell::{OnceCell, RefCell},
    rc::Rc,
};
/// Application formatting boundary; consumers can format their own command payloads.
pub type ExcelParameterFormatter = Rc<dyn Fn(&dyn Any) -> Result<String, String>>;
struct ExcelState {
    reactive: Rc<ReactiveObject>,
    status: RefCell<Option<String>>,
    selected_cell: RefCell<Option<String>>,
    formatter: ExcelParameterFormatter,
}
reactive_object!(ExcelState);
impl ExcelState {
    fn set_status(&self, value: Option<String>) {
        self.raise_and_set_if_changed(&self.status, value, "Status");
    }
    fn command_text(&self, id: &str, p: Option<&dyn Any>, separator: &str, end: &str) -> String {
        if let Some(p) = p {
            format!(
                "{id}{separator}{}{end}",
                (self.formatter)(p).expect("Excel command parameter display")
            )
        } else {
            id.to_owned()
        }
    }
}
/// Commands own the small shared presentation state, not the entire model graph.
/// Retained commands remain usable without a command -> VM -> command cycle.
pub struct ExcelViewModel {
    reactive: Rc<ReactiveObject>,
    state: Rc<ExcelState>,
    ribbon: OnceCell<Rc<RibbonModel>>,
    commands: Rc<RibbonCommandCatalog>,
    bold: Rc<RibbonToggleButtonModel>,
    italic: Rc<RibbonToggleButtonModel>,
    underline: Rc<RibbonToggleButtonModel>,
    fill_color: Rc<RibbonColorPickerModel>,
    font_color: Rc<RibbonColorPickerModel>,
    font_family: Rc<RibbonComboBoxModel>,
    font_size: Rc<RibbonComboBoxModel>,
    number_format: Rc<RibbonComboBoxModel>,
    align: Rc<RefCell<[Rc<RibbonToggleButtonModel>; 3]>>,
    chart_tools: Rc<RibbonContextualGroupModel>,
    toggle_chart_command: Rc<dyn ICommand>,
}
reactive_object!(ExcelViewModel);
impl PartialEq for ExcelViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl ExcelViewModel {
    pub fn new(formatter: ExcelParameterFormatter) -> Rc<Self> {
        let reactive = Rc::new(ReactiveObject::new());
        let state = Rc::new(ExcelState {
            reactive: reactive.clone(),
            status: RefCell::new(Some("Ready".into())),
            selected_cell: RefCell::new(Some("A1".into())),
            formatter,
        });
        let commands = RibbonCommandCatalog::new();
        let current = state.clone();
        // Subscription belongs to the catalog: a retained catalog keeps its original
        // execution/status behavior. It captures no catalog/model/VM back-reference.
        commands.executed().add(Rc::new(move |(_, e)| {
            let id = e
                .id()
                .map(|s| s.to_utf8().expect("Excel command ID"))
                .unwrap_or_default();
            current.set_status(Some(current.command_text(
                &id,
                e.parameter().as_deref(),
                " → ",
                "",
            )));
        }));
        for &(id, label, shortcut) in COMMANDS {
            let current = state.clone();
            commands.register_action(
                s(id),
                s(label),
                Rc::new(move |p| {
                    current.set_status(Some(current.command_text(label, p, " (", ")")))
                }),
                None,
                RibbonCommandMetadata {
                    shortcut: if shortcut.is_empty() {
                        None
                    } else {
                        s(shortcut)
                    },
                    category: s("Excel"),
                    ..Default::default()
                },
            );
        }
        let current = state.clone();
        commands.register_action(
            s("formatCellsLauncher"),
            s("Format Cells dialog"),
            Rc::new(move |_| current.set_status(Some("Format Cells dialog".into()))),
            None,
            Default::default(),
        );
        let bold = toggle("bold", "Bold", RibbonIcons::Bold);
        bold.set_shortcut(s("Ctrl+B"));
        bold.set_key_tip(s("1"));
        let italic = toggle("italic", "Italic", RibbonIcons::Italic);
        italic.set_shortcut(s("Ctrl+I"));
        italic.set_key_tip(s("2"));
        let underline = toggle("underline", "Underline", RibbonIcons::Underline);
        underline.set_shortcut(s("Ctrl+U"));
        underline.set_key_tip(s("3"));
        let fill_color = RibbonColorPickerModel::with_options(
            s("fillColor"),
            s("Fill Color"),
            icon(RibbonIcons::Fill),
            None,
        );
        fill_color.set_selected_color(Some(
            ferroribbon_core::theming::RibbonColor::parse(s("#FFFF00").as_ref()).unwrap(),
        ));
        fill_color.set_show_no_color(true);
        fill_color.set_show_automatic(false);
        fill_color.set_key_tip(s("H"));
        let font_color = RibbonColorPickerModel::with_options(
            s("fontColor"),
            s("Font Color"),
            icon(RibbonIcons::FontColor),
            None,
        );
        font_color.set_selected_color(Some(
            ferroribbon_core::theming::RibbonColor::parse(s("#C00000").as_ref()).unwrap(),
        ));
        font_color.set_key_tip(s("FC"));
        let font_family = RibbonComboBoxModel::with_items(
            s("fontFamily"),
            s("Font"),
            [
                "Aptos Narrow",
                "Arial",
                "Calibri",
                "Cambria",
                "Consolas",
                "Segoe UI",
                "Times New Roman",
                "Verdana",
            ]
            .into_iter()
            .map(|s| Some(RibbonObject::value(RibbonString::from(s)))),
        );
        font_family.set_text(s("Aptos Narrow"));
        font_family.set_is_editable(true);
        font_family.set_input_width(128.);
        font_family.set_preview_font_family(true);
        font_family.set_key_tip(s("FF"));
        let font_size = RibbonComboBoxModel::with_items(
            s("fontSize"),
            s("Font Size"),
            [
                8., 9., 10., 11., 12., 14., 16., 18., 20., 24., 28., 36., 48., 72.,
            ]
            .into_iter()
            .map(|n| Some(RibbonObject::value(n))),
        );
        font_size.set_text(s("11"));
        font_size.set_is_editable(true);
        font_size.set_input_width(46.);
        font_size.set_key_tip(s("FS"));
        let number_format = RibbonComboBoxModel::with_items(
            s("numberFormat"),
            s("Number Format"),
            [
                "General",
                "Number",
                "Currency",
                "Accounting",
                "Short Date",
                "Long Date",
                "Time",
                "Percentage",
                "Fraction",
                "Scientific",
                "Text",
            ]
            .into_iter()
            .map(|s| Some(RibbonObject::value(RibbonString::from(s)))),
        );
        number_format.set_text(s("General"));
        number_format.set_input_width(110.);
        number_format.set_key_tip(s("N"));
        let align = [
            toggle("alignLeft", "Align Left", RibbonIcons::AlignLeft),
            toggle("alignCenter", "Center", RibbonIcons::AlignCenter),
            toggle("alignRight", "Align Right", RibbonIcons::AlignRight),
        ];
        for a in &align {
            a.set_group_name(s("halign"));
        }
        align[0].set_is_checked(true);
        let chart_tools = RibbonContextualGroupModel::new(s("chart"), s("Chart Tools"));
        chart_tools.set_color(
            ferroribbon_core::theming::RibbonColor::parse(s("#107C41").as_ref()).unwrap(),
        );
        chart_tools.set_activation(RibbonContextualActivation::SelectOnShow);
        let chart = chart_tools.clone();
        let current = state.clone();
        let toggle_chart_command: Rc<dyn ICommand> =
            ferroribbon_core::commands::RibbonRelayCommand::new(move || {
                chart.set_is_visible(!chart.is_visible());
                current.set_status(Some(
                    if chart.is_visible() {
                        "Chart selected"
                    } else {
                        "Chart deselected"
                    }
                    .into(),
                ));
            });
        let vm = Rc::new(Self {
            reactive,
            state,
            ribbon: OnceCell::new(),
            commands,
            bold,
            italic,
            underline,
            fill_color,
            font_color,
            font_family,
            font_size,
            number_format,
            align: Rc::new(RefCell::new(align)),
            chart_tools,
            toggle_chart_command,
        });
        assert!(vm.ribbon.set(super::excel_ribbon_model::build(&vm)).is_ok());
        vm
    }
    pub fn ribbon(&self) -> Rc<RibbonModel> {
        self.ribbon.get().expect("Excel ribbon initialized").clone()
    }
    pub fn commands(&self) -> Rc<RibbonCommandCatalog> {
        self.commands.clone()
    }
    pub fn bold(&self) -> Rc<RibbonToggleButtonModel> {
        self.bold.clone()
    }
    pub fn italic(&self) -> Rc<RibbonToggleButtonModel> {
        self.italic.clone()
    }
    pub fn underline(&self) -> Rc<RibbonToggleButtonModel> {
        self.underline.clone()
    }
    pub fn fill_color(&self) -> Rc<RibbonColorPickerModel> {
        self.fill_color.clone()
    }
    pub fn font_color(&self) -> Rc<RibbonColorPickerModel> {
        self.font_color.clone()
    }
    pub fn font_family(&self) -> Rc<RibbonComboBoxModel> {
        self.font_family.clone()
    }
    pub fn font_size(&self) -> Rc<RibbonComboBoxModel> {
        self.font_size.clone()
    }
    pub fn number_format(&self) -> Rc<RibbonComboBoxModel> {
        self.number_format.clone()
    }
    pub fn align(&self) -> Rc<RefCell<[Rc<RibbonToggleButtonModel>; 3]>> {
        self.align.clone()
    }
    pub fn chart_tools(&self) -> Rc<RibbonContextualGroupModel> {
        self.chart_tools.clone()
    }
    pub fn toggle_chart_command(&self) -> Rc<dyn ICommand> {
        self.toggle_chart_command.clone()
    }
    pub fn status(&self) -> Option<String> {
        self.state.status.borrow().clone()
    }
    pub fn set_status(&self, v: Option<String>) {
        self.state.set_status(v);
    }
    pub(crate) fn item_invoked(&self, item_id: Option<String>, command_id: Option<String>) {
        self.set_status(item_id.or(command_id).or_else(|| self.status()));
    }
    pub fn selected_cell(&self) -> Option<String> {
        self.state.selected_cell.borrow().clone()
    }
    pub fn set_selected_cell(&self, v: Option<String>) {
        self.raise_and_set_if_changed(&self.state.selected_cell, v, "SelectedCell");
    }
    pub(crate) fn status_command(&self, message: &'static str) -> Rc<dyn ICommand> {
        let state = self.state.clone();
        ferroribbon_core::commands::RibbonRelayCommand::new(move || {
            state.set_status(Some(message.into()))
        })
    }
    pub(crate) fn table_command(&self) -> Rc<dyn ICommand> {
        let state = self.state.clone();
        ferroribbon_core::commands::RibbonRelayCommand::new_with_parameter(
            move |p| {
                let size = p
                    .and_then(|p| p.downcast_ref::<RibbonGridSize>())
                    .copied()
                    .unwrap_or_default();
                state.set_status(Some(format!("Inserted {size} table")));
            },
            None,
        )
    }
}
pub(crate) fn s(value: &str) -> Option<RibbonString> {
    Some(value.into())
}
pub(crate) fn icon(value: &str) -> Option<Rc<RibbonIcon>> {
    Some(RibbonIcon::glyph(s(value)))
}
pub(crate) fn toggle(id: &str, label: &str, glyph: &str) -> Rc<RibbonToggleButtonModel> {
    RibbonToggleButtonModel::with_options(
        s(id),
        s(label),
        icon(glyph),
        None,
        ferroribbon_core::RibbonItemSize::Small,
    )
}

const COMMANDS: &[(&str, &str, &str)] = &[
    ("paste", "Paste", "Ctrl+V"),
    ("cut", "Cut", "Ctrl+X"),
    ("copy", "Copy", "Ctrl+C"),
    ("formatPainter", "Format Painter", ""),
    ("insertCells", "Insert", ""),
    ("deleteCells", "Delete", ""),
    ("formatCells", "Format Cells", "Ctrl+1"),
    ("autoSum", "AutoSum", "Alt+="),
    ("fill", "Fill", ""),
    ("clear", "Clear", ""),
    ("sortFilter", "Sort & Filter", ""),
    ("findSelect", "Find & Select", "Ctrl+F"),
    ("pivotTable", "PivotTable", ""),
    ("table", "Table", "Ctrl+T"),
    ("recommendedCharts", "Recommended Charts", ""),
    ("insertFunction", "Insert Function", "Shift+F3"),
    ("calculateNow", "Calculate Now", "F9"),
    ("refreshAll", "Refresh All", "Ctrl+Alt+F5"),
    ("freezePanes", "Freeze Panes", ""),
    ("newWindow", "New Window", ""),
    ("increaseDecimal", "Increase Decimal", ""),
    ("decreaseDecimal", "Decrease Decimal", ""),
    ("percent", "Percent Style", "Ctrl+Shift+%"),
    ("comma", "Comma Style", ""),
    ("wrapText", "Wrap Text", ""),
    ("mergeCenter", "Merge & Center", ""),
    ("chartStyles", "Change Colors", ""),
    ("switchRowColumn", "Switch Row/Column", ""),
    ("selectData", "Select Data", ""),
    ("changeChartType", "Change Chart Type", ""),
];
#[cfg(test)]
mod ownership_tests {
    use super::*;
    use ferroui_system::component_model::INotifyPropertyChanged;
    #[test]
    fn retained_commands_and_catalog_keep_live_state_without_retaining_the_model_graph() {
        let vm = ExcelViewModel::new(Rc::new(|p| {
            Ok(p.downcast_ref::<String>().expect("fixture string").clone())
        }));
        let state = Rc::downgrade(&vm.state);
        let facade = Rc::downgrade(&vm);
        let ribbon = Rc::downgrade(&vm.ribbon());
        let catalog = vm.commands();
        let command = vm.status_command("still live");
        let trace = Rc::new(RefCell::new(Vec::new()));
        let out = trace.clone();
        let weak = state.clone();
        vm.property_changed().add(Rc::new(move |p| {
            if p == "Status" {
                out.borrow_mut()
                    .push(weak.upgrade().unwrap().status.borrow().clone());
            }
        }));
        drop(vm);
        assert!(facade.upgrade().is_none());
        assert!(ribbon.upgrade().is_none());
        command.execute(Some(&42));
        assert_eq!(
            trace.borrow().last().unwrap().as_deref(),
            Some("still live")
        );
        assert!(catalog.execute(
            Some(&"paste".into()),
            Some(Rc::new("after facade".to_owned()))
        ));
        assert_eq!(
            trace.borrow().as_slice(),
            &[
                Some("still live".into()),
                Some("Paste (after facade)".into()),
                Some("paste → after facade".into())
            ]
        );
        drop(command);
        drop(catalog);
        assert!(state.upgrade().is_none());
    }
}
ferroui_base::ferro_markup_type!(class ExcelViewModel{namespace:"FerroRibbon.Sample",this:Rc<ExcelViewModel>,handles:[Rc<ExcelViewModel>,Option<Rc<ExcelViewModel>>],notify_property_changed:ExcelViewModel,properties:[Ribbon:Rc<RibbonModel>{get:|vm:&Rc<ExcelViewModel>|vm.ribbon()},Status:Option<String>{get:|vm:&Rc<ExcelViewModel>|vm.status(),set:|vm:&Rc<ExcelViewModel>,v|vm.set_status(v)},SelectedCell:Option<String>{get:|vm:&Rc<ExcelViewModel>|vm.selected_cell(),set:|vm:&Rc<ExcelViewModel>,v|vm.set_selected_cell(v)}]});
