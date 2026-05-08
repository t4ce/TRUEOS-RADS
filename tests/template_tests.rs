use std::fs;

use trueos_rads::generator::{create_project, create_project_from_template};
use trueos_rads::model::{AppKind, ControlKind};
use trueos_rads::project_templates::{
    BLANK_UI2_TEMPLATE_ID, CANVAS_APP_TEMPLATE_ID, DEFAULT_PROJECT_TEMPLATE_ID,
    FORM_APP_TEMPLATE_ID, SERVICE_APP_TEMPLATE_ID, SHELL_APP_TEMPLATE_ID, TOOL_WINDOW_TEMPLATE_ID,
    available_project_templates, default_project_template,
};

#[test]
fn template_catalog_exposes_expected_starters() {
    let templates = available_project_templates();
    let ids = templates
        .iter()
        .map(|template| template.id)
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            BLANK_UI2_TEMPLATE_ID,
            FORM_APP_TEMPLATE_ID,
            CANVAS_APP_TEMPLATE_ID,
            TOOL_WINDOW_TEMPLATE_ID,
            SERVICE_APP_TEMPLATE_ID,
            SHELL_APP_TEMPLATE_ID,
        ]
    );
    assert_eq!(default_project_template().id, DEFAULT_PROJECT_TEMPLATE_ID);

    for template in templates {
        assert!(!template.description.is_empty());
        assert!(template.starter_main_rs.contains("fn main()"));
        assert!(!template.capabilities.is_empty());
        if template.app_kind.has_ui2() {
            assert!(template.initial_html.contains("<!doctype html>"));
            assert!(template.initial_css.contains("body"));
            assert!(template.starter_events_rs.contains("wire_main_window"));
            let window = template.window.as_ref().unwrap();
            assert!(window.geometry.w > 0);
            assert!(window.geometry.h > 0);
        } else {
            assert!(template.window.is_none());
        }
    }
}

#[test]
fn create_project_keeps_existing_form_starter_contract() {
    let dir = tempfile::tempdir().unwrap();
    let project = create_project(dir.path(), "Hello UI2").unwrap();

    assert_eq!(project.windows[0].controls.len(), 3);
    assert_eq!(project.app_kind, AppKind::Ui2);
    assert_eq!(project.windows[0].geometry.w, 720);
    assert!(project.root.join("ui/index.html").exists());
    assert!(project.root.join("ui/styles.css").exists());
    assert!(project.root.join("ui/windows/main.ui2.json").exists());

    let html = fs::read_to_string(project.root.join("ui/index.html")).unwrap();
    let events = fs::read_to_string(project.root.join("src/events.rs")).unwrap();

    assert!(html.contains("class=\"form-shell\""));
    assert!(events.contains("UI2 event stubs registered"));
    assert!(events.contains("pub fn on_run_button_click()"));
    assert!(events.contains("pub fn on_input_text_change()"));
}

#[test]
fn generator_uses_selected_canvas_template() {
    let dir = tempfile::tempdir().unwrap();
    let project =
        create_project_from_template(dir.path(), "Canvas Pad", CANVAS_APP_TEMPLATE_ID).unwrap();

    let window = &project.windows[0];
    assert_eq!(window.geometry.w, 960);
    assert_eq!(window.geometry.h, 640);
    assert!(
        project
            .blueprint
            .capabilities
            .iter()
            .any(|capability| capability.key == "ui2.canvas" && capability.enabled)
    );
    assert!(window.controls.iter().any(|control| {
        matches!(&control.kind, ControlKind::Canvas) && control.name == "drawingCanvas"
    }));

    let layout = fs::read_to_string(project.root.join("ui/main.ui2")).unwrap();
    let html = fs::read_to_string(project.root.join("ui/index.html")).unwrap();
    let css = fs::read_to_string(project.root.join("ui/styles.css")).unwrap();
    let ui_rs = fs::read_to_string(project.root.join("src/ui.rs")).unwrap();
    let events = fs::read_to_string(project.root.join("src/events.rs")).unwrap();

    assert!(layout.contains("canvas drawingCanvas"));
    assert!(html.contains("<canvas name=\"drawingCanvas\""));
    assert!(html.contains("data-template=\"canvas-app\""));
    assert!(css.contains(".canvas-shell"));
    assert!(ui_rs.contains("pub const MAIN_HTML"));
    assert!(ui_rs.contains("pub const MAIN_CSS"));
    assert!(events.contains("canvas app event stubs registered"));
    assert!(events.contains("pub fn on_drawing_canvas_draw()"));
}

#[test]
fn blank_template_generates_empty_window() {
    let dir = tempfile::tempdir().unwrap();
    let project =
        create_project_from_template(dir.path(), "Blank Slate", BLANK_UI2_TEMPLATE_ID).unwrap();

    assert!(project.windows[0].controls.is_empty());
    assert!(
        project
            .blueprint
            .capabilities
            .iter()
            .any(|capability| capability.key == "ui2.events" && !capability.enabled)
    );

    let events = fs::read_to_string(project.root.join("src/events.rs")).unwrap();
    assert!(events.contains("blank UI2 window ready"));
    assert!(!events.contains("pub fn on_"));
}

#[test]
fn unknown_template_id_reports_available_ids() {
    let dir = tempfile::tempdir().unwrap();
    let err = create_project_from_template(dir.path(), "Missing", "missing-template")
        .unwrap_err()
        .to_string();

    assert!(err.contains("unknown project template 'missing-template'"));
    assert!(err.contains(BLANK_UI2_TEMPLATE_ID));
    assert!(err.contains(FORM_APP_TEMPLATE_ID));
    assert!(err.contains(CANVAS_APP_TEMPLATE_ID));
    assert!(err.contains(TOOL_WINDOW_TEMPLATE_ID));
    assert!(err.contains(SERVICE_APP_TEMPLATE_ID));
    assert!(err.contains(SHELL_APP_TEMPLATE_ID));
}

#[test]
fn service_and_shell_templates_have_no_ui2_surface() {
    let dir = tempfile::tempdir().unwrap();
    let service =
        create_project_from_template(dir.path(), "Index Service", SERVICE_APP_TEMPLATE_ID).unwrap();
    let shell =
        create_project_from_template(dir.path(), "Ops Shell", SHELL_APP_TEMPLATE_ID).unwrap();

    assert_eq!(service.app_kind, AppKind::Service);
    assert_eq!(shell.app_kind, AppKind::Shell);
    assert!(service.windows.is_empty());
    assert!(shell.windows.is_empty());
    assert_eq!(service.blueprint.ui_layout, "");
    assert_eq!(shell.blueprint.ui_layout, "");
    assert!(!service.root.join("src/ui.rs").exists());
    assert!(!shell.root.join("src/ui.rs").exists());
    assert!(!service.root.join("ui/main.ui2").exists());
    assert!(!shell.root.join("ui/main.ui2").exists());

    let service_main = fs::read_to_string(service.root.join("src/main.rs")).unwrap();
    let shell_main = fs::read_to_string(shell.root.join("src/main.rs")).unwrap();
    assert!(service_main.contains("service Index Service starting"));
    assert!(shell_main.contains("Ops Shell shell ready"));
}
