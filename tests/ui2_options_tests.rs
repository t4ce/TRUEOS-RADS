use std::fs;

use trueos_rads::designer::{
    ObjectInspectorData, ObjectInspectorEditor, ObjectInspectorField, ObjectInspectorSection,
    UpdateWindowRequest, object_inspector_for_window, update_window,
};
use trueos_rads::generator::create_project;
use trueos_rads::model::{Ui2Window, WindowDecorations};
use trueos_rads::ui2_options::{
    Ui2HtmlCssDescription, Ui2ResizeMode, Ui2ScrollbarMode, Ui2Size, Ui2WindowOptions,
};

#[test]
fn legacy_window_json_gets_window_options_and_html_css_defaults() {
    let legacy = serde_json::json!({
        "id": "22222222-2222-4222-8222-222222222222",
        "name": "MainWindow",
        "caption": "Legacy UI",
        "geometry": {
            "x": 80,
            "y": 80,
            "w": 720,
            "h": 460
        },
        "decorations": {
            "titlebar": true,
            "close": true,
            "minimize": true,
            "maximize": true,
            "resizable": true
        },
        "controls": []
    });

    let window: Ui2Window = serde_json::from_value(legacy).unwrap();

    assert_eq!(window.options.min_size, Ui2Size::new(320, 240));
    assert_eq!(window.options.max_size, None);
    assert_eq!(window.options.resize_mode, Ui2ResizeMode::Both);
    assert_eq!(window.options.scrollbars, Ui2ScrollbarMode::None);
    assert!(window.options.hit_test_visible);
    assert!(!window.options.preserve_scale);
    assert_eq!(window.decorations, WindowDecorations::default());
    assert!(window.ui_description.html.contains("ui2-window"));
    assert!(window.ui_description.css.contains(".ui2-window"));
}

#[test]
fn window_options_and_ui_description_update_and_round_trip() {
    let mut window = Ui2Window::main_window("Options");
    let window_id = window.id;
    let options = Ui2WindowOptions {
        min_size: Ui2Size::new(480, 320),
        max_size: Some(Ui2Size::new(1280, 900)),
        resize_mode: Ui2ResizeMode::Width,
        scrollbars: Ui2ScrollbarMode::Auto,
        hit_test_visible: false,
        preserve_scale: true,
    };
    let ui_description = Ui2HtmlCssDescription {
        html: r#"<main class="custom-window"></main>"#.to_string(),
        css: ".custom-window { display: grid; }\n".to_string(),
    };

    assert!(update_window(
        &mut window,
        UpdateWindowRequest {
            window_id,
            caption: Some("Options Dialog".to_string()),
            geometry: None,
            decorations: None,
            options: Some(options.clone()),
            ui_description: Some(ui_description.clone()),
        },
    ));

    assert_eq!(window.caption, "Options Dialog");
    assert_eq!(window.options, options);
    assert_eq!(window.ui_description, ui_description);

    let value = serde_json::to_value(&window).unwrap();
    assert_eq!(value["options"]["min_size"]["width"].as_u64(), Some(480));
    assert_eq!(value["options"]["max_size"]["height"].as_u64(), Some(900));
    assert_eq!(value["options"]["resize_mode"].as_str(), Some("width"));
    assert_eq!(value["options"]["scrollbars"].as_str(), Some("auto"));
    assert_eq!(value["options"]["hit_test_visible"].as_bool(), Some(false));
    assert_eq!(value["options"]["preserve_scale"].as_bool(), Some(true));
    assert_eq!(
        value["ui_description"]["html"].as_str(),
        Some(r#"<main class="custom-window"></main>"#)
    );
}

#[test]
fn window_inspector_exposes_app_wanted_options_and_ui_description() {
    let mut window = Ui2Window::main_window("Inspector");
    window.options.max_size = Some(Ui2Size::new(1024, 768));
    window.options.scrollbars = Ui2ScrollbarMode::Both;
    window.options.preserve_scale = true;
    window.decorations.always_on_top = true;

    let inspector = object_inspector_for_window(&window);
    let snapshot = inspector.window_options.as_ref().unwrap();
    assert_eq!(snapshot.caption, "Inspector");
    assert_eq!(snapshot.max_size, Some(Ui2Size::new(1024, 768)));
    assert!(
        snapshot
            .decoration_flags
            .iter()
            .any(|flag| flag == "always-on-top")
    );
    assert!(
        inspector
            .ui_description
            .as_ref()
            .unwrap()
            .html
            .contains("<main")
    );

    let options = section(&inspector, "window-options");
    assert_eq!(field(options, "min-width").value, "320");
    assert_eq!(field(options, "max-width").value, "1024");
    assert_eq!(field(options, "scrollbars").value, "both");
    assert_eq!(field(options, "preserve-scale").value, "true");

    match &field(options, "resize-mode").editor {
        ObjectInspectorEditor::Select { options } => {
            assert!(options.iter().any(|option| option == "both"));
        }
        editor => panic!("expected resize-mode select editor, got {editor:?}"),
    }

    let decorations = section(&inspector, "decorations");
    assert!(
        field(decorations, "decoration-flags")
            .value
            .contains("always-on-top")
    );

    let ui_description = section(&inspector, "ui-description");
    assert_text_area(field(ui_description, "ui-description.html"), "html");
    assert_text_area(field(ui_description, "ui-description.css"), "css");
}

#[test]
fn generated_html_and_css_are_stored_on_the_window_model() {
    let dir = tempfile::tempdir().unwrap();
    let project = create_project(dir.path(), "HTML CSS Model").unwrap();
    let stored_window: Ui2Window =
        serde_json::from_str(&fs::read_to_string(project.root.join("ui/main.ui2.json")).unwrap())
            .unwrap();
    let html = fs::read_to_string(project.root.join("ui/index.html")).unwrap();
    let css = fs::read_to_string(project.root.join("ui/styles.css")).unwrap();

    assert!(stored_window.ui_description.html.contains("form-shell"));
    assert_eq!(stored_window.ui_description.html, html);
    assert_eq!(stored_window.ui_description.css, css);
}

fn section<'a>(inspector: &'a ObjectInspectorData, id: &str) -> &'a ObjectInspectorSection {
    inspector
        .sections
        .iter()
        .find(|section| section.id == id)
        .unwrap_or_else(|| panic!("missing section {id}"))
}

fn field<'a>(section: &'a ObjectInspectorSection, key: &str) -> &'a ObjectInspectorField {
    section
        .fields
        .iter()
        .find(|field| field.key == key)
        .unwrap_or_else(|| panic!("missing field {key}"))
}

fn assert_text_area(field: &ObjectInspectorField, language: &str) {
    match &field.editor {
        ObjectInspectorEditor::TextArea {
            language: Some(actual),
        } => assert_eq!(actual, language),
        editor => panic!("expected {language} text area editor, got {editor:?}"),
    }
}
