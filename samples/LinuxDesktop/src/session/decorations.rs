//! Openbox decoration resources for the window manager started by the login supervisor.
use crate::shell::appearance::{Appearance, Layout};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::path::Path;
use std::process::Command;

pub(super) fn apply(state: &Path, appearance: Appearance) -> io::Result<()> {
    let (buttons, justify, padding, resize_handle) = match appearance.layout() {
        Layout::Taskbar => ("NLIMC", "left", 7, 1),
        Layout::MenuBarDock => ("CIML", "center", 4, 1),
        Layout::BottomNavigation => ("LNMC", "left", 10, 3),
    };
    let p = appearance.palette();
    let resource = format!(
        "border.width: 1\npadding.width: {padding}\npadding.height: {padding}\n\
         window.handle.width: {resize_handle}\nwindow.client.padding.width: 0\n\
         window.label.text.justify: {justify}\nwindow.*.title.bg: Flat Solid\n\
         window.active.title.bg.color: {surface}\nwindow.inactive.title.bg.color: {tile}\n\
         window.active.border.color: {accent}\nwindow.inactive.border.color: {edge}\n\
         window.active.title.separator.color: {edge}\nwindow.inactive.title.separator.color: {edge}\n\
         window.*.label.bg: Parentrelative\nwindow.active.label.text.color: {text}\n\
         window.inactive.label.text.color: {muted}\nwindow.*.button.*.bg: Parentrelative\n\
         window.active.button.*.image.color: {text}\nwindow.inactive.button.*.image.color: {muted}\n\
         window.active.button.hover.bg: Flat Solid\nwindow.active.button.hover.bg.color: {selected}\n\
         window.active.button.pressed.bg: Flat Solid\nwindow.active.button.pressed.bg.color: {accent}\n\
         window.*.handle.bg: Flat Solid\nwindow.*.handle.bg.color: {edge}\nwindow.*.grip.bg: Parentrelative\n\
         menu.border.color: {edge}\nmenu.title.bg: Flat Solid\nmenu.title.bg.color: {tile}\n\
         menu.title.text.color: {text}\nmenu.items.bg: Flat Solid\nmenu.items.bg.color: {surface}\n\
         menu.items.text.color: {text}\nmenu.items.disabled.text.color: {muted}\n\
         menu.items.active.bg: Flat Solid\nmenu.items.active.bg.color: {selected}\n\
         menu.items.active.text.color: {on_selected}\n",
        surface = p.surface,
        tile = p.tile,
        accent = p.accent,
        edge = p.edge,
        text = p.text,
        muted = p.muted,
        selected = p.selected,
        on_selected = p.on_selected,
    );
    let mut hasher = DefaultHasher::new();
    (appearance.id(), buttons, &resource).hash(&mut hasher);
    let name = format!("FerroDesktop-{:016x}", hasher.finish());
    let config = config_with_theme(&fs::read_to_string(state.join("base.xml"))?, &name, buttons)?;
    let runtime = state.join("openbox.xml");
    let directory = state.join("data/themes").join(&name).join("openbox-3");
    fs::create_dir_all(&directory)?;
    fs::write(directory.join("themerc"), resource)?;
    if matches!(appearance.layout(), Layout::MenuBarDock) {
        // Openbox masks are monochrome. Cut the symbol out of a circular button so the actions
        // remain distinguishable even when an application supplies its own accent colours.
        for (file, rows) in MAC_BUTTONS {
            fs::write(directory.join(file), xbm(rows))?;
        }
    }
    // The supervisor owns the directory and a single UI thread writes its config. Openbox can
    // observe only the complete XML document after all referenced theme files are ready.
    let next = state.join("openbox.xml.next");
    fs::write(&next, config)?;
    fs::rename(next, runtime)?;
    let result = Command::new("openbox").arg("--reconfigure").status()?;
    if result.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("Openbox reconfigure returned {result}")))
    }
}

const MAC_BUTTONS: [(&str, [u8; 8]); 4] = [
    ("close.xbm", [0x3c, 0x7e, 0xdb, 0xe7, 0xe7, 0xdb, 0x7e, 0x3c]),
    ("iconify.xbm", [0x3c, 0x7e, 0xff, 0xc3, 0xff, 0xff, 0x7e, 0x3c]),
    ("max.xbm", [0x3c, 0x7e, 0xef, 0xc3, 0xef, 0xff, 0x7e, 0x3c]),
    ("max_toggled.xbm", [0x3c, 0x7e, 0xcf, 0xef, 0xf7, 0xf3, 0x7e, 0x3c]),
];

fn xbm(rows: [u8; 8]) -> String {
    let data = rows
        .iter()
        .map(|byte| format!("0x{byte:02x}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("#define glyph_width 8\n#define glyph_height 8\nstatic unsigned char glyph_bits[] = {{ {data} }};\n")
}

/// Preserve keybindings, window rules, mouse actions, and custom fonts from the selected
/// configuration. Only two options in its theme section belong to the appearance profile.
fn config_with_theme(source: &str, name: &str, buttons: &str) -> io::Result<String> {
    let doc = roxmltree::Document::parse(source).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let root = doc.root_element();
    if !root.has_tag_name("openbox_config") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Expected an openbox_config root",
        ));
    }
    let mut theme = format!("<theme><name>{name}</name><titleLayout>{buttons}</titleLayout>");
    let mut config = source.to_owned();
    if let Some(old) = root.children().find(|node| node.has_tag_name("theme")) {
        for child in old
            .children()
            .filter(|n| !n.has_tag_name("name") && !n.has_tag_name("titleLayout"))
        {
            theme.push_str(&source[child.range()]);
        }
        theme.push_str("</theme>");
        config.replace_range(old.range(), &theme);
    } else {
        theme.push_str("</theme>");
        let range = root.range();
        let fragment = &source[range.clone()];
        if let Some(start) = fragment.rfind("</") {
            config.insert_str(range.start + start, &theme);
        } else {
            // Minimal self-closing root, optionally with attributes.
            config.replace_range(range.end - 2..range.end, &format!(">{theme}</openbox_config>"));
        }
    }
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_user_configuration_survives_frame_appearance_changes() {
        let xml = r#"<openbox_config><mouse/><theme><name>Old</name><font place="ActiveWindow"/><titleLayout>LC</titleLayout></theme><keyboard/></openbox_config>"#;
        let result = config_with_theme(xml, "FerroDesktop", "CIML").unwrap();
        assert!(result.contains("<name>FerroDesktop</name><titleLayout>CIML</titleLayout>"));
        for untouched in ["<mouse/>", "<keyboard/>", r#"<font place="ActiveWindow"/>"#] {
            assert!(result.contains(untouched));
        }
        assert!(!result.contains("Old"));
        assert!(config_with_theme("<openbox_config/>", "F", "LC")
            .unwrap()
            .ends_with("</theme></openbox_config>"));
    }
}
