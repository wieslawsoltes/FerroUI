use crate::{TextArea, TextDocument, TextEditor, TextMateInstallation};
use ferroui_base::reactive::Subscription;
use ferroui_base::{
    FerroLocator, FerroProperty, IFerroDependencyResolver, Ref, ServiceHandle, WeakRef,
};
use ferroui_edit_core::document::{IDocument, WeakTextDocument};
use ferroui_edit_core::editing::EditorOptions;
use ferroui_edit_core::folding::FoldingManager;
use ferroui_edit_core::highlighting::{DocumentHighlighter, IHighlighter};
use std::any::TypeId;
use std::cell::RefCell;
use std::rc::Rc;

/// Observes the previous and current values of one editor component property.
pub type EditorChangeHandler<T> = dyn Fn(&T, &T);

/// Access to the bound document, options, and scoped services of an editor.
/// An unowned text area still has its own view services.
pub trait TextEditorComponent {
    fn editor(&self) -> Option<Ref<TextEditor>>;

    /// The shared view locator. Lookup falls back to the current document.
    fn services(&self) -> Rc<FerroLocator>;

    fn bound_document(&self) -> Option<TextDocument> {
        self.editor().map(|editor| editor.document())
    }

    fn editor_options(&self) -> Option<EditorOptions> {
        self.editor().map(|editor| editor.options())
    }

    /// Observes changes of document identity. Text edits keep the same binding.
    fn subscribe_document_changed(
        &self,
        handler: Rc<EditorChangeHandler<TextDocument>>,
    ) -> Option<Subscription> {
        let editor = self.editor()?;
        Some(observe(
            &editor,
            TextEditor::document_property().as_property(),
            TextEditor::document,
            handler,
        ))
    }

    /// Observes value changes to the bound editor's options.
    fn subscribe_options_changed(
        &self,
        handler: Rc<EditorChangeHandler<EditorOptions>>,
    ) -> Option<Subscription> {
        let editor = self.editor()?;
        Some(observe(
            &editor,
            TextEditor::options_property().as_property(),
            TextEditor::options,
            handler,
        ))
    }
}

impl TextEditorComponent for TextEditor {
    fn editor(&self) -> Option<Ref<TextEditor>> {
        Some(self.to_ref())
    }

    fn services(&self) -> Rc<FerroLocator> {
        self.text_area().component_services()
    }
}

impl TextEditorComponent for TextArea {
    fn editor(&self) -> Option<Ref<TextEditor>> {
        TextArea::editor(self)
    }

    fn services(&self) -> Rc<FerroLocator> {
        self.component_services()
    }
}

/// Native services that travel with a live document when it is shared or rebound.
pub trait TextDocumentServices {
    fn services(&self) -> Rc<FerroLocator>;
}

impl TextDocumentServices for TextDocument {
    fn services(&self) -> Rc<FerroLocator> {
        self.get_or_insert_extension_with(|| {
            Rc::new(FerroLocator::with_parent_scope(Rc::new(
                DocumentServiceDefaults(self.downgrade()),
            )))
        })
    }
}

fn observe<T: Clone + PartialEq + 'static>(
    editor: &TextEditor,
    property: &'static FerroProperty,
    read: fn(&TextEditor) -> T,
    handler: Rc<EditorChangeHandler<T>>,
) -> Subscription {
    let previous = RefCell::new(read(editor));
    let weak = editor.to_ref().downgrade();
    Subscription::new(editor.property_changed(move |event| {
        if event.property() != property {
            return;
        }
        if let Some(editor) = weak.upgrade() {
            let current = read(&editor);
            let old = previous.replace(current.clone());
            if old != current {
                handler(&old, &current);
            }
        }
    }))
}

pub(crate) fn view_services(area: &TextArea) -> Rc<FerroLocator> {
    Rc::new(FerroLocator::with_parent_scope(Rc::new(
        ViewServiceDefaults(area.to_ref().downgrade()),
    )))
}

struct ViewServiceDefaults(WeakRef<TextArea>);

impl IFerroDependencyResolver for ViewServiceDefaults {
    fn get_service_untyped(&self, service: TypeId) -> Option<ServiceHandle> {
        let area = self.0.upgrade()?;
        if service == TypeId::of::<Ref<TextArea>>() {
            return Some(handle(Rc::new(area)));
        }
        let editor = area.editor()?;
        if service == TypeId::of::<Ref<TextEditor>>() {
            return Some(handle(Rc::new(editor)));
        }
        let builtin = if service == TypeId::of::<FoldingManager>() {
            editor.folding_manager().map(handle)
        } else if service == TypeId::of::<DocumentHighlighter>() {
            editor.document_highlighter().map(handle)
        } else if service == TypeId::of::<dyn IHighlighter>() {
            editor
                .document_highlighter()
                .map(|value| handle(value as Rc<dyn IHighlighter>))
        } else if service == TypeId::of::<TextMateInstallation>() {
            editor.textmate_installation().map(handle)
        } else {
            None
        };
        builtin.or_else(|| editor.document().services().get_service_untyped(service))
    }
}

struct DocumentServiceDefaults(WeakTextDocument);

