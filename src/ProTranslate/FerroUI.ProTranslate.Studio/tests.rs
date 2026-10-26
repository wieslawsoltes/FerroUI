use crate::*;
use ferroui_protranslate_formats::*;
use std::{cell::Cell, rc::Rc};

pub(crate) fn document() -> StudioDocument {
    let mut catalog = TranslationCatalog {
        name: Some("Product".into()),
        source_culture: Some("en-US".into()),
        ..Default::default()
    };
    catalog
        .metadata
        .insert("vendor.document".into(), "{\"retain\":true}".into());
    let entry = catalog.add("Orders.Total", Some("pl-PL"), "Suma: {0:N2}");
    entry.source = Some("Total: {0:N2}".into());
    entry.context = Some("checkout".into());
    entry.comment = Some("Translator note".into());
    entry.state = Some("translated".into());
    entry.format = Some("composite".into());
    entry.plural_category = Some("other".into());
    entry
        .metadata
        .insert("vendor.entry".into(), "[1,2,3]".into());
    catalog.add("Orders.Empty", Some("pl-PL"), "").source = Some("No orders".into());
    catalog
        .add("Orders.Total", Some("de-DE"), "Summe: {0:N2}")
        .source = Some("Total: {0:N2}".into());
    StudioDocument::new(
        "session_1",
        ImportedCatalog {
            catalog,
            diagnostics: Vec::new(),
            file_name: "Product.xlf".into(),
            format: TranslationFileFormat::Xliff20,
            target_culture: "pl-PL".into(),
        },
        1000,
    )
    .unwrap()
}

fn paths() -> StudioPaths {
    StudioPaths::new("sessions", "recovery.json", "backups", "settings.json").unwrap()
}

#[test]
fn session_preserves_every_catalog_field_and_duplicate_identity() {
    let mut doc = document();
    let mut session = doc.snapshot(2000);
    session
        .catalog
        .entries
        .push(session.catalog.entries[0].clone());
    session.entry_status.push(EntryStatus {
        id: EntryId(session.next_entry_id),
        review: TranslationReviewState::Review,
        revision: session.revision,
    });
    session.next_entry_id += 1;
    doc.restore(session.clone(), false).unwrap();
    let roundtrip = TranslationSession::from_json(&session.to_json().unwrap()).unwrap();
    assert_eq!(roundtrip, session);
    assert_eq!(
        doc.filtered_ids(&EntryFilter {
            search: "total".into(),
            culture: Some("pl-PL".into()),
            ..Default::default()
        })
        .len(),
        2
    );
    let duplicate_id = doc.session().entry_status.last().unwrap().id;
    doc.set_target(duplicate_id, "Separate duplicate").unwrap();
    assert_eq!(doc.entry(EntryId(1)).unwrap().0.value, "Suma: {0:N2}");
}

#[test]
fn edits_reviews_notes_add_delete_and_bounded_history_restore_full_metadata() {
    let mut doc = document();
    let original = doc.catalog().clone();
    let id = EntryId(1);
    doc.set_target(id, "Changed {0:N2}").unwrap();
    assert_eq!(
        doc.entry(id).unwrap().1.review,
        TranslationReviewState::Review
    );
    doc.set_notes(id, Some("New note".into())).unwrap();
    doc.set_review(id, TranslationReviewState::Approved)
        .unwrap();
    let added = doc
        .add_entry(TranslationCatalogEntry::new("New", Some("pl-PL"), "Nowe"))
        .unwrap();
    doc.delete_entry(id).unwrap();
    assert_eq!(doc.undo_count(), 5);
    for _ in 0..5 {
        assert!(doc.undo().unwrap());
    }
    assert_eq!(doc.catalog(), &original);
    assert!(doc.entry(added).is_none());
    for _ in 0..5 {
        assert!(doc.redo().unwrap());
    }
    assert!(doc.entry(id).is_none());
    doc.undo().unwrap();
    doc.set_history_limit(2);
    doc.set_target(id, "one").unwrap();
    doc.set_target(id, "two").unwrap();
    doc.set_target(id, "three").unwrap();
    assert_eq!(doc.undo_count(), 2);
    assert_eq!(doc.redo_count(), 0);
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(doc.entry(id).unwrap().0.value, "one");
    assert!(!doc.undo().unwrap());
}

