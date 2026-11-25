mod xaml;

use ferroui_base::Ref;
use ferroui_controls::{ContentControl, Window};
use ferroui_view::prelude::*;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
};

const BUILD_LABEL: &str = "Original build";

fn main() {
    // Keep this separate from the source file so the workbench watches authored saves.
    let receipts = PathBuf::from(std::env::var_os("FERROUI_TEST_RUNTIME_RECEIPTS").unwrap());
    if BUILD_LABEL == "Rejected startup" {
        std::fs::write(
            receipts.join("rejected-startup"),
            std::process::id().to_string(),
        )
        .unwrap();
        std::process::exit(1);
    }
    #[cfg(target_os = "linux")]
    if std::env::var_os("FERROUI_TEST_RUNTIME_DEBUG").is_some() {
        // SAFETY: This test app grants ptrace access to its owning VS Code host and
        // its debugger children. The zero arguments are unused by PR_SET_PTRACER.
        let result = unsafe {
            libc::prctl(
                libc::PR_SET_PTRACER,
                libc::getppid() as libc::c_ulong,
                0 as libc::c_ulong,
                0 as libc::c_ulong,
                0 as libc::c_ulong,
            )
        };
        if result == -1 {
            let error = std::io::Error::last_os_error();
            assert_eq!(
                error.raw_os_error(),
                Some(libc::EINVAL),
                "scope Linux debugger attachment: {error}"
            );
        }
    }
    let state = receipts.join("review-count");
    let host = Rc::new(RefCell::new(None));
    let mounted_host = host.clone();

    ferroui_view::launch_themed(
        || ferroui_themes_simple::SimpleTheme::new().as_style(),
        move || {
            let content = ContentControl::new();
            *host.borrow_mut() = Some(content.clone());
            window()
                .title("Release Review")
                .width(440.0)
                .height(320.0)
                .child(review().content(content).state(state))
                .into_element()
        },
        move |window| {
            xaml::watch(window, mounted_host.borrow_mut().take().unwrap(), &receipts);
            inspect(window, &receipts);
        },
    );
}

#[component(rerender)]
fn Review(content: Ref<ContentControl>, state: PathBuf) -> Element {
    let count = use_signal(|| {
        std::fs::read_to_string(&state)
            .ok()
            .and_then(|text| text.parse::<u32>().ok())
            .unwrap_or_default()
    });
    rsx! {
        StackPanel { margin: 24.0, spacing: 12.0,
            TextBlock { name: "RsxStatus", text: "Initial RSX status" }
            TextBlock { name: "BuildStatus", text: BUILD_LABEL }
            { Element::Existing(content.clone().upcast()) }
            TextBlock { name: "SavedCount", text: format!("Reviews saved: {}", count.get()) }
            Button { name: "ReviewRecord", on_click: move |_| {
                let next = count.get_untracked() + 1;
                save_review(&state, next);
                count.set(next);
            }, "Approve record" }
        }
    }
}

#[inline(never)]
fn save_review(state: &Path, count: u32) {
    std::fs::write(state, count.to_string()).expect("save the review count");
}

fn inspect(window: &Ref<Window>, receipts: &std::path::Path) {
    let root = window.clone().upcast();
    ferroui_inspect::Inspector::shared()
        .borrow_mut()
        .register_root(&root);
    let server = ferroui_mcp::serve(ferroui_mcp::Options {
        permissions: ferroui_mcp::Permissions {
            allow_input: true,
            ..Default::default()
        },
        ..Default::default()
    })
    .expect("start the release fixture inspector");
    let pid = std::process::id();
    let data = serde_json::json!({
        "url": format!("http://{}/mcp", server.address.unwrap()),
        "token": server.token.as_deref().unwrap(),
    });
    let temporary = receipts.join(format!("{pid}-inspector.tmp"));
    std::fs::write(&temporary, data.to_string()).unwrap();
    std::fs::rename(temporary, receipts.join(format!("{pid}-inspector.json"))).unwrap();
    let server = RefCell::new(Some(server));
    window
        .closed(move || {
            server.borrow_mut().take();
        })
        .detach();
}
