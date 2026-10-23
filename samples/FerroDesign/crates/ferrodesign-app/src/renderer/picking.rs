//! Canvas picking shares the renderer's prepared mask geometry and pixels.
//! Linked scopes are built once with scene topology. An ordinary pointer query
//! visits only masks affecting its spatially selected candidate.
use super::*;

pub(super) struct MaskScope {
    index: usize,
    limit: usize,
    previous: Option<usize>,
}

pub(super) struct PickKey {
    editor: usize,
    token: EditorRenderToken,
    scene: Option<(usize, u64, u64)>,
    roots: (usize, usize, Option<NodeId>, Option<NodeId>),
}

fn roots_identity(roots: &[NodeId]) -> (usize, usize, Option<NodeId>, Option<NodeId>) {
    (
        roots.as_ptr() as usize,
        roots.len(),
        roots.first().copied(),
        roots.last().copied(),
    )
}

pub(super) fn prepare_scopes(scene: &mut [PreparedNode], scopes: &mut Vec<MaskScope>) {
    scopes.clear();
    let end = scene.len();
    assign_scopes(scene, scopes, 0, end, None);
}

fn assign_scopes(
    scene: &mut [PreparedNode],
    scopes: &mut Vec<MaskScope>,
    mut index: usize,
    end: usize,
    mut active: Option<usize>,
) {
    while index < end {
        let next = scene[index].end;
        scene[index].mask_scope = active;
        assign_scopes(scene, scopes, index + 1, next, active);
        if scene[index].mask.is_some() {
            let scope = scopes.len();
            scopes.push(MaskScope {
                index,
                limit: end,
                previous: active,
            });
            active = Some(scope);
        }
        index = next;
    }
}

impl ScenePainter {
    /// Filter a geometric canvas hit against every applicable sibling mask.
    /// The candidate mask itself is independently selectable; its role clips
    /// only subsequent siblings. Isolated presentation/export roots naturally
    /// exclude mask scopes outside those roots.
    ///
    /// The editor token is checked before using prepared data, including when
    /// undo or an inspector edit occurs before the next render. Alpha/luminance
    /// reads are lazy and retained under the backdrop cache's existing budget.
    pub fn mask_contains(
        &mut self,
        editor: &Editor,
        roots: &[NodeId],
        candidate: NodeId,
        world: Point,
    ) -> bool {
        if !world.x.is_finite() || !world.y.is_finite() {
            return false;
        }
        let token = editor.render_change_token();
        let editor_identity = editor as *const Editor as usize;
        let roots_identity = roots_identity(roots);
        let ready = self.picking_key.as_ref().is_some_and(|key| {
            key.editor == editor_identity
                && key.token == token
                && self.editor_token.as_ref() == Some(&token)
                && key.scene == self.scene_key
                && key.roots == roots_identity
        });
        if !ready {
            self.ensure_editor_prepared(editor, roots);
            self.picking_key = Some(PickKey {
                editor: editor_identity,
                token,
                scene: self.scene_key,
                roots: roots_identity,
            });
        }
        let Some(&candidate) = self.prepared_indices.get(&candidate) else {
            // Hidden, zero-opacity and invalid prepared branches cannot
            // intercept visible artwork; they remain editable in Layers.
            return false;
        };
        let mut link = self.prepared[candidate].mask_scope;
        if link.is_none() {
            return true;
        }
        let mut chain = Vec::new();
        while let Some(scope) = link {
            chain.push(scope);
            link = self.mask_scopes[scope].previous;
        }
        let mut opened = Vec::new();
        let mut alpha = 1.0;
        let mut accepted = true;
        for scope in chain.into_iter().rev() {
            let scope = &self.mask_scopes[scope];
            let node = &self.prepared[scope.index];
            let Some(inverse) = node.world.inverse() else {
                accepted = false;
                break;
            };
            let local = inverse.apply(world);
            if node.opacity <= 0.0 || !node.local_subtree_bounds.contains(local) {
                accepted = false;
                break;
            }
            if node.mask == Some(MaskType::Vector) {
                if !node.visual.vector_mask.fill_contains(ui_point(local)) {
                    accepted = false;
                    break;
                }
            } else {
                alpha *= self.backdrops.mask_alpha(
                    &self.prepared,
                    &self.prepared_roots,
                    scope.index,
                    scope.limit,
                    local,
                ) as f64
                    / 255.0;
                // Match the rendered eight-bit coverage: several nonzero
                // masks can multiply to a fully transparent output pixel.
                if alpha < 0.5 / 255.0 {
                    accepted = false;
                    break;
                }
            }
            self.backdrops.push_mask_scope(scope.index);
            opened.push(scope.index);
        }
        for index in opened.into_iter().rev() {
            self.backdrops.pop_mask_scope(index);
        }
        accepted
    }
}