#[test]
fn transaction_is_one_undo_operation_and_rolls_back_errors() {
    let mut doc = document();
    let before = doc.catalog().clone();
    doc.transaction(|doc| {
        doc.set_target(EntryId(1), "One")?;
        doc.set_target(EntryId(2), "Two")?;
        Ok(())
    })
    .unwrap();
    assert_eq!(doc.undo_count(), 1);
    doc.undo().unwrap();
    assert_eq!(doc.catalog(), &before);
    let snapshot = doc.snapshot(42);
    let failure = doc.transaction(|doc| {
        doc.set_target(EntryId(1), "Temporary")?;
        doc.delete_entry(EntryId(999))?;
        Ok(())
    });
    assert!(failure.is_err());
    assert_eq!(doc.snapshot(42), snapshot);
    assert_eq!(doc.redo_count(), 1);
}

#[test]
fn filters_and_coverage_are_derived_from_actual_catalog() {
    let mut doc = document();
    let rows = doc.filtered_ids(&EntryFilter {
        search: "NO ORDERS".into(),
        state: Some(TranslationReviewState::Missing),
        culture: Some("PL-pl".into()),
    });
    assert_eq!(rows, vec![EntryId(2)]);
    assert_eq!(
        doc.metrics(Some("pl-PL")),
        ReviewMetrics {
            total: 2,
            approved: 1,
            missing: 1,
            review: 0
        }
    );
    doc.add_target_culture("ja-JP", "Japanese").unwrap();
    let coverage = doc.coverage();
    assert_eq!(
        coverage
            .iter()
            .find(|c| c.culture_name == "de-DE")
            .unwrap()
            .metrics
            .completeness(),
        0.5
    );
    assert_eq!(
        coverage
            .iter()
            .find(|c| c.culture_name == "ja-JP")
            .unwrap()
            .metrics
            .missing,
        2
    );
    doc.set_target_culture("ja-JP").unwrap();
    assert_eq!(
        doc.catalog().entries.len(),
        3,
        "changing target must preserve the document"
    );
}

#[test]
fn every_format_gateway_dispatches_and_import_errors_do_not_mutate_document() {
    let mut catalog = TranslationCatalog {
        source_culture: Some("en-US".into()),
        ..Default::default()
    };
    catalog.add("Hello", Some("pl-PL"), "Cześć").source = Some("Hello".into());
    let options = TranslationFormatOptions {
        culture: Some("pl-PL".into()),
        source_culture: Some("en-US".into()),
        ..Default::default()
    };
    for format in TranslationFileFormat::ALL {
        let mut representable = catalog.clone();
        if format == TranslationFileFormat::AppleStringsdict {
            representable.entries[0].key = "Hello.other".into();
            representable.entries[0].plural_category = Some("other".into());
        }
        let output = CatalogGateway::export(&representable, format, &options).unwrap();
        let imported =
            CatalogGateway::import(&output.content, format, &options, "catalog.txt", "pl-PL")
                .unwrap();
        assert!(!imported.catalog.entries.is_empty(), "{format:?}");
    }
    let doc = document();
    let before = doc.catalog().clone();
    assert!(CatalogGateway::import(
        "{broken",
        TranslationFileFormat::ProTranslateJson,
        &options,
        "catalog.json",
        "pl-PL"
    )
    .is_err());
    assert_eq!(doc.catalog(), &before);
}

