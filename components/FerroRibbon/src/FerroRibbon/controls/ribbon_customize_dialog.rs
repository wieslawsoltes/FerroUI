//! Native RibbonCustomizeDialog.cs. Copyright Wiesław Šoltés, MIT.
//! Working editor is UI-independent; this class owns actual dialog controls,
//! source actions, native focus/modal ownership and subscriptions.
use super::{
    Ribbon, RibbonDialogHost, RibbonDialogOutcome, RibbonItemHelper,
    ribbon_customization_list::{RibbonCustomizationList, RibbonCustomizationListRow as NativeRow},
    ribbon_dialog_session::{RibbonDialogOpenError, RibbonDialogSession},
    ribbon_quick_access_tool_bar::as_item,
};
use crate::theming::RibbonTheme;
use ferroribbon_core::{
    RibbonQuickAccessPosition, RibbonString,
    customization_editor::{
        RibbonCustomizationCommand, RibbonCustomizationEditError, RibbonCustomizationEditor,
        RibbonCustomizationGroup, RibbonCustomizationRowKind, RibbonCustomizationTab,
    },
    localization::RibbonStrings,
    state::RibbonState,
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment},
    media::{FontWeight, IBrush},
    reactive::IDisposable,
};
use ferroui_controls::{
    Button, CheckBox, ColumnDefinitions, ContentControlImpl, Control, ControlImpl, Grid,
    StackPanel, TextBlock, TextBox, TopLevel, primitives::TemplatedControlImpl,
};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
mod nested;
mod text;
use text::CommandText;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonCustomizePage {
    Ribbon,
    QuickAccessToolbar,
}
ferroui_base::ferro_markup_enum!(RibbonCustomizePage {Ribbon, QuickAccessToolbar},{namespace:"FerroRibbon.Controls"});
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RibbonCustomizeDialogError {
    AlreadyOpen,
    RootNotVisible,
    MissingOverlay,
    MissingLocalizedText(&'static str),
    UnsupportedUtf16 {
        property: &'static str,
        units: Vec<u16>,
    },
    State(String),
    Edit(RibbonCustomizationEditError),
    TextService(String),
}
impl std::fmt::Display for RibbonCustomizeDialogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Ribbon customization dialog: {self:?}")
    }
}
impl std::error::Error for RibbonCustomizeDialogError {}
impl From<RibbonCustomizationEditError> for RibbonCustomizeDialogError {
    fn from(v: RibbonCustomizationEditError) -> Self {
        Self::Edit(v)
    }
}
impl From<RibbonDialogOpenError> for RibbonCustomizeDialogError {
    fn from(v: RibbonDialogOpenError) -> Self {
        match v {
            RibbonDialogOpenError::AlreadyOpen => Self::AlreadyOpen,
            RibbonDialogOpenError::RootNotVisible => Self::RootNotVisible,
            RibbonDialogOpenError::MissingOverlay => Self::MissingOverlay,
        }
    }
}
type DialogResult<T> = Result<T, RibbonCustomizeDialogError>;
fn utf8(value: Option<RibbonString>, property: &'static str) -> DialogResult<Option<String>> {
    value
        .map(|value| {
            String::from_utf16(value.as_utf16()).map_err(|_| {
                RibbonCustomizeDialogError::UnsupportedUtf16 {
                    property,
                    units: value.as_utf16().to_vec(),
                }
            })
        })
        .transpose()
}
fn boxed(value: Option<String>) -> Option<BoxedValue> {
    value.map(|s| Rc::new(s) as BoxedValue)
}
fn snapshot_tabs(ribbon: &Ribbon) -> Vec<RibbonCustomizationTab> {
    ribbon
        .tabs()
        .iter()
        .map(|tab| {
            let tab = tab.expect("customization tab");
            RibbonCustomizationTab {
                id: tab.effective_id().map(Into::into),
                header: tab.header().map(Into::into),
                original_header: tab.original_header.borrow().clone().map(Into::into),
                is_custom: tab.is_custom.get(),
                is_contextual: tab.is_contextual(),
                groups: tab
                    .groups()
                    .iter()
                    .map(|group| {
                        let group = group.expect("customization group");
                        RibbonCustomizationGroup {
                            id: group.effective_id().map(Into::into),
                            header: group.header().map(Into::into),
                            original_header: group.original_header.borrow().clone().map(Into::into),
                        }
                    })
                    .collect(),
            }
        })
        .collect()
}
fn collect_commands(ribbon: &Ribbon) -> DialogResult<Vec<RibbonCustomizationCommand>> {
    let mut commands: Vec<RibbonCustomizationCommand> = Vec::new();
    for tab in ribbon.tabs().iter() {
        let tab = tab.expect("customization tab");
        if tab.is_custom.get() {
            continue;
        }
        for group in tab.groups().iter() {
            let group = group.expect("customization group");
            for element in group.get_all_items().iter() {
                if let Some(item) = as_item(&element)
                    && let (Some(id), Some(label)) = (item.id(), item.label())
                    && item.can_add_to_quick_access()
                    && RibbonItemHelper::get_source_item(&element).is_none()
                    && commands.iter().all(|c| c.id != id)
                {
                    commands.push(RibbonCustomizationCommand {
                        id,
                        label: Some(RibbonString::from_utf16(
                            label
                                .as_utf16()
                                .iter()
                                .map(|u| if *u == 10 { 32 } else { *u })
                                .collect::<Vec<_>>(),
                        )),
                        path: Some(
                            format!(
                                "{} › {}",
                                tab.header().unwrap_or_default(),
                                group.header().unwrap_or_default()
                            )
                            .into(),
                        ),
                        icon: crate::metadata::model_objects::from_native(item.icon()),
                    });
                }
            }
        }
    }
    for command in &commands {
        utf8(command.label.clone(), "Command.Label")?;
        utf8(command.path.clone(), "Command.Path")?;
    }
    let comparer = CommandText::current()?;
    commands.sort_by(|a, b| comparer.compare(a.label.as_ref().unwrap(), b.label.as_ref().unwrap()));
    Ok(commands)
}
#[derive(Clone, Copy)]
enum Action {
    Add,
    Remove,
    Up,
    Down,
    NewTab,
    NewGroup,
    Rename,
    Reset,
    ImportExport,
    QatAdd,
    QatRemove,
    QatUp,
    QatDown,
    QatReset,
}
#[repr(C)]
pub struct RibbonCustomizeDialog {
    base: RibbonDialogHost,
    ribbon: Ref<Ribbon>,
    editor: RefCell<RibbonCustomizationEditor>,
    command_list: Rc<RibbonCustomizationList>,
    structure_list: Rc<RibbonCustomizationList>,
    qat_list: Rc<RibbonCustomizationList>,
    qat_commands: Rc<RibbonCustomizationList>,
    ribbon_page: Ref<Grid>,
    qat_page: Ref<Grid>,
    search: Ref<TextBox>,
    qat_below: Ref<CheckBox>,
    body: Ref<StackPanel>,
    ribbon_toggle: Ref<Button>,
    qat_toggle: Ref<Button>,
    page: Cell<RibbonCustomizePage>,
    scope: RefCell<Option<Weak<RibbonDialogSession>>>,
    nested_error: RefCell<Option<RibbonCustomizeDialogError>>,
    pending_nested: RefCell<Vec<Rc<nested::NestedWait>>>,
    sizing: RefCell<Option<Rc<dyn IDisposable>>>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    last_outcome: Cell<Option<RibbonDialogOutcome>>,
}
ferro_class!(RibbonCustomizeDialog:RibbonDialogHost);
ferro_class_info!(RibbonCustomizeDialog{markup:{namespace:"FerroRibbon.Controls",properties:[WorkingState:Rc<RibbonState>{get:RibbonCustomizeDialog::working_state}]}});
ferro_impl_classes!(RibbonCustomizeDialog:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,ContentControlImpl,TemplatedControlImpl);
impl RibbonCustomizeDialog {
    pub fn new(ribbon: Ref<Ribbon>, page: RibbonCustomizePage) -> DialogResult<Ref<Self>> {
        let state = ribbon
            .get_state()
            .map_err(RibbonCustomizeDialogError::State)?;
        let editor = RibbonCustomizationEditor::new(
            state,
            snapshot_tabs(&ribbon),
            collect_commands(&ribbon)?,
            RibbonStrings::current().custom_group_suffix(),
        )?;
        let this = instantiate(Self {
            base: RibbonDialogHost::construct(),
            ribbon,
            editor: RefCell::new(editor),
            command_list: RibbonCustomizationList::new(),
            structure_list: RibbonCustomizationList::new(),
            qat_list: RibbonCustomizationList::new(),
            qat_commands: RibbonCustomizationList::new(),
            ribbon_page: Grid::new(),
            qat_page: Grid::new(),
            search: TextBox::new(),
            qat_below: CheckBox::new(),
            body: StackPanel::new(),
            ribbon_toggle: Button::new(),
            qat_toggle: Button::new(),
            page: Cell::new(page),
            scope: RefCell::default(),
            nested_error: RefCell::default(),
            pending_nested: RefCell::default(),
            sizing: RefCell::default(),
            subscriptions: RefCell::default(),
            last_outcome: Cell::new(None),
        });
        // Source constructor installs ApplyState as its first PrimaryButtonClick
        // subscriber, including when a later subscriber cancels closure.
        let weak = this.downgrade();
        let apply = this.button_click(crate::compat::ContentDialogButton::Primary, move |_| {
            if let Some(dialog) = weak.upgrade() {
                dialog
                    .apply_working_state()
                    .expect("customization PrimaryButtonClick ApplyState");
            }
        });
        this.subscriptions.borrow_mut().push(apply);
        let weak = this.downgrade();
        let opened = this.dialog_opened(move |_| {
            if let Some(dialog) = weak.upgrade() {
                let scope = RibbonDialogSession::for_host(&dialog);
                dialog.last_outcome.set(None);
                if let Some(scope) = scope {
                    dialog.scope.replace(Some(Rc::downgrade(&scope)));
                    dialog.attach_sizing(&scope);
                }
            }
        });
        this.subscriptions.borrow_mut().push(opened);
        let weak = this.downgrade();
        let closed = this.dialog_closed(move |args| {
            if let Some(dialog) = weak.upgrade()
                && !dialog.is_open()
            {
                dialog.detach_sizing();
                dialog.scope.replace(None);
                dialog.last_outcome.set(Some(args.result()));
            }
        });
        this.subscriptions.borrow_mut().push(closed);
        this.build(page)?;
        Ok(this)
    }
    pub fn working_state(&self) -> Rc<RibbonState> {
        self.editor.borrow().working_state()
    }
    pub fn page(&self) -> RibbonCustomizePage {
        self.page.get()
    }
    pub fn is_open(&self) -> bool {
        self.scope
            .borrow()
            .as_ref()
            .and_then(Weak::upgrade)
            .is_some_and(|scope| scope.is_open())
    }
    /// Acceptance-only convenience retained for existing Rust application code.
    pub fn closed(&self, handler: impl Fn(bool) + 'static) -> Rc<dyn IDisposable> {
        self.closed_with_outcome(move |outcome| handler(outcome == RibbonDialogOutcome::Primary))
    }
    pub fn last_outcome(&self) -> Option<RibbonDialogOutcome> {
        self.last_outcome.get()
    }
    pub fn closed_with_outcome(
        &self,
        handler: impl Fn(RibbonDialogOutcome) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.dialog_closed(move |args| handler(args.result()))
    }
    pub fn show(&self, root: Ref<TopLevel>) -> DialogResult<()> {
        let weak = self.to_ref().downgrade();
        let scope = RibbonDialogSession::new(root, self.to_ref().upcast(), move |primary| {
            if let Some(dialog) = weak.upgrade() {
                dialog.finish_with_outcome(RibbonDialogOutcome::from_button(primary));
            }
        })?;
        self.last_outcome.set(None);
        self.scope.replace(Some(Rc::downgrade(&scope)));
        scope.show();
        Ok(())
    }
    /// Primary applies the working copy before closing; cancellation discards
    /// it without applying. The same controls/state survive later show calls.
    pub fn close(&self, primary: bool) -> DialogResult<()> {
        self.close_with_outcome(if primary {
            RibbonDialogOutcome::Primary
        } else {
            RibbonDialogOutcome::Dismissed
        })
    }
    pub fn close_with_outcome(&self, outcome: RibbonDialogOutcome) -> DialogResult<()> {
        if !self.is_open() || self.is_dialog_closing() {
            return Ok(());
        }
        if outcome == RibbonDialogOutcome::Primary {
            self.apply_working_state()?;
        }
        let button = match outcome {
            RibbonDialogOutcome::Primary => crate::compat::ContentDialogButton::Primary,
            RibbonDialogOutcome::Secondary => crate::compat::ContentDialogButton::Secondary,
            _ => crate::compat::ContentDialogButton::None,
        };
        self.request_close(button);
        Ok(())
    }
    fn apply_working_state(&self) -> DialogResult<()> {
        self.ribbon
            .apply_saved_state(Some(self.working_state()))
            .map_err(RibbonCustomizeDialogError::State)
    }
    fn finish_with_outcome(&self, outcome: RibbonDialogOutcome) {
        let scope = self.scope.borrow().as_ref().and_then(Weak::upgrade);
        let Some(scope) = scope.filter(|s| s.is_open()) else {
            return;
        };
        // Set before dismissal can run focus/unload callbacks that re-show this
        // same instance. A new show resets its own result; do not overwrite it.
        self.last_outcome.set(Some(outcome));
        if scope.dismiss() {
            let current = self.scope.borrow().as_ref().and_then(Weak::upgrade);
            if current.is_some_and(|current| Rc::ptr_eq(&current, &scope)) {
                self.detach_sizing();
                self.scope.replace(None);
            }
        }
    }
    fn attach_sizing(&self, scope: &RibbonDialogSession) {
        self.detach_sizing();
        if let Some(owner) = scope.owner() {
            let weak = self.to_ref().downgrade();
            let weak_owner = owner.downgrade();
            self.sizing.replace(Some(owner.property_changed(move |e| {
                if e.property() == TopLevel::client_size_property().as_property()
                    && let (Some(dialog), Some(owner)) = (weak.upgrade(), weak_owner.upgrade())
                {
                    dialog.update_width(owner.client_size().width);
                }
            })));
            self.update_width(owner.client_size().width);
        }
    }
    fn detach_sizing(&self) {
        if let Some(token) = self.sizing.take() {
            token.dispose();
        }
    }
    fn update_width(&self, width: f64) {
        if width > 0. {
            self.body.set_width((width - 96.).clamp(320., 860.));
        }
    }
    fn build(&self, page: RibbonCustomizePage) -> DialogResult<()> {
        let strings = RibbonStrings::current();
        let ribbon_title = utf8(strings.customize_ribbon(), "CustomizeRibbon")?
            .ok_or(RibbonCustomizeDialogError::MissingLocalizedText(
                "CustomizeRibbon",
            ))?
            .trim_end_matches('.')
            .to_string();
        self.set_title(boxed(if page == RibbonCustomizePage::Ribbon {
            Some(ribbon_title.clone())
        } else {
            utf8(
                strings.customize_quick_access_toolbar(),
                "CustomizeQuickAccessToolbar",
            )?
        }));
        self.set_primary_button_text(utf8(strings.ok(), "Ok")?);
        self.set_close_button_text(utf8(strings.cancel(), "Cancel")?);
        self.set_default_button(crate::compat::ContentDialogButton::Primary);
        self.resources()
            .set("ContentDialogMaxWidth", Some(Rc::new(960f64)));
        self.resources()
            .set("ContentDialogMaxHeight", Some(Rc::new(760f64)));
        self.body.set_width(860.);
        let header = StackPanel::new();
        header.set_orientation(Orientation::Horizontal);
        header.set_spacing(4.);
        header.set_margin(ferroui_base::Thickness::new(0., 0., 0., 10.));
        self.ribbon_toggle.set_content(boxed(Some(ribbon_title)));
        self.qat_toggle.set_content(boxed(utf8(
            strings.quick_access_toolbar(),
            "QuickAccessToolbar",
        )?));
        for (button, page) in [
            (&self.ribbon_toggle, RibbonCustomizePage::Ribbon),
            (&self.qat_toggle, RibbonCustomizePage::QuickAccessToolbar),
        ] {
            button.set_padding(ferroui_base::Thickness::new(12., 6., 12., 6.));
            let weak = self.to_ref().downgrade();
            button.click(move |_, _| {
                if let Some(dialog) = weak.upgrade() {
                    dialog.show_page(page);
                }
            });
            let weak = self.to_ref().downgrade();
            self.subscriptions.borrow_mut().push(
                button
                    .actual_theme_variant_changed(move || {
                        if let Some(dialog) = weak.upgrade() {
                            dialog.update_page_visuals();
                        }
                    })
                    .detach(),
            );
            header.children().add(button.clone());
        }
        self.build_ribbon_page(&strings)?;
        self.build_qat_page(&strings)?;
        let pages = Grid::new();
        pages.children().add(self.ribbon_page.clone());
        pages.children().add(self.qat_page.clone());
        self.show_page(page);
        self.body.children().add(header);
        self.body.children().add(pages);
        self.set_content(Some(Control::boxed(self.body.clone())));
        self.structure_list
            .set_name(utf8(strings.main_tabs(), "MainTabs")?.as_deref());
        self.command_list
            .set_name(utf8(strings.choose_commands(), "ChooseCommands")?.as_deref());
        self.qat_list
            .set_name(utf8(strings.quick_access_toolbar(), "QuickAccessToolbar")?.as_deref());
        self.search
            .set_placeholder_text(utf8(strings.search(), "Search")?.as_deref());
        let weak = self.to_ref().downgrade();
        self.search.text_changed(move |_, _| {
            if let Some(dialog) = weak.upgrade() {
                dialog
                    .fill_commands()
                    .expect("customization command filter");
            }
        });
        self.fill_commands()?;
        self.fill_structure()?;
        self.fill_qat()?;
        Ok(())
    }
    pub fn show_page(&self, page: RibbonCustomizePage) {
        self.page.set(page);
        self.ribbon_page
            .set_is_visible(page == RibbonCustomizePage::Ribbon);
        self.qat_page
            .set_is_visible(page == RibbonCustomizePage::QuickAccessToolbar);
        self.update_page_visuals();
    }
    fn update_page_visuals(&self) {
        for (button, checked) in [
            (
                &self.ribbon_toggle,
                self.page.get() == RibbonCustomizePage::Ribbon,
            ),
            (
                &self.qat_toggle,
                self.page.get() == RibbonCustomizePage::QuickAccessToolbar,
            ),
        ] {
            button.set_font_weight(if checked {
                FontWeight::SemiBold
            } else {
                FontWeight::Normal
            });
            let brush = if checked {
                RibbonTheme::get_brush(Some(button), Some("RibbonAccentBrush"))
                    .expect("customization tab accent")
                    .map(Rc::<dyn IBrush>::from)
            } else {
                None
            };
            button.set_border_brush(brush);
            button.set_border_thickness(ferroui_base::Thickness::new(
                0.,
                0.,
                0.,
                if checked { 2. } else { 0. },
            ));
        }
    }
    fn stack(children: impl IntoIterator<Item = Ref<Control>>) -> Ref<StackPanel> {
        let panel = StackPanel::new();
        panel.set_spacing(6.);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        for child in children {
            panel.children().add(child);
        }
        panel
    }
    fn labeled(label: Option<String>, content: Ref<Control>) -> Ref<StackPanel> {
        let panel = StackPanel::new();
        panel.set_spacing(6.);
        let title = TextBlock::new();
        title.set_text(label.as_deref());
        title.set_font_weight(FontWeight::SemiBold);
        panel.children().add(title);
        panel.children().add(content);
        panel
    }
    fn columns(
        left: Ref<Control>,
        middle: Ref<Control>,
        right: Ref<Control>,
        buttons: Ref<Control>,
    ) -> Ref<Grid> {
        let grid = Grid::new();
        grid.set_column_spacing(12.);
        grid.set_column_definitions(
            ColumnDefinitions::parse("*,Auto,*,Auto").expect("source customization columns"),
        );
        for (i, child) in [left, middle, right, buttons].into_iter().enumerate() {
            Grid::set_column(&child, i as i32);
            grid.children().add(child);
        }
        grid
    }
    fn command(&self, label: Option<RibbonString>, action: Action) -> DialogResult<Ref<Control>> {
        let button = Button::new();
        button.set_content(boxed(utf8(label, "Customization.Command")?));
        button.set_horizontal_alignment(HorizontalAlignment::Stretch);
        button.set_min_width(110.);
        let weak = self.to_ref().downgrade();
        button.click(move |_, _| {
            if let Some(dialog) = weak.upgrade() {
                dialog.action(action).expect("customization editing action");
            }
        });
        Ok(button.upcast())
    }
    fn build_ribbon_page(&self, s: &RibbonStrings) -> DialogResult<()> {
        let left = Self::labeled(
            utf8(s.choose_commands(), "ChooseCommands")?,
            Self::stack([
                self.search.clone().upcast(),
                self.command_list.root().upcast(),
            ])
            .upcast(),
        );
        let middle = Self::stack([
            self.command(s.add(), Action::Add)?,
            self.command(s.remove(), Action::Remove)?,
        ]);
        let right = Self::labeled(
            utf8(s.main_tabs(), "MainTabs")?,
            self.structure_list.root().upcast(),
        );
        let buttons = Self::stack([
            self.command(s.move_up(), Action::Up)?,
            self.command(s.move_down(), Action::Down)?,
            self.command(s.new_tab(), Action::NewTab)?,
            self.command(s.new_group(), Action::NewGroup)?,
            self.command(s.rename(), Action::Rename)?,
            self.command(s.reset(), Action::Reset)?,
            self.command(s.import_export(), Action::ImportExport)?,
        ]);
        self.ribbon_page.children().add(Self::columns(
            left.upcast(),
            middle.upcast(),
            right.upcast(),
            buttons.upcast(),
        ));
        let weak = self.to_ref().downgrade();
        self.subscriptions
            .borrow_mut()
            .push(self.structure_list.toggle_changed(move |id, visible| {
                if let Some(dialog) = weak.upgrade() {
                    dialog
                        .editor
                        .borrow_mut()
                        .set_visible(id.as_ref(), visible)
                        .expect("customization visibility");
                }
            }));
        Ok(())
    }
    fn build_qat_page(&self, s: &RibbonStrings) -> DialogResult<()> {
        self.qat_commands.fill(self.command_rows()?);
        let left = Self::labeled(
            utf8(s.choose_commands(), "ChooseCommands")?,
            self.qat_commands.root().upcast(),
        );
        let middle = Self::stack([
            self.command(s.add(), Action::QatAdd)?,
            self.command(s.remove(), Action::QatRemove)?,
        ]);
        let right = Self::labeled(
            utf8(s.quick_access_toolbar(), "QuickAccessToolbar")?,
            self.qat_list.root().upcast(),
        );
        self.qat_below
            .set_content(boxed(utf8(s.show_below_ribbon(), "ShowBelowRibbon")?));
        self.qat_below.set_is_checked(Some(
            self.working_state().quick_access_position() == RibbonQuickAccessPosition::BelowRibbon,
        ));
        let weak = self.to_ref().downgrade();
        let weak_check = self.qat_below.downgrade();
        self.qat_below.is_checked_changed(move |_, _| {
            if let (Some(dialog), Some(check)) = (weak.upgrade(), weak_check.upgrade())
                && let Some(checked) = check.is_checked()
            {
                dialog
                    .working_state()
                    .set_quick_access_position(if checked {
                        RibbonQuickAccessPosition::BelowRibbon
                    } else {
                        RibbonQuickAccessPosition::AboveRibbon
                    });
            }
        });
        let buttons = Self::stack([
            self.command(s.move_up(), Action::QatUp)?,
            self.command(s.move_down(), Action::QatDown)?,
            self.command(s.reset(), Action::QatReset)?,
            self.qat_below.clone().upcast(),
        ]);
        self.qat_page.children().add(Self::columns(
            left.upcast(),
            middle.upcast(),
            right.upcast(),
            buttons.upcast(),
        ));
        Ok(())
    }
    fn command_rows(&self) -> DialogResult<Vec<Rc<NativeRow>>> {
        self.editor
            .borrow()
            .commands()
            .iter()
            .map(|c| {
                Ok(Rc::new(NativeRow {
                    id: Some(c.id.clone()),
                    label: utf8(c.label.clone(), "Command.Label")?,
                    detail: utf8(c.path.clone(), "Command.Path")?,
                    icon: crate::metadata::model_objects::to_native(c.icon.clone())
                        .map_err(|error| RibbonCustomizeDialogError::State(error.to_string()))?,
                    level: 0,
                    is_checked: None,
                }))
            })
            .collect()
    }
    fn fill_commands(&self) -> DialogResult<()> {
        let query = self.search.text().unwrap_or_default();
        let mut rows = self.command_rows()?;
        if !ferroribbon_core::text::trim(&query.encode_utf16().collect::<Vec<_>>()).is_empty() {
            let comparer = CommandText::current()?;
            rows.retain(|row| {
                comparer.contains(row.label.as_deref().unwrap_or(""), &query)
                    || comparer.contains(row.detail.as_deref().unwrap_or(""), &query)
            });
        }
        self.command_list.fill(rows);
        Ok(())
    }
    fn sync_editor_selection(&self) {
        let selected = self.structure_list.selected().and_then(|r| r.id.clone());
        self.editor.borrow_mut().select(selected.as_ref());
    }
    fn fill_structure(&self) -> DialogResult<()> {
        self.sync_editor_selection();
        {
            let mut editor = self.editor.borrow_mut();
            editor.set_tabs(snapshot_tabs(&self.ribbon));
            editor.set_custom_suffix(RibbonStrings::current().custom_group_suffix());
            editor.refresh()?;
        }
        let (rows, selected) = {
            let editor = self.editor.borrow();
            (
                editor.rows().to_vec(),
                editor.selected().and_then(|r| r.id.clone()),
            )
        };
        let rows = rows
            .iter()
            .map(|row| {
                Ok(Rc::new(NativeRow {
                    id: row.id.clone(),
                    label: utf8(row.label.clone(), "Structure.Label")?,
                    detail: None,
                    icon: None,
                    level: row.kind.level() as i32,
                    is_checked: if row.kind == RibbonCustomizationRowKind::Command {
                        None
                    } else {
                        Some(!row.hidden)
                    },
                }))
            })
            .collect::<DialogResult<Vec<_>>>()?;
        self.structure_list.fill(rows);
        if let Some(selected) = selected {
            self.structure_list.select(Some(&selected));
        }
        Ok(())
    }
    fn fill_qat(&self) -> DialogResult<()> {
        let own = self
            .ribbon
            .quick_access_tool_bar()
            .map(|bar| {
                bar.items()
                    .iter()
                    .filter_map(|c| c.and_then(|c| as_item(&c)))
                    .filter_map(|c| {
                        c.id().map(|id| RibbonCustomizationCommand {
                            id,
                            label: c.label(),
                            path: None,
                            icon: crate::metadata::model_objects::from_native(c.icon()),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let rows = self
            .editor
            .borrow()
            .quick_access_rows(&own)?
            .into_iter()
            .map(|row| {
                Ok(Rc::new(NativeRow {
                    id: row.id,
                    label: utf8(row.label, "Qat.Label")?,
                    detail: utf8(row.detail, "Qat.Detail")?,
                    icon: crate::metadata::model_objects::to_native(row.icon)
                        .map_err(|error| RibbonCustomizeDialogError::State(error.to_string()))?,
                    level: 0,
                    is_checked: None,
                }))
            })
            .collect::<DialogResult<Vec<_>>>()?;
        self.qat_list.fill(rows);
        Ok(())
    }
    fn id(prefix: &str) -> RibbonString {
        format!(
            "custom.{prefix}.{}",
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        )
        .into()
    }
    fn action(&self, action: Action) -> DialogResult<()> {
        if matches!(action, Action::Rename) {
            return self.rename();
        }
        if matches!(action, Action::ImportExport) {
            return self.import_export();
        }
        if matches!(
            action,
            Action::QatAdd | Action::QatRemove | Action::QatUp | Action::QatDown | Action::QatReset
        ) {
            return self.qat_action(action);
        }
        self.sync_editor_selection();
        let before = self.editor.borrow().rows().to_vec();
        let selected_tab = self
            .ribbon
            .selected_tab()
            .and_then(|t| t.effective_id())
            .map(Into::into);
        let strings = RibbonStrings::current();
        {
            let mut editor = self.editor.borrow_mut();
            editor.set_tabs(snapshot_tabs(&self.ribbon));
            editor.set_custom_suffix(strings.custom_group_suffix());
            match action {
                Action::Add => {
                    if let Some(id) = self.command_list.selected().and_then(|row| row.id.clone()) {
                        editor.add_command(
                            &id,
                            Self::id("group"),
                            strings.new_group(),
                            selected_tab,
                        )?;
                    } else {
                        return Ok(());
                    }
                }
                Action::Remove => editor.remove_selected()?,
                Action::Up => editor.move_selected(-1)?,
                Action::Down => editor.move_selected(1)?,
                Action::NewTab => editor.new_tab(
                    Self::id("tab"),
                    strings.new_tab(),
                    Self::id("group"),
                    strings.new_group(),
                )?,
                Action::NewGroup => {
                    editor.new_group(Self::id("group"), strings.new_group(), selected_tab)?
                }
                Action::Reset => editor.reset_customization()?,
                _ => unreachable!(),
            }
        }
        // The editor restores selection by ID after source edits. Fill the native
        // rows without replacing that selection with the outgoing list's value.
        if self.editor.borrow().rows().len() == before.len()
            && self
                .editor
                .borrow()
                .rows()
                .iter()
                .zip(&before)
                .all(|(a, b)| Rc::ptr_eq(a, b))
        {
            return Ok(());
        }
        self.render_edited_structure()
    }
    fn render_edited_structure(&self) -> DialogResult<()> {
        let (rows, id) = {
            let editor = self.editor.borrow();
            (
                editor.rows().to_vec(),
                editor.selected().and_then(|r| r.id.clone()),
            )
        };
        let rows = rows
            .into_iter()
            .map(|row| {
                Ok(Rc::new(NativeRow {
                    id: row.id.clone(),
                    label: utf8(row.label.clone(), "Structure.Label")?,
                    detail: None,
                    icon: None,
                    level: row.kind.level() as i32,
                    is_checked: (row.kind != RibbonCustomizationRowKind::Command)
                        .then_some(!row.hidden),
                }))
            })
            .collect::<DialogResult<Vec<_>>>()?;
        self.structure_list.fill(rows);
        if let Some(id) = id {
            self.structure_list.select(Some(&id));
        }
        Ok(())
    }
    fn qat_action(&self, action: Action) -> DialogResult<()> {
        match action {
            Action::QatAdd => {
                if let Some(id) = self.qat_commands.selected().and_then(|row| row.id.clone()) {
                    let added = self.editor.borrow().add_quick_access(id)?;
                    if added {
                        self.fill_qat()?;
                    }
                }
            }
            Action::QatRemove => {
                if let Some(row) = self.qat_list.selected() {
                    self.editor.borrow().remove_quick_access(row.id.clone())?;
                    self.fill_qat()?;
                }
            }
            Action::QatUp | Action::QatDown => {
                if let Some(row) = self.qat_list.selected()
                    && self.editor.borrow().move_quick_access(
                        row.id.clone(),
                        if matches!(action, Action::QatUp) {
                            -1
                        } else {
                            1
                        },
                    )?
                {
                    self.fill_qat()?;
                    self.qat_list.select(row.id.as_ref());
                }
            }
            Action::QatReset => {
                let ids = self
                    .ribbon
                    .default_quick_access_item_ids()
                    .unwrap_or_else(|| self.ribbon.get_quick_access_item_ids());
                self.editor
                    .borrow()
                    .reset_quick_access(ids.iter(), self.ribbon.default_quick_access_position());
                self.qat_below.set_is_checked(Some(
                    self.working_state().quick_access_position()
                        == RibbonQuickAccessPosition::BelowRibbon,
                ));
                self.fill_qat()?;
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
impl Drop for RibbonCustomizeDialog {
    fn drop(&mut self) {
        if let Some(token) = self.sizing.get_mut().take() {
            token.dispose();
        }
        for token in self.subscriptions.get_mut().drain(..) {
            token.dispose();
        }
    }
}

#[cfg(test)]
#[path = "../customize_dialog_tests.rs"]
mod tests;
