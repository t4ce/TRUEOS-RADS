use crate::model::{
    AppBlueprint, AppKind, BlueprintMetadata, Capability, ControlKind, EventBinding,
    PackageArtifact, PackageBlueprint, Property, RadsProject, Rect, Ui2Control, Ui2Window,
    ValidProjectName, WindowDecorationMode, WindowDecorations,
};
use crate::ui2_options::{
    Ui2HorizontalScrollbarSide, Ui2HtmlCssDescription, Ui2ResizeMode, Ui2ScrollbarMode, Ui2Size,
    Ui2VerticalScrollbarSide, Ui2WindowOptions,
};
use std::path::PathBuf;
use uuid::Uuid;

pub const BLANK_UI2_TEMPLATE_ID: &str = "blank-ui2";
pub const FORM_APP_TEMPLATE_ID: &str = "form-app";
pub const CANVAS_APP_TEMPLATE_ID: &str = "canvas-app";
pub const TOOL_WINDOW_TEMPLATE_ID: &str = "tool-window";
pub const SERVICE_APP_TEMPLATE_ID: &str = "service-app";
pub const SHELL_APP_TEMPLATE_ID: &str = "shell-app";
pub const DEFAULT_PROJECT_TEMPLATE_ID: &str = FORM_APP_TEMPLATE_ID;

#[derive(Debug)]
pub struct ProjectTemplate {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub app_kind: AppKind,
    pub initial_html: &'static str,
    pub initial_css: &'static str,
    pub starter_main_rs: &'static str,
    pub starter_events_rs: &'static str,
    pub window: Option<TemplateWindow>,
    pub capabilities: &'static [TemplateCapability],
}

#[derive(Debug)]
pub struct TemplateWindow {
    pub name: &'static str,
    pub caption: &'static str,
    pub geometry: Rect,
    pub decorations: WindowDecorations,
    pub options: Ui2WindowOptions,
    pub controls: &'static [TemplateControl],
}

#[derive(Debug)]
pub struct TemplateControl {
    pub kind: ControlKind,
    pub name: &'static str,
    pub caption: &'static str,
    pub geometry: Rect,
    pub properties: &'static [TemplateProperty],
    pub events: &'static [TemplateEvent],
}

#[derive(Debug)]
pub struct TemplateProperty {
    pub key: &'static str,
    pub value: &'static str,
}

#[derive(Debug)]
pub struct TemplateEvent {
    pub event: &'static str,
    pub handler: &'static str,
}

#[derive(Debug)]
pub struct TemplateCapability {
    pub key: &'static str,
    pub enabled: bool,
    pub note: &'static str,
}

pub fn available_project_templates() -> &'static [ProjectTemplate] {
    &PROJECT_TEMPLATES
}

pub fn default_project_template() -> &'static ProjectTemplate {
    find_project_template(DEFAULT_PROJECT_TEMPLATE_ID).expect("default project template must exist")
}

pub fn default_project_template_for_kind(app_kind: AppKind) -> &'static ProjectTemplate {
    let template_id = match app_kind {
        AppKind::Ui2 => DEFAULT_PROJECT_TEMPLATE_ID,
        AppKind::Service => SERVICE_APP_TEMPLATE_ID,
        AppKind::Shell => SHELL_APP_TEMPLATE_ID,
    };
    find_project_template(template_id).expect("default project template for app kind must exist")
}

pub fn find_project_template(id: &str) -> Option<&'static ProjectTemplate> {
    PROJECT_TEMPLATES.iter().find(|template| template.id == id)
}

