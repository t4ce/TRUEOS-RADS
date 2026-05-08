use std::fs;

use trueos_rads::designer::{
    AddControlRequest, SnapSettings, add_control, default_palette, default_palette_categories,
    object_inspector_for_selection,
};
use trueos_rads::generator::create_project;
use trueos_rads::model::{
    AppBlueprint, ControlKind, PackageBlueprint, ProjectNameError, RadsProject, Ui2Window, slugify,
    validate_project_name,
};
use trueos_rads::templates::control_to_ui2_snippet;

#[test]
fn starter_generation_writes_expected_contract_files() {
    let dir = tempfile::tempdir().unwrap();

    let project = create_project(dir.path(), "Hello UI2").unwrap();

    assert_eq!(project.name, "Hello UI2");
    assert_eq!(project.slug, "hello-ui2");
    assert_eq!(project.blueprint.schema, "trueos.app.blueprint/v1");
    assert_eq!(project.blueprint.app_id, "dev.trueos.hello-ui2");
    assert_eq!(project.blueprint.entrypoint, "src/main.rs");
    assert_eq!(project.blueprint.ui_layout, "ui/main.ui2");
    assert_eq!(project.package.schema, "trueos.package.blueprint/v1");
    assert_eq!(project.package.package_id, "dev.trueos.hello-ui2.package");
    assert_eq!(project.windows.len(), 1);

    for relative in [
        "rads.project.json",
        "app.blueprint.json",
        "package/package.blueprint.json",
        "ui/main.ui2.json",
        "ui/main.ui2",
        "Cargo.toml",
        "src/main.rs",
        "src/ui.rs",
        "src/events.rs",
        "package/manifest.trueos.json",
        "README.md",
    ] {
        assert!(
            project.root.join(relative).exists(),
            "missing generated file: {relative}"
        );
    }

    let stored_project: RadsProject =
        serde_json::from_str(&fs::read_to_string(project.root.join("rads.project.json")).unwrap())
            .unwrap();
    let stored_window: Ui2Window =
        serde_json::from_str(&fs::read_to_string(project.root.join("ui/main.ui2.json")).unwrap())
            .unwrap();

    assert_eq!(stored_project.id, project.id);
    assert_eq!(stored_window.id, project.windows[0].id);
    assert_eq!(stored_window.controls.len(), 3);

    let layout = fs::read_to_string(project.root.join("ui/main.ui2")).unwrap();
    assert!(layout.contains("decorations { titlebar: true"));
    assert!(layout.contains("decoration-flags [titlebar, close, minimize, maximize, resizable]"));
    assert!(layout.contains("textbox inputText"));
    assert!(layout.contains("change -> on_input_text_change"));
}

#[test]
fn designer_adds_palette_control_with_snap_and_inspector_data() {
    let mut window = Ui2Window::main_window("Palette Smoke");
    let window_id = window.id;

    let control_id = add_control(
        &mut window,
        AddControlRequest {
            window_id,
            kind: ControlKind::TextBox,
            x: 57,
            y: 159,
            id: None,
            name: None,
            caption: Some("Search".to_string()),
            snap: Some(SnapSettings {
                enabled: true,
                grid: 8,
            }),
        },
    )
    .unwrap();

    let control = window
        .controls
        .iter()
        .find(|control| control.id == control_id)
        .unwrap();

    assert_eq!(control.name, "textbox1");
    assert_eq!(control.caption, "Search");
    assert_eq!(control.geometry.x, 56);
    assert_eq!(control.geometry.y, 160);
    assert_eq!(control.geometry.w, 180);
    assert_eq!(control.geometry.h, 32);
    assert!(
        control
            .properties
            .iter()
            .any(|property| property.key == "placeholder")
    );
    assert_eq!(control.events[0].event, "change");
    assert_eq!(control.events[0].handler, "on_textbox1_change");

    let snippet = control_to_ui2_snippet(control);
    assert!(snippet.contains("textbox textbox1 at 56,160 size 180x32"));
    assert!(snippet.contains("caption \"Search\""));
    assert!(snippet.contains("events [change -> on_textbox1_change]"));

    let inspector = object_inspector_for_selection(&window, Some(control_id));
    assert!(
        inspector
            .sections
            .iter()
            .any(|section| section.id == "properties")
    );
    assert!(
        inspector
            .sections
            .iter()
            .any(|section| section.id == "events")
    );
}

#[test]
fn palette_and_slugify_cover_documented_names() {
    let categories = default_palette_categories();
    assert_eq!(categories.len(), 4);
    assert!(categories.iter().all(|category| !category.items.is_empty()));

    let kinds = default_palette()
        .into_iter()
        .map(|item| item.kind)
        .collect::<Vec<_>>();

    assert!(kinds.iter().any(|kind| matches!(kind, ControlKind::Button)));
    assert!(kinds.iter().any(|kind| matches!(kind, ControlKind::Label)));
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, ControlKind::TextBox))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, ControlKind::CheckBox))
    );
    assert!(kinds.iter().any(|kind| matches!(kind, ControlKind::Panel)));
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, ControlKind::ListBox))
    );
    assert!(kinds.iter().any(|kind| matches!(kind, ControlKind::Canvas)));
    assert!(kinds.iter().any(|kind| matches!(kind, ControlKind::Menu)));
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, ControlKind::Toolbar))
    );

    assert_eq!(slugify(" Hello, UI2!! "), "hello-ui2");
    assert_eq!(slugify("RADS_pack.auto"), "rads-pack-auto");
    assert_eq!(
        validate_project_name("../escape").unwrap_err(),
        ProjectNameError::ContainsPathSeparator
    );
    assert_eq!(
        validate_project_name("  RADS Pack  ").unwrap().slug,
        "rads-pack"
    );
}

#[test]
fn checked_in_generated_example_matches_public_model_shapes() {
    let project: RadsProject = serde_json::from_str(include_str!(
        "../examples/generated-project/rads.project.json"
    ))
    .unwrap();
    let app: AppBlueprint = serde_json::from_str(include_str!(
        "../examples/generated-project/app.blueprint.json"
    ))
    .unwrap();
    let package: PackageBlueprint = serde_json::from_str(include_str!(
        "../examples/generated-project/package/package.blueprint.json"
    ))
    .unwrap();
    let window: Ui2Window = serde_json::from_str(include_str!(
        "../examples/generated-project/ui/main.ui2.json"
    ))
    .unwrap();

    assert_eq!(project.slug, "hello-ui2");
    assert_eq!(app.ui_layout, "ui/main.ui2");
    assert_eq!(package.package_id, "dev.trueos.hello-ui2.package");
    assert_eq!(window.controls.len(), 4);

    let layout = include_str!("../examples/generated-project/ui/main.ui2");
    assert!(layout.contains("textbox textbox1"));
    assert!(layout.contains("change -> on_textbox1_change"));
}
