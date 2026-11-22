use super::DesignerWorkspace;
use crate::DesignerAsset;
use ferroui_base::{
    threading::{Dispatcher, DispatcherPriority},
    Ref,
};
use ferroui_controls::{ContextMenu, Control, MenuItem};
use ferroui_designer_core::NodeId;
use std::rc::Rc;

impl DesignerWorkspace {
    pub(super) fn prepare_raster_menu(
        self: &Rc<Self>,
        asset: &DesignerAsset,
        menu: &Ref<ContextMenu>,
    ) -> bool {
        if self.pending_source.get() {
            return false;
        }
        let session = self.session.borrow();
        if session.validate_raster_asset(&asset.id).is_err() {
            return false;
        }
        menu.items().clear();
        crate::chrome::apply_designer_menu_resources(menu, &self.root);
        if let ([node], Ok(language)) = (session.selection(), self.try_language_service()) {
            let properties =
                session.raster_properties(*node, language.schema_for(session.document_uri()));
            let generation = session.generation();
            let node = *node;
            let choose_property = properties.len() > 1;
            for property in properties {
                let item = MenuItem::new();
                let label = if choose_property {
                    format!("Set {property}")
                } else {
                    "Use as image source".into()
                };
                item.set_header(Some(Rc::new(label)));
                let weak = Rc::downgrade(self);
                let uri = session.document_uri().to_owned();
                let address = asset.id.clone();
                item.click(move |_, event| {
                    if let Some(workspace) = weak.upgrade() {
                        workspace.report(
                            workspace
                                .apply_raster_menu(&uri, generation, node, &property, &address),
                        );
                        workspace.refresh(true);
                    }
                    event.set_handled(true);
                });
                menu.items().add(Some(Control::boxed(&item)));
            }
        }
        let movable = session
            .preview
            .packaged_assets()
            .filter(|catalog| catalog.has_move_provenance(&asset.id))
            .and_then(|catalog| catalog.resolve(&asset.id).ok().flatten())
            .is_some_and(|input| {
                self.project
                    .borrow()
                    .workspace
                    .as_ref()
                    .is_some_and(|project| input.file.starts_with(&project.solution.root))
            });
        if movable {
            let item = MenuItem::new();
            item.set_header(Some(Rc::new("Move or rename asset…".to_owned())));
            let weak = Rc::downgrade(self);
            let address = asset.id.clone();
            item.click(move |_, event| {
                let (weak, address) = (weak.clone(), address.clone());
                Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(workspace) = weak.upgrade() {
                            workspace.report(workspace.show_asset_move(address));
                        }
                    },
                    DispatcherPriority::NORMAL,
                );
                event.set_handled(true);
            });
            menu.items().add(Some(Control::boxed(&item)));
        }
        menu.items().count() != 0
    }

    fn apply_raster_menu(
        &self,
        uri: &str,
        generation: u64,
        node: NodeId,
        property: &str,
        address: &str,
    ) -> Result<(), String> {
        let mut session = self.session.borrow_mut();
        if self.pending_source.get()
            || session.document_uri() != uri
            || session.generation() != generation
            || session.selection() != [node]
        {
            return Err("The design selection changed. Open the image menu again.".into());
        }
        let schema = self
            .try_language_service()?
            .schema_for(session.document_uri())
            .clone();
        session.set_published_raster(node, Some(property), address, &schema)
    }
}
