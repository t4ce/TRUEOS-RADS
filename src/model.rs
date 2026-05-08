use crate::ui2_options::{Ui2HtmlCssDescription, Ui2WindowOptions};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use uuid::Uuid;

pub const MAX_PROJECT_NAME_LEN: usize = 80;
pub const MAX_PROJECT_SLUG_LEN: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadsProject {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub root: PathBuf,
    #[serde(default)]
    pub app_kind: AppKind,
    pub blueprint: AppBlueprint,
    pub package: PackageBlueprint,
    pub windows: Vec<Ui2Window>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppKind {
    Ui2,
    Service,
    Shell,
}

impl AppKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ui2 => "UI2 App",
            Self::Service => "Background Service",
            Self::Shell => "Shell App",
        }
    }

    pub fn runtime(self) -> &'static str {
        match self {
            Self::Ui2 => "TRUEOS/UI2",
            Self::Service => "TRUEOS/service",
            Self::Shell => "TRUEOS/shell",
        }
    }

    pub fn package_target(self) -> &'static str {
        match self {
            Self::Ui2 => "trueos-ui2",
            Self::Service => "trueos-service",
            Self::Shell => "trueos-shell",
        }
    }

    pub fn has_ui2(self) -> bool {
        matches!(self, Self::Ui2)
    }
}

