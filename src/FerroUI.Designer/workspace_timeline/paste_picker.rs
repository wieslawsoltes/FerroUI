use super::*;
use ferroui_controls::{PlacementMode, ScrollViewer};

pub(super) struct DestinationChoice {
    pub(super) destination: KeyframeDestination,
    label: String,
}

impl DesignerWorkspace {
    pub(super) fn paste_choices(
        &self,
        clipboard: &KeyframeClipboard,
    ) -> Result<Vec<Vec<DestinationChoice>>, String> {
        let session = self.session.borrow();
        clipboard
            .tracks
            .iter()
            .map(|copied| {
                let mut matches = session
                    .timeline()
                    .tracks
                    .iter()
                    .filter(|track| copied.compatible_with(track))
                    .collect::<Vec<_>>();
                matches.sort_by_key(|track| track.property != copied.property);
                if matches.is_empty() {
                    return Err(format!(
                        "Add a compatible destination track for {}",
                        copied.property
                    ));
                }
                Ok(matches
                    .into_iter()
                    .map(|track| {
                        let name = session
                            .document()
                            .ast()
                            .node(track.target)
                            .map(|node| node.xaml_name().unwrap_or_else(|| node.local_name()));
                        let label = format!("{} · {}", name.unwrap_or("Target"), track.property);
                        DestinationChoice {
                            destination: KeyframeDestination {
                                target: track.target,
                                property: track.property.clone(),
                            },
                            label,
                        }
                    })
                    .collect())
            })
            .collect()
    }

    pub(super) fn open_timeline_paste(
        self: &Rc<Self>,
        context: TimelineContext,
        anchor: &Control,
        paste: KeyframePaste,
        choices: Vec<Vec<DestinationChoice>>,
    ) {
        self.close_timeline_paste();
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::TopEdgeAlignedLeft);
        flyout.flyout_presenter_classes().add("designer-chrome");
        flyout.popup().set_should_use_overlay_layer(true);
        let body = StackPanel::new();
        body.set_width(340.0);
        body.set_spacing(8.0);
        let heading = key_caption(&format!(
            "{} at {} s",
            if paste.reverse {
                "Paste Reversed"
            } else {
                "Paste Keyframes"
            },
            paste.time
        ));
        body.children().add(heading.upcast::<Control>());
        let rows = StackPanel::new();
        rows.set_spacing(6.0);
        let scroll = ScrollViewer::new();
        scroll.set_max_height(256.0);
        scroll.set_content(Some(Control::boxed(&rows)));
        body.children().add(scroll.upcast::<Control>());
        let mut picks = Vec::new();
        for (index, (copied, available)) in paste.clipboard.tracks.iter().zip(choices).enumerate() {
            let label = format!(
                "{}. {} · {} keys",
                index + 1,
                copied.property,
                copied.keyframes.len()
            );
            rows.children().add(key_caption(&label).upcast::<Control>());
            let pick = ComboBox::new();
            pick.classes().add("designer-field");
            pick.set_horizontal_alignment(HorizontalAlignment::Stretch);
            AutomationProperties::set_name(&pick, Some(&format!("Destination for {label}")));
            for choice in &available {
                pick.items().add(Some(Rc::new(choice.label.clone())));
            }
            if available.len() == 1 {
                pick.set_selected_index(0);
            }
            picks.push((pick.downgrade(), available));
            rows.children().add(pick.upcast::<Control>());
        }
        let error = TextBlock::new();
        error.set_text_wrapping(ferroui_base::media::TextWrapping::Wrap);
        error.set_is_visible(false);
        body.children().add(error.clone().upcast::<Control>());
        let apply = button("Paste");
        apply.classes().add("designer-motion");
        apply.classes().add("designer-timeline-tool");
        let weak = Rc::downgrade(self);
        let error = error.downgrade();
        let active = Rc::new(Cell::new(true));
        let open = active.clone();
        flyout.closed(move || open.set(false)).detach();
        apply.click(move |_, _| {
            if !active.get() {
                return;
            }
            let Some(workspace) = weak.upgrade() else {
                return;
            };
            let mut paste = paste.clone();
            let destinations = picks
                .iter()
                .map(|(pick, available)| {
                    let index: usize = pick.upgrade()?.selected_index().try_into().ok()?;
                    available
                        .get(index)
                        .map(|choice| choice.destination.clone())
                })
                .collect::<Option<Vec<_>>>();
            let result = if let Some(destinations) = destinations {
                paste.destinations = destinations;
                workspace.apply_timeline_paste(&context, paste)
            } else {
                Err("Choose a destination for each copied track".into())
            };
            if let (Err(message), Some(error)) = (result, error.upgrade()) {
                error.set_text(Some(&message));
                error.set_is_visible(true);
            }
        });
        body.children().add(apply.upcast::<Control>());
        flyout.set_content(Some(Control::boxed(&body)));
        self.timeline_ui.borrow_mut().keyframe_paste = Some(flyout.clone());
        flyout.show_at(anchor);
    }
}
