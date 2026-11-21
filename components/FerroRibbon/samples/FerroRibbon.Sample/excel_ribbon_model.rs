// Copyright (c) 2026 Wiesław Šoltés. MIT. ExcelViewModel.BuildRibbon, same graph/order/aliases.
use super::excel_view_model::{ExcelViewModel, icon, s, toggle};
use ferroribbon_core::{
    RibbonBackstagePlacement, RibbonGroupItemsLayout, RibbonItemSize as Size,
    layout::RibbonSizeDefinition, model::*,
};
use std::rc::Rc;
fn button(
    id: &str,
    label: &str,
    glyph: &str,
    size: Size,
    command: Option<&str>,
) -> Rc<RibbonButtonModel> {
    let b = RibbonButtonModel::with_options(s(id), s(label), icon(glyph), None, size);
    b.set_command_id(command.and_then(s));
    b
}
fn split(
    id: &str,
    label: &str,
    glyph: &str,
    size: Size,
    command: Option<&str>,
) -> Rc<RibbonSplitButtonModel> {
    let b = RibbonSplitButtonModel::with_options(s(id), s(label), icon(glyph), None, size);
    b.set_command_id(command.and_then(s));
    b
}
fn drop_down(id: &str, label: &str, glyph: &str, size: Size) -> Rc<RibbonDropDownButtonModel> {
    RibbonDropDownButtonModel::with_options(s(id), s(label), icon(glyph), size)
}
fn group(id: &str, label: &str, glyph: &str) -> Rc<RibbonGroupModel> {
    RibbonGroupModel::with_options(s(id), s(label), icon(glyph))
}
fn menu(
    id: &str,
    label: &str,
    glyph: Option<&str>,
    command: Option<&str>,
) -> Rc<RibbonMenuItemModel> {
    let m = RibbonMenuItemModel::with_options(s(id), s(label), glyph.and_then(icon), None);
    m.set_command_id(s(command.unwrap_or(id)));
    m
}
pub(crate) fn build(vm: &ExcelViewModel) -> Rc<RibbonModel> {
    let model = RibbonModel::new();
    model.set_command_catalog(Some(vm.commands()));
    model.set_application_button_label(s("File"));
    model.set_title(s("Budget 2026.xlsx"));
    model.contextual_groups().add(Some(vm.chart_tools()));
    let home = RibbonTabModel::new(s("home"), s("Home"));
    home.set_key_tip(s("H"));
    let clipboard = group("clipboard", "Clipboard", RibbonIcons::Paste);
    clipboard.set_dialog_launcher_command_id(s("formatCellsLauncher"));
    clipboard.set_reduction_order(-1);
    let paste = split(
        "paste",
        "Paste",
        RibbonIcons::Paste,
        Size::Large,
        Some("paste"),
    );
    paste.set_size_definition(Some(RibbonSizeDefinition::always_large()));
    paste.set_key_tip(s("V"));
    paste.set_shortcut(s("Ctrl+V"));
    paste
        .menu_items()
        .add(Some(RibbonMenuHeaderModel::new(s("Paste"))));
    for (id, label, glyph) in [
        ("pasteValues", "Values", RibbonIcons::Paste),
        ("pasteFormulas", "Formulas", RibbonIcons::Function),
        ("pasteTranspose", "Transpose", RibbonIcons::Rotate),
    ] {
        paste
            .menu_items()
            .add(Some(menu(id, label, Some(glyph), Some("paste"))));
    }
    paste
        .menu_items()
        .add(Some(RibbonMenuSeparatorModel::new()));
    paste.menu_items().add(Some(menu(
        "pasteSpecial",
        "Paste Special...",
        None,
        Some("paste"),
    )));
    clipboard.items().add(Some(paste.clone()));
    for (id, label, glyph, key) in [
        ("cut", "Cut", RibbonIcons::Cut, "X"),
        ("copy", "Copy", RibbonIcons::Copy, "C"),
        (
            "formatPainter",
            "Format Painter",
            RibbonIcons::FormatPainter,
            "FP",
        ),
    ] {
        let b = button(id, label, glyph, Size::Medium, Some(id));
        b.set_key_tip(s(key));
        clipboard.items().add(Some(b));
    }
    home.groups().add(Some(clipboard));
    let font = group("font", "Font", RibbonIcons::Font);
    font.set_items_layout(RibbonGroupItemsLayout::Rows);
    font.set_row_count(2);
    font.set_dialog_launcher_command_id(s("formatCellsLauncher"));
    let font_sizes = RibbonButtonGroupModel::with_items(
        s("fontSizeButtons"),
        [
            Some(button(
                "growFont",
                "Increase Font Size",
                RibbonIcons::FontIncrease,
                Size::Medium,
                None,
            ) as Rc<dyn RibbonItemModelImpl>),
            Some(button(
                "shrinkFont",
                "Decrease Font Size",
                RibbonIcons::FontDecrease,
                Size::Medium,
                None,
            )),
        ],
    );
    font.items().add(Some(RibbonRowModel::with_items(
        s("fontRow1"),
        [
            Some(vm.font_family() as Rc<dyn RibbonItemModelImpl>),
            Some(vm.font_size()),
            Some(font_sizes),
        ],
    )));
    let borders = split(
        "borders",
        "Borders",
        RibbonIcons::Borders,
        Size::Small,
        None,
    );
    borders.set_key_tip(s("B"));
    for label in [
        "Bottom Border",
        "Top Border",
        "Left Border",
        "Right Border",
        "No Border",
        "All Borders",
        "Outside Borders",
        "Thick Box Border",
    ] {
        borders
            .menu_items()
            .add(Some(RibbonMenuItemModel::with_options(
                s(&format!("border.{label}")),
                s(label),
                icon(RibbonIcons::Borders),
                None,
            )));
    }
    let styles = RibbonButtonGroupModel::with_items(
        s("fontStyles"),
        [
            Some(vm.bold() as Rc<dyn RibbonItemModelImpl>),
            Some(vm.italic()),
            Some(vm.underline()),
        ],
    );
    font.items().add(Some(RibbonRowModel::with_items(
        s("fontRow2"),
        [
            Some(styles as Rc<dyn RibbonItemModelImpl>),
            Some(borders),
            Some(vm.fill_color()),
            Some(vm.font_color()),
        ],
    )));
    home.groups().add(Some(font));
    let alignment = group("alignment", "Alignment", RibbonIcons::AlignCenter);
    alignment.set_dialog_launcher_command_id(s("formatCellsLauncher"));
    alignment
        .items()
        .add(Some(RibbonButtonGroupModel::with_items(
            s("halignGroup"),
            vm.align()
                .borrow()
                .clone()
                .into_iter()
                .map(|a| Some(a as Rc<dyn RibbonItemModelImpl>)),
        )));
    let wrap = toggle("wrapText", "Wrap Text", RibbonIcons::Refresh);
    wrap.set_size(Size::Medium);
    wrap.set_command_id(s("wrapText"));
    alignment.items().add(Some(wrap));
    alignment.items().add(Some(split(
        "mergeCenter",
        "Merge & Center",
        RibbonIcons::Merge,
        Size::Medium,
        Some("mergeCenter"),
    )));
    home.groups().add(Some(alignment));
    let number = group("number", "Number", RibbonIcons::Calculator);
    number.set_items_layout(RibbonGroupItemsLayout::Rows);
    number.set_row_count(2);
    number.set_dialog_launcher_command_id(s("formatCellsLauncher"));
    number.items().add(Some(RibbonRowModel::with_items(
        s("numberRow1"),
        [Some(vm.number_format() as Rc<dyn RibbonItemModelImpl>)],
    )));
    let buttons = RibbonButtonGroupModel::new(s("numberButtons"));
    for (id, label, glyph, cmd) in [
        ("currency", "Accounting Number Format", "$", "comma"),
        ("percent", "Percent Style", "%", "percent"),
        ("comma", "Comma Style", ",", "comma"),
        (
            "increaseDecimal",
            "Increase Decimal",
            ".0",
            "increaseDecimal",
        ),
        (
            "decreaseDecimal",
            "Decrease Decimal",
            ".00",
            "decreaseDecimal",
        ),
    ] {
        buttons
            .items()
            .add(Some(button(id, label, glyph, Size::Medium, Some(cmd))));
    }
    number.items().add(Some(RibbonRowModel::with_items(
        s("numberRow2"),
        [Some(buttons as Rc<dyn RibbonItemModelImpl>)],
    )));
    home.groups().add(Some(number));
    let styles = group("styles", "Styles", RibbonIcons::Palette);
    styles.set_reduction_order(1);
    let conditional = drop_down(
        "conditionalFormatting",
        "Conditional Formatting",
        RibbonIcons::Filter,
        Size::Large,
    );
    conditional.set_key_tip(s("L"));
    for label in [
        "Highlight Cells Rules",
        "Top/Bottom Rules",
        "Data Bars",
        "Color Scales",
        "Icon Sets",
    ] {
        let sub = RibbonMenuItemModel::new(s(&format!("cf.{label}")), s(label));
        for (n, text) in [
            (1, "Greater Than..."),
            (2, "Less Than..."),
            (3, "Between..."),
        ] {
            sub.items().add(Some(RibbonMenuItemModel::new(
                s(&format!("cf.{label}.{n}")),
                s(text),
            )));
        }
        conditional.menu_items().add(Some(sub));
    }
    conditional
        .menu_items()
        .add(Some(RibbonMenuSeparatorModel::new()));
    conditional.menu_items().add(Some(RibbonMenuItemModel::new(
        s("cf.new"),
        s("New Rule..."),
    )));
    conditional.menu_items().add(Some(RibbonMenuItemModel::new(
        s("cf.manage"),
        s("Manage Rules..."),
    )));
    styles.items().add(Some(conditional));
    let cell_styles = RibbonGalleryModel::new(s("cellStyles"), s("Cell Styles"));
    cell_styles.set_icon(icon(RibbonIcons::Palette));
    cell_styles.set_max_columns(4);
    cell_styles.set_min_columns(2);
    cell_styles.set_item_width(84.);
    cell_styles.set_item_height(30.);
    cell_styles.set_show_labels(false);
    cell_styles.set_drop_down_columns(6);
    for (label, category) in [
        ("Normal", "Good, Bad and Neutral"),
        ("Bad", "Good, Bad and Neutral"),
        ("Good", "Good, Bad and Neutral"),
        ("Neutral", "Good, Bad and Neutral"),
        ("Calculation", "Data and Model"),
        ("Check Cell", "Data and Model"),
        ("Input", "Data and Model"),
        ("Output", "Data and Model"),
        ("Title", "Titles and Headings"),
        ("Heading 1", "Titles and Headings"),
        ("Heading 2", "Titles and Headings"),
        ("Total", "Titles and Headings"),
    ] {
        let item = RibbonGalleryItemModel::with_options(
            s(&format!("cs.{label}")),
            s(label),
            None,
            s(category),
            None,
        );
        item.set_content(Some(RibbonObject::value(
            ferroribbon_core::RibbonString::from(label),
        )));
        cell_styles.items().add(Some(item));
    }
    cell_styles
        .menu_items()
        .add(Some(RibbonMenuItemModel::with_options(
            s("cs.new"),
            s("New Cell Style..."),
            icon(RibbonIcons::Add),
            None,
        )));
    cell_styles
        .menu_items()
        .add(Some(RibbonMenuItemModel::with_options(
            s("cs.merge"),
            s("Merge Styles..."),
            icon(RibbonIcons::Merge),
            None,
        )));
    cell_styles.set_rows(2);
    styles.items().add(Some(cell_styles));
    home.groups().add(Some(styles));
    let cells = group("cells", "Cells", RibbonIcons::Table);
    cells.items().add(Some(split(
        "insertCells",
        "Insert",
        RibbonIcons::InsertRow,
        Size::Medium,
        Some("insertCells"),
    )));
    cells.items().add(Some(split(
        "deleteCells",
        "Delete",
        RibbonIcons::DeleteRow,
        Size::Medium,
        Some("deleteCells"),
    )));
    cells.items().add(Some(drop_down(
        "formatCells",
        "Format",
        RibbonIcons::Table,
        Size::Medium,
    )));
    home.groups().add(Some(cells));
    let editing = group("editing", "Editing", RibbonIcons::Sum);
    editing.items().add(Some(split(
        "autoSum",
        "AutoSum",
        RibbonIcons::Sum,
        Size::Medium,
        Some("autoSum"),
    )));
    for (id, label, glyph, size) in [
        ("fill", "Fill", RibbonIcons::ChevronDown, Size::Medium),
        ("clear", "Clear", RibbonIcons::Clear, Size::Medium),
        (
            "sortFilter",
            "Sort & Filter",
            RibbonIcons::Filter,
            Size::Large,
        ),
        (
            "findSelect",
            "Find & Select",
            RibbonIcons::Find,
            Size::Large,
        ),
    ] {
        editing.items().add(Some(drop_down(id, label, glyph, size)));
    }
    home.groups().add(Some(editing));
    model.tabs().add(Some(home));
    let insert = RibbonTabModel::new(s("insert"), s("Insert"));
    insert.set_key_tip(s("N"));
    let tables = group("tables", "Tables", RibbonIcons::Table);
    tables.items().add(Some(split(
        "pivotTable",
        "PivotTable",
        RibbonIcons::Pivot,
        Size::Large,
        Some("pivotTable"),
    )));
    tables.items().add(Some(button(
        "recommendedPivot",
        "Recommended PivotTables",
        RibbonIcons::Pivot,
        Size::Large,
        None,
    )));
    let table = drop_down("table", "Table", RibbonIcons::Table, Size::Large);
    table
        .menu_items()
        .add(Some(RibbonGridPickerModel::with_command(
            s("tablePicker"),
            s("Insert Table"),
            Some(vm.table_command()),
        )));
    tables.items().add(Some(table));
    insert.groups().add(Some(tables));
    let charts = group("charts", "Charts", RibbonIcons::Chart);
    charts.set_dialog_launcher_command_id(s("recommendedCharts"));
    charts.items().add(Some(button(
        "recommendedCharts",
        "Recommended Charts",
        RibbonIcons::Chart,
        Size::Large,
        Some("recommendedCharts"),
    )));
    for label in ["Column", "Line", "Pie", "Bar", "Area", "Scatter"] {
        charts.items().add(Some(drop_down(
            &format!("chart.{label}"),
            label,
            RibbonIcons::Chart,
            Size::Small,
        )));
    }
    let select = button(
        "selectChart",
        "Select sample chart",
        RibbonIcons::Chart,
        Size::Large,
        None,
    );
    select.set_command(Some(vm.toggle_chart_command()));
    charts.items().add(Some(select));
    insert.groups().add(Some(charts));
    model.tabs().add(Some(insert));
    let formulas = RibbonTabModel::new(s("formulas"), s("Formulas"));
    formulas.set_key_tip(s("M"));
    let library = group("functionLibrary", "Function Library", RibbonIcons::Function);
    library.items().add(Some(button(
        "insertFunction",
        "Insert Function",
        RibbonIcons::Function,
        Size::Large,
        Some("insertFunction"),
    )));
    for label in [
        "AutoSum",
        "Recently Used",
        "Financial",
        "Logical",
        "Text",
        "Date & Time",
        "Lookup & Reference",
        "Math & Trig",
    ] {
        library.items().add(Some(drop_down(
            &format!("fx.{label}"),
            label,
            RibbonIcons::Function,
            Size::Large,
        )));
    }
    formulas.groups().add(Some(library));
    let calculation = group("calculation", "Calculation", RibbonIcons::Calculator);
    calculation.items().add(Some(button(
        "calculateNow",
        "Calculate Now",
        RibbonIcons::Calculator,
        Size::Medium,
        Some("calculateNow"),
    )));
    calculation.items().add(Some(button(
        "calculateSheet",
        "Calculate Sheet",
        RibbonIcons::Calculator,
        Size::Medium,
        None,
    )));
    formulas.groups().add(Some(calculation));
    model.tabs().add(Some(formulas));
    let data = RibbonTabModel::new(s("data"), s("Data"));
    data.set_key_tip(s("A"));
    let queries = group("queries", "Queries & Connections", RibbonIcons::Database);
    queries.items().add(Some(split(
        "refreshAll",
        "Refresh All",
        RibbonIcons::Refresh,
        Size::Large,
        Some("refreshAll"),
    )));
    data.groups().add(Some(queries));
    let sort_filter = group("sortFilterGroup", "Sort & Filter", RibbonIcons::Filter);
    for (id, label, glyph, size) in [
        (
            "sortAZ",
            "Sort A to Z",
            RibbonIcons::SortAscending,
            Size::Medium,
        ),
        (
            "sortZA",
            "Sort Z to A",
            RibbonIcons::SortDescending,
            Size::Medium,
        ),
        ("sortDialog", "Sort", RibbonIcons::Sort, Size::Large),
    ] {
        sort_filter
            .items()
            .add(Some(button(id, label, glyph, size, None)));
    }
    let filter = toggle("filter", "Filter", RibbonIcons::Filter);
    filter.set_size(Size::Large);
    filter.set_shortcut(s("Ctrl+Shift+L"));
    sort_filter.items().add(Some(filter));
    data.groups().add(Some(sort_filter));
    model.tabs().add(Some(data));
    let view = RibbonTabModel::new(s("view"), s("View"));
    view.set_key_tip(s("W"));
    let show = group("show", "Show", RibbonIcons::View);
    for (id, label) in [
        ("gridlines", "Gridlines"),
        ("formulaBar", "Formula Bar"),
        ("headings", "Headings"),
    ] {
        show.items().add(Some(RibbonCheckBoxModel::with_checked(
            s(id),
            s(label),
            true,
        )));
    }
    view.groups().add(Some(show));
    let zoom = group("zoom", "Zoom", RibbonIcons::Zoom);
    zoom.items().add(Some(button(
        "zoomDialog",
        "Zoom",
        RibbonIcons::Zoom,
        Size::Large,
        None,
    )));
    zoom.items().add(Some(button(
        "zoom100",
        "100%",
        RibbonIcons::Zoom,
        Size::Large,
        None,
    )));
    let value = RibbonSpinnerModel::with_value(s("zoomValue"), s("Zoom:"), 100.);
    value.set_minimum(10.);
    value.set_maximum(400.);
    value.set_increment(10.);
    value.set_unit(s("%"));
    value.set_format(s("0"));
    zoom.items().add(Some(value));
    view.groups().add(Some(zoom));
    let window = group("window", "Window", RibbonIcons::Window);
    window.items().add(Some(button(
        "newWindow",
        "New Window",
        RibbonIcons::Window,
        Size::Large,
        Some("newWindow"),
    )));
    window.items().add(Some(drop_down(
        "freezePanes",
        "Freeze Panes",
        RibbonIcons::Freeze,
        Size::Large,
    )));
    view.groups().add(Some(window));
    model.tabs().add(Some(view));
    let design = RibbonTabModel::new(s("chartDesign"), s("Chart Design"));
    design.set_contextual_group_id(s("chart"));
    design.set_key_tip(s("JC"));
    let layouts = group("chartLayouts", "Chart Layouts", RibbonIcons::Layout);
    layouts.items().add(Some(drop_down(
        "addElement",
        "Add Chart Element",
        RibbonIcons::Add,
        Size::Large,
    )));
    layouts.items().add(Some(drop_down(
        "quickLayout",
        "Quick Layout",
        RibbonIcons::Layout,
        Size::Large,
    )));
    design.groups().add(Some(layouts));
    let styles = group("chartStylesGroup", "Chart Styles", RibbonIcons::Palette);
    styles.set_reduction_order(1);
    styles.items().add(Some(drop_down(
        "chartColors",
        "Change Colors",
        RibbonIcons::Palette,
        Size::Large,
    )));
    let gallery = RibbonGalleryModel::new(s("chartStyleGallery"), s("Chart Styles"));
    gallery.set_max_columns(6);
    gallery.set_min_columns(3);
    gallery.set_item_width(58.);
    gallery.set_item_height(52.);
    gallery.set_show_labels(false);
    for i in 1..=14 {
        gallery
            .items()
            .add(Some(RibbonGalleryItemModel::with_options(
                s(&format!("chartStyle{i}")),
                s(&format!("Style {i}")),
                icon(RibbonIcons::Chart),
                None,
                None,
            )));
    }
    styles.items().add(Some(gallery));
    design.groups().add(Some(styles));
    let data = group("chartData", "Data", RibbonIcons::Database);
    data.items().add(Some(button(
        "switchRowColumn",
        "Switch Row/Column",
        RibbonIcons::Rotate,
        Size::Large,
        Some("switchRowColumn"),
    )));
    data.items().add(Some(button(
        "selectData",
        "Select Data",
        RibbonIcons::Table,
        Size::Large,
        Some("selectData"),
    )));
    design.groups().add(Some(data));
    let kind = group("type", "Type", RibbonIcons::Chart);
    kind.items().add(Some(button(
        "changeChartType",
        "Change Chart Type",
        RibbonIcons::Chart,
        Size::Large,
        Some("changeChartType"),
    )));
    design.groups().add(Some(kind));
    model.tabs().add(Some(design));
    for (id, label, glyph, key, message) in [
        ("save", "Save", RibbonIcons::Save, "Ctrl+S", "Saved"),
        ("undo", "Undo", RibbonIcons::Undo, "Ctrl+Z", "Undo"),
        ("redo", "Redo", RibbonIcons::Redo, "Ctrl+Y", "Redo"),
    ] {
        let b = button(id, label, glyph, Size::Medium, None);
        b.set_shortcut(s(key));
        b.set_command(Some(vm.status_command(message)));
        model.quick_access_items().add(Some(b));
    }
    model.quick_access_candidates().add(Some(paste));
    model.quick_access_candidates().add(Some(vm.bold()));
    let comments = button(
        "comments",
        "Comments",
        RibbonIcons::Comment,
        Size::Medium,
        None,
    );
    comments.set_show_label_in_simplified(Some(true));
    model.tab_strip_items().add(Some(comments));
    let share = split("share", "Share", RibbonIcons::Share, Size::Small, None);
    share.set_show_label_in_simplified(Some(true));
    model.tab_strip_items().add(Some(share));
    model.backstage().set_title(s("Excel"));
    for (id, label, glyph, content) in [
        (
            "home",
            "Home",
            RibbonIcons::Home,
            "Start a new workbook or open a recent one.",
        ),
        (
            "new",
            "New",
            RibbonIcons::New,
            "Blank workbook · Budget · Invoice · Calendar",
        ),
        ("open", "Open", RibbonIcons::Open, "Recent workbooks"),
    ] {
        model
            .backstage()
            .items()
            .add(Some(RibbonBackstageItemModel::with_options(
                s(id),
                s(label),
                icon(glyph),
                Some(RibbonObject::value(ferroribbon_core::RibbonString::from(
                    content,
                ))),
            )));
    }
    let save =
        RibbonBackstageItemModel::with_options(s("save"), s("Save"), icon(RibbonIcons::Save), None);
    save.set_command(Some(vm.status_command("Saved from backstage")));
    save.set_has_separator_before(true);
    model.backstage().items().add(Some(save));
    model
        .backstage()
        .items()
        .add(Some(RibbonBackstageItemModel::with_options(
            s("export"),
            s("Export"),
            icon(RibbonIcons::Export),
            Some(RibbonObject::value(ferroribbon_core::RibbonString::from(
                "Create a PDF or change the file type.",
            ))),
        )));
    let options = RibbonBackstageItemModel::with_options(
        s("options"),
        s("Options"),
        icon(RibbonIcons::Settings),
        Some(RibbonObject::value(ferroribbon_core::RibbonString::from(
            "Excel options",
        ))),
    );
    options.set_placement(RibbonBackstagePlacement::Bottom);
    model.backstage().items().add(Some(options));
    model
}