struct FailingStorage {
    inner: MemoryStudioStorage,
    fail: Cell<bool>,
}
impl StudioStorage for FailingStorage {
    fn write_new_atomic(&self, path: &str, bytes: &[u8]) -> Result<bool, StudioError> {
        if self.fail.get() {
            Err(StudioError::Storage("Injected write failure".into()))
        } else {
            self.inner.write_new_atomic(path, bytes)
        }
    }
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, StudioError> {
        self.inner.read(path)
    }
    fn write_atomic(&self, path: &str, bytes: &[u8]) -> Result<(), StudioError> {
        if self.fail.get() {
            Err(StudioError::Storage("Injected write failure".into()))
        } else {
            self.inner.write_atomic(path, bytes)
        }
    }
    fn remove(&self, path: &str) -> Result<(), StudioError> {
        self.inner.remove(path)
    }
    fn list(&self, path: &str) -> Result<Vec<StorageEntry>, StudioError> {
        self.inner.list(path)
    }
}

#[test]
fn failed_save_preserves_dirty_state_previous_file_and_backup_history() {
    let storage = Rc::new(FailingStorage {
        inner: MemoryStudioStorage::default(),
        fail: Cell::new(false),
    });
    let sessions = SessionService::new(storage.clone(), paths());
    let mut doc = document();
    let saved = sessions.save_session(&mut doc, None, 2000).unwrap();
    assert!(saved.document_is_clean);
    assert!(!doc.is_dirty());
    let old = storage.read(&saved.path).unwrap();
    sessions.create_backup(doc.session(), 2000).unwrap();
    doc.set_target(EntryId(1), "unsaved").unwrap();
    storage.fail.set(true);
    assert!(sessions.save_session(&mut doc, None, 3000).is_err());
    assert!(doc.is_dirty());
    assert_eq!(storage.read(&saved.path).unwrap(), old);
    assert!(sessions.backup_and_rotate(doc.session(), 3000, 1).is_err());
    assert_eq!(sessions.list_backups().unwrap().len(), 1);
    let stale = doc.snapshot(4000);
    doc.set_notes(EntryId(1), Some("later edit".into()))
        .unwrap();
    assert!(!doc.mark_saved(&stale));
    assert!(doc.is_dirty());
}

#[test]
fn recovery_reports_corruption_restores_metadata_and_remains_dirty() {
    let storage = Rc::new(MemoryStudioStorage::default());
    let sessions = SessionService::new(storage.clone(), paths());
    let mut doc = document();
    doc.set_notes(EntryId(1), Some("unsaved note".into()))
        .unwrap();
    sessions.save_recovery(&doc, 2000).unwrap();
    assert!(doc.is_dirty());
    let expected = doc.catalog().clone();
    doc.set_target(EntryId(1), "other").unwrap();
    assert!(sessions.restore_recovery(&mut doc).unwrap());
    assert_eq!(doc.catalog(), &expected);
    assert!(doc.is_dirty());
    assert_eq!(doc.undo_count(), 0);
    assert_eq!(doc.generation(), 1);
    assert!(sessions.recovery_session().unwrap().is_some());
    storage.write_atomic("recovery.json", b"not json").unwrap();
    assert!(sessions.recovery_session().is_err());
    sessions.clear_recovery().unwrap();
    assert!(sessions.recovery_session().unwrap().is_none());
}

#[test]
fn backups_do_not_collide_rotate_newest_and_reject_unrelated_paths() {
    let storage = Rc::new(MemoryStudioStorage::default());
    let sessions = SessionService::new(storage.clone(), paths());
    let doc = document();
    let first = sessions.create_backup(doc.session(), 2000).unwrap();
    let second = sessions.create_backup(doc.session(), 2000).unwrap();
    assert_ne!(first.path, second.path);
    let restarted = SessionService::new(storage.clone(), paths());
    let third = restarted.backup_and_rotate(doc.session(), 2000, 2).unwrap();
    assert_ne!(third.path, first.path);
    assert_ne!(third.path, second.path);
    let backups = sessions.list_backups().unwrap();
    assert_eq!(backups.len(), 2);
    assert_eq!(backups[0].path, third.path);
    assert!(storage.read(&first.path).unwrap().is_none());
    assert_eq!(
        sessions
            .restore_backup(&second.path)
            .unwrap()
            .unwrap()
            .catalog,
        doc.catalog().clone()
    );
    assert!(sessions.delete_backup("sessions/session_1.json").is_err());
    storage
        .write_atomic("backups/backup_corrupt.json", b"broken")
        .unwrap();
    assert!(sessions
        .list_backups()
        .unwrap()
        .last()
        .unwrap()
        .error
        .is_some());
}

