use crate::model::{ControlKind, RadsProject, Ui2Control, Ui2Window, slugify};
use crate::project_templates::{self, ProjectTemplate};
use std::collections::BTreeSet;
use std::path::Path;

pub fn cargo_toml(project: &RadsProject) -> String {
    format!(
        r#"[package]
name = "{}"
version = "{}"
edition = "2024"
description = "{}"
license = "{}"

[dependencies]
v = {{ path = "{}" }}
"#,
        project.slug,
        project.blueprint.version,
        escape_toml_string(&project.blueprint.description),
        escape_toml_string(&project.blueprint.license),
        escape_toml_string(&trueos_v_dependency_path())
    )
}

pub fn main_rs(project: &RadsProject) -> String {
    main_rs_for_template(
        project,
        project_templates::default_project_template_for_kind(project.app_kind),
    )
}

pub fn main_rs_for_template(project: &RadsProject, template: &ProjectTemplate) -> String {
    render_project_tokens(
        template.starter_main_rs,
        project,
        template,
        escape_rust_string,
    )
}

pub fn ui_rs(project: &RadsProject) -> String {
    if project.windows.is_empty() {
        return r#"use v::vui2;

pub const APP_ID: &str = "";
pub const APP_DISPLAY_NAME: &str = "";

pub fn create_main_window() -> Option<vui2::OwnedWindow> {
    None
}

pub fn create_all_windows() -> Vec<vui2::OwnedWindow> {
    Vec::new()
}
"#
        .to_string();
    }

    let constants = project
        .windows
        .iter()
        .enumerate()
        .map(|(index, window)| {
            let stem = window_file_stem(window, index);
            let decorations = escape_rust_string(&window.decorations.to_ui2_literal());
            if index == 0 {
                format!(
                    r#"pub const WINDOW_{index}_MODEL: &str = include_str!("../ui/main.ui2.json");
pub const WINDOW_{index}_HTML: &str = include_str!("../ui/index.html");
pub const WINDOW_{index}_CSS: &str = include_str!("../ui/styles.css");
pub const WINDOW_{index}_DECORATIONS: &str = "{decorations}";
"#
                )
            } else {
                format!(
                    r#"pub const WINDOW_{index}_MODEL: &str = include_str!("../ui/windows/{stem}.ui2.json");
pub const WINDOW_{index}_HTML: &str = include_str!("../ui/windows/{stem}.html");
pub const WINDOW_{index}_CSS: &str = include_str!("../ui/windows/{stem}.css");
pub const WINDOW_{index}_DECORATIONS: &str = "{decorations}";
"#
                )
            }
        })
        .collect::<Vec<_>>()
        .join("");
    let functions = project
        .windows
        .iter()
        .enumerate()
        .map(|(index, window)| window_create_function(index, window))
        .collect::<Vec<_>>()
        .join("\n");
    let pushes = (0..project.windows.len())
        .map(|index| {
            format!(
                r#"    if let Some(window) = create_window_{index}() {{
        windows.push(window);
    }}
"#
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"use v::vui2;

pub const APP_ID: &str = "{}";
pub const APP_DISPLAY_NAME: &str = "{}";
pub const MAIN_LAYOUT: &str = include_str!("../ui/main.ui2");
pub const MAIN_HTML: &str = WINDOW_0_HTML;
pub const MAIN_CSS: &str = WINDOW_0_CSS;
pub const MAIN_WINDOW_DECORATIONS: &str = WINDOW_0_DECORATIONS;
{}

pub fn create_main_window() -> Option<vui2::OwnedWindow> {{
    create_window_0()
}}

{}

pub fn create_all_windows() -> Vec<vui2::OwnedWindow> {{
    let mut windows = Vec::new();
{}    windows
}}
"#,
        escape_rust_string(&project.blueprint.app_id),
        escape_rust_string(&project.blueprint.display_name),
        constants,
        functions,
        pushes
    )
}

fn window_create_function(index: usize, window: &Ui2Window) -> String {
    let decoration_options = window_decoration_options_literal(window);
    format!(
        r#"pub fn create_window_{index}() -> Option<vui2::OwnedWindow> {{
    let rect = vui2::Rect {{
        x: {},
        y: {},
        width: {},
        height: {},
    }};
    let options = vui2::CreateOptions {{
        decorations: {},
        ..vui2::CreateOptions::default()
    }};
    let window = vui2::OwnedWindow::create_with_options("{}", rect, options)?;
    let id = window.id();
    id.set_title("{}");
    Some(window)
}}
"#,
        window.geometry.x,
        window.geometry.y,
        window.geometry.w,
        window.geometry.h,
        decoration_options,
        escape_rust_string(&window.caption),
        escape_rust_string(&window.caption)
    )
}

