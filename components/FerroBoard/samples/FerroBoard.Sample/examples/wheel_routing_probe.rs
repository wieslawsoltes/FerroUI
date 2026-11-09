//! Opt-in native ScrollViewer reproducer. No keyboard instance, storage or clipboard service.
use ferroboard_sample::App;
use ferroui_base::{
    Thickness,
    input::{InputElement, InputManager, PointerWheelEventArgs, raw::RawMouseWheelEventArgs},
    interactivity::RoutingStrategies,
    reactive::AnonymousObserver,
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{AppBuilder, Control, ScrollViewer, StackPanel, TextBlock, TextBox, Window};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_system::threading::CancellationToken;
use serde_json::{Value, json};
use std::{cell::RefCell, error::Error, fs::OpenOptions, io::Write, path::PathBuf, rc::Rc};

#[derive(Clone)]
struct Log(Rc<RefCell<Box<dyn Write>>>);
impl Log {
    fn event(&self, value: Value) {
        let mut file = self.0.borrow_mut();
        serde_json::to_writer(&mut *file, &value).expect("write probe JSON");
        writeln!(file).expect("write probe newline");
        file.flush().expect("flush probe log");
    }
}
fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args_os().skip(1);
    let output = PathBuf::from(
        args.next()
            .ok_or("provide --stdout or a new SSD JSONL output path")?,
    );
    if args.next().is_some() {
        return Err("expected one output path".into());
    }
    let writer: Box<dyn Write> = if output.as_os_str() == "--stdout" {
        Box::new(std::io::stdout())
    } else {
        Box::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output)?,
        )
    };
    let log = Log(Rc::new(RefCell::new(writer)));
    AppBuilder::configure::<App>()
        .use_platform_detect()
        .setup_without_starting();
    let window = Window::new();
    window.set_title(Some("FerroBoard wheel routing probe".into()));
    window.set_width(640.0);
    window.set_height(600.0);
    let scroller = ScrollViewer::new();
    scroller.set_name(Some("ProbeScroller".into()));
    let rows = StackPanel::new();
    rows.set_margin(Thickness::uniform(16.0));
    for index in 0..40 {
        let label = TextBlock::new();
        label.set_text(Some(&format!(
            "Label {index:02} — wheel over a label or text field"
        )));
        label.set_margin(Thickness::uniform(6.0));
        rows.children().add(label.upcast::<Control>());
        let input = TextBox::new();
        input.set_name(Some(format!("ProbeField{index:02}")));
        input.set_text(Some(&format!("Synthetic row {index:02}")));
        input.set_margin(Thickness::uniform(6.0));
        rows.children().add(input.upcast::<Control>());
    }
    scroller.set_content(Some(Control::boxed(rows)));
    window.set_content(Some(Control::boxed(scroller.clone())));
    let manager = InputManager::instance().ok_or("native input manager unavailable")?;
    let raw_log = log.clone();
    let raw = manager.pre_process().subscribe(Rc::new(AnonymousObserver::new(
        move |event: Rc<dyn ferroui_base::input::raw::IRawInputEventArgs>| {
            if let Some(wheel) = event.downcast_ref::<RawMouseWheelEventArgs>() {
                raw_log.event(json!({"kind":"raw_wheel","x":wheel.position().x,"y":wheel.position().y,"dx":wheel.delta().x,"dy":wheel.delta().y}));
            }
        },
    )));
    let mut routes = Vec::new();
    for (phase, route) in [
        ("tunnel", RoutingStrategies::TUNNEL),
        ("bubble", RoutingStrategies::BUBBLE),
    ] {
        let routed_log = log.clone();
        routes.push(window.add_handler_with(
            InputElement::pointer_wheel_changed_event(),
            move |_, event: &PointerWheelEventArgs| {
                routed_log.event(json!({"kind":"routed_wheel","phase":phase,"dx":event.delta().x,"dy":event.delta().y,"handled":event.handled(),"source":event.source().map(|source|source.get_type().name().to_owned())}));
            },
            route,
            true,
        ));
    }
    let changed_log = log.clone();
    let weak = scroller.downgrade();
    let changed = scroller.scroll_changed(move |_, _| {
        if let Some(scroll) = weak.upgrade() {
            changed_log.event(json!({"kind":"scroll_changed","x":scroll.offset().x,"y":scroll.offset().y,"extent_height":scroll.extent().height,"viewport_height":scroll.viewport().height}));
        }
    });
    let closed = window.closed(|| {
        Dispatcher::ui_thread().begin_invoke_shutdown(DispatcherPriority::NORMAL);
    });
    window.show();
    log.event(json!({"kind":"shown","scope":"plain native ScrollViewer; no keyboard instance or sample session","os":std::env::consts::OS}));
    Dispatcher::ui_thread().main_loop(&CancellationToken::none());
    raw.dispose();
    closed.dispose();
    for token in routes {
        window.remove_handler(InputElement::pointer_wheel_changed_event(), token);
    }
    scroller.remove_handler(ScrollViewer::scroll_changed_event(), changed);
    log.event(json!({"kind":"closed"}));
    Ok(())
}
