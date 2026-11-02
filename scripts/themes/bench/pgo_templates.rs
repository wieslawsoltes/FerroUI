use ferroui_base::{
    controls::ResourceKey, metadata::from_markup_value, styling::ControlTheme, Ref, TypeInfo,
    Visual,
};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::{Application, Button, NavigationView, ScrollViewer};
use ferroui_themes_winui::WinUITheme;
use std::time::Instant;

fn count_visuals(visual: &Visual) -> usize {
    1 + visual.visual_children_snapshot().map_or(0, |children| {
        children.iter().map(|child| count_visuals(child)).sum()
    })
}

fn activate(theme: &Ref<ControlTheme>, new_control: fn() -> Ref<TemplatedControl>) -> usize {
    let control = new_control();
    control.set_theme(theme);
    control.apply_styling();
    control.apply_template();
    assert_eq!(
        control.visual_children_count(),
        1,
        "the template must produce one visual root"
    );
    count_visuals(&control)
}

pub fn run() {
    let app = Application::new();
    Application::bind_current(app.clone());
    let theme = WinUITheme::new();
    app.styles().add(theme.as_style());
    let factories: [(&TypeInfo, fn() -> Ref<TemplatedControl>); 3] = [
        (Button::TYPE, || Button::new().upcast()),
        (ScrollViewer::TYPE, || ScrollViewer::new().upcast()),
        (NavigationView::TYPE, || NavigationView::new().upcast()),
    ];
    for (ty, create) in factories {
        let value = theme
            .try_get_resource(&ResourceKey::Type(ty), None)
            .flatten();
        let style = from_markup_value::<Ref<ControlTheme>>(&value).expect("typed theme");
        assert!(style.target_type().is_some_and(|t| std::ptr::eq(ty, t)));
        let start = Instant::now();
        let count = activate(&style, create);
        let first = start.elapsed().as_nanos();
        let start = Instant::now();
        for _ in 0..12 {
            assert_eq!(activate(&style, create), count);
        }
        println!(
            "{} {count} {first} {}",
            ty.name(),
            start.elapsed().as_nanos() / 12
        );
    }
}