fn window_decoration_options_literal(window: &Ui2Window) -> String {
    let decorations = &window.decorations;
    let options = &window.options;
    format!(
        r#"vui2::WindowDecorationOptions {{
            mode: vui2::WindowDecorationMode::{},
            titlebar_visible: {},
            bottom_bar_visible: {},
            title_icon_visible: {},
            buttons: vui2::WindowDecorationButtons {{
                toggle_composition: {},
                fork: {},
                minimize: {},
                restore: {},
                toggle_maximize: {},
                preserve_vm: {},
                close: {},
            }},
            resize_button_visible: {},
            rotate_buttons_visible: {},
            vertical_scrollbar_visible: {},
            horizontal_scrollbar_visible: {},
            vertical_scrollbar_side: vui2::VerticalScrollbarSide::{},
            horizontal_scrollbar_side: vui2::HorizontalScrollbarSide::{},
            resize_mode: vui2::WindowResizeMode::Auto,
            resize_maintain_aspect: false,
            content_preserve_scale: {},
        }}"#,
        decorations.mode.vui2_variant(),
        decorations.titlebar,
        decorations.bottom_bar,
        decorations.title_icon,
        decorations.toggle_composition,
        decorations.fork,
        decorations.minimize,
        decorations.restore,
        decorations.maximize,
        decorations.preserve_vm,
        decorations.close,
        decorations.resizable && decorations.resize_button,
        decorations.rotate_buttons,
        options.scrollbars.vertical_visible(),
        options.scrollbars.horizontal_visible(),
        options.vertical_scrollbar_side.vui2_variant(),
        options.horizontal_scrollbar_side.vui2_variant(),
        options.preserve_scale,
    )
}

pub fn events_rs(project: &RadsProject) -> String {
    events_rs_for_template(
        project,
        project_templates::default_project_template_for_kind(project.app_kind),
    )
}

pub fn events_rs_for_template(project: &RadsProject, template: &ProjectTemplate) -> String {
    let mut body = render_project_tokens(
        template.starter_events_rs,
        project,
        template,
        escape_rust_string,
    );
    if !body.ends_with('\n') {
        body.push('\n');
    }
    let mut seen = BTreeSet::new();
    for window in &project.windows {
        for control in &window.controls {
            for event in &control.events {
                if seen.insert(event.handler.clone()) {
                    body.push_str(&format!(
                        r#"
pub fn {}() {{
    v::vshell::line("{} fired on {}");
}}
"#,
                        event.handler,
                        escape_rust_string(&event.event),
                        escape_rust_string(&control.name)
                    ));
                }
            }
        }
    }
    body
}

pub fn html(project: &RadsProject, template: &ProjectTemplate) -> String {
    project
        .windows
        .first()
        .map(|window| html_for_window(project, template, window))
        .unwrap_or_default()
}

pub fn css(project: &RadsProject, template: &ProjectTemplate) -> String {
    project
        .windows
        .first()
        .map(|window| css_for_window(project, template, window))
        .unwrap_or_default()
}

pub fn html_for_window(
    project: &RadsProject,
    template: &ProjectTemplate,
    window: &Ui2Window,
) -> String {
    if !window.ui_description.html.trim().is_empty() {
        return window.ui_description.html.clone();
    }
    render_project_tokens(template.initial_html, project, template, escape_html)
}

pub fn css_for_window(
    project: &RadsProject,
    template: &ProjectTemplate,
    window: &Ui2Window,
) -> String {
    if !window.ui_description.css.trim().is_empty() {
        return window.ui_description.css.clone();
    }
    render_project_tokens(template.initial_css, project, template, str::to_string)
}

pub fn package_manifest(project: &RadsProject) -> String {
    let mut blueprints = serde_json::json!({
        "app": "app.blueprint.json",
        "package": "package/package.blueprint.json"
    });
    if project.app_kind.has_ui2() && !project.windows.is_empty() {
        if let Some(blueprints) = blueprints.as_object_mut() {
            blueprints.insert("layout".to_string(), serde_json::json!("ui/main.ui2"));
            blueprints.insert("html".to_string(), serde_json::json!("ui/index.html"));
            blueprints.insert("styles".to_string(), serde_json::json!("ui/styles.css"));
        }
    }
    serde_json::to_string_pretty(&serde_json::json!({
        "schema": "trueos.package.manifest/v1",
        "app_id": project.blueprint.app_id,
        "package_id": project.package.package_id,
        "display_name": project.blueprint.display_name,
        "version": project.blueprint.version,
        "app_kind": project.app_kind,
        "entrypoint": project.package.entrypoint,
        "artifacts": project.package.artifacts,
        "blueprints": blueprints,
        "metadata": project.package.metadata
    }))
    .expect("package manifest template is serializable")
}

pub fn readme(project: &RadsProject) -> String {
    readme_for_template(
        project,
        project_templates::default_project_template_for_kind(project.app_kind),
    )
}

