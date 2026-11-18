use crate::protocol::{DragOrigin, DropHint, DropPosition, DropTarget, Placement};
use crate::renderer::{Renderer, Stage};
use crate::toolbox;
use ferroui_base::Point;
use ferroui_designer::{OutlinePlacement, PlacementOrigin, PlacementTarget};
use ferroui_designer_core::{Document, Edit};

impl Renderer {
    pub(crate) fn stage(&self, frame: u64) -> Result<&Stage, String> {
        self.stage
            .as_ref()
            .filter(|stage| stage.token == frame)
            .ok_or_else(|| {
                "The rendered layout changed. Wait for the current preview and try again.".into()
            })
    }
}

impl Stage {
    pub(crate) fn plan(&self, placement: &Placement) -> Result<(Vec<Edit>, DropHint), String> {
        let image;
        let origin = match &placement.origin {
            DragOrigin::Asset { control } => {
                PlacementOrigin::Asset(&toolbox::find(control)?.source)
            }
            DragOrigin::Image { source } => {
                if self.placement_policy.is_some() {
                    return Err("Place a project image in XAML or use a Rust image expression".into());
                }
                image = image_fragment(source)?;
                PlacementOrigin::Asset(&image)
            }
            DragOrigin::Node { node, grab } => PlacementOrigin::Node {
                node: *node,
                grab: grab.map(|point| Point::new(point.x, point.y)),
            },
        };
        let target = match placement.target {
            DropTarget::Outline { node, position } => {
                let position = match position {
                    DropPosition::Inside => OutlinePlacement::Inside,
                    DropPosition::Before => OutlinePlacement::Before,
                    DropPosition::After => OutlinePlacement::After,
                };
                PlacementTarget::Outline { node, position }
            }
            DropTarget::Canvas { x, y } => {
                if !(0.0..=f64::from(self.width)).contains(&x)
                    || !(0.0..=f64::from(self.height)).contains(&y)
                {
                    return Err("Drop inside the visible artboard".into());
                }
                PlacementTarget::Canvas(Point::new(x, y))
            }
        };
        let session = self.session.borrow();
        let plan = session.plan_placement(origin, target)?;
        if let Some(policy) = &self.placement_policy {
            policy.validate(&placement.origin, session.document(), &plan.edits)?;
        }
        Ok((
            plan.edits,
            DropHint {
                bounds: plan.bounds,
                description: plan.description,
            },
        ))
    }
}

fn image_fragment(source: &str) -> Result<String, String> {
    if source.is_empty() || source.len() > 4096 || source.starts_with('/') || source.contains([':', '\\', '#', '?'])
        || source.chars().any(char::is_control)
    {
        return Err("Choose a relative image from the project Assets view".into());
    }
    let mut document = Document::new(r#"<Image xmlns="https://github.com/ferroui" Stretch="Uniform" Width="120"/>"#)
        .map_err(|error| error.to_string())?;
    let node = document.ast().root();
    document.apply("Set image source", vec![Edit::SetAttribute { node, name: "Source".into(), value: source.into() }])
        .map_err(|error| error.to_string())?;
    Ok(document.source().into())
}