#[test]
fn autosave_ticks_are_explicit_skip_clean_documents_and_retry_failures() {
    let storage = Rc::new(FailingStorage {
        inner: MemoryStudioStorage::default(),
        fail: Cell::new(false),
    });
    let sessions = SessionService::new(storage.clone(), paths());
    let mut doc = document();
    let mut timer = AutoSaveScheduler::new(10).unwrap();
    timer.start(0);
    assert!(!timer.tick(9999, &doc, &sessions).unwrap());
    assert!(timer.tick(10000, &doc, &sessions).unwrap());
    sessions.save_session(&mut doc, None, 10001).unwrap();
    assert!(!timer.tick(20000, &doc, &sessions).unwrap());
    doc.set_notes(EntryId(1), Some("dirty".into())).unwrap();
    storage.fail.set(true);
    assert!(timer.tick(30000, &doc, &sessions).is_err());
    storage.fail.set(false);
    assert!(timer.tick(30001, &doc, &sessions).unwrap());
    timer.stop();
    assert!(!timer.tick(99999, &doc, &sessions).unwrap());
}

#[test]
fn session_validation_rejects_bad_schema_duplicate_ids_and_traversal() {
    let mut snapshot = document().snapshot(0);
    snapshot.schema_version = 999;
    assert!(snapshot.validate().is_err());
    snapshot.schema_version = 1;
    snapshot.entry_status[1].id = snapshot.entry_status[0].id;
    assert!(snapshot.validate().is_err());
    let storage = MemoryStudioStorage::default();
    for path in [
        "../escape",
        "/absolute",
        "a/../b",
        "a\\b",
        "C:/file",
        "a//b",
    ] {
        assert!(storage.write_atomic(path, b"bad").is_err());
    }
}

#[test]
fn persistence_namespaces_cannot_overlap_files_or_directories() {
    for (sessions, recovery, backups, settings) in [
        ("sessions", "backups", "sessions/foo.json", "settings.json"),
        ("sessions", "sessions/foo.json", "backups", "settings.json"),
        (
            "sessions",
            "recovery.json",
            "backups",
            "backups/settings.json",
        ),
        (
            "sessions",
            "backups/recovery.json",
            "backups",
            "settings.json",
        ),
        (
            "sessions",
            "recovery.json",
            "sessions/backups",
            "settings.json",
        ),
        (
            "sessions",
            "recovery.json",
            "backups",
            "recovery.json/settings.json",
        ),
        ("sessions", "recovery.json", "backups", "sessions"),
    ] {
        assert!(
            StudioPaths::new(sessions, recovery, backups, settings).is_err(),
            "{sessions}, {recovery}, {backups}, {settings}"
        );
    }
    assert!(StudioPaths::new(
        "data/sessions",
        "data/recovery.json",
        "data/backups",
        "data/settings.json"
    )
    .is_ok());
}