impl Default for AppKind {
    fn default() -> Self {
        Self::Ui2
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppBlueprint {
    pub schema: String,
    pub app_id: String,
    pub slug: String,
    pub display_name: String,
    pub version: String,
    pub entrypoint: String,
    pub ui_layout: String,
    pub description: String,
    pub license: String,
    pub authors: Vec<String>,
    pub capabilities: Vec<Capability>,
    pub metadata: BlueprintMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageBlueprint {
    pub schema: String,
    pub package_id: String,
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub entrypoint: String,
    pub artifacts: Vec<PackageArtifact>,
    pub metadata: BlueprintMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageArtifact {
    pub kind: String,
    pub path: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueprintMetadata {
    pub generator: String,
    pub generator_version: String,
    pub ui_runtime: String,
    pub schema_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub key: String,
    pub enabled: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ui2Window {
    pub id: Uuid,
    pub name: String,
    pub caption: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_twemoji: Option<String>,
    pub geometry: Rect,
    pub decorations: WindowDecorations,
    #[serde(default)]
    pub options: Ui2WindowOptions,
    #[serde(default)]
    pub ui_description: Ui2HtmlCssDescription,
    pub controls: Vec<Ui2Control>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowDecorations {
    pub titlebar: bool,
    pub close: bool,
    pub minimize: bool,
    pub maximize: bool,
    pub resizable: bool,
    pub always_on_top: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ui2Control {
    pub id: Uuid,
    pub kind: ControlKind,
    pub name: String,
    pub caption: String,
    pub geometry: Rect,
    pub properties: Vec<Property>,
    pub events: Vec<EventBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ControlKind {
    Button,
    Label,
    TextBox,
    CheckBox,
    Panel,
    ListBox,
    Canvas,
    Menu,
    Toolbar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBinding {
    pub event: String,
    pub handler: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidProjectName {
    pub display: String,
    pub slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectNameError {
    Empty,
    TooLong { max: usize },
    ContainsPathSeparator,
    ContainsControlCharacter,
    NoSlug,
    ReservedName,
}

impl fmt::Display for ProjectNameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "project name cannot be empty"),
            Self::TooLong { max } => {
                write!(f, "project name must be {max} characters or fewer")
            }
            Self::ContainsPathSeparator => {
                write!(f, "project name cannot contain path separators")
            }
            Self::ContainsControlCharacter => {
                write!(f, "project name cannot contain control characters")
            }
            Self::NoSlug => {
                write!(
                    f,
                    "project name must contain at least one ASCII letter or number"
                )
            }
            Self::ReservedName => write!(f, "project name is reserved by the filesystem"),
        }
    }
}

impl std::error::Error for ProjectNameError {}

impl RadsProject {
    pub fn starter(name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        let validated =
            validate_project_name(&name.into()).expect("starter project names must be valid");
        Self::from_valid_name(validated, root)
    }

    pub fn from_valid_name(name: ValidProjectName, root: impl Into<PathBuf>) -> Self {
        let metadata = BlueprintMetadata {
            generator: "trueos-rads".to_string(),
            generator_version: env!("CARGO_PKG_VERSION").to_string(),
            ui_runtime: AppKind::Ui2.runtime().to_string(),
            schema_version: "0.1".to_string(),
        };
        let app_id = format!("dev.trueos.{}", name.slug);
        let package_id = format!("{app_id}.package");
        let mut main_window = Ui2Window::main_window(name.display.clone());
        main_window.controls.push(Ui2Control::new(
            ControlKind::TextBox,
            "inputText",
            "Type here",
            32,
            144,
            260,
            34,
        ));

        Self {
            id: Uuid::new_v4(),
            root: root.into(),
            app_kind: AppKind::Ui2,
            blueprint: AppBlueprint {
                schema: "trueos.app.blueprint/v1".to_string(),
                app_id: app_id.clone(),
                slug: name.slug.clone(),
                display_name: name.display.clone(),
                version: "0.1.0".to_string(),
                entrypoint: "src/main.rs".to_string(),
                ui_layout: "ui/main.ui2".to_string(),
                description: format!("{} generated with TRUEOS RADS.", name.display),
                license: "MIT OR Apache-2.0".to_string(),
                authors: vec!["TRUEOS RADS".to_string()],
                capabilities: vec![
                    Capability {
                        key: "ui2.window".to_string(),
                        enabled: true,
                        note: "Create and manage UI2 windows".to_string(),
                    },
                    Capability {
                        key: "ui2.events".to_string(),
                        enabled: true,
                        note: "Bind generated UI2 event handlers".to_string(),
                    },
                    Capability {
                        key: "fs.user".to_string(),
                        enabled: false,
                        note: "Read and write user-selected files".to_string(),
                    },
                    Capability {
                        key: "net.client".to_string(),
                        enabled: false,
                        note: "Open outbound network connections".to_string(),
                    },
                ],
                metadata: metadata.clone(),
            },
            package: PackageBlueprint {
                schema: "trueos.package.blueprint/v1".to_string(),
                package_id,
                app_id,
                name: name.slug.clone(),
                version: "0.1.0".to_string(),
                entrypoint: "src/main.rs".to_string(),
                artifacts: vec![
                    PackageArtifact {
                        kind: "binary".to_string(),
                        path: "target/trueos/app.tapp".to_string(),
                        target: "trueos-ui2".to_string(),
                    },
                    PackageArtifact {
                        kind: "layout".to_string(),
                        path: "ui/main.ui2".to_string(),
                        target: "ui2-layout".to_string(),
                    },
                ],
                metadata,
            },
            name: name.display.clone(),
            slug: name.slug.clone(),
            windows: vec![main_window],
        }
    }
}

impl Ui2Window {
    pub fn main_window(app_name: impl Into<String>) -> Self {
        let app_name = app_name.into();
        Self {
            id: Uuid::new_v4(),
            name: "MainWindow".to_string(),
            caption: app_name,
            title_twemoji: None,
            geometry: Rect {
                x: 80,
                y: 80,
                w: 720,
                h: 460,
            },
            decorations: WindowDecorations::default(),
            options: Ui2WindowOptions::default(),
            ui_description: Ui2HtmlCssDescription::default(),
            controls: vec![
                Ui2Control::new(
                    ControlKind::Label,
                    "titleLabel",
                    "TRUEOS UI2 app",
                    32,
                    34,
                    220,
                    28,
                ),
                Ui2Control::new(
                    ControlKind::Button,
                    "runButton",
                    "Click me",
                    32,
                    86,
                    128,
                    38,
                ),
            ],
        }
    }

    pub fn named_window(
        name: impl Into<String>,
        caption: impl Into<String>,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            caption: caption.into(),
            title_twemoji: None,
            geometry: Rect { x, y, w, h },
            decorations: WindowDecorations::default(),
            options: Ui2WindowOptions::default(),
            ui_description: Ui2HtmlCssDescription::default(),
            controls: Vec::new(),
        }
    }
}

impl Ui2Control {
    pub fn new(
        kind: ControlKind,
        name: impl Into<String>,
        caption: impl Into<String>,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    ) -> Self {
        let name = name.into();
        let event_name = match kind {
            ControlKind::Button | ControlKind::CheckBox => "click",
            ControlKind::TextBox => "change",
            ControlKind::ListBox => "select",
            ControlKind::Canvas => "draw",
            _ => "ready",
        };
        let properties = default_properties(&kind);
        Self {
            id: Uuid::new_v4(),
            kind,
            name: name.clone(),
            caption: caption.into(),
            geometry: Rect { x, y, w, h },
            properties,
            events: vec![EventBinding {
                event: event_name.to_string(),
                handler: event_handler_name(&name, event_name),
            }],
        }
    }
}

impl WindowDecorations {
    pub fn to_flags(&self) -> Vec<&'static str> {
        [
            (self.titlebar, "titlebar"),
            (self.close, "close"),
            (self.minimize, "minimize"),
            (self.maximize, "maximize"),
            (self.resizable, "resizable"),
            (self.always_on_top, "always-on-top"),
        ]
        .into_iter()
        .filter_map(|(enabled, flag)| enabled.then_some(flag))
        .collect()
    }

    pub fn to_ui2_literal(&self) -> String {
        format!(
            "{{ titlebar: {}, close: {}, minimize: {}, maximize: {}, resizable: {}, always_on_top: {} }}",
            self.titlebar,
            self.close,
            self.minimize,
            self.maximize,
            self.resizable,
            self.always_on_top
        )
    }
}

impl Default for WindowDecorations {
    fn default() -> Self {
        Self {
            titlebar: true,
            close: true,
            minimize: true,
            maximize: true,
            resizable: true,
            always_on_top: false,
        }
    }
}

pub fn validate_project_name(input: &str) -> Result<ValidProjectName, ProjectNameError> {
    let display = input.trim();
    if display.is_empty() {
        return Err(ProjectNameError::Empty);
    }
    if display.chars().count() > MAX_PROJECT_NAME_LEN {
        return Err(ProjectNameError::TooLong {
            max: MAX_PROJECT_NAME_LEN,
        });
    }
    if display.contains('/') || display.contains('\\') {
        return Err(ProjectNameError::ContainsPathSeparator);
    }
    if display.chars().any(char::is_control) {
        return Err(ProjectNameError::ContainsControlCharacter);
    }

    let slug = slugify(display);
    if slug.is_empty() {
        return Err(ProjectNameError::NoSlug);
    }
    if is_reserved_name(&slug) {
        return Err(ProjectNameError::ReservedName);
    }

    Ok(ValidProjectName {
        display: display.to_string(),
        slug,
    })
}

pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            if out.len() >= MAX_PROJECT_SLUG_LEN {
                break;
            }
            out.push(ch);
            last_dash = false;
        } else if !last_dash && !out.is_empty() && out.len() < MAX_PROJECT_SLUG_LEN {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

pub fn event_handler_name(control_name: &str, event_name: &str) -> String {
    format!(
        "on_{}_{}",
        identifier_fragment(control_name),
        identifier_fragment(event_name)
    )
}

fn default_properties(kind: &ControlKind) -> Vec<Property> {
    let pairs = match kind {
        ControlKind::Button => vec![("variant", "primary")],
        ControlKind::Label => vec![("role", "heading")],
        ControlKind::TextBox => vec![("placeholder", "Type here")],
        ControlKind::CheckBox => vec![("checked", "false")],
        ControlKind::Panel => vec![("padding", "16")],
        ControlKind::ListBox => vec![("items", "One,Two,Three")],
        ControlKind::Canvas => vec![("surface", "software")],
        ControlKind::Menu => vec![("items", "File,Edit,View")],
        ControlKind::Toolbar => vec![("dock", "top")],
    };
    pairs
        .into_iter()
        .map(|(key, value)| Property {
            key: key.to_string(),
            value: value.to_string(),
        })
        .collect()
}

fn identifier_fragment(input: &str) -> String {
    let mut out = String::new();
    let mut previous_was_separator = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() && !out.is_empty() && !previous_was_separator {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !out.is_empty() && !previous_was_separator {
            out.push('_');
            previous_was_separator = true;
        }
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "control".to_string()
    } else if trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("c_{trimmed}")
    } else {
        trimmed
    }
}

fn is_reserved_name(slug: &str) -> bool {
    matches!(
        slug,
        "." | ".."
            | "con"
            | "prn"
            | "aux"
            | "nul"
            | "com1"
            | "com2"
            | "com3"
            | "com4"
            | "com5"
            | "com6"
            | "com7"
            | "com8"
            | "com9"
            | "lpt1"
            | "lpt2"
            | "lpt3"
            | "lpt4"
            | "lpt5"
            | "lpt6"
            | "lpt7"
            | "lpt8"
            | "lpt9"
    )
}
