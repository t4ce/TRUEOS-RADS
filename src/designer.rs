use crate::model::{
    ControlKind, EventBinding, Property, Rect, Ui2Control, Ui2Window, WindowDecorationMode,
    WindowDecorations, event_handler_name,
};
use crate::ui2_options::{
    Ui2HorizontalScrollbarSide, Ui2HtmlCssDescription, Ui2ResizeMode, Ui2ScrollbarMode, Ui2Size,
    Ui2VerticalScrollbarSide, Ui2WindowOptions,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaletteCategory {
    pub name: String,
    pub items: Vec<PaletteItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaletteItem {
    pub kind: ControlKind,
    pub label: String,
    pub category: String,
    pub default_w: u32,
    pub default_h: u32,
    pub default_caption: String,
    pub default_properties: Vec<Property>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignerState {
    pub active_window: Uuid,
    pub selected_control: Option<Uuid>,
    pub grid: u32,
    pub snap_to_grid: bool,
    pub palette: Vec<PaletteCategory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddControlRequest {
    pub window_id: Uuid,
    pub kind: ControlKind,
    pub x: i32,
    pub y: i32,
    pub id: Option<Uuid>,
    pub name: Option<String>,
    pub caption: Option<String>,
    pub snap: Option<SnapSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveControlRequest {
    pub window_id: Uuid,
    pub control_id: Uuid,
    pub geometry: Rect,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SnapSettings {
    pub enabled: bool,
    pub grid: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Alignment {
    Left,
    Top,
    Right,
    Bottom,
    HorizontalCenter,
    VerticalCenter,
    SameWidth,
    SameHeight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateControlRequest {
    pub window_id: Uuid,
    pub control_id: Uuid,
    pub name: Option<String>,
    pub caption: Option<String>,
    pub geometry: Option<Rect>,
    pub properties: Option<Vec<Property>>,
    pub events: Option<Vec<EventBinding>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWindowRequest {
    pub window_id: Uuid,
    pub caption: Option<String>,
    pub geometry: Option<Rect>,
    pub decorations: Option<WindowDecorations>,
    pub options: Option<Ui2WindowOptions>,
    pub ui_description: Option<Ui2HtmlCssDescription>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlPropertyChangeRequest {
    pub window_id: Uuid,
    pub control_id: Uuid,
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectControlRequest {
    pub window_id: Uuid,
    pub control_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateDesignerStateRequest {
    pub active_window: Option<Uuid>,
    pub selected_control: Option<Uuid>,
    pub clear_selection: bool,
    pub grid: Option<u32>,
    pub snap_to_grid: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlignControlsRequest {
    pub window_id: Uuid,
    pub control_ids: Vec<Uuid>,
    pub anchor_control_id: Option<Uuid>,
    pub alignment: Alignment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectorRow {
    pub key: String,
    pub value: String,
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInspectorData {
    pub window_id: Uuid,
    pub target: ObjectInspectorTarget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_options: Option<Ui2WindowInspectorOptions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_description: Option<Ui2HtmlCssDescription>,
    pub sections: Vec<ObjectInspectorSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ui2WindowInspectorOptions {
    pub caption: String,
    pub min_size: Ui2Size,
    pub max_size: Option<Ui2Size>,
    pub resize_mode: Ui2ResizeMode,
    pub scrollbars: Ui2ScrollbarMode,
    pub vertical_scrollbar_side: Ui2VerticalScrollbarSide,
    pub horizontal_scrollbar_side: Ui2HorizontalScrollbarSide,
    pub hit_test_visible: bool,
    pub preserve_scale: bool,
    pub decoration_flags: Vec<String>,
}

impl Ui2WindowInspectorOptions {
    pub fn from_window(window: &Ui2Window) -> Self {
        Self {
            caption: window.caption.clone(),
            min_size: window.options.min_size,
            max_size: window.options.max_size,
            resize_mode: window.options.resize_mode,
            scrollbars: window.options.scrollbars,
            vertical_scrollbar_side: window.options.vertical_scrollbar_side,
            horizontal_scrollbar_side: window.options.horizontal_scrollbar_side,
            hit_test_visible: window.options.hit_test_visible,
            preserve_scale: window.options.preserve_scale,
            decoration_flags: window
                .decorations
                .to_flags()
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }
}

impl Default for Ui2WindowInspectorOptions {
    fn default() -> Self {
        let options = Ui2WindowOptions::default();
        Self {
            caption: String::new(),
            min_size: options.min_size,
            max_size: options.max_size,
            resize_mode: options.resize_mode,
            scrollbars: options.scrollbars,
            vertical_scrollbar_side: options.vertical_scrollbar_side,
            horizontal_scrollbar_side: options.horizontal_scrollbar_side,
            hit_test_visible: options.hit_test_visible,
            preserve_scale: options.preserve_scale,
            decoration_flags: WindowDecorations::default()
                .to_flags()
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObjectInspectorTarget {
    Window {
        id: Uuid,
        name: String,
    },
    Control {
        id: Uuid,
        name: String,
        kind: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInspectorSection {
    pub id: String,
    pub label: String,
    pub fields: Vec<ObjectInspectorField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInspectorField {
    pub key: String,
    pub label: String,
    pub value: String,
    pub editor: ObjectInspectorEditor,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObjectInspectorEditor {
    Text,
    TextArea { language: Option<String> },
    Number { min: Option<i32>, step: i32 },
    Bool,
    Select { options: Vec<String> },
}

pub fn default_palette() -> Vec<PaletteItem> {
    default_palette_categories()
        .into_iter()
        .flat_map(|category| category.items)
        .collect()
}

pub fn default_palette_categories() -> Vec<PaletteCategory> {
    vec![
        PaletteCategory {
            name: "Standard".to_string(),
            items: vec![
                item(ControlKind::Button, "Button", "Standard", 120, 34, "Button"),
                item(ControlKind::Label, "Label", "Standard", 160, 24, "Label"),
                item(ControlKind::TextBox, "Text Box", "Standard", 180, 32, ""),
                item(
                    ControlKind::CheckBox,
                    "Check Box",
                    "Standard",
                    160,
                    30,
                    "Check me",
                ),
            ],
        },
        PaletteCategory {
            name: "Containers".to_string(),
            items: vec![
                item(ControlKind::Panel, "Panel", "Containers", 240, 160, ""),
                item(ControlKind::Canvas, "Canvas", "Containers", 260, 180, ""),
            ],
        },
        PaletteCategory {
            name: "Data".to_string(),
            items: vec![item(ControlKind::ListBox, "List Box", "Data", 220, 160, "")],
        },
        PaletteCategory {
            name: "Navigation".to_string(),
            items: vec![
                item(ControlKind::Menu, "Menu", "Navigation", 360, 30, "File"),
                item(
                    ControlKind::Toolbar,
                    "Toolbar",
                    "Navigation",
                    360,
                    42,
                    "Toolbar",
                ),
            ],
        },
    ]
}

pub fn add_control(window: &mut Ui2Window, request: AddControlRequest) -> Option<Uuid> {
    if window.id != request.window_id {
        return None;
    }

    let palette = default_palette();
    let item = palette
        .iter()
        .find(|item| std::mem::discriminant(&item.kind) == std::mem::discriminant(&request.kind))?;
    let base = format!("{:?}", request.kind);
    let name = request
        .name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| unique_control_name(window, &base));
    let caption = request
        .caption
        .unwrap_or_else(|| item.default_caption.clone());
    let geometry = match request.snap {
        Some(settings) if settings.enabled => Rect {
            x: snap_value(request.x, settings.grid),
            y: snap_value(request.y, settings.grid),
            w: item.default_w,
            h: item.default_h,
        },
        Some(_) => Rect {
            x: request.x,
            y: request.y,
            w: item.default_w,
            h: item.default_h,
        },
        None => Rect {
            x: snap(request.x, 8),
            y: snap(request.y, 8),
            w: item.default_w,
            h: item.default_h,
        },
    };
    let mut control = Ui2Control::new(
        request.kind,
        name,
        caption,
        geometry.x,
        geometry.y,
        geometry.w,
        geometry.h,
    );
    if let Some(id) = request.id {
        control.id = id;
    }
    if control.caption.is_empty() {
        control.caption = item.label.clone();
    }
    control.properties = item.default_properties.clone();
    let id = control.id;
    window.controls.push(control);
    Some(id)
}

pub fn move_control(window: &mut Ui2Window, request: MoveControlRequest) -> bool {
    if window.id != request.window_id {
        return false;
    }
    let Some(control) = window
        .controls
        .iter_mut()
        .find(|control| control.id == request.control_id)
    else {
        return false;
    };
    control.geometry = Rect {
        x: snap(request.geometry.x, 8),
        y: snap(request.geometry.y, 8),
        w: request.geometry.w.max(8),
        h: request.geometry.h.max(8),
    };
    true
}

pub fn update_control(window: &mut Ui2Window, request: UpdateControlRequest) -> bool {
    if window.id != request.window_id {
        return false;
    }
    let Some(control) = window
        .controls
        .iter_mut()
        .find(|control| control.id == request.control_id)
    else {
        return false;
    };

    if let Some(name) = request.name {
        control.name = sanitize_identifier(&name);
        for event in &mut control.events {
            event.handler = event_handler_name(&control.name, &event.event);
        }
    }
    if let Some(caption) = request.caption {
        control.caption = caption;
    }
    if let Some(geometry) = request.geometry {
        control.geometry = geometry;
    }
    if let Some(properties) = request.properties {
        control.properties = properties;
    }
    if let Some(events) = request.events {
        control.events = events;
    }
    true
}

pub fn update_window(window: &mut Ui2Window, request: UpdateWindowRequest) -> bool {
    if window.id != request.window_id {
        return false;
    }
    if let Some(caption) = request.caption {
        window.caption = caption;
    }
    if let Some(geometry) = request.geometry {
        window.geometry = geometry;
    }
    if let Some(decorations) = request.decorations {
        window.decorations = decorations;
    }
    if let Some(options) = request.options {
        window.options = options;
    }
    if let Some(ui_description) = request.ui_description {
        window.ui_description = ui_description;
    }
    true
}

pub fn change_control_property(
    window: &mut Ui2Window,
    request: ControlPropertyChangeRequest,
) -> bool {
    if window.id != request.window_id {
        return false;
    }
    let Some(control) = window
        .controls
        .iter_mut()
        .find(|control| control.id == request.control_id)
    else {
        return false;
    };

    match request.value {
        Some(value) => match control
            .properties
            .iter_mut()
            .find(|property| property.key == request.key)
        {
            Some(property) => property.value = value,
            None => control.properties.push(Property {
                key: request.key,
                value,
            }),
        },
        None => control
            .properties
            .retain(|property| property.key != request.key),
    }
    true
}

pub fn select_control(state: &mut DesignerState, request: SelectControlRequest) -> bool {
    if state.active_window != request.window_id {
        return false;
    }
    state.selected_control = request.control_id;
    true
}

pub fn update_designer_state(
    state: &mut DesignerState,
    request: UpdateDesignerStateRequest,
) -> bool {
    if let Some(active_window) = request.active_window {
        state.active_window = active_window;
        if request.selected_control.is_none() && !request.clear_selection {
            state.selected_control = None;
        }
    }
    if request.clear_selection {
        state.selected_control = None;
    } else if let Some(selected_control) = request.selected_control {
        state.selected_control = Some(selected_control);
    }
    if let Some(grid) = request.grid {
        state.grid = grid.max(1);
    }
    if let Some(snap_to_grid) = request.snap_to_grid {
        state.snap_to_grid = snap_to_grid;
    }
    true
}

pub fn align_controls(window: &mut Ui2Window, request: AlignControlsRequest) -> usize {
    if window.id != request.window_id || request.control_ids.is_empty() {
        return 0;
    }

    let anchor_id = request.anchor_control_id.unwrap_or(request.control_ids[0]);
    let Some(anchor) = window
        .controls
        .iter()
        .find(|control| control.id == anchor_id)
        .map(|control| control.geometry)
    else {
        return 0;
    };

    let mut changed = 0;
    for control in window.controls.iter_mut().filter(|control| {
        control.id != anchor_id && request.control_ids.iter().any(|id| *id == control.id)
    }) {
        control.geometry = align_rect(control.geometry, anchor, request.alignment);
        changed += 1;
    }
    changed
}

pub fn snap_value(value: i32, grid: u32) -> i32 {
    let grid = grid.max(1) as i32;
    if value >= 0 {
        ((value + grid / 2) / grid) * grid
    } else {
        ((value - grid / 2) / grid) * grid
    }
}

pub fn snap_size(value: u32, grid: u32) -> u32 {
    snap_value(value as i32, grid).max(1) as u32
}

pub fn snap_rect(rect: Rect, settings: SnapSettings) -> Rect {
    if !settings.enabled {
        return rect;
    }
    Rect {
        x: snap_value(rect.x, settings.grid),
        y: snap_value(rect.y, settings.grid),
        w: snap_size(rect.w, settings.grid),
        h: snap_size(rect.h, settings.grid),
    }
}

pub fn align_rect(rect: Rect, anchor: Rect, alignment: Alignment) -> Rect {
    let mut next = rect;
    match alignment {
        Alignment::Left => next.x = anchor.x,
        Alignment::Top => next.y = anchor.y,
        Alignment::Right => next.x = anchor.x + anchor.w as i32 - rect.w as i32,
        Alignment::Bottom => next.y = anchor.y + anchor.h as i32 - rect.h as i32,
        Alignment::HorizontalCenter => {
            next.x = anchor.x + anchor.w as i32 / 2 - rect.w as i32 / 2;
        }
        Alignment::VerticalCenter => {
            next.y = anchor.y + anchor.h as i32 / 2 - rect.h as i32 / 2;
        }
        Alignment::SameWidth => next.w = anchor.w,
        Alignment::SameHeight => next.h = anchor.h,
    }
    next
}

pub fn inspector_for_window(window: &Ui2Window) -> Vec<InspectorRow> {
    vec![
        row("name", &window.name, false),
        row("caption", &window.caption, true),
        row("x", &window.geometry.x.to_string(), true),
        row("y", &window.geometry.y.to_string(), true),
        row("width", &window.geometry.w.to_string(), true),
        row("height", &window.geometry.h.to_string(), true),
        row(
            "decorations",
            &window.decorations.to_flags().join(", "),
            true,
        ),
    ]
}

pub fn inspector_for_control(control: &Ui2Control) -> Vec<InspectorRow> {
    let mut rows = vec![
        row("name", &control.name, true),
        row("kind", &format!("{:?}", control.kind), false),
        row("caption", &control.caption, true),
        row("x", &control.geometry.x.to_string(), true),
        row("y", &control.geometry.y.to_string(), true),
        row("width", &control.geometry.w.to_string(), true),
        row("height", &control.geometry.h.to_string(), true),
    ];
    rows.extend(
        control
            .properties
            .iter()
            .map(|property| row(&property.key, &property.value, true)),
    );
    rows
}

pub fn object_inspector_for_selection(
    window: &Ui2Window,
    selected_control: Option<Uuid>,
) -> ObjectInspectorData {
    match selected_control.and_then(|id| window.controls.iter().find(|control| control.id == id)) {
        Some(control) => object_inspector_for_control(window.id, control),
        None => object_inspector_for_window(window),
    }
}

pub fn object_inspector_for_window(window: &Ui2Window) -> ObjectInspectorData {
    ObjectInspectorData {
        window_id: window.id,
        target: ObjectInspectorTarget::Window {
            id: window.id,
            name: window.name.clone(),
        },
        window_options: Some(Ui2WindowInspectorOptions::from_window(window)),
        ui_description: Some(window.ui_description.clone()),
        sections: vec![
            ObjectInspectorSection {
                id: "identity".to_string(),
                label: "Identity".to_string(),
                fields: vec![
                    text_field("name", "Name", &window.name, true),
                    text_field("caption", "Caption", &window.caption, false),
                ],
            },
            ObjectInspectorSection {
                id: "geometry".to_string(),
                label: "Geometry".to_string(),
                fields: rect_fields(window.geometry),
            },
            ObjectInspectorSection {
                id: "window-options".to_string(),
                label: "Window Options".to_string(),
                fields: window_option_fields(window),
            },
            ObjectInspectorSection {
                id: "decorations".to_string(),
                label: "Decorations".to_string(),
                fields: vec![
                    text_field(
                        "decoration-flags",
                        "Decoration flags",
                        &window.decorations.to_flags().join(", "),
                        true,
                    ),
                    bool_field("titlebar", "Titlebar", window.decorations.titlebar),
                    bool_field("bottom-bar", "Bottom bar", window.decorations.bottom_bar),
                    bool_field("title-icon", "Title icon", window.decorations.title_icon),
                    bool_field(
                        "toggle-composition",
                        "Toggle composition",
                        window.decorations.toggle_composition,
                    ),
                    bool_field("fork", "Fork", window.decorations.fork),
                    bool_field("close", "Close", window.decorations.close),
                    bool_field("minimize", "Minimize", window.decorations.minimize),
                    bool_field("restore", "Restore", window.decorations.restore),
                    bool_field("maximize", "Maximize", window.decorations.maximize),
                    bool_field("preserve-vm", "Preserve VM", window.decorations.preserve_vm),
                    bool_field("resizable", "Resizable", window.decorations.resizable),
                    bool_field(
                        "resize-button",
                        "Resize button",
                        window.decorations.resize_button,
                    ),
                    bool_field(
                        "rotate-buttons",
                        "Rotate buttons",
                        window.decorations.rotate_buttons,
                    ),
                    bool_field(
                        "always-on-top",
                        "Always on top",
                        window.decorations.always_on_top,
                    ),
                ],
            },
            ObjectInspectorSection {
                id: "ui-description".to_string(),
                label: "UI Description".to_string(),
                fields: vec![
                    text_area_field(
                        "ui-description.html",
                        "HTML",
                        &window.ui_description.html,
                        "html",
                    ),
                    text_area_field(
                        "ui-description.css",
                        "CSS",
                        &window.ui_description.css,
                        "css",
                    ),
                ],
            },
        ],
    }
}

pub fn object_inspector_for_control(window_id: Uuid, control: &Ui2Control) -> ObjectInspectorData {
    ObjectInspectorData {
        window_id,
        target: ObjectInspectorTarget::Control {
            id: control.id,
            name: control.name.clone(),
            kind: control_kind_key(&control.kind).to_string(),
        },
        window_options: None,
        ui_description: None,
        sections: vec![
            ObjectInspectorSection {
                id: "identity".to_string(),
                label: "Identity".to_string(),
                fields: vec![
                    text_field("name", "Name", &control.name, false),
                    text_field("caption", "Caption", &control.caption, false),
                    ObjectInspectorField {
                        key: "kind".to_string(),
                        label: "Kind".to_string(),
                        value: control_kind_key(&control.kind).to_string(),
                        editor: ObjectInspectorEditor::Select {
                            options: control_kind_options(),
                        },
                        read_only: true,
                    },
                ],
            },
            ObjectInspectorSection {
                id: "geometry".to_string(),
                label: "Geometry".to_string(),
                fields: rect_fields(control.geometry),
            },
            ObjectInspectorSection {
                id: "properties".to_string(),
                label: "Properties".to_string(),
                fields: control_property_fields(control),
            },
            ObjectInspectorSection {
                id: "events".to_string(),
                label: "Events".to_string(),
                fields: control
                    .events
                    .iter()
                    .map(|event| {
                        text_field(
                            &event.event,
                            &property_label(&event.event),
                            &event.handler,
                            false,
                        )
                    })
                    .collect(),
            },
        ],
    }
}

fn control_property_fields(control: &Ui2Control) -> Vec<ObjectInspectorField> {
    let mut fields = control
        .properties
        .iter()
        .map(|property| ObjectInspectorField {
            key: property.key.clone(),
            label: property_label(&property.key),
            value: property.value.clone(),
            editor: property_editor(&property.key, &property.value),
            read_only: false,
        })
        .collect::<Vec<_>>();

    if matches!(control.kind, ControlKind::Button)
        && !control
            .properties
            .iter()
            .any(|property| property.key == "glyph")
    {
        fields.push(ObjectInspectorField {
            key: "glyph".to_string(),
            label: "Glyph".to_string(),
            value: String::new(),
            editor: ObjectInspectorEditor::Text,
            read_only: false,
        });
    }

    fields
}

pub fn default_properties(kind: &ControlKind) -> Vec<Property> {
    match kind {
        ControlKind::Button => vec![
            property("enabled", "true"),
            property("default", "false"),
            property("tab-stop", "true"),
        ],
        ControlKind::Label => vec![
            property("auto-size", "true"),
            property("text-align", "left"),
            property("font-weight", "normal"),
        ],
        ControlKind::TextBox => vec![
            property("placeholder", ""),
            property("read-only", "false"),
            property("max-length", "0"),
            property("tab-stop", "true"),
        ],
        ControlKind::CheckBox => vec![
            property("checked", "false"),
            property("enabled", "true"),
            property("tab-stop", "true"),
        ],
        ControlKind::Panel => vec![
            property("border", "true"),
            property("padding", "8"),
            property("background", "surface"),
        ],
        ControlKind::ListBox => vec![
            property("items", ""),
            property("selected-index", "-1"),
            property("multi-select", "false"),
        ],
        ControlKind::Canvas => vec![
            property("background", "transparent"),
            property("double-buffered", "true"),
        ],
        ControlKind::Menu => vec![
            property("items", "File,Edit,Help"),
            property("enabled", "true"),
        ],
        ControlKind::Toolbar => vec![
            property("orientation", "horizontal"),
            property("show-captions", "false"),
            property("enabled", "true"),
        ],
    }
}

fn item(
    kind: ControlKind,
    label: &str,
    category: &str,
    default_w: u32,
    default_h: u32,
    default_caption: &str,
) -> PaletteItem {
    let default_properties = default_properties(&kind);
    PaletteItem {
        kind,
        label: label.to_string(),
        category: category.to_string(),
        default_w,
        default_h,
        default_caption: default_caption.to_string(),
        default_properties,
    }
}

fn unique_control_name(window: &Ui2Window, base: &str) -> String {
    let stem = sanitize_identifier(base);
    for n in 1.. {
        let candidate = format!("{stem}{n}");
        if !window
            .controls
            .iter()
            .any(|control| control.name == candidate)
        {
            return candidate;
        }
    }
    unreachable!()
}

fn sanitize_identifier(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "control".to_string()
    } else if trimmed.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("c_{trimmed}")
    } else {
        trimmed.to_string()
    }
}

fn snap(value: i32, grid: i32) -> i32 {
    ((value + grid / 2) / grid) * grid
}

fn row(key: &str, value: &str, editable: bool) -> InspectorRow {
    InspectorRow {
        key: key.to_string(),
        value: value.to_string(),
        editable,
    }
}

fn property(key: &str, value: &str) -> Property {
    Property {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn rect_fields(rect: Rect) -> Vec<ObjectInspectorField> {
    vec![
        number_field("x", "X", rect.x, None),
        number_field("y", "Y", rect.y, None),
        number_field("width", "Width", rect.w as i32, Some(1)),
        number_field("height", "Height", rect.h as i32, Some(1)),
    ]
}

fn window_option_fields(window: &Ui2Window) -> Vec<ObjectInspectorField> {
    let options = &window.options;
    vec![
        number_field(
            "min-width",
            "Min width",
            options.min_size.width as i32,
            Some(1),
        ),
        number_field(
            "min-height",
            "Min height",
            options.min_size.height as i32,
            Some(1),
        ),
        optional_number_field(
            "max-width",
            "Max width",
            options.max_size.map(|size| size.width),
            Some(1),
        ),
        optional_number_field(
            "max-height",
            "Max height",
            options.max_size.map(|size| size.height),
            Some(1),
        ),
        select_field(
            "resize-mode",
            "Resize mode",
            options.resize_mode.as_str(),
            Ui2ResizeMode::options(),
        ),
        select_field(
            "scrollbars",
            "Scrollbars",
            options.scrollbars.as_str(),
            Ui2ScrollbarMode::options(),
        ),
        select_field(
            "vertical-scrollbar-side",
            "Vertical side",
            options.vertical_scrollbar_side.as_str(),
            Ui2VerticalScrollbarSide::options(),
        ),
        select_field(
            "horizontal-scrollbar-side",
            "Horizontal side",
            options.horizontal_scrollbar_side.as_str(),
            Ui2HorizontalScrollbarSide::options(),
        ),
        select_field(
            "decoration-mode",
            "Decoration mode",
            window.decorations.mode.as_str(),
            WindowDecorationMode::options(),
        ),
        bool_field(
            "hit-test-visible",
            "Hit-test visible",
            options.hit_test_visible,
        ),
        bool_field("preserve-scale", "Preserve scale", options.preserve_scale),
    ]
}

fn text_field(key: &str, label: &str, value: &str, read_only: bool) -> ObjectInspectorField {
    ObjectInspectorField {
        key: key.to_string(),
        label: label.to_string(),
        value: value.to_string(),
        editor: ObjectInspectorEditor::Text,
        read_only,
    }
}

fn text_area_field(key: &str, label: &str, value: &str, language: &str) -> ObjectInspectorField {
    ObjectInspectorField {
        key: key.to_string(),
        label: label.to_string(),
        value: value.to_string(),
        editor: ObjectInspectorEditor::TextArea {
            language: Some(language.to_string()),
        },
        read_only: false,
    }
}

fn number_field(key: &str, label: &str, value: i32, min: Option<i32>) -> ObjectInspectorField {
    ObjectInspectorField {
        key: key.to_string(),
        label: label.to_string(),
        value: value.to_string(),
        editor: ObjectInspectorEditor::Number { min, step: 1 },
        read_only: false,
    }
}

fn optional_number_field(
    key: &str,
    label: &str,
    value: Option<u32>,
    min: Option<i32>,
) -> ObjectInspectorField {
    ObjectInspectorField {
        key: key.to_string(),
        label: label.to_string(),
        value: value.map(|value| value.to_string()).unwrap_or_default(),
        editor: ObjectInspectorEditor::Number { min, step: 1 },
        read_only: false,
    }
}

fn bool_field(key: &str, label: &str, value: bool) -> ObjectInspectorField {
    ObjectInspectorField {
        key: key.to_string(),
        label: label.to_string(),
        value: value.to_string(),
        editor: ObjectInspectorEditor::Bool,
        read_only: false,
    }
}

fn select_field(key: &str, label: &str, value: &str, options: Vec<String>) -> ObjectInspectorField {
    ObjectInspectorField {
        key: key.to_string(),
        label: label.to_string(),
        value: value.to_string(),
        editor: ObjectInspectorEditor::Select { options },
        read_only: false,
    }
}

fn property_editor(key: &str, value: &str) -> ObjectInspectorEditor {
    if matches!(value, "true" | "false") {
        ObjectInspectorEditor::Bool
    } else if key.ends_with("index") || key.ends_with("length") || key == "padding" {
        ObjectInspectorEditor::Number { min: None, step: 1 }
    } else {
        ObjectInspectorEditor::Text
    }
}

fn property_label(key: &str) -> String {
    key.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn control_kind_key(kind: &ControlKind) -> &'static str {
    match kind {
        ControlKind::Button => "button",
        ControlKind::Label => "label",
        ControlKind::TextBox => "text-box",
        ControlKind::CheckBox => "check-box",
        ControlKind::Panel => "panel",
        ControlKind::ListBox => "list-box",
        ControlKind::Canvas => "canvas",
        ControlKind::Menu => "menu",
        ControlKind::Toolbar => "toolbar",
    }
}

fn control_kind_options() -> Vec<String> {
    [
        "button",
        "label",
        "text-box",
        "check-box",
        "panel",
        "list-box",
        "canvas",
        "menu",
        "toolbar",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_items_include_property_defaults() {
        let text_box = default_palette()
            .into_iter()
            .find(|item| matches!(item.kind, ControlKind::TextBox))
            .expect("text box palette item");

        assert_eq!(text_box.category, "Standard");
        assert!(
            text_box
                .default_properties
                .iter()
                .any(|property| property.key == "placeholder")
        );
    }

    #[test]
    fn control_property_changes_can_set_and_remove_values() {
        let mut window = Ui2Window::main_window("Designer");
        let window_id = window.id;
        let control_id = window.controls[0].id;

        assert!(change_control_property(
            &mut window,
            ControlPropertyChangeRequest {
                window_id,
                control_id,
                key: "visible".to_string(),
                value: Some("true".to_string()),
            },
        ));
        assert!(
            window.controls[0]
                .properties
                .iter()
                .any(|property| property.key == "visible" && property.value == "true")
        );

        assert!(change_control_property(
            &mut window,
            ControlPropertyChangeRequest {
                window_id,
                control_id,
                key: "visible".to_string(),
                value: None,
            },
        ));
        assert!(
            !window.controls[0]
                .properties
                .iter()
                .any(|property| property.key == "visible")
        );
    }

    #[test]
    fn alignment_snap_selection_and_inspector_are_data_only() {
        let mut window = Ui2Window::main_window("Designer");
        let window_id = window.id;
        let anchor_id = window.controls[0].id;
        let target_id = window.controls[1].id;

        let snapped = snap_rect(
            Rect {
                x: 13,
                y: 18,
                w: 119,
                h: 35,
            },
            SnapSettings {
                enabled: true,
                grid: 8,
            },
        );
        assert_eq!(snapped.x, 16);
        assert_eq!(snapped.w, 120);

        let changed = align_controls(
            &mut window,
            AlignControlsRequest {
                window_id,
                control_ids: vec![anchor_id, target_id],
                anchor_control_id: Some(anchor_id),
                alignment: Alignment::Left,
            },
        );
        assert_eq!(changed, 1);
        assert_eq!(window.controls[0].geometry.x, window.controls[1].geometry.x);

        let mut state = DesignerState {
            active_window: window_id,
            selected_control: None,
            grid: 8,
            snap_to_grid: true,
            palette: default_palette_categories(),
        };
        assert!(select_control(
            &mut state,
            SelectControlRequest {
                window_id,
                control_id: Some(target_id),
            },
        ));
        assert_eq!(state.selected_control, Some(target_id));

        let inspector = object_inspector_for_selection(&window, state.selected_control);
        assert!(
            inspector
                .sections
                .iter()
                .any(|section| section.id == "properties")
        );
    }
}