impl ProjectTemplate {
    pub fn build_project(&self, name: ValidProjectName, root: impl Into<PathBuf>) -> RadsProject {
        let metadata = BlueprintMetadata {
            generator: "trueos-rads".to_string(),
            generator_version: env!("CARGO_PKG_VERSION").to_string(),
            ui_runtime: self.app_kind.runtime().to_string(),
            schema_version: "0.1".to_string(),
        };
        let app_id = format!("dev.trueos.{}", name.slug);
        let package_id = format!("{app_id}.package");
        let windows = self
            .window
            .as_ref()
            .map(|window| self.build_window(window, &name, &app_id))
            .into_iter()
            .collect::<Vec<_>>();
        let has_ui = self.app_kind.has_ui2() && !windows.is_empty();

        RadsProject {
            id: Uuid::new_v4(),
            root: root.into(),
            app_kind: self.app_kind,
            blueprint: AppBlueprint {
                schema: "trueos.app.blueprint/v1".to_string(),
                app_id: app_id.clone(),
                slug: name.slug.clone(),
                display_name: name.display.clone(),
                version: "0.1.0".to_string(),
                entrypoint: "src/main.rs".to_string(),
                ui_layout: if has_ui { "ui/main.ui2" } else { "" }.to_string(),
                description: format!(
                    "{} generated from the {} {} template.",
                    name.display,
                    self.app_kind.label(),
                    self.name
                ),
                license: "MIT OR Apache-2.0".to_string(),
                authors: vec!["TRUEOS RADS".to_string()],
                capabilities: self
                    .capabilities
                    .iter()
                    .map(TemplateCapability::to_capability)
                    .collect(),
                metadata: metadata.clone(),
            },
            package: PackageBlueprint {
                schema: "trueos.package.blueprint/v1".to_string(),
                package_id,
                app_id,
                name: name.slug.clone(),
                version: "0.1.0".to_string(),
                entrypoint: "src/main.rs".to_string(),
                artifacts: package_artifacts(self.app_kind, has_ui),
                metadata,
            },
            name: name.display.clone(),
            slug: name.slug.clone(),
            windows,
        }
    }

    fn build_window(
        &self,
        template_window: &TemplateWindow,
        name: &ValidProjectName,
        app_id: &str,
    ) -> Ui2Window {
        Ui2Window {
            id: Uuid::new_v4(),
            name: template_window.name.to_string(),
            caption: render_template_text(template_window.caption, name, app_id, self),
            title_twemoji: None,
            geometry: template_window.geometry,
            decorations: template_window.decorations.clone(),
            options: template_window.options.clone(),
            ui_description: Ui2HtmlCssDescription {
                html: render_template_html(self.initial_html, name, app_id, self),
                css: render_template_text(self.initial_css, name, app_id, self),
            },
            controls: template_window
                .controls
                .iter()
                .map(|control| control.to_control(name, app_id, self))
                .collect(),
        }
    }
}

fn package_artifacts(app_kind: AppKind, has_ui: bool) -> Vec<PackageArtifact> {
    let mut artifacts = vec![PackageArtifact {
        kind: "binary".to_string(),
        path: "target/trueos/app.tapp".to_string(),
        target: app_kind.package_target().to_string(),
    }];
    if has_ui {
        artifacts.extend([
            PackageArtifact {
                kind: "layout".to_string(),
                path: "ui/main.ui2".to_string(),
                target: "ui2-layout".to_string(),
            },
            PackageArtifact {
                kind: "html".to_string(),
                path: "ui/index.html".to_string(),
                target: "ui2-markup".to_string(),
            },
            PackageArtifact {
                kind: "stylesheet".to_string(),
                path: "ui/styles.css".to_string(),
                target: "ui2-style".to_string(),
            },
        ]);
    }
    artifacts
}

impl TemplateControl {
    fn to_control(
        &self,
        name: &ValidProjectName,
        app_id: &str,
        template: &ProjectTemplate,
    ) -> Ui2Control {
        let mut control = Ui2Control::new(
            self.kind.clone(),
            self.name,
            render_template_text(self.caption, name, app_id, template),
            self.geometry.x,
            self.geometry.y,
            self.geometry.w,
            self.geometry.h,
        );
        if !self.properties.is_empty() {
            control.properties = self
                .properties
                .iter()
                .map(|property| Property {
                    key: property.key.to_string(),
                    value: render_template_text(property.value, name, app_id, template),
                })
                .collect();
        }
        if !self.events.is_empty() {
            control.events = self
                .events
                .iter()
                .map(|event| EventBinding {
                    event: event.event.to_string(),
                    handler: render_template_text(event.handler, name, app_id, template),
                })
                .collect();
        }
        control
    }
}

impl TemplateCapability {
    fn to_capability(&self) -> Capability {
        Capability {
            key: self.key.to_string(),
            enabled: self.enabled,
            note: self.note.to_string(),
        }
    }
}