#[test]
fn portable_paths_reject_case_and_trailing_character_aliases_but_preserve_unicode() {
    for (sessions, recovery, backups, settings) in [
        (
            "Sessions",
            "sessions/project.json",
            "Backups",
            "settings.json",
        ),
        ("Sessions", "Recovery.json", "Backups", "RECOVERY.JSON"),
        (
            "Sessions",
            "Recovery.json",
            "Backups",
            "BACKUPS/settings.json",
        ),
        (
            "Données",
            "dONNÉES/project.json",
            "Backups",
            "settings.json",
        ),
        ("Straße", "STRASSE/project.json", "Backups", "settings.json"),
    ] {
        assert!(StudioPaths::new(sessions, recovery, backups, settings).is_err());
    }

    let storage = MemoryStudioStorage::default();
    for path in [
        "Sessions.",
        "Sessions ",
        "Sessions./project.json",
        "Sessions /project.json",
        "Recovery.json.",
        "Recovery.json ",
    ] {
        assert!(StudioPaths::new("Sessions", path, "Backups", "settings.json").is_err());
        assert!(storage
            .write_atomic(path, b"must not create an alias")
            .is_err());
    }

    let paths = StudioPaths::new(
        "翻訳 資料/Συνεδρίες",
        "翻訳 資料/Ανάκτηση.json",
        "翻訳 資料/备份",
        "翻訳 資料/設定.json",
    )
    .unwrap();
    assert_eq!(paths.sessions_directory(), "翻訳 資料/Συνεδρίες");
    assert_eq!(paths.settings_path(), "翻訳 資料/設定.json");
    storage
        .write_atomic(paths.settings_path(), "保存".as_bytes())
        .unwrap();
    assert_eq!(
        storage.read(paths.settings_path()).unwrap(),
        Some("保存".as_bytes().to_vec())
    );
}

#[cfg(all(feature = "filesystem", not(target_arch = "wasm32")))]
#[test]
fn filesystem_replaces_files_atomically_and_cleans_temporary_files() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Rc::new(FileStudioStorage::new(directory.path()).unwrap());
    storage.write_atomic("nested/file.json", b"old").unwrap();
    storage.write_atomic("nested/file.json", b"new").unwrap();
    assert!(!storage
        .write_new_atomic("nested/file.json", b"do not replace")
        .unwrap());
    assert_eq!(
        storage.read("nested/file.json").unwrap(),
        Some(b"new".to_vec())
    );
    assert_eq!(storage.list("nested").unwrap().len(), 1);
    let sessions = SessionService::new(storage.clone(), paths());
    let mut doc = document();
    let saved = sessions.save_session(&mut doc, None, 3000).unwrap();
    assert_eq!(
        sessions.load_session(&saved.path).unwrap().unwrap().catalog,
        doc.catalog().clone()
    );
    sessions.create_backup(doc.session(), 3000).unwrap();
    assert_eq!(sessions.list_backups().unwrap().len(), 1);
}

#[test]
fn storage_catalog_gateway_preserves_diagnostics_and_never_writes_failed_exports() {
    let storage = MemoryStudioStorage::default();
    let doc = document();
    let options = TranslationFormatOptions {
        culture: Some("pl-PL".into()),
        source_culture: Some("en-US".into()),
        ..Default::default()
    };
    let exported = CatalogGateway::export_to_storage(
        &storage,
        "catalog.json",
        doc.catalog(),
        TranslationFileFormat::ProTranslateJson,
        &options,
    )
    .unwrap();
    assert!(
        !exported.diagnostics.is_empty(),
        "lossy exports report discarded metadata"
    );
    let imported = CatalogGateway::import_from_storage(
        &storage,
        "catalog.json",
        TranslationFileFormat::ProTranslateJson,
        &options,
        "pl-PL",
    )
    .unwrap();
    assert_eq!(imported.catalog.entries.len(), 2);
    storage
        .write_atomic("catalog.stringsdict", b"previous")
        .unwrap();
    let mut invalid = doc.catalog().clone();
    invalid.entries[0].key.clear();
    assert!(CatalogGateway::export_to_storage(
        &storage,
        "catalog.stringsdict",
        &invalid,
        TranslationFileFormat::AppleStringsdict,
        &options
    )
    .is_err());
    assert_eq!(
        storage.read("catalog.stringsdict").unwrap(),
        Some(b"previous".to_vec())
    );
}

