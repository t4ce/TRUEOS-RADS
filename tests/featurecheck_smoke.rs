use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::Value;
use trueos_rads::designer::{
    AddControlRequest, SnapSettings, add_control, default_palette, default_palette_categories,
    object_inspector_for_selection,
};
use trueos_rads::generator::create_project;
use trueos_rads::jobs::{JobKind, JobManager, JobStatus};
use trueos_rads::model::{ControlKind, RadsProject, Ui2Window};
use trueos_rads::templates::{control_to_ui2_snippet, ui2_layout};

const GENERATED_FILES: &[&str] = &[
    "rads.project.json",
    "app.blueprint.json",
    "package/package.blueprint.json",
    "ui/main.ui2",
    "ui/main.ui2.json",
    "ui/index.html",
    "ui/styles.css",
    "Cargo.toml",
    "src/main.rs",
    "src/ui.rs",
    "src/events.rs",
    "package/manifest.trueos.json",
    "README.md",
];

#[test]
fn featurecheck_docs_cover_pass_g_topics_and_a_through_g() {
    let matrix = include_str!("../docs/featurecheck-matrix.md");
    for id in ["A", "B", "C", "D", "E", "F", "G"] {
        assert!(
            matrix.contains(&format!("| {id} |")),
            "missing featurecheck row {id}"
        );
    }
    for topic in [
        "Template creation",
        "HTML/CSS UI2 shell description",
        "Preview, editor, and code tabs",
        "Local featurecheck APIs",
        "BP DIST stages",
    ] {
        assert!(matrix.contains(topic), "matrix missing topic: {topic}");
    }

    let template_doc = include_str!("../docs/template-creation.md");
    for file in GENERATED_FILES {
        assert!(
            template_doc.contains(file),
            "template doc missing generated file contract for {file}"
        );
    }

    let ui_doc = include_str!("../docs/html-css-ui2-description.md");
    for term in [
        "Preview",
        "Editor",
        "Code",
        ".control.text-box",
        ".design-stage",
        "ui/main.ui2",
        "ui/index.html",
        "ui/styles.css",
        "src/events.rs",
        "/api/project/file",
    ] {
        assert!(ui_doc.contains(term), "UI2 HTML/CSS doc missing {term}");
    }

    let api_doc = include_str!("../docs/api-sketch.md");
    for term in ["/api/templates", "/api/project/files", "/api/project/file"] {
        assert!(api_doc.contains(term), "API sketch missing {term}");
    }

    let bp_doc = include_str!("../docs/bp-dist-stages.md");
    for stage in 0..=7 {
        assert!(
            bp_doc.contains(&format!("BP DIST {stage}")),
            "BP DIST doc missing stage {stage}"
        );
    }

    let smoke = include_str!("../scripts/smoke.sh");
    assert!(smoke.contains("cargo fmt --check"));
    assert!(smoke.contains("cargo test"));
}

