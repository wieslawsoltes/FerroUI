//! Native theme information for the remote source palette.
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::styling::ThemeVariant;
use ferroui_remote_protocol::designer::{
    DesignPaletteContextMessage, DESIGN_PALETTE_QUERY, DESIGN_PALETTE_VERSION,
};
use std::cell::Cell;
use url::Url;

ferroui_base::ferro_thread_local! {
    static ENABLED: Cell<bool> = const { Cell::new(false) };
}

pub(super) fn configure(transport: Option<&Url>, remote: bool) {
    ENABLED.set(
        super::hosted_sources::requested(transport, remote)
            && transport.is_some_and(|uri| {
                uri.query_pairs().any(|(key, version)| {
                    key == DESIGN_PALETTE_QUERY && version == DESIGN_PALETTE_VERSION
                })
            }),
    );
}

pub(super) fn capture(revision: i64) -> Option<DesignPaletteContextMessage> {
    if !ENABLED.get() {
        return None;
    }
    let context = ThemeVariant::high_contrast_variant().and_then(theme_keys);
    Some(DesignPaletteContextMessage {
        revision,
        contrast: context,
        assemblies: MarkupAssembly::registered_assemblies()
            .into_iter()
            .map(|assembly| assembly.name.into())
            .collect(),
    })
}

fn theme_keys(theme: ThemeVariant) -> Option<Vec<String>> {
    let mut keys = Vec::new();
    let mut current = Some(theme);
    while let Some(variant) = current {
        keys.push(variant.key().as_str()?.into());
        current = variant.inherit_variant();
    }
    Some(keys)
}