fn render_template_text(
    input: &str,
    name: &ValidProjectName,
    app_id: &str,
    template: &ProjectTemplate,
) -> String {
    input
        .replace("{{APP_DISPLAY_NAME}}", &name.display)
        .replace("{{APP_ID}}", app_id)
        .replace("{{PROJECT_SLUG}}", &name.slug)
        .replace("{{TEMPLATE_ID}}", template.id)
        .replace("{{TEMPLATE_NAME}}", template.name)
}

fn render_template_html(
    input: &str,
    name: &ValidProjectName,
    app_id: &str,
    template: &ProjectTemplate,
) -> String {
    input
        .replace("{{APP_DISPLAY_NAME}}", &escape_html(&name.display))
        .replace("{{APP_ID}}", &escape_html(app_id))
        .replace("{{PROJECT_SLUG}}", &escape_html(&name.slug))
        .replace("{{TEMPLATE_ID}}", &escape_html(template.id))
        .replace("{{TEMPLATE_NAME}}", &escape_html(template.name))
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#039;")
}

const COMMON_MAIN_RS: &str = r#"mod events;
mod ui;

fn main() {
    let windows = ui::create_all_windows();
    if windows.is_empty() {
        v::vshell::line("failed to create UI2 windows");
        return;
    }

    events::wire_main_window();
    v::vshell::line("started {{WINDOW_CAPTION}}");

    let _windows = windows;
}
"#;

const BLANK_EVENTS_RS: &str = r#"pub fn wire_main_window() {
    v::vshell::line("blank UI2 window ready");
}
"#;

const FORM_EVENTS_RS: &str = r#"pub fn wire_main_window() {
    v::vshell::line("UI2 event stubs registered");
}
"#;

const CANVAS_EVENTS_RS: &str = r#"pub fn wire_main_window() {
    v::vshell::line("canvas app event stubs registered");
}
"#;

const TOOL_EVENTS_RS: &str = r#"pub fn wire_main_window() {
    v::vshell::line("tool window event stubs registered");
}
"#;

const NO_UI_HTML: &str = "";
const NO_UI_CSS: &str = "";

const SERVICE_MAIN_RS: &str = r#"fn main() {
    v::vshell::line("service {{APP_DISPLAY_NAME}} starting");
    v::vshell::line("background service loop is ready to wire to TRUEOS tasks");
}
"#;

const SERVICE_EVENTS_RS: &str = r#"pub fn register_service_handlers() {
    v::vshell::line("service handlers registered");
}
"#;

const SHELL_MAIN_RS: &str = r#"fn main() {
    v::vshell::line("{{APP_DISPLAY_NAME}} shell ready");
    v::vshell::line("wire commands here as the shell surface grows");
}
"#;

const SHELL_EVENTS_RS: &str = r#"pub fn register_shell_commands() {
    v::vshell::line("shell command table registered");
}
"#;

const BLANK_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{{APP_DISPLAY_NAME}}</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <main id="app" data-app-id="{{APP_ID}}" data-template="{{TEMPLATE_ID}}"></main>
</body>
</html>
"#;

const BLANK_CSS: &str = r#"html,
body {
  margin: 0;
  min-height: 100%;
  font-family: system-ui, sans-serif;
  background: #f7f8fa;
  color: #1e242c;
}

#app {
  min-height: 100vh;
}
"#;

const FORM_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{{APP_DISPLAY_NAME}}</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <main class="form-shell" data-app-id="{{APP_ID}}" data-template="{{TEMPLATE_ID}}">
    <h1>{{APP_DISPLAY_NAME}}</h1>
    <label>
      Input
      <input name="inputText" placeholder="Type here">
    </label>
    <button name="runButton" type="button">Click me</button>
  </main>
</body>
</html>
"#;

const FORM_CSS: &str = r#"html,
body {
  margin: 0;
  min-height: 100%;
  font-family: system-ui, sans-serif;
  background: #f3f6f8;
  color: #20262e;
}

.form-shell {
  display: grid;
  gap: 16px;
  max-width: 360px;
  padding: 32px;
}

label {
  display: grid;
  gap: 8px;
}

input,
button {
  font: inherit;
}
"#;

