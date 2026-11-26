use crate::{LanguageService, Position, ProjectLibraryRequest, ProjectLibraryResolution};
use ferroui_designer_project::{Solution, file_uri};
use ferroui_designer_tools::ComponentKind;
use std::fs;

#[test]
fn project_library_follows_indexed_files_and_revalidates_editor_drafts() {
    let files = crate::project_navigation_tests::CatalogFixture::create();
    let root = &files.0;
    files.write("Cards.xaml", concat!(
        "<ResourceDictionary xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\r\n",
        "  <!-- 🌱 --><DataTemplate x:Key='CustomerCard'><TextBlock/></DataTemplate>\r\n",
        "  <Style Selector='Button.featured'/>\r\n",
        "  <Style Selector='^ Button'/>\r\n",
        "  <SolidColorBrush x:Key='PrivateBrush' Color='Red'/>\r\n",
        "</ResourceDictionary>",
    ));
    fs::create_dir(root.join("nested")).unwrap();
    files.write(
        "nested/Detail.xaml",
        "<UserControl xmlns='https://github.com/ferroui'/>",
    );
    let mut service = LanguageService::default();
    service
        .index_solution(&Solution::discover(root).unwrap())
        .unwrap();
    service
        .index_solution(&Solution::discover(root.join("nested")).unwrap())
        .unwrap();
    let folder = file_uri(root).unwrap();
    let uri = file_uri(root.join("Cards.xaml")).unwrap();
    let mut request = ProjectLibraryRequest {
        folder: folder.clone(),
        query: String::new(),
        offset: 0,
        limit: 1,
    };

    let page = service.project_library(&request).unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.entries[0].path, "Cards.xaml");
    let entry = page.entries[0].clone();
    let definition = &entry.definition;
    assert_eq!(
        definition.range.start,
        Position {
            line: 1,
            character: 14
        }
    );
    assert_eq!(definition.kind, ComponentKind::DataTemplate);
    request.offset = 1;
    assert_eq!(
        service.project_library(&request).unwrap().entries[0]
            .definition
            .kind,
        ComponentKind::Style
    );
    request.offset = 0;
    request.query = "customer cards".into();
    assert_eq!(service.project_library(&request).unwrap().total, 1);
    request.folder = file_uri(root.join("nested")).unwrap();
    request.query.clear();
    assert_eq!(
        service.project_library(&request).unwrap().entries[0].path,
        "Detail.xaml"
    );

    let source = fs::read_to_string(root.join("Cards.xaml")).unwrap();
    service.open(&uri, &source, 1).unwrap();
    let mut target = ProjectLibraryResolution {
        entry,
        uri: uri.clone(),
        version: 1,
    };
    assert_eq!(service.project_library_uri(&target.entry).unwrap(), uri);
    let mut outside = target.entry.clone();
    outside.path = "nested/Detail.xaml".into();
    assert!(service.project_library_uri(&outside).is_err());
    service.resolve_project_library(&target).unwrap();
    service.replace(&uri, "<invalid", 2).unwrap();
    request.folder = folder;
    assert_eq!(service.project_library(&request).unwrap().total, 0);
    assert!(service.resolve_project_library(&target).is_err());
    service
        .replace(&uri, source.replace("CustomerCard", "DraftCard"), 3)
        .unwrap();
    target.version = 3;
    assert!(service.resolve_project_library(&target).is_err());
    let page = service.project_library(&request).unwrap();
    assert_eq!(page.entries[0].definition.name, "DraftCard");
    service.close(&uri).unwrap();
    assert_eq!(
        service.project_library(&request).unwrap().entries[0]
            .definition
            .name,
        "CustomerCard"
    );
    files.write("Cards.xaml", &source.replace("CustomerCard", "SavedCard"));
    service.refresh_project_file(&uri).unwrap();
    assert_eq!(
        service.project_library(&request).unwrap().entries[0]
            .definition
            .name,
        "SavedCard"
    );
    fs::remove_file(root.join("Cards.xaml")).unwrap();
    service.refresh_project_file(&uri).unwrap();
    assert_eq!(service.project_library(&request).unwrap().total, 0);
}
