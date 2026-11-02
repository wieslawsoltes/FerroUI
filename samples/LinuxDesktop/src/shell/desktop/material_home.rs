//! Material Home puts saved favorites first and fills empty hotseat slots from
//! launches in this session. Each shortcut belongs to its real desktop entry.
use super::*;
use crate::services::ApplicationCatalog;
use ferroui_base::input::InputElement;
use ferroui_controls::{Control, MenuFlyout, MenuItem, PlacementMode, Separator};

const SLOT: f64 = 80.0;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Shortcut {
    index: usize,
    suggested: bool,
}

fn shortcuts(
    catalog: &ApplicationCatalog,
    pins: &[String],
    recent: &[usize],
    slots: usize,
) -> Vec<Shortcut> {
    let mut items = Vec::with_capacity(slots);
    let pinned = pins
        .iter()
        .filter_map(|id| catalog.index_of(id))
        .map(|index| Shortcut {
            index,
            suggested: false,
        });
    let suggested = recent.iter().copied().map(|index| Shortcut {
        index,
        suggested: true,
    });
    for shortcut in pinned.chain(suggested) {
        if items.len() == slots {
            break;
        }
        if !items
            .iter()
            .any(|item: &Shortcut| item.index == shortcut.index)
        {
            items.push(shortcut);
        }
    }
    items
}

pub(super) fn has_visible_client(snapshot: &crate::session::Snapshot) -> bool {
    snapshot
        .windows
        .iter()
        .any(|w| !w.minimized && w.workspace.is_none_or(|i| i == snapshot.current_workspace))
}

pub(super) fn hotseat(shell: &Rc<Shell>) -> Element {
    let width = shell.metrics().width;
    let slots = memo(move |_| (((width.get() - 24.0) / SLOT) as usize).min(5));
    let snapshot = shell.snapshot;
    let catalog = shell.catalog.clone();
    let pins = shell.favorites;
    let recent = shell.recent_apps;
    let state = shell.clone();
    let (_, bottom) = look().insets();
    row(0.0)
        .is_visible(move || !snapshot.with(has_visible_client))
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Bottom)
        .margin((12.0, 0.0, 12.0, bottom + 80.0))
        .child(for_each(
            move || {
                pins.with(|pins| {
                    recent.with(|recent| shortcuts(&catalog, pins, recent, slots.get()))
                })
            },
            |item| item.index,
            move |item| shortcut_button(&state, item),
        ))
        .into()
}

fn shortcut_button(shell: &Rc<Shell>, shortcut: Shortcut) -> El<Button> {
    let index = shortcut.index;
    let app = &shell.catalog.applications()[index];
    let caption = if shortcut.suggested {
        format!("{}, suggested app", app.name)
    } else {
        app.name.clone()
    };
    let state = shell.clone();
    let menu = shortcut_menu(shell, shortcut);
    let icon = border()
        .width(50)
        .height(50)
        .horizontal_alignment(H::Center)
        .corner_radius(25)
        .background(resource("MaterialSurfaceContainerHighBrush"))
        .border_brush(resource("MaterialPrimaryBrush"))
        .border_thickness(if shortcut.suggested { 3 } else { 0 })
        .child(super::super::launcher::app_icon(shell, app));
    action(&caption, move || {
        state.launch(&state.catalog.applications()[index])
    })
    .class("text")
    .width(SLOT)
    .height(84)
    .max_width(SLOT)
    .padding((3.0, 2.0))
    .corner_radius(16)
    .border_thickness(0)
    .background("Transparent")
    .automation_properties_help_text("Open the shortcut menu for app actions and pinning.")
    .child(
        stack_panel().spacing(4.0).child(icon).child(
            shell_text(&app.name, 12.0, colors().bar_ink)
                .max_width(SLOT - 8.0)
                .font_weight(FontWeight::Medium)
                .horizontal_alignment(H::Center),
        ),
    )
    .with(move |button| {
        InputElement::set_is_holding_enabled(button, true);
        InputElement::set_is_hold_with_mouse_enabled(button, true);
        button.set_context_flyout(&menu);
    })
}

fn shortcut_menu(shell: &Rc<Shell>, shortcut: Shortcut) -> ferroui_base::Ref<MenuFlyout> {
    let index = shortcut.index;
    let app = &shell.catalog.applications()[index];
    let menu = MenuFlyout::new();
    super::popup_host::configure_menu(&menu, 8.0);
    menu.set_placement(PlacementMode::Top);
    let items = menu.items();
    fn item(name: &str, invoke: impl Fn() + 'static) -> ferroui_base::Ref<MenuItem> {
        let action = MenuItem::new();
        action.set_header(Some(Rc::new(name.to_owned())));
        action.add_handler(MenuItem::click_event(), move |_, _| invoke());
        action
    }
    for action in &app.actions {
        let id = action.id.clone();
        let state = shell.clone();
        items.add(Some(Control::boxed(item(&action.label, move || {
            state.launch_entry(&state.catalog.applications()[index], Some(id.clone()));
        }))));
    }
    if !app.actions.is_empty() {
        items.add(Some(Control::boxed(Separator::new())));
    }
    let state = shell.clone();
    let label = if shortcut.suggested {
        "Pin to favorites"
    } else {
        "Remove from favorites"
    };
    items.add(Some(Control::boxed(item(label, move || {
        super::super::launcher::toggle_pin(&state, index)
    }))));
    menu
}