const CANVAS_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{{APP_DISPLAY_NAME}}</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <main class="canvas-shell" data-app-id="{{APP_ID}}" data-template="{{TEMPLATE_ID}}">
    <header>
      <h1>{{APP_DISPLAY_NAME}}</h1>
      <button name="clearButton" type="button">Clear</button>
    </header>
    <canvas name="drawingCanvas" width="720" height="420"></canvas>
  </main>
</body>
</html>
"#;

const CANVAS_CSS: &str = r#"html,
body {
  margin: 0;
  min-height: 100%;
  font-family: system-ui, sans-serif;
  background: #eef2f5;
  color: #17202a;
}

.canvas-shell {
  display: grid;
  gap: 14px;
  padding: 24px;
}

.canvas-shell header {
  align-items: center;
  display: flex;
  justify-content: space-between;
}

canvas {
  background: #ffffff;
  border: 1px solid #9aa8b5;
}
"#;

const TOOL_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{{APP_DISPLAY_NAME}}</title>
  <link rel="stylesheet" href="styles.css">
</head>
<body>
  <main class="tool-shell" data-app-id="{{APP_ID}}" data-template="{{TEMPLATE_ID}}">
    <nav aria-label="Tools">
      <button name="newButton" type="button">New</button>
      <button name="runButton" type="button">Run</button>
    </nav>
    <section>
      <h1>{{APP_DISPLAY_NAME}}</h1>
      <ul name="itemList">
        <li>Project</li>
        <li>Assets</li>
        <li>Build</li>
      </ul>
    </section>
  </main>
</body>
</html>
"#;

const TOOL_CSS: &str = r#"html,
body {
  margin: 0;
  min-height: 100%;
  font-family: system-ui, sans-serif;
  background: #f5f5f2;
  color: #222623;
}

.tool-shell {
  display: grid;
  grid-template-columns: 112px 1fr;
  min-height: 100vh;
}

nav {
  align-content: start;
  background: #2f463d;
  display: grid;
  gap: 8px;
  padding: 12px;
}

section {
  padding: 20px;
}
"#;

const BLANK_CONTROLS: [TemplateControl; 0] = [];

const FORM_CONTROLS: [TemplateControl; 3] = [
    TemplateControl {
        kind: ControlKind::Label,
        name: "titleLabel",
        caption: "TRUEOS UI2 app",
        geometry: Rect {
            x: 32,
            y: 34,
            w: 220,
            h: 28,
        },
        properties: &[],
        events: &[],
    },
    TemplateControl {
        kind: ControlKind::Button,
        name: "runButton",
        caption: "Click me",
        geometry: Rect {
            x: 32,
            y: 86,
            w: 128,
            h: 38,
        },
        properties: &[],
        events: &[],
    },
    TemplateControl {
        kind: ControlKind::TextBox,
        name: "inputText",
        caption: "Type here",
        geometry: Rect {
            x: 32,
            y: 144,
            w: 260,
            h: 34,
        },
        properties: &[],
        events: &[],
    },
];

const CANVAS_PROPERTIES: [TemplateProperty; 2] = [
    TemplateProperty {
        key: "surface",
        value: "software",
    },
    TemplateProperty {
        key: "background",
        value: "#ffffff",
    },
];

const CLEAR_BUTTON_PROPERTIES: [TemplateProperty; 1] = [TemplateProperty {
    key: "variant",
    value: "secondary",
}];

const CANVAS_CONTROLS: [TemplateControl; 3] = [
    TemplateControl {
        kind: ControlKind::Label,
        name: "titleLabel",
        caption: "Canvas workspace",
        geometry: Rect {
            x: 24,
            y: 24,
            w: 220,
            h: 28,
        },
        properties: &[],
        events: &[],
    },
    TemplateControl {
        kind: ControlKind::Button,
        name: "clearButton",
        caption: "Clear",
        geometry: Rect {
            x: 812,
            y: 22,
            w: 96,
            h: 34,
        },
        properties: &CLEAR_BUTTON_PROPERTIES,
        events: &[],
    },
    TemplateControl {
        kind: ControlKind::Canvas,
        name: "drawingCanvas",
        caption: "",
        geometry: Rect {
            x: 24,
            y: 72,
            w: 884,
            h: 520,
        },
        properties: &CANVAS_PROPERTIES,
        events: &[],
    },
];