#[test]
fn template_creation_doc_matches_generated_project_contract() {
    let dir = tempfile::tempdir().unwrap();
    let project = create_project(dir.path(), "Featurecheck Template").unwrap();

    for file in GENERATED_FILES {
        assert!(
            project.root.join(file).exists(),
            "generated project missing {file}"
        );
    }

    let stored_project: RadsProject =
        serde_json::from_str(&fs::read_to_string(project.root.join("rads.project.json")).unwrap())
            .unwrap();
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(project.root.join("package/manifest.trueos.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(stored_project.slug, "featurecheck-template");
    assert_eq!(
        stored_project.blueprint.app_id,
        stored_project.package.app_id
    );
    assert_eq!(
        manifest["blueprints"]["app"].as_str().unwrap(),
        "app.blueprint.json"
    );
    assert_eq!(
        manifest["blueprints"]["package"].as_str().unwrap(),
        "package/package.blueprint.json"
    );
    assert_eq!(
        manifest["blueprints"]["layout"].as_str().unwrap(),
        "ui/main.ui2"
    );

    let readme = fs::read_to_string(project.root.join("README.md")).unwrap();
    let ui_rs = fs::read_to_string(project.root.join("src/ui.rs")).unwrap();
    let html = fs::read_to_string(project.root.join("ui/index.html")).unwrap();
    let css = fs::read_to_string(project.root.join("ui/styles.css")).unwrap();
    assert!(readme.contains("cargo check"));
    assert!(ui_rs.contains("pub const MAIN_LAYOUT"));
    assert!(ui_rs.contains("pub const MAIN_WINDOW_DECORATIONS"));
    assert!(html.contains("data-app-id=\"dev.trueos.featurecheck-template\""));
    assert!(html.contains("styles.css"));
    assert!(css.contains("font-family"));
}

#[test]
fn checked_in_example_cross_references_blueprints_manifest_and_layout() {
    let project: Value = serde_json::from_str(include_str!(
        "../examples/generated-project/rads.project.json"
    ))
    .unwrap();
    let app: Value = serde_json::from_str(include_str!(
        "../examples/generated-project/app.blueprint.json"
    ))
    .unwrap();
    let package: Value = serde_json::from_str(include_str!(
        "../examples/generated-project/package/package.blueprint.json"
    ))
    .unwrap();
    let manifest: Value = serde_json::from_str(include_str!(
        "../examples/generated-project/package/manifest.trueos.json"
    ))
    .unwrap();
    let window: Value = serde_json::from_str(include_str!(
        "../examples/generated-project/ui/main.ui2.json"
    ))
    .unwrap();
    let layout = include_str!("../examples/generated-project/ui/main.ui2");
    let html = include_str!("../examples/generated-project/ui/index.html");
    let css = include_str!("../examples/generated-project/ui/styles.css");

    assert_eq!(project["blueprint"]["app_id"], app["app_id"]);
    assert_eq!(project["package"]["package_id"], package["package_id"]);
    assert_eq!(app["app_id"], package["app_id"]);
    assert_eq!(app["app_id"], manifest["app_id"]);
    assert_eq!(package["package_id"], manifest["package_id"]);
    assert_eq!(
        manifest["blueprints"]["app"].as_str().unwrap(),
        "app.blueprint.json"
    );
    assert_eq!(
        manifest["blueprints"]["package"].as_str().unwrap(),
        "package/package.blueprint.json"
    );
    assert_eq!(
        manifest["blueprints"]["layout"].as_str().unwrap(),
        "ui/main.ui2"
    );

    let capabilities = app["capabilities"].as_array().unwrap();
    assert!(capabilities.iter().any(|cap| cap["key"] == "ui2.window"));
    assert!(capabilities.iter().any(|cap| cap["key"] == "ui2.events"));
    assert!(capabilities.iter().any(|cap| cap["key"] == "fs.user"));
    assert!(capabilities.iter().any(|cap| cap["key"] == "net.client"));

    let artifacts = package["artifacts"].as_array().unwrap();
    for path in ["ui/main.ui2", "ui/index.html", "ui/styles.css"] {
        assert!(
            artifacts.iter().any(|artifact| artifact["path"] == path),
            "package blueprint missing artifact {path}"
        );
        assert!(
            manifest["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .any(|artifact| artifact["path"] == path),
            "manifest missing artifact {path}"
        );
    }

    let controls = window["controls"].as_array().unwrap();
    assert_eq!(controls.len(), 4);
    for control in controls {
        let name = control["name"].as_str().unwrap();
        let handler = control["events"][0]["handler"].as_str().unwrap();
        assert!(layout.contains(name), "layout missing control {name}");
        assert!(layout.contains(handler), "layout missing handler {handler}");
    }

    assert_eq!(window["options"]["resize_mode"], "both");
    assert!(
        window["ui_description"]["html"]
            .as_str()
            .unwrap()
            .contains("ui2-window")
    );
    assert!(html.contains("data-template=\"form-app\""));
    assert!(html.contains("name=\"runButton\""));
    assert!(css.contains(".form-shell"));
}

#[test]
fn public_featurecheck_apis_render_designer_and_ui2_shapes() {
    let categories = default_palette_categories();
    assert_eq!(
        categories
            .iter()
            .map(|category| category.name.as_str())
            .collect::<Vec<_>>(),
        ["Standard", "Containers", "Data", "Navigation"]
    );
    assert_eq!(default_palette().len(), 9);

    let mut window = Ui2Window::main_window("Featurecheck APIs");
    let window_id = window.id;
    let control_id = add_control(
        &mut window,
        AddControlRequest {
            window_id,
            kind: ControlKind::Canvas,
            x: 101,
            y: 103,
            id: None,
            name: None,
            caption: Some("Drawing".to_string()),
            snap: Some(SnapSettings {
                enabled: true,
                grid: 10,
            }),
        },
    )
    .unwrap();
    let control = window
        .controls
        .iter()
        .find(|control| control.id == control_id)
        .unwrap();
    assert_eq!(control.name, "canvas1");
    assert_eq!(control.geometry.x, 100);
    assert_eq!(control.geometry.y, 100);
    assert!(control.events.iter().any(|event| event.event == "draw"));

    let snippet = control_to_ui2_snippet(control);
    assert!(snippet.contains("canvas canvas1 at 100,100"));
    assert!(snippet.contains("draw -> on_canvas1_draw"));

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

    let project = RadsProject::starter("Featurecheck Layout", PathBuf::from("/tmp/rads-layout"));
    let layout = ui2_layout(&project);
    assert!(layout.contains("app \"Featurecheck Layout\""));
    assert!(layout.contains("window MainWindow"));
    assert!(layout.contains("decoration-flags [titlebar, close, minimize, maximize, resizable]"));
    assert!(layout.contains("textbox inputText"));
}

#[tokio::test]
async fn public_job_manager_pack_api_emits_bp_dist_stage_lines() {
    let manager = JobManager::new();
    let mut events = manager.subscribe();
    let dir = tempfile::tempdir().unwrap();
    let project = trueos_rads::generator::create_project(dir.path(), "Pack Smoke").unwrap();
    let job_id = manager
        .spawn(JobKind::Pack {
            project: project.root.clone(),
        })
        .await;

    let mut seen = Vec::new();
    let mut passed = false;
    for _ in 0..16 {
        let event = tokio::time::timeout(Duration::from_secs(2), events.recv())
            .await
            .expect("timed out waiting for pack job event")
            .expect("job event channel closed");
        if event.job_id != job_id {
            continue;
        }
        if matches!(event.status, JobStatus::Passed) {
            passed = true;
        }
        seen.push(event.line);
        if passed {
            break;
        }
    }

    assert!(passed, "pack job did not finish: {seen:?}");
    for line in [
        "queued",
        "started",
        "validating blueprint",
        "collecting UI2 layouts and assets",
        "writing package plan",
        "finished",
    ] {
        assert!(
            seen.iter().any(|seen_line| seen_line.contains(line)),
            "pack job did not emit {line}: {seen:?}"
        );
    }

    let jobs = manager.list().await;
    assert_eq!(jobs.len(), 1);
    assert!(matches!(jobs[0].status, JobStatus::Passed));
}