#[test]
fn placeholder_validation_handles_escapes_alignment_and_every_syntax() {
    use TranslationFileFormat::*;
    assert_eq!(
        normalized_placeholders("{{literal}} {0,8:N2} {1:D}", ProTranslateJson).unwrap(),
        vec!["{0}", "{1}"]
    );
    let errors = validate_placeholders("{0} {1:D}", "{0} {2:D}", Resx);
    assert_eq!(
        errors.iter().map(|error| error.code).collect::<Vec<_>>(),
        vec!["PTS004", "PTS005"]
    );
    assert!(validate_placeholders("{0}", "unclosed {0", Csv)
        .iter()
        .any(|error| error.code == "PTS003"));
    assert_eq!(
        normalized_placeholders("{{user.name}} {{amount, number}}", I18NextJson).unwrap(),
        vec!["{{user.name}}", "{{amount}}"]
    );
    assert_eq!(
        normalized_placeholders("%% %1$s %2$.2f", GettextPo).unwrap(),
        vec!["%1$", "%2$"]
    );
    assert_eq!(
        normalized_placeholders(
            "{count, plural, one {1 item for {name}} other {# items for {name}}}",
            FlutterArb
        )
        .unwrap(),
        vec!["{count}", "{name}"]
    );
    assert!(normalized_placeholders("{count, plural, one {one}}", FlutterArb).is_err());
    assert_eq!(
        normalized_placeholders("%@", AppleXcstrings).unwrap(),
        vec!["%@"]
    );
    assert_eq!(
        validate_placeholders("source", "", AndroidResources)[0].code,
        "PTS001"
    );
}

#[test]
fn format_helper_builds_typed_snippets_and_separates_autocomplete_labels() {
    use TranslationFileFormat::*;
    let mut helper = FormatHelper {
        placeholder: "{0:N2}".into(),
        kind: BuilderFormatType::Number,
        percentage: true,
        ..Default::default()
    };
    assert_eq!(helper.build(ProTranslateJson).unwrap(), "{0:P2}");
    helper.placeholder = "%2$s".into();
    helper.percentage = false;
    assert_eq!(helper.build(GettextPo).unwrap(), "%2$.2f");
    helper.placeholder = "{{amount}}".into();
    assert_eq!(helper.build(I18NextJson).unwrap(), "{{amount, number}}");
    helper.placeholder = "{count}".into();
    helper.kind = BuilderFormatType::Plural;
    helper.branches.insert("one".into(), "1 item".into());
    helper.branches.insert("other".into(), "# items".into());
    assert_eq!(
        helper.build(FlutterArb).unwrap(),
        "{count, plural, one {1 item} other {# items}}"
    );
    assert!(helper.build(GettextPo).is_err());
    let suggestions = autocomplete("Price {0:N2}", Resx, "{").unwrap();
    assert!(suggestions
        .iter()
        .any(|item| item.insertion == "{0:C}" && item.label == "Currency"));
    assert!(suggestions
        .iter()
        .all(|item| !item.insertion.contains("(Currency)")));
    assert!(autocomplete("Hello {name}", FlutterArb, "?")
        .unwrap()
        .is_empty());
}

#[test]
fn preview_uses_explicit_culture_date_and_real_composite_errors() {
    use ferroui_protranslate_core::{Culture, LocalizedDateTime};
    use TranslationFileFormat::*;
    let date = LocalizedDateTime::from_unix_timestamp(0, 0).unwrap();
    let culture = Culture::parse("pl-PL").unwrap();
    let mut values = FormatPreviewValues::new(date);
    let preview = simulate_format(
        "{0:N2} / {1:yyyy-MM-dd}",
        ProTranslateJson,
        &culture,
        &values,
    )
    .unwrap();
    assert!(preview.contains("234,56"));
    assert!(preview.ends_with("1970-01-01"));
    assert!(simulate_format("{99}", ProTranslateJson, &culture, &values).is_err());
    values.text = "Jan".into();
    values.count = 1;
    assert_eq!(
        simulate_format(
            "{count, plural, one {1 item for {name}} other {# items for {name}}}",
            FlutterArb,
            &culture,
            &values
        )
        .unwrap(),
        "1 item for Jan"
    );
    values.count = 5;
    assert_eq!(
        simulate_format(
            "{count, plural, one {1 item} other {# items}}",
            FlutterArb,
            &culture,
            &values
        )
        .unwrap(),
        "5 items"
    );
    assert_eq!(
        simulate_format(
            "{count, plural, offset:1 one {# guest} other {# guests}}",
            FlutterArb,
            &culture,
            &values
        )
        .unwrap(),
        "4 guests"
    );
    values.selection = "male".into();
    assert_eq!(
        simulate_format(
            "{gender, select, male {Mr. {name}} other {{name}}}",
            FlutterArb,
            &culture,
            &values
        )
        .unwrap(),
        "Mr. Jan"
    );
    assert_eq!(
        simulate_format("Hello {{name}}", I18NextJson, &culture, &values).unwrap(),
        "Hello Jan"
    );
    assert_eq!(
        simulate_format("%% %1$s", GettextPo, &culture, &values).unwrap(),
        "% Jan"
    );
}