impl IFerroDependencyResolver for DocumentServiceDefaults {
    fn get_service_untyped(&self, service: TypeId) -> Option<ServiceHandle> {
        if service == TypeId::of::<TextDocument>() {
            return Some(handle(Rc::new(self.0.upgrade()?)));
        }
        if service == TypeId::of::<dyn IDocument>() {
            return Some(handle(Rc::new(self.0.upgrade()?) as Rc<dyn IDocument>));
        }
        None
    }
}

fn handle<T: ?Sized + 'static>(value: Rc<T>) -> ServiceHandle {
    Rc::new(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::test_scope;
    use ferroui_base::LocatorExtensions;

    trait Label {
        fn name(&self) -> &'static str;
    }

    impl Label for &'static str {
        fn name(&self) -> &'static str {
            self
        }
    }

    #[test]
    fn view_services_follow_the_current_document_and_keep_view_precedence() {
        let _app = test_scope();
        let editor = TextEditor::new();
        let area = editor.text_area();
        let first = editor.document();
        first
            .services()
            .bind::<dyn Label>()
            .to_constant(Rc::new("first document"));
        let second = TextDocument::from_text("second");
        second
            .services()
            .bind::<dyn Label>()
            .to_constant(Rc::new("second document"));
        let shared = TextEditor::new();
        shared.set_document(first.clone());

        let services = area.services();
        assert!(Rc::ptr_eq(&services, &editor.services()));
        assert!(services
            .get_service::<Ref<TextArea>>()
            .unwrap()
            .ptr_eq(&area));
        assert!(services
            .get_service::<Ref<TextEditor>>()
            .unwrap()
            .ptr_eq(&editor));
        assert_eq!(
            services.get_service::<TextDocument>().as_deref(),
            Some(&first)
        );
        assert_eq!(
            first
                .services()
                .get_required_service::<dyn IDocument>()
                .text(),
            ""
        );
        assert_eq!(
            services.get_required_service::<dyn Label>().name(),
            "first document"
        );

        services.bind::<dyn Label>().to_constant(Rc::new("view"));
        assert_eq!(
            shared.services().get_required_service::<dyn Label>().name(),
            "first document"
        );
        editor.set_document(second.clone());
        assert_eq!(services.get_required_service::<dyn Label>().name(), "view");
        assert!(services.unbind::<dyn Label>());
        assert_eq!(
            services.get_required_service::<dyn Label>().name(),
            "second document"
        );
        assert_eq!(
            services.get_service::<TextDocument>().as_deref(),
            Some(&second)
        );

        assert!(services.get_service::<FoldingManager>().is_none());
        let folding = editor.install_folding();
        assert!(Rc::ptr_eq(
            &services.get_required_service::<FoldingManager>(),
            &folding
        ));
        editor.uninstall_folding();
        assert!(services.get_service::<FoldingManager>().is_none());
    }

    #[test]
    fn subscriptions_distinguish_binding_changes_from_text_edits() {
        let _app = test_scope();
        let editor = TextEditor::new();
        let view = editor.text_area();
        let document = editor.document();
        let replacement = TextDocument::from_text("replacement");
        let changes = Rc::new(RefCell::new(Vec::new()));
        let binding = view
            .subscribe_document_changed(Rc::new({
                let changes = changes.clone();
                let services = view.services();
                move |old, new| {
                    assert_eq!(services.get_service::<TextDocument>().as_deref(), Some(new));
                    changes.borrow_mut().push((old.clone(), new.clone()));
                }
            }))
            .unwrap();
        let options = Rc::new(RefCell::new(Vec::new()));
        let settings = editor
            .subscribe_options_changed(Rc::new({
                let options = options.clone();
                move |old, new| {
                    options
                        .borrow_mut()
                        .push((old.indentation_size, new.indentation_size))
                }
            }))
            .unwrap();

        document.insert(0, "existing document edit").unwrap();
        assert!(changes.borrow().is_empty());
        editor.set_document(replacement.clone());
        editor.set_document(replacement.clone());
        assert_eq!(*changes.borrow(), [(document, replacement.clone())]);
        assert_eq!(view.bound_document(), Some(replacement));

        let mut changed = editor.options();
        changed.indentation_size = 2;
        editor.set_options(changed.clone());
        editor.set_options(changed.clone());
        assert_eq!(view.editor_options(), Some(changed));
        assert_eq!(*options.borrow(), [(4, 2)]);

        drop(binding);
        drop(settings);
        editor.clear_value(TextEditor::document_property());
        editor.clear_value(TextEditor::options_property());
        assert_eq!(changes.borrow().len(), 1);
        assert_eq!(options.borrow().len(), 1);
    }

    #[test]
    fn retained_locators_do_not_keep_the_view_or_document_alive() {
        let _app = test_scope();
        let (services, document_services) = {
            let editor = TextEditor::new();
            (editor.services(), editor.document().services())
        };
        assert!(services.get_service::<Ref<TextArea>>().is_none());
        assert!(services.get_service::<Ref<TextEditor>>().is_none());
        assert!(document_services.get_service::<TextDocument>().is_none());
        assert!(document_services.get_service::<dyn IDocument>().is_none());

        let view = TextArea::new();
        assert!(view.bound_document().is_none());
        assert!(view.editor_options().is_none());
        assert!(view
            .services()
            .get_required_service::<Ref<TextArea>>()
            .ptr_eq(&view));
    }
}