const TOOLBAR_PROPERTIES: [TemplateProperty; 2] = [
    TemplateProperty {
        key: "dock",
        value: "top",
    },
    TemplateProperty {
        key: "items",
        value: "New,Run,Settings",
    },
];

const TOOL_LIST_PROPERTIES: [TemplateProperty; 1] = [TemplateProperty {
    key: "items",
    value: "Project,Assets,Build",
}];

const TOOL_CONTROLS: [TemplateControl; 4] = [
    TemplateControl {
        kind: ControlKind::Toolbar,
        name: "mainToolbar",
        caption: "Tools",
        geometry: Rect {
            x: 0,
            y: 0,
            w: 560,
            h: 38,
        },
        properties: &TOOLBAR_PROPERTIES,
        events: &[],
    },
    TemplateControl {
        kind: ControlKind::Label,
        name: "titleLabel",
        caption: "Tool window",
        geometry: Rect {
            x: 20,
            y: 58,
            w: 180,
            h: 24,
        },
        properties: &[],
        events: &[],
    },
    TemplateControl {
        kind: ControlKind::ListBox,
        name: "itemList",
        caption: "Items",
        geometry: Rect {
            x: 20,
            y: 96,
            w: 180,
            h: 212,
        },
        properties: &TOOL_LIST_PROPERTIES,
        events: &[],
    },
    TemplateControl {
        kind: ControlKind::Panel,
        name: "detailsPanel",
        caption: "Details",
        geometry: Rect {
            x: 224,
            y: 58,
            w: 316,
            h: 250,
        },
        properties: &[],
        events: &[],
    },
];

const BLANK_CAPABILITIES: [TemplateCapability; 4] = [
    TemplateCapability {
        key: "ui2.window",
        enabled: true,
        note: "Create and manage a UI2 window",
    },
    TemplateCapability {
        key: "ui2.events",
        enabled: false,
        note: "No starter controls require generated event handlers",
    },
    TemplateCapability {
        key: "fs.user",
        enabled: false,
        note: "Read and write user-selected files",
    },
    TemplateCapability {
        key: "net.client",
        enabled: false,
        note: "Open outbound network connections",
    },
];

const FORM_CAPABILITIES: [TemplateCapability; 4] = [
    TemplateCapability {
        key: "ui2.window",
        enabled: true,
        note: "Create and manage UI2 windows",
    },
    TemplateCapability {
        key: "ui2.events",
        enabled: true,
        note: "Bind generated UI2 event handlers",
    },
    TemplateCapability {
        key: "fs.user",
        enabled: false,
        note: "Read and write user-selected files",
    },
    TemplateCapability {
        key: "net.client",
        enabled: false,
        note: "Open outbound network connections",
    },
];

const CANVAS_CAPABILITIES: [TemplateCapability; 5] = [
    TemplateCapability {
        key: "ui2.window",
        enabled: true,
        note: "Create and manage UI2 windows",
    },
    TemplateCapability {
        key: "ui2.events",
        enabled: true,
        note: "Bind generated UI2 event handlers",
    },
    TemplateCapability {
        key: "ui2.canvas",
        enabled: true,
        note: "Draw to a UI2 canvas control",
    },
    TemplateCapability {
        key: "fs.user",
        enabled: false,
        note: "Read and write user-selected files",
    },
    TemplateCapability {
        key: "net.client",
        enabled: false,
        note: "Open outbound network connections",
    },
];

const TOOL_CAPABILITIES: [TemplateCapability; 5] = [
    TemplateCapability {
        key: "ui2.window",
        enabled: true,
        note: "Create and manage a compact UI2 tool window",
    },
    TemplateCapability {
        key: "ui2.events",
        enabled: true,
        note: "Bind generated UI2 event handlers",
    },
    TemplateCapability {
        key: "ui2.toolbar",
        enabled: true,
        note: "Use toolbar-style navigation controls",
    },
    TemplateCapability {
        key: "fs.user",
        enabled: false,
        note: "Read and write user-selected files",
    },
    TemplateCapability {
        key: "net.client",
        enabled: false,
        note: "Open outbound network connections",
    },
];

