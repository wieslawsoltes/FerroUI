//! The private compositor can stage live clients over the existing wallpaper.
//! Delegated sessions retain their compact client lists on a separate surface.
use super::*;

pub(super) fn map(rects: &[ferroui_base::Rect]) -> Element {
    const WIDTH: f64 = 72.0;
    const HEIGHT: f64 = 40.0;
    let mut map = canvas().width(WIDTH).height(HEIGHT).clip_to_bounds(true);
    for r in rects {
        map = map.child(
            border()
                .canvas_left(r.x * WIDTH)
                .canvas_top(r.y * HEIGHT)
                .width(r.width * WIDTH)
                .height(r.height * HEIGHT)
                .background(colors().tile)
                .border_brush(colors().accent)
                .border_thickness(1)
                .corner_radius(2),
        );
    }
    border()
        .corner_radius(4)
        .clip_to_bounds(true)
        .background(colors().surface)
        .is_hit_test_visible(false)
        .child(map)
        .into()
}

pub(super) fn stage(
    shell: &Rc<Shell>,
    layout: Layout,
    title: &str,
    tabs: Element,
    empty: Element,
    windows: Element,
) -> Element {
    let width = shell.metrics().width;
    let snapshot = shell.snapshot;
    let window_count = memo(move |_| {
        snapshot.with(|s| {
            s.windows
                .iter()
                .filter(|w| w.workspace.is_none_or(|ws| ws == s.current_workspace))
                .count()
                .max(1)
        })
    });
    let recents = layout == Layout::BottomNavigation;
    let spaces_on_top = layout == Layout::MenuBarDock;
    let cards = if recents {
        row(10.0).child(windows).into_element()
    } else {
        uniform_grid()
            .columns(move || {
                let fit = (width.get() / 340.0).floor().clamp(1.0, 4.0) as usize;
                fit.min(window_count.get()) as i32
            })
            .child(windows)
            .into_element()
    };
    let close = shell.clone();
    let close_ref = NodeRef::<ferroui_controls::Button>::new();
    let rail = bar_surface().padding((14.0, 8.0)).child(
        grid()
            .column_definitions("Auto,*,Auto")
            .child(
                shell_text(title, 16.0, Ink::Primary)
                    .font_weight(FontWeight::SemiBold)
                    .margin((0., 0., 12., 0.))
                    .is_visible(move || width.get() >= 560.),
            )
            .child(
                scroll_viewer()
                    .grid_column(1)
                    .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)
                    .vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                    .child(tabs),
            )
            .child(
                icon_action("Exit overview", Icon::Close, move || close.dismiss())
                    .node_ref(close_ref)
                    .on_loaded(move |_| {
                        if let Some(button) = close_ref.get_untracked() {
                            button.focus();
                        }
                    })
                    .grid_column(2)
                    .vertical_alignment(V::Center),
            ),
    );
    let gallery = scroll_viewer()
        .horizontal_scroll_bar_visibility(if recents {
            ScrollBarVisibility::Auto
        } else {
            ScrollBarVisibility::Disabled
        })
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .vertical_alignment(V::Center)
        .child(stack_panel().child(empty).child(cards));
    grid()
        .row_definitions(if spaces_on_top {
            "Auto,*,Auto"
        } else {
            "*,Auto,Auto"
        })
        .child(rail.grid_row(if spaces_on_top { 0 } else { 1 }))
        .child(
            grid()
                .grid_row(if spaces_on_top { 1 } else { 0 })
                .max_width(move || {
                    if recents {
                        1440.0
                    } else {
                        (window_count.get().min(4) * 430) as f64
                    }
                })
                .margin((16., 8.))
                .child(gallery),
        )
        .child(feedback(shell.status).grid_row(2))
        .into()
}
