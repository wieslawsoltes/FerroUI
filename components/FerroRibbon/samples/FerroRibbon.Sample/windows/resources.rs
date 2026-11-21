use std::{env, fs, path::Path};

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;

pub(super) fn link(root: &Path) {
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows")
        || env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc")
        || env::var_os("CARGO_FEATURE_DESKTOP").is_none()
    {
        return;
    }
    let icon = root.join("windows/icon.ico");
    let manifest = root.join("windows/app.manifest");
    for input in [&icon, &manifest, &root.join("windows/resources.rs")] {
        println!("cargo::rerun-if-changed={}", input.display());
    }

    let out = std::path::PathBuf::from(env::var_os("OUT_DIR").expect("Cargo output"))
        .join("ferroribbon-icon.res");
    fs::write(
        &out,
        icon_resources(&fs::read(icon).expect("original Windows icon")),
    )
    .expect("write Windows icon resource");
    println!(
        "cargo::rustc-link-arg-bin=ferroribbon-sample={}",
        out.display()
    );
    println!("cargo::rustc-link-arg-bin=ferroribbon-sample=/MANIFEST:EMBED");
    println!(
        "cargo::rustc-link-arg-bin=ferroribbon-sample=/MANIFESTINPUT:{}",
        manifest.display()
    );
}

fn icon_resources(icon: &[u8]) -> Vec<u8> {
    assert_eq!(icon.get(..4), Some(&[0, 0, 1, 0][..]), "Windows ICO header");
    let count = usize::from(u16::from_le_bytes(icon[4..6].try_into().unwrap()));
    let end = 6 + 16 * count;
    assert!(count > 0 && end <= icon.len(), "Windows ICO directory");
    let mut group = icon[..6].to_vec();
    let mut resources = Vec::new();
    append_resource(&mut resources, 0, 0, &[]);
    for (index, entry) in icon[6..end].chunks_exact(16).enumerate() {
        let id = (index + 1) as u16;
        let size = u32::from_le_bytes(entry[8..12].try_into().unwrap()) as usize;
        let offset = u32::from_le_bytes(entry[12..16].try_into().unwrap()) as usize;
        let image = icon
            .get(offset..)
            .and_then(|data| data.get(..size))
            .expect("Windows ICO image");
        // PE records store the original image payload and replace its offset with an ordinal.
        append_resource(&mut resources, RT_ICON, id, image);
        group.extend_from_slice(&entry[..12]);
        group.extend_from_slice(&id.to_le_bytes());
    }
    append_resource(&mut resources, RT_GROUP_ICON, 1, &group);
    resources
}

fn append_resource(resources: &mut Vec<u8>, kind: u16, id: u16, data: &[u8]) {
    resources.extend_from_slice(
        &u32::try_from(data.len())
            .expect("resource size")
            .to_le_bytes(),
    );
    resources.extend_from_slice(&32_u32.to_le_bytes());
    for field in [u16::MAX, kind, u16::MAX, id] {
        resources.extend_from_slice(&field.to_le_bytes());
    }
    // Data version, memory flags, language, version and characteristics.
    resources.extend_from_slice(&[0; 16]);
    resources.extend_from_slice(data);
    resources.resize(resources.len().next_multiple_of(4), 0);
}