const SERVICE_CAPABILITIES: [TemplateCapability; 4] = [
    TemplateCapability {
        key: "service.background",
        enabled: true,
        note: "Run as a classic background service",
    },
    TemplateCapability {
        key: "service.events",
        enabled: true,
        note: "Register service lifecycle handlers",
    },
    TemplateCapability {
        key: "fs.user",
        enabled: false,
        note: "Read and write user-selected files",
    },
    TemplateCapability {
        key: "net.client",
        enabled: false,
        note: "Open outbound network connections",
    },
];

const SHELL_CAPABILITIES: [TemplateCapability; 4] = [
    TemplateCapability {
        key: "shell.commands",
        enabled: true,
        note: "Expose command-oriented shell entry points",
    },
    TemplateCapability {
        key: "shell.streams",
        enabled: true,
        note: "Write text streams through TRUEOS shell surfaces",
    },
    TemplateCapability {
        key: "fs.user",
        enabled: false,
        note: "Read and write user-selected files",
    },
    TemplateCapability {
        key: "net.client",
        enabled: false,
        note: "Open outbound network connections",
    },
];

static PROJECT_TEMPLATES: [ProjectTemplate; 6] = [
    ProjectTemplate {
        id: BLANK_UI2_TEMPLATE_ID,
        name: "Blank UI2",
        description: "An empty UI2 window with starter markup and styling.",
        app_kind: AppKind::Ui2,
        initial_html: BLANK_HTML,
        initial_css: BLANK_CSS,
        starter_main_rs: COMMON_MAIN_RS,
        starter_events_rs: BLANK_EVENTS_RS,
        window: Some(TemplateWindow {
            name: "MainWindow",
            caption: "{{APP_DISPLAY_NAME}}",
            geometry: Rect {
                x: 80,
                y: 80,
                w: 640,
                h: 420,
            },
            decorations: WindowDecorations {
                mode: WindowDecorationMode::System,
                titlebar: true,
                bottom_bar: true,
                title_icon: true,
                toggle_composition: true,
                fork: true,
                close: true,
                minimize: true,
                restore: true,
                maximize: true,
                preserve_vm: true,
                resizable: true,
                resize_button: true,
                rotate_buttons: false,
                always_on_top: false,
            },
            options: Ui2WindowOptions {
                min_size: Ui2Size::new(320, 240),
                max_size: None,
                resize_mode: Ui2ResizeMode::Both,
                scrollbars: Ui2ScrollbarMode::None,
                vertical_scrollbar_side: Ui2VerticalScrollbarSide::Left,
                horizontal_scrollbar_side: Ui2HorizontalScrollbarSide::Bottom,
                hit_test_visible: true,
                preserve_scale: false,
            },
            controls: &BLANK_CONTROLS,
        }),
        capabilities: &BLANK_CAPABILITIES,
    },
    ProjectTemplate {
        id: FORM_APP_TEMPLATE_ID,
        name: "Form App",
        description: "A small event-driven form with a label, button, and text input.",
        app_kind: AppKind::Ui2,
        initial_html: FORM_HTML,
        initial_css: FORM_CSS,
        starter_main_rs: COMMON_MAIN_RS,
        starter_events_rs: FORM_EVENTS_RS,
        window: Some(TemplateWindow {
            name: "MainWindow",
            caption: "{{APP_DISPLAY_NAME}}",
            geometry: Rect {
                x: 80,
                y: 80,
                w: 720,
                h: 460,
            },
            decorations: WindowDecorations {
                mode: WindowDecorationMode::System,
                titlebar: true,
                bottom_bar: true,
                title_icon: true,
                toggle_composition: true,
                fork: true,
                close: true,
                minimize: true,
                restore: true,
                maximize: true,
                preserve_vm: true,
                resizable: true,
                resize_button: true,
                rotate_buttons: false,
                always_on_top: false,
            },
            options: Ui2WindowOptions {
                min_size: Ui2Size::new(320, 240),
                max_size: None,
                resize_mode: Ui2ResizeMode::Both,
                scrollbars: Ui2ScrollbarMode::None,
                vertical_scrollbar_side: Ui2VerticalScrollbarSide::Left,
                horizontal_scrollbar_side: Ui2HorizontalScrollbarSide::Bottom,
                hit_test_visible: true,
                preserve_scale: false,
            },
            controls: &FORM_CONTROLS,
        }),
        capabilities: &FORM_CAPABILITIES,
    },
    ProjectTemplate {
        id: CANVAS_APP_TEMPLATE_ID,
        name: "Canvas App",
        description: "A drawing-oriented starter with a large canvas and clear action.",
        app_kind: AppKind::Ui2,
        initial_html: CANVAS_HTML,
        initial_css: CANVAS_CSS,
        starter_main_rs: COMMON_MAIN_RS,
        starter_events_rs: CANVAS_EVENTS_RS,
        window: Some(TemplateWindow {
            name: "MainWindow",
            caption: "{{APP_DISPLAY_NAME}}",
            geometry: Rect {
                x: 60,
                y: 60,
                w: 960,
                h: 640,
            },
            decorations: WindowDecorations {
                mode: WindowDecorationMode::System,
                titlebar: true,
                bottom_bar: true,
                title_icon: true,
                toggle_composition: true,
                fork: true,
                close: true,
                minimize: true,
                restore: true,
                maximize: true,
                preserve_vm: true,
                resizable: true,
                resize_button: true,
                rotate_buttons: false,
                always_on_top: false,
            },
            options: Ui2WindowOptions {
                min_size: Ui2Size::new(640, 420),
                max_size: None,
                resize_mode: Ui2ResizeMode::Both,
                scrollbars: Ui2ScrollbarMode::None,
                vertical_scrollbar_side: Ui2VerticalScrollbarSide::Left,
                horizontal_scrollbar_side: Ui2HorizontalScrollbarSide::Bottom,
                hit_test_visible: true,
                preserve_scale: true,
            },
            controls: &CANVAS_CONTROLS,
        }),
        capabilities: &CANVAS_CAPABILITIES,
    },
    ProjectTemplate {
        id: TOOL_WINDOW_TEMPLATE_ID,
        name: "Tool Window",
        description: "A compact utility window with toolbar, list, and detail panel.",
        app_kind: AppKind::Ui2,
        initial_html: TOOL_HTML,
        initial_css: TOOL_CSS,
        starter_main_rs: COMMON_MAIN_RS,
        starter_events_rs: TOOL_EVENTS_RS,
        window: Some(TemplateWindow {
            name: "MainWindow",
            caption: "{{APP_DISPLAY_NAME}}",
            geometry: Rect {
                x: 120,
                y: 120,
                w: 560,
                h: 360,
            },
            decorations: WindowDecorations {
                mode: WindowDecorationMode::System,
                titlebar: true,
                bottom_bar: true,
                title_icon: true,
                toggle_composition: true,
                fork: true,
                close: true,
                minimize: false,
                restore: false,
                maximize: false,
                preserve_vm: true,
                resizable: false,
                resize_button: false,
                rotate_buttons: false,
                always_on_top: true,
            },
            options: Ui2WindowOptions {
                min_size: Ui2Size::new(560, 360),
                max_size: Some(Ui2Size::new(560, 360)),
                resize_mode: Ui2ResizeMode::None,
                scrollbars: Ui2ScrollbarMode::Auto,
                vertical_scrollbar_side: Ui2VerticalScrollbarSide::Left,
                horizontal_scrollbar_side: Ui2HorizontalScrollbarSide::Bottom,
                hit_test_visible: true,
                preserve_scale: false,
            },
            controls: &TOOL_CONTROLS,
        }),
        capabilities: &TOOL_CAPABILITIES,
    },
    ProjectTemplate {
        id: SERVICE_APP_TEMPLATE_ID,
        name: "Service App",
        description: "A classic background service app with no UI2 window surface.",
        app_kind: AppKind::Service,
        initial_html: NO_UI_HTML,
        initial_css: NO_UI_CSS,
        starter_main_rs: SERVICE_MAIN_RS,
        starter_events_rs: SERVICE_EVENTS_RS,
        window: None,
        capabilities: &SERVICE_CAPABILITIES,
    },
    ProjectTemplate {
        id: SHELL_APP_TEMPLATE_ID,
        name: "Shell App",
        description: "A command-oriented shell app separated from UI2 window composition.",
        app_kind: AppKind::Shell,
        initial_html: NO_UI_HTML,
        initial_css: NO_UI_CSS,
        starter_main_rs: SHELL_MAIN_RS,
        starter_events_rs: SHELL_EVENTS_RS,
        window: None,
        capabilities: &SHELL_CAPABILITIES,
    },
];