#[test]
fn settings_load_defaults_preserve_corruption_and_require_explicit_secret_persistence() {
    let storage = Rc::new(MemoryStudioStorage::default());
    let service = SettingsService::new(storage.clone(), "settings.json");
    let defaults = service.load_or_default();
    assert!(defaults.warning.is_none());
    assert_eq!(defaults.settings.ai_providers.len(), 3);
    let mut settings = defaults.settings;
    settings.ai_providers[0].api_key = "test-secret".into();
    settings.ai_providers[0].is_enabled = true;
    service.save(&settings).unwrap();
    let loaded = service.load().unwrap().unwrap();
    assert!(loaded.ai_providers[0].api_key.is_empty());
    assert_eq!(settings.ai_providers[0].api_key, "test-secret");
    service
        .save_with_credentials(&settings, CredentialPersistence::Include)
        .unwrap();
    assert_eq!(service.load().unwrap().unwrap(), settings);
    assert!(!format!("{settings:?}").contains("test-secret"));
    storage.write_atomic("settings.json", b"corrupt").unwrap();
    let loaded = service.load_or_default();
    assert!(loaded.warning.is_some());
    assert_eq!(
        storage.read("settings.json").unwrap(),
        Some(b"corrupt".to_vec())
    );
    settings.max_backup_count = 0;
    assert!(service.save(&settings).is_err());
}

#[test]
fn settings_edit_cancel_and_failed_save_leave_original_configuration_intact() {
    let storage = Rc::new(FailingStorage {
        inner: MemoryStudioStorage::default(),
        fail: Cell::new(true),
    });
    let service = SettingsService::new(storage.clone(), "settings.json");
    let original = StudioSettings::default();
    let mut editor = SettingsEditor::new(original.clone());
    editor.draft_mut().auto_save_enabled = false;
    assert!(editor.save(&service, CredentialPersistence::Omit).is_err());
    assert_eq!(editor.cancel(), original);
    let mut editor = SettingsEditor::new(original);
    editor.draft_mut().max_backup_count = 5;
    storage.fail.set(false);
    let saved = editor.save(&service, CredentialPersistence::Omit).unwrap();
    assert_eq!(saved.max_backup_count, 5);
    assert_eq!(editor.cancel(), saved);
}

fn make_suggestions(plan: &TranslationPlan, texts: &[&str]) -> Vec<TranslationSuggestion> {
    plan.suggestions(&AiTranslationResult {
        entries: plan
            .request()
            .entries
            .iter()
            .zip(texts)
            .map(|(entry, text)| AiTranslationResultEntry {
                key: entry.key.clone(),
                translated_text: (*text).into(),
                confidence: 0.9,
            })
            .collect(),
        provider: AiProvider::OpenAI,
        model_used: "test-model".into(),
        tokens_used: 12,
        duration_ms: 34,
    })
    .unwrap()
}

