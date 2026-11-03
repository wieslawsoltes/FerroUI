use ferroboard_core::layouts::{
    ComponentName, KeyboardResources, LayoutJson, LayoutType, ResourceFileProvider,
};
use std::{cell::Cell, error::Error, rc::Rc};

type NativeResult<T> = Result<T, Box<dyn Error>>;

struct OnDrop(Option<Box<dyn FnOnce()>>);
impl OnDrop {
    fn keep_alive(&self) {}
}
impl Drop for OnDrop {
    fn drop(&mut self) {
        if let Some(action) = self.0.take() {
            action();
        }
    }
}
fn provider(action: impl FnOnce() + 'static) -> ResourceFileProvider {
    let capture = OnDrop(Some(Box::new(action)));
    Rc::new(move |_| {
        capture.keep_alive();
        Ok(None)
    })
}

#[test]
fn replacing_an_extension_releases_its_provider_after_all_new_loaders_are_visible()
-> NativeResult<()> {
    let resources = Rc::new(KeyboardResources::create_empty());
    let owner = Rc::downgrade(&resources);
    let drops = Rc::new(Cell::new(0));
    let count = drops.clone();
    let manifest = Rc::new(LayoutJson::read_extension_manifest(
        r#"{"meta":{"id":"consumer"},"layouts":{"characters":[{"id":"lazy"}]},"popupMappings":[{"id":"popup"}]}"#,
    )?);
    let layout_reads = Rc::new(Cell::new(0));
    let popup_reads = Rc::new(Cell::new(0));
    resources.add_extension(
        manifest.clone(),
        provider(move || {
            let owner = owner.upgrade().unwrap();
            assert!(
                owner
                    .get_layout(
                        LayoutType::Characters,
                        &ComponentName::new("consumer", "lazy")
                    )
                    .unwrap()
                    .is_some()
            );
            assert!(
                owner
                    .get_popup_mapping(&ComponentName::new("consumer", "popup"))
                    .unwrap()
                    .is_some()
            );
            count.set(count.get() + 1);
        }),
    )?;
    let layouts = layout_reads.clone();
    let popups = popup_reads.clone();
    resources.add_extension(
        manifest,
        Rc::new(move |path| {
            if path.to_utf8().unwrap().starts_with("layouts/") {
                layouts.set(layouts.get() + 1);
                Ok(Some(r#"[["β"]]"#.into()))
            } else {
                popups.set(popups.get() + 1);
                Ok(Some("{}".into()))
            }
        }),
    )?;
    assert_eq!(
        (drops.get(), layout_reads.get(), popup_reads.get()),
        (1, 1, 1)
    );
    resources.get_layout(
        LayoutType::Characters,
        &ComponentName::new("consumer", "lazy"),
    )?;
    resources.get_popup_mapping(&ComponentName::new("consumer", "popup"))?;
    assert_eq!(
        (layout_reads.get(), popup_reads.get()),
        (1, 1),
        "Drop observes the actual new shared caches"
    );
    Ok(())
}

#[test]
fn direct_layout_replacement_releases_its_retired_provider_outside_resource_borrows()
-> NativeResult<()> {
    let resources = Rc::new(KeyboardResources::create_empty());
    let layout = Rc::new(LayoutJson::read_layout_document(
        r#"{"type":"characters","id":"lazy","arrangement":[["β"]]}"#,
        "consumer".into(),
    )?);
    let expected = layout.clone();
    let owner = Rc::downgrade(&resources);
    let drops = Rc::new(Cell::new(0));
    let count = drops.clone();
    resources.add_extension(
        Rc::new(LayoutJson::read_extension_manifest(
            r#"{"meta":{"id":"consumer"},"layouts":{"characters":[{"id":"lazy"}]}}"#,
        )?),
        provider(move || {
            let actual = owner
                .upgrade()
                .unwrap()
                .get_layout(
                    LayoutType::Characters,
                    &ComponentName::new("consumer", "lazy"),
                )
                .unwrap()
                .unwrap();
            assert!(Rc::ptr_eq(&actual, &expected));
            count.set(count.get() + 1);
        }),
    )?;
    resources.add_layout(layout);
    assert_eq!(drops.get(), 1);
    Ok(())
}

#[test]
fn direct_popup_replacement_releases_its_retired_provider_outside_resource_borrows()
-> NativeResult<()> {
    let resources = Rc::new(KeyboardResources::create_empty());
    let mapping = Rc::new(LayoutJson::read_popup_mapping(
        "{}",
        "popup".into(),
        "consumer".into(),
        None,
    )?);
    let expected = mapping.clone();
    let owner = Rc::downgrade(&resources);
    let drops = Rc::new(Cell::new(0));
    let count = drops.clone();
    resources.add_extension(
        Rc::new(LayoutJson::read_extension_manifest(
            r#"{"meta":{"id":"consumer"},"popupMappings":[{"id":"popup"}] }"#,
        )?),
        provider(move || {
            let actual = owner
                .upgrade()
                .unwrap()
                .get_popup_mapping(&ComponentName::new("consumer", "popup"))
                .unwrap()
                .unwrap();
            assert!(Rc::ptr_eq(&actual, &expected));
            count.set(count.get() + 1);
        }),
    )?;
    resources.add_popup_mapping(mapping);
    assert_eq!(drops.get(), 1);
    Ok(())
}

#[test]
fn partial_extension_failure_releases_retired_providers_without_hiding_the_error()
-> NativeResult<()> {
    use ferroboard_core::layouts::{LayoutMetadata, ResourceError};
    use std::sync::Arc;
    let resources = Rc::new(KeyboardResources::create_empty());
    let owner = Rc::downgrade(&resources);
    let drops = Rc::new(Cell::new(0));
    let count = drops.clone();
    let manifest = Rc::new(LayoutJson::read_extension_manifest(
        r#"{"meta":{"id":"consumer"},"layouts":{"characters":[{"id":"lazy"}]}}"#,
    )?);
    resources.add_extension(
        manifest.clone(),
        provider(move || {
            let owner = owner.upgrade().unwrap();
            assert!(
                owner
                    .get_layout(
                        LayoutType::Characters,
                        &ComponentName::new("consumer", "lazy")
                    )
                    .unwrap()
                    .is_some()
            );
            count.set(count.get() + 1);
        }),
    )?;
    let changes = Rc::new(Cell::new(0));
    let seen = changes.clone();
    resources
        .changed()
        .add(Rc::new(move |_| seen.set(seen.get() + 1)));
    manifest
        .layouts
        .borrow_mut()
        .push(Arc::new(LayoutMetadata::new(
            LayoutType(999),
            "invalid".into(),
        )));
    let result = resources.add_extension(manifest, Rc::new(|_| Ok(Some(r#"[["γ"]]"#.into()))));
    assert!(matches!(result, Err(ResourceError::LayoutType(_))));
    assert_eq!(drops.get(), 1);
    assert_eq!(
        changes.get(),
        0,
        "source partial failure does not emit Changed"
    );
    Ok(())
}
