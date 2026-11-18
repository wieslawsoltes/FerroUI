use crate::renderer::{Presentation, Renderer};
use ferroui_base::{Ref, Visual};
use ferroui_controls::{Control, TopLevel};
use ferroui_inspect::{Inspector, Permissions};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Input {
    frame: u64,
    next_frame: u64,
    action: Action,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum Action {
    Click { x: f64, y: f64 },
    Scroll { x: f64, y: f64, dx: f64, dy: f64 },
    Text { text: String },
    Key { key: String },
}

pub(crate) fn input(renderer: &mut Renderer, request: Input) -> Result<Presentation, String> {
    let stage = renderer.stage(request.frame)?;
    if request.next_frame <= request.frame {
        return Err("Use a new canvas frame for preview input".into());
    }
    let host: Ref<Control> = stage.session.borrow().preview_host().upcast();
    {
        let inspector = Inspector::shared();
        let mut inspector = inspector.borrow_mut();
        let (tool, args) = match request.action {
            Action::Click { x, y } => {
                let (x, y) = point(&host, x, y)?;
                (
                    "click",
                    json!({ "node_id": inspector.node_id(&host), "x": x, "y": y }),
                )
            }
            Action::Scroll { x, y, dx, dy } => {
                if !(-64.0..=64.0).contains(&dx) || !(-64.0..=64.0).contains(&dy) {
                    return Err("Scroll in steps of at most 64 wheel increments".into());
                }
                let (x, y) = point(&host, x, y)?;
                (
                    "scroll",
                    json!({ "node_id": inspector.node_id(&host), "x": x, "y": y, "delta_x": dx, "delta_y": dy }),
                )
            }
            Action::Text { text } if !text.is_empty() && text.len() <= 8192 => {
                let control = focused(&host)?;
                (
                    "type_text",
                    json!({ "node_id": inspector.node_id(&control), "text": text }),
                )
            }
            Action::Key { key } if !key.is_empty() && key.len() <= 64 => {
                let control = focused(&host)?;
                (
                    "press_key",
                    json!({ "node_id": inspector.node_id(&control), "key": key }),
                )
            }
            Action::Text { .. } | Action::Key { .. } => {
                return Err("Enter a valid preview input".into())
            }
        };
        inspector.execute(
            tool,
            &args,
            Permissions {
                allow_input: true,
                ..Default::default()
            },
        )?;
    }
    renderer.present(request.next_frame)
}

fn point(host: &Control, x: f64, y: f64) -> Result<(f64, f64), String> {
    if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
        return Err("Choose a point inside the canvas".into());
    }
    let size = host.bounds().size();
    Ok((size.width * x, size.height * y))
}

fn focused(host: &Ref<Control>) -> Result<Ref<Control>, String> {
    TopLevel::get_top_level(Some(host))
        .and_then(|top| top.focus_manager().get_focused_element())
        .and_then(|element| element.cast::<Control>())
        .filter(|control| {
            host.upcast_ref::<Visual>()
                .is_visual_ancestor_of(control.upcast_ref::<Visual>())
        })
        .ok_or_else(|| "Click a focusable control in the preview first".into())
}