#[test]
fn suggestions_accept_atomically_into_review_with_undo_and_reject_without_edits() {
    let mut doc = document();
    let original = doc.catalog().clone();
    let plan = doc
        .prepare_translation(&[EntryId(1), EntryId(2)], "Product context")
        .unwrap();
    assert_eq!(plan.request().source_language, "en-US");
    assert_eq!(plan.request().target_language, "pl-PL");
    assert_eq!(plan.request().context, "Product context");
    assert_eq!(plan.request().entries[0].key, "1");
    let suggestions = make_suggestions(&plan, &["Razem: {0:N2}", "Brak zamówień"]);
    assert_eq!(doc.accept_suggestions(&suggestions).unwrap(), 2);
    assert_eq!(doc.undo_count(), 1);
    assert_eq!(
        doc.entry(EntryId(1)).unwrap().1.review,
        TranslationReviewState::Review
    );
    assert_eq!(
        doc.entry(EntryId(2)).unwrap().1.review,
        TranslationReviewState::Review
    );
    doc.undo().unwrap();
    assert_eq!(doc.catalog(), &original);
    assert!(
        matches!(
            doc.accept_suggestion(&suggestions[0]),
            Err(StudioError::StaleSuggestion)
        ),
        "undo must not resurrect a request baseline"
    );
    let before = doc.snapshot(9);
    suggestions[0].clone().reject();
    assert_eq!(doc.snapshot(9), before);
}

#[test]
fn suggestions_reject_later_edits_deletion_restoration_and_invalid_placeholders() {
    let mut doc = document();
    let plan = doc
        .prepare_translation(&[EntryId(1), EntryId(2)], "")
        .unwrap();
    let suggestions = make_suggestions(&plan, &["Razem: {0:N2}", "Brak"]);
    doc.set_notes(
        EntryId(2),
        Some("edited while transport was pending".into()),
    )
    .unwrap();
    let before = doc.catalog().clone();
    assert!(matches!(
        doc.accept_suggestions(&suggestions),
        Err(StudioError::StaleSuggestion)
    ));
    assert_eq!(doc.catalog(), &before);
    doc.delete_entry(EntryId(1)).unwrap();
    doc.undo().unwrap();
    assert!(!suggestions[0].is_current(&doc));
    let new_plan = doc.prepare_translation(&[EntryId(1)], "").unwrap();
    let invalid = make_suggestions(&new_plan, &["Lost the placeholder"]);
    assert!(doc.accept_suggestion(&invalid[0]).is_err());
    assert_eq!(doc.catalog(), &before);
    let current = make_suggestions(&new_plan, &["Razem: {0:N2}"]);
    doc.restore(doc.snapshot(3), false).unwrap();
    assert!(!current[0].is_current(&doc));
}

#[test]
fn missing_translation_plans_and_response_mapping_do_not_confuse_duplicate_keys() {
    let mut doc = document();
    let missing = doc.prepare_missing_translations("context").unwrap();
    assert_eq!(missing.request().entries.len(), 1);
    assert_eq!(missing.request().entries[0].key, "2");
    assert!(
        doc.prepare_translation(&[EntryId(3)], "").is_err(),
        "other target culture must not be translated with the wrong language"
    );
    let mut session = doc.snapshot(2);
    session
        .catalog
        .entries
        .push(session.catalog.entries[0].clone());
    session.entry_status.push(EntryStatus {
        id: EntryId(4),
        review: TranslationReviewState::Review,
        revision: session.revision,
    });
    session.next_entry_id = 5;
    doc.restore(session, false).unwrap();
    let plan = doc
        .prepare_translation(&[EntryId(1), EntryId(4)], "")
        .unwrap();
    assert_eq!(
        plan.request()
            .entries
            .iter()
            .map(|entry| entry.key.as_str())
            .collect::<Vec<_>>(),
        vec!["1", "4"]
    );
    let result = AiTranslationResult {
        entries: vec![AiTranslationResultEntry {
            key: "Orders.Total".into(),
            translated_text: "foreign".into(),
            confidence: 1.0,
        }],
        provider: AiProvider::Claude,
        model_used: "test".into(),
        tokens_used: 1,
        duration_ms: 1,
    };
    assert!(plan.suggestions(&result).is_err());
}
