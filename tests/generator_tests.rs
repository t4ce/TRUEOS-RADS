use trueos_rads::generator::create_project;
use trueos_rads::generator::write_project_files;
use trueos_rads::model::{ProjectNameError, Ui2Window, validate_project_name};

#[test]
fn validates_project_names_before_generation() {
    assert_eq!(
        validate_project_name("   ").unwrap_err(),
        ProjectNameError::Empty
    );
    assert_eq!(
        validate_project_name("../escape").unwrap_err(),
        ProjectNameError::ContainsPathSeparator
    );
    assert_eq!(
        validate_project_name("!!!").unwrap_err(),
        ProjectNameError::NoSlug
    );
    assert_eq!(
        validate_project_name("con").unwrap_err(),
        ProjectNameError::ReservedName
    );

    let valid = validate_project_name("  Hello, UI2 App!  ").unwrap();
    assert_eq!(valid.display, "Hello, UI2 App!");
    assert_eq!(valid.slug, "hello-ui2-app");
}

#[test]
fn creates_rich_starter_project_files() {
    let dir = tempfile::tempdir().unwrap();
    let project = create_project(dir.path(), "Hello UI2").unwrap();

    assert_eq!(project.slug, "hello-ui2");
    assert_eq!(project.blueprint.schema, "trueos.app.blueprint/v1");
    assert_eq!(project.package.schema, "trueos.package.blueprint/v1");
    assert!(project.root.join("rads.project.json").exists());
    assert!(project.root.join("app.blueprint.json").exists());
    assert!(project.root.join("package/package.blueprint.json").exists());
    assert!(project.root.join("package/manifest.trueos.json").exists());
    assert!(project.root.join("ui/main.ui2.json").exists());
    assert!(project.root.join("ui/main.ui2").exists());
    assert!(project.root.join("src/main.rs").exists());
    assert!(project.root.join("src/ui.rs").exists());
    assert!(project.root.join("src/events.rs").exists());
    assert!(project.root.join("README.md").exists());
}

#[test]
fn generated_layout_serializes_decorations_and_events() {
    let dir = tempfile::tempdir().unwrap();
    let project = create_project(dir.path(), "Decor Test").unwrap();

    let layout = std::fs::read_to_string(project.root.join("ui/main.ui2")).unwrap();
    assert!(layout.contains("decorations { titlebar: true"));
    assert!(layout.contains("decoration-flags [titlebar, close, minimize, maximize, resizable]"));
    assert!(layout.contains("button runButton"));
    assert!(layout.contains("click -> on_run_button_click"));
    assert!(layout.contains("textbox inputText"));
    assert!(layout.contains("change -> on_input_text_change"));
}

#[test]
fn generated_rust_contains_ui_module_and_event_stubs() {
    let dir = tempfile::tempdir().unwrap();
    let project = create_project(dir.path(), "Stub Test").unwrap();

    let main_rs = std::fs::read_to_string(project.root.join("src/main.rs")).unwrap();
    let ui_rs = std::fs::read_to_string(project.root.join("src/ui.rs")).unwrap();
    let events_rs = std::fs::read_to_string(project.root.join("src/events.rs")).unwrap();

    assert!(main_rs.contains("mod events;"));
    assert!(main_rs.contains("mod ui;"));
    assert!(ui_rs.contains("pub const MAIN_LAYOUT"));
    assert!(ui_rs.contains("pub const MAIN_WINDOW_DECORATIONS"));
    assert!(events_rs.contains("pub fn wire_main_window()"));
    assert!(events_rs.contains("pub fn on_run_button_click()"));
    assert!(events_rs.contains("pub fn on_input_text_change()"));
}

#[test]
fn generated_metadata_is_present_in_blueprints() {
    let dir = tempfile::tempdir().unwrap();
    let project = create_project(dir.path(), "Meta Test").unwrap();

    let app_blueprint = std::fs::read_to_string(project.root.join("app.blueprint.json")).unwrap();
    let package_blueprint =
        std::fs::read_to_string(project.root.join("package/package.blueprint.json")).unwrap();
    let manifest =
        std::fs::read_to_string(project.root.join("package/manifest.trueos.json")).unwrap();

    assert!(app_blueprint.contains("\"metadata\""));
    assert!(app_blueprint.contains("\"ui_layout\": \"ui/main.ui2\""));
    assert!(package_blueprint.contains("\"package_id\": \"dev.trueos.meta-test.package\""));
    assert!(manifest.contains("\"schema\": \"trueos.package.manifest/v1\""));
    assert!(manifest.contains("\"layout\": \"ui/main.ui2\""));
}

#[test]
fn generated_ui2_project_supports_multiple_windows() {
    let dir = tempfile::tempdir().unwrap();
    let mut project = create_project(dir.path(), "Many Windows").unwrap();
    let mut second = Ui2Window::named_window("SettingsWindow", "Settings", 140, 140, 520, 360);
    second.title_twemoji = Some("⚡".to_string());
    project.windows.push(second);
    write_project_files(&project).unwrap();

    let layout = std::fs::read_to_string(project.root.join("ui/main.ui2")).unwrap();
    let ui_rs = std::fs::read_to_string(project.root.join("src/ui.rs")).unwrap();

    assert!(layout.contains("window MainWindow"));
    assert!(layout.contains("window SettingsWindow"));
    assert!(layout.contains("title-twemoji \"⚡\""));
    assert!(project.root.join("ui/windows/main.ui2.json").exists());
    assert!(
        project
            .root
            .join("ui/windows/settingswindow.ui2.json")
            .exists()
    );
    assert!(project.root.join("ui/windows/settingswindow.html").exists());
    assert!(project.root.join("ui/windows/settingswindow.css").exists());
    assert!(ui_rs.contains("pub fn create_window_0()"));
    assert!(ui_rs.contains("pub fn create_window_1()"));
    assert!(ui_rs.contains("pub fn create_all_windows()"));
}