pub fn readme_for_template(project: &RadsProject, template: &ProjectTemplate) -> String {
    let ui_files = if project.app_kind.has_ui2() && !project.windows.is_empty() {
        r#"- `ui/main.ui2`: readable UI2 layout with serialized decorations and event bindings.
- `ui/main.ui2.json`: JSON copy of the main window model.
- `ui/windows/`: per-window JSON, HTML, and CSS files for secondary UI2 windows.
- `ui/index.html`: starter markup for the main window.
- `ui/styles.css`: starter stylesheet for the main window.
- `src/ui.rs`: UI2 window creation helper.
"#
    } else {
        ""
    };
    format!(
        r#"# {}

Generated with TRUEOS RADS.

Template: {} (`{}`)
App kind: {}

## Files

- `app.blueprint.json`: app metadata and capabilities.
- `package/package.blueprint.json`: package metadata and output artifacts.
{}- `src/main.rs`: app entrypoint.
- `src/events.rs`: generated event stubs.

## Run

```sh
cargo check
```
"#,
        project.name,
        template.name,
        template.id,
        project.app_kind.label(),
        ui_files
    )
}

pub fn ui2_layout(project: &RadsProject) -> String {
    let mut body = String::new();
    body.push_str(&format!(
        "app {:?} id {:?} version {:?}\n",
        project.blueprint.display_name, project.blueprint.app_id, project.blueprint.version
    ));
    for window in &project.windows {
        body.push('\n');
        body.push_str(&window_to_ui2_snippet(window));
    }
    body
}

pub fn window_to_ui2_snippet(window: &Ui2Window) -> String {
    let mut body = String::new();
    body.push_str(&format!(
        "window {} caption {:?} at {},{} size {}x{} {{\n",
        window.name,
        window.caption,
        window.geometry.x,
        window.geometry.y,
        window.geometry.w,
        window.geometry.h
    ));
    body.push_str(&format!(
        "  decorations {}\n",
        window.decorations.to_ui2_literal()
    ));
    body.push_str(&format!(
        "  window-options {{ resize_mode: {}, scrollbars: {}, vertical_scrollbar_side: {}, horizontal_scrollbar_side: {}, preserve_scale: {} }}\n",
        window.options.resize_mode.as_str(),
        window.options.scrollbars.as_str(),
        window.options.vertical_scrollbar_side.as_str(),
        window.options.horizontal_scrollbar_side.as_str(),
        window.options.preserve_scale
    ));
    if let Some(icon) = window
        .title_twemoji
        .as_deref()
        .filter(|icon| !icon.is_empty())
    {
        body.push_str(&format!("  title-twemoji {:?}\n", icon));
    }
    let flags = window.decorations.to_flags().join(", ");
    body.push_str(&format!("  decoration-flags [{}]\n", flags));
    body.push_str("  layout absolute grid 8\n");
    for control in &window.controls {
        body.push_str(&format!("  {}\n", control_to_ui2_snippet(control)));
    }
    body.push_str("}\n");
    body
}

pub fn control_to_ui2_snippet(control: &Ui2Control) -> String {
    let properties = control
        .properties
        .iter()
        .map(|property| format!("{}={:?}", property.key, property.value))
        .collect::<Vec<_>>()
        .join(", ");
    let events = control
        .events
        .iter()
        .map(|event| format!("{} -> {}", event.event, event.handler))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{} {} at {},{} size {}x{} caption {:?} props [{}] events [{}]",
        control_kind_name(&control.kind),
        control.name,
        control.geometry.x,
        control.geometry.y,
        control.geometry.w,
        control.geometry.h,
        control.caption,
        properties,
        events
    )
}

fn control_kind_name(kind: &ControlKind) -> &'static str {
    match kind {
        ControlKind::Button => "button",
        ControlKind::Label => "label",
        ControlKind::TextBox => "textbox",
        ControlKind::CheckBox => "checkbox",
        ControlKind::Panel => "panel",
        ControlKind::ListBox => "listbox",
        ControlKind::Canvas => "canvas",
        ControlKind::Menu => "menu",
        ControlKind::Toolbar => "toolbar",
    }
}

fn escape_rust_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn escape_toml_string(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn trueos_v_dependency_path() -> String {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    if let Some(path) = manifest_dir
        .parent()
        .map(|parent| parent.join("TRUEOS/crates/trueos-v"))
        .filter(|path| path.exists())
    {
        return path.to_string_lossy().into_owned();
    }
    "../../TRUEOS/crates/trueos-v".to_string()
}

fn window_file_stem(window: &Ui2Window, index: usize) -> String {
    if index == 0 {
        return "main".to_string();
    }
    let stem = slugify(&window.name);
    if stem.is_empty() {
        format!("window-{}", index + 1)
    } else {
        stem
    }
}

fn render_project_tokens(
    input: &str,
    project: &RadsProject,
    template: &ProjectTemplate,
    escape: fn(&str) -> String,
) -> String {
    let window_caption = project
        .windows
        .first()
        .map(|window| window.caption.as_str())
        .unwrap_or(project.name.as_str());
    input
        .replace(
            "{{APP_DISPLAY_NAME}}",
            &escape(&project.blueprint.display_name),
        )
        .replace("{{APP_ID}}", &escape(&project.blueprint.app_id))
        .replace("{{PROJECT_SLUG}}", &escape(&project.slug))
        .replace("{{TEMPLATE_ID}}", &escape(template.id))
        .replace("{{TEMPLATE_NAME}}", &escape(template.name))
        .replace("{{WINDOW_CAPTION}}", &escape(window_caption))
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
