use crate::designer;
use crate::file_store;
use crate::generator;
use crate::jobs::{JobKind, JobManager, JobStage, JobStatus};
use crate::localcoder;
use crate::model::{
    AppKind, EventBinding, Property, RadsProject, Rect, Ui2Control, Ui2Window, WindowDecorations,
};
use crate::project_templates;
use crate::ui2_options::Ui2HtmlCssDescription;
use crate::watcher;
use anyhow::Context;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use notify::RecommendedWatcher;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::sync::broadcast::error::RecvError;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub struct AppState {
    workspace: PathBuf,
    jobs: JobManager,
    active: Arc<Mutex<Option<RadsProject>>>,
    runtime: Arc<Mutex<RuntimeState>>,
    full_auto: Arc<AtomicBool>,
}

struct RuntimeState {
    watch: bool,
    watched_project: Option<PathBuf>,
    watcher: Option<RecommendedWatcher>,
}

#[derive(Debug, Deserialize)]
pub struct NewProjectRequest {
    pub name: String,
    pub template_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NewProjectResponse {
    pub project: RadsProject,
}

#[derive(Debug, Deserialize)]
pub struct LoadProjectRequest {
    pub path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub project: RadsProject,
}

#[derive(Debug, Deserialize)]
pub struct RuntimeRequest {
    pub full_auto: Option<bool>,
    pub watch: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RuntimeResponse {
    pub full_auto: bool,
    pub watch: bool,
    pub active_project: Option<PathBuf>,
    pub watched_project: Option<PathBuf>,
    pub build_possible: bool,
    pub current_stage: Option<JobStage>,
    pub current_status: Option<JobStatus>,
    pub current_job_id: Option<uuid::Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWindowRequest {
    pub window_id: uuid::Uuid,
    pub window: Option<Ui2Window>,
    pub name: Option<String>,
    pub caption: Option<String>,
    pub geometry: Option<Rect>,
    pub decorations: Option<WindowDecorations>,
    #[serde(default)]
    pub properties: Vec<Property>,
}

#[derive(Debug, Deserialize)]
pub struct NewWindowRequest {
    pub name: Option<String>,
    pub caption: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateControlRequest {
    pub window_id: uuid::Uuid,
    pub control_id: uuid::Uuid,
    pub control: Option<Ui2Control>,
    pub name: Option<String>,
    pub caption: Option<String>,
    pub geometry: Option<Rect>,
    pub properties: Option<Vec<Property>>,
    pub events: Option<Vec<EventBinding>>,
}

#[derive(Debug, Deserialize)]
pub struct RunJobRequest {
    pub kind: String,
}

#[derive(Debug, Serialize)]
pub struct RunJobResponse {
    pub job_id: uuid::Uuid,
}

#[derive(Debug, Deserialize)]
pub struct ProjectFileQuery {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct WriteProjectFileRequest {
    pub path: Option<PathBuf>,
    #[serde(alias = "content", alias = "body")]
    pub contents: String,
    #[serde(default)]
    pub create_dirs: bool,
}

#[derive(Debug, Deserialize)]
pub struct ImportAssetRequest {
    pub name: String,
    pub contents_base64: String,
}

#[derive(Debug, Serialize)]
pub struct AssetsResponse {
    pub root: PathBuf,
    pub assets: Vec<AssetEntry>,
}

#[derive(Debug, Serialize)]
pub struct AssetEntry {
    pub name: String,
    pub path: PathBuf,
    pub extension: String,
    pub size: u64,
    pub warning: Option<&'static str>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUiDescriptionRequest {
    pub window_id: Option<uuid::Uuid>,
    pub description: Option<String>,
    pub ui_description: Option<Ui2HtmlCssDescription>,
    pub html: Option<String>,
    pub css: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TemplatesResponse {
    pub source: &'static str,
    pub templates: Vec<ProjectTemplateSummary>,
}

#[derive(Debug, Serialize)]
pub struct ProjectTemplateSummary {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub app_kind: AppKind,
    pub tags: Vec<&'static str>,
    pub files: Vec<&'static str>,
}

pub async fn serve(workspace: PathBuf) -> anyhow::Result<()> {
    let state = AppState {
        workspace: workspace.join("rads-workspace"),
        jobs: JobManager::new(),
        active: Arc::new(Mutex::new(None)),
        runtime: Arc::new(Mutex::new(RuntimeState {
            watch: false,
            watched_project: None,
            watcher: None,
        })),
        full_auto: Arc::new(AtomicBool::new(true)),
    };
    tokio::fs::create_dir_all(&state.workspace)
        .await
        .context("failed to create RADS workspace")?;

    let app = Router::new()
        .route("/", get(index))
        .route("/api/palette", get(palette))
        .route("/api/templates", get(templates))
        .route("/api/assets/trueos-twemoji", get(trueos_twemoji_asset))
        .route("/api/localcoder/status", get(localcoder_status))
        .route("/api/localcoder/chat", post(localcoder_chat))
        .route(
            "/assets/trueos/twemoji/atlas.png",
            get(trueos_twemoji_atlas_png),
        )
        .route(
            "/assets/trueos/twemoji/atlas-set.json",
            get(trueos_twemoji_atlas_json),
        )
        .route("/api/files", get(list_project_files))
        .route(
            "/api/files/{*path}",
            get(read_project_file_path)
                .post(write_project_file_path)
                .put(write_project_file_path),
        )
        .route("/api/project", get(active_project).post(new_project))
        .route("/api/project/assets", get(list_assets).post(import_asset))
        .route("/api/project/files", get(list_project_files))
        .route(
            "/api/project/file",
            get(read_project_file_query)
                .post(write_project_file_json)
                .put(write_project_file_json),
        )
        .route(
            "/api/project/file/{*path}",
            get(read_project_file_path)
                .post(write_project_file_path)
                .put(write_project_file_path),
        )
        .route(
            "/api/project/control",
            post(add_control).patch(update_control),
        )
        .route("/api/project/control/{control_id}", delete(delete_control))
        .route("/api/project/load", post(load_project))
        .route("/api/project/save", post(save_project))
        .route(
            "/api/project/ui-description",
            post(update_ui_description).patch(update_ui_description),
        )
        .route(
            "/api/project/ui",
            post(update_ui_description).patch(update_ui_description),
        )
        .route(
            "/api/project/description",
            post(update_ui_description).patch(update_ui_description),
        )
        .route(
            "/api/project/window",
            post(update_window).patch(update_window),
        )
        .route("/api/project/window/new", post(new_window))
        .route("/api/project/window/create", post(new_window))
        .route("/api/runtime", get(runtime).post(update_runtime))
        .route("/api/jobs", get(list_jobs).post(run_job))
        .route("/api/events", get(events))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(state);

    let listener = TcpListener::bind(("127.0.0.1", 7377)).await?;
    tracing::info!("TRUEOS RADS listening on http://127.0.0.1:7377");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../static/index.html"))
}

async fn palette() -> Json<Vec<designer::PaletteItem>> {
    Json(designer::default_palette())
}

async fn templates() -> Json<TemplatesResponse> {
    Json(TemplatesResponse {
        source: "project_templates",
        templates: project_templates::available_project_templates()
            .iter()
            .map(|template| ProjectTemplateSummary {
                id: template.id,
                name: template.name,
                description: template.description,
                app_kind: template.app_kind,
                tags: template_tags(template.app_kind),
                files: template_files(template.app_kind, template.window.is_some()),
            })
            .collect(),
    })
}

async fn trueos_twemoji_asset() -> Json<serde_json::Value> {
    let atlas_png = trueos_twemoji_path("atlas.png");
    let atlas_set = trueos_twemoji_path("atlas-set.json");
    Json(serde_json::json!({
        "source": "TRUEOS/src/gfx/althlasfont/twemoji-1x",
        "available": atlas_png.exists() && atlas_set.exists(),
        "font_stack": "Twitter Color Emoji, Twemoji Mozilla, Noto Color Emoji, Apple Color Emoji, Segoe UI Emoji",
        "atlas_png": "/assets/trueos/twemoji/atlas.png",
        "atlas_set": "/assets/trueos/twemoji/atlas-set.json",
        "kernel_asset_path": trueos_twemoji_dir().display().to_string(),
    }))
}

async fn trueos_twemoji_atlas_png() -> Result<impl IntoResponse, String> {
    let bytes = tokio::fs::read(trueos_twemoji_path("atlas.png"))
        .await
        .map_err(|err| format!("TRUEOS twemoji atlas unavailable: {err}"))?;
    Ok(([(header::CONTENT_TYPE, "image/png")], bytes))
}

async fn trueos_twemoji_atlas_json() -> Result<impl IntoResponse, String> {
    let bytes = tokio::fs::read(trueos_twemoji_path("atlas-set.json"))
        .await
        .map_err(|err| format!("TRUEOS twemoji atlas set unavailable: {err}"))?;
    Ok(([(header::CONTENT_TYPE, "application/json")], bytes))
}

async fn localcoder_status() -> Json<localcoder::LocalCoderStatusResponse> {
    Json(localcoder::status().await)
}

async fn localcoder_chat(
    Json(request): Json<localcoder::LocalCoderChatRequest>,
) -> Result<Json<localcoder::LocalCoderChatResponse>, (StatusCode, Json<localcoder::LocalCoderError>)>
{
    localcoder::chat(request)
        .await
        .map(Json)
        .map_err(|err| (localcoder_status_code(err.kind), Json(err)))
}

async fn active_project(State(state): State<AppState>) -> impl IntoResponse {
    let active = state.active.lock().await;
    Json(active.clone())
}

async fn list_assets(State(state): State<AppState>) -> Result<Json<AssetsResponse>, String> {
    let project = active_project_or_err(&state).await?;
    list_assets_for_project(project.root.join("assets")).await
}

async fn list_assets_for_project(root: PathBuf) -> Result<Json<AssetsResponse>, String> {
    let mut assets = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir(&root).await {
        while let Some(entry) = entries.next_entry().await.map_err(|err| err.to_string())? {
            let metadata = entry.metadata().await.map_err(|err| err.to_string())?;
            if !metadata.is_file() {
                continue;
            }
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Ok(extension) = asset_extension(name) else {
                continue;
            };
            assets.push(AssetEntry {
                name: name.to_string(),
                path: PathBuf::from("assets").join(name),
                extension: extension.to_string(),
                size: metadata.len(),
                warning: asset_warning(extension),
            });
        }
    }
    assets.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(Json(AssetsResponse { root, assets }))
}

async fn import_asset(
    State(state): State<AppState>,
    Json(request): Json<ImportAssetRequest>,
) -> Result<Json<AssetsResponse>, String> {
    let project = active_project_or_err(&state).await?;
    let name = sanitize_asset_name(&request.name)?;
    let _extension = asset_extension(&name)?;
    let bytes = BASE64
        .decode(request.contents_base64.as_bytes())
        .map_err(|err| format!("asset contents are not valid base64: {err}"))?;
    let assets_dir = project.root.join("assets");
    tokio::fs::create_dir_all(&assets_dir)
        .await
        .map_err(|err| format!("failed to create assets directory: {err}"))?;
    tokio::fs::write(assets_dir.join(&name), bytes)
        .await
        .map_err(|err| format!("failed to write asset `{name}`: {err}"))?;
    list_assets_for_project(assets_dir).await
}

async fn list_project_files(
    State(state): State<AppState>,
) -> Result<Json<file_store::ProjectFileListResponse>, String> {
    let project = active_project_or_err(&state).await?;
    let files = file_store::list_project_files(project.root)
        .await
        .map_err(|err| err.to_string())?;
    Ok(Json(files))
}

async fn read_project_file_query(
    State(state): State<AppState>,
    Query(query): Query<ProjectFileQuery>,
) -> Result<Json<file_store::ProjectFileReadResponse>, String> {
    read_project_file_inner(&state, query.path).await
}

async fn read_project_file_path(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Json<file_store::ProjectFileReadResponse>, String> {
    read_project_file_inner(&state, PathBuf::from(path)).await
}

async fn write_project_file_json(
    State(state): State<AppState>,
    Json(request): Json<WriteProjectFileRequest>,
) -> Result<Json<file_store::ProjectFileReadResponse>, String> {
    let path = request.path.ok_or("path is required")?;
    write_project_file_inner(&state, path, request.contents, request.create_dirs).await
}

async fn write_project_file_path(
    State(state): State<AppState>,
    Path(path): Path<String>,
    Json(request): Json<WriteProjectFileRequest>,
) -> Result<Json<file_store::ProjectFileReadResponse>, String> {
    write_project_file_inner(
        &state,
        request.path.unwrap_or_else(|| PathBuf::from(path)),
        request.contents,
        request.create_dirs,
    )
    .await
}

async fn read_project_file_inner(
    state: &AppState,
    path: PathBuf,
) -> Result<Json<file_store::ProjectFileReadResponse>, String> {
    let project = active_project_or_err(state).await?;
    let file = file_store::read_project_file(&project.root, path)
        .await
        .map_err(|err| err.to_string())?;
    Ok(Json(file))
}

async fn write_project_file_inner(
    state: &AppState,
    path: PathBuf,
    contents: String,
    create_dirs: bool,
) -> Result<Json<file_store::ProjectFileReadResponse>, String> {
    let project = active_project_or_err(state).await?;
    let file = file_store::write_project_file(&project.root, path, contents, create_dirs)
        .await
        .map_err(|err| err.to_string())?;
    Ok(Json(file))
}

async fn active_project_or_err(state: &AppState) -> Result<RadsProject, String> {
    state
        .active
        .lock()
        .await
        .clone()
        .ok_or("no active project".to_string())
}

async fn new_project(
    State(state): State<AppState>,
    Json(request): Json<NewProjectRequest>,
) -> Result<Json<NewProjectResponse>, String> {
    let project = match request.template_id.as_deref() {
        Some(template_id) if !template_id.is_empty() => {
            generator::create_project_from_template(&state.workspace, &request.name, template_id)
        }
        _ => generator::create_project(&state.workspace, &request.name),
    }
    .map_err(|err| err.to_string())?;
    *state.active.lock().await = Some(project.clone());
    refresh_watch(&state, Some(project.root.clone())).await?;
    Ok(Json(NewProjectResponse { project }))
}

async fn load_project(
    State(state): State<AppState>,
    Json(request): Json<LoadProjectRequest>,
) -> Result<Json<ProjectResponse>, String> {
    let project_file = resolve_project_file(&state.workspace, request.path).await?;
    let body = tokio::fs::read_to_string(&project_file)
        .await
        .map_err(|err| format!("failed to read {}: {err}", project_file.display()))?;
    let mut project: RadsProject = serde_json::from_str(&body)
        .map_err(|err| format!("failed to parse {}: {err}", project_file.display()))?;
    if let Some(root) = project_file.parent() {
        project.root = root.to_path_buf();
    }

    *state.active.lock().await = Some(project.clone());
    refresh_watch(&state, Some(project.root.clone())).await?;
    Ok(Json(ProjectResponse { project }))
}

async fn save_project(State(state): State<AppState>) -> Result<Json<ProjectResponse>, String> {
    let project = state
        .active
        .lock()
        .await
        .clone()
        .ok_or("no active project")?;
    generator::write_project_files(&project).map_err(|err| err.to_string())?;
    Ok(Json(ProjectResponse { project }))
}

async fn update_ui_description(
    State(state): State<AppState>,
    Json(request): Json<UpdateUiDescriptionRequest>,
) -> Result<Json<ProjectResponse>, String> {
    let mut project = state
        .active
        .lock()
        .await
        .clone()
        .ok_or("no active project")?;
    let window = if let Some(window_id) = request.window_id {
        project
            .windows
            .iter_mut()
            .find(|window| window.id == window_id)
            .ok_or("window not found")?
    } else {
        project
            .windows
            .first_mut()
            .ok_or("project has no windows")?
    };

    let mut description = request
        .ui_description
        .unwrap_or_else(|| window.ui_description.clone());
    if let Some(html) = request.html {
        description.html = html;
    }
    if let Some(css) = request.css {
        description.css = css;
    }
    window.ui_description = description.clone();
    if let Some(summary) = request.description {
        project.blueprint.description = summary;
    }

    generator::write_project_files(&project).map_err(|err| err.to_string())?;
    *state.active.lock().await = Some(project.clone());
    Ok(Json(ProjectResponse { project }))
}

async fn new_window(
    State(state): State<AppState>,
    Json(request): Json<NewWindowRequest>,
) -> Result<Json<RadsProject>, String> {
    let mut project = state
        .active
        .lock()
        .await
        .clone()
        .ok_or("no active project")?;
    if !project.app_kind.has_ui2() {
        return Err("only UI2 apps can create UI2 windows".to_string());
    }

    let index = project.windows.len() + 1;
    let caption = request
        .caption
        .filter(|caption| !caption.trim().is_empty())
        .unwrap_or_else(|| format!("Window {index}"));
    let requested_name = request
        .name
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| caption.clone());
    let name = unique_window_name(&project, &requested_name);
    let offset = ((index - 1) as i32 * 28).min(160);
    let mut window = Ui2Window::named_window(name, caption, 96 + offset, 96 + offset, 680, 420);
    window.ui_description = Ui2HtmlCssDescription {
        html: format!(
            r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{}</title>
  <link rel="stylesheet" href="{}.css">
</head>
<body>
  <main class="ui2-window">
    <h1>{}</h1>
    <p>Design this UI2 window in RADS.</p>
  </main>
</body>
</html>
"#,
            escape_html_text(&window.caption),
            crate::model::slugify(&window.name),
            escape_html_text(&window.caption)
        ),
        css: r#"html,
body {
  margin: 0;
  min-height: 100%;
  font-family: system-ui, "Twitter Color Emoji", "Twemoji Mozilla", sans-serif;
  background: #f4f7f8;
  color: #182024;
}

.ui2-window {
  display: grid;
  gap: 12px;
  padding: 28px;
}
"#
        .to_string(),
    };
    project.windows.push(window);
    generator::write_project_files(&project).map_err(|err| err.to_string())?;
    *state.active.lock().await = Some(project.clone());
    Ok(Json(project))
}

async fn add_control(
    State(state): State<AppState>,
    Json(request): Json<designer::AddControlRequest>,
) -> Result<Json<RadsProject>, String> {
    let mut active = state.active.lock().await;
    let project = active.as_mut().ok_or("no active project")?;
    let window = project
        .windows
        .iter_mut()
        .find(|window| window.id == request.window_id)
        .ok_or("window not found")?;
    designer::add_control(window, request).ok_or("control could not be added")?;
    generator::write_project_files(project).map_err(|err| err.to_string())?;
    Ok(Json(project.clone()))
}

async fn update_window(
    State(state): State<AppState>,
    Json(request): Json<UpdateWindowRequest>,
) -> Result<Json<RadsProject>, String> {
    let mut project = state
        .active
        .lock()
        .await
        .clone()
        .ok_or("no active project")?;
    let window = project
        .windows
        .iter_mut()
        .find(|window| window.id == request.window_id)
        .ok_or("window not found")?;

    if let Some(window_patch) = request.window {
        window.name = window_patch.name;
        window.caption = window_patch.caption;
        window.title_twemoji = window_patch.title_twemoji;
        window.geometry = window_patch.geometry;
        window.decorations = window_patch.decorations;
        window.controls = window_patch.controls;
    }
    if let Some(name) = request.name {
        window.name = name;
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
    for property in request.properties {
        apply_window_property(window, property)?;
    }

    generator::write_project_files(&project).map_err(|err| err.to_string())?;
    *state.active.lock().await = Some(project.clone());
    Ok(Json(project))
}

async fn update_control(
    State(state): State<AppState>,
    Json(request): Json<UpdateControlRequest>,
) -> Result<Json<RadsProject>, String> {
    let mut project = state
        .active
        .lock()
        .await
        .clone()
        .ok_or("no active project")?;
    let window = project
        .windows
        .iter_mut()
        .find(|window| window.id == request.window_id)
        .ok_or("window not found")?;

    if let Some(control_patch) = request.control {
        let Some(control) = window
            .controls
            .iter_mut()
            .find(|control| control.id == request.control_id)
        else {
            return Err("control not found".to_string());
        };
        *control = control_patch;
    } else {
        let updated = designer::update_control(
            window,
            designer::UpdateControlRequest {
                window_id: request.window_id,
                control_id: request.control_id,
                name: request.name,
                caption: request.caption,
                geometry: request.geometry,
                properties: request.properties,
                events: request.events,
            },
        );
        if !updated {
            return Err("control not found".to_string());
        }
    }

    generator::write_project_files(&project).map_err(|err| err.to_string())?;
    *state.active.lock().await = Some(project.clone());
    Ok(Json(project))
}

async fn delete_control(
    State(state): State<AppState>,
    Path(control_id): Path<uuid::Uuid>,
) -> Result<Json<RadsProject>, String> {
    let mut project = state
        .active
        .lock()
        .await
        .clone()
        .ok_or("no active project")?;
    let mut deleted = false;
    for window in &mut project.windows {
        let before = window.controls.len();
        window.controls.retain(|control| control.id != control_id);
        deleted |= window.controls.len() != before;
    }
    if !deleted {
        return Err("control not found".to_string());
    }

    generator::write_project_files(&project).map_err(|err| err.to_string())?;
    *state.active.lock().await = Some(project.clone());
    Ok(Json(project))
}

async fn runtime(State(state): State<AppState>) -> impl IntoResponse {
    Json(runtime_response(&state).await)
}

async fn update_runtime(
    State(state): State<AppState>,
    Json(request): Json<RuntimeRequest>,
) -> Result<Json<RuntimeResponse>, String> {
    if let Some(full_auto) = request.full_auto {
        state.full_auto.store(full_auto, Ordering::Relaxed);
    }

    if let Some(watch) = request.watch {
        let active_root = state
            .active
            .lock()
            .await
            .as_ref()
            .map(|project| project.root.clone());
        set_watch(&state, watch, active_root).await?;
    }

    Ok(Json(runtime_response(&state).await))
}

async fn resolve_project_file(
    workspace: &std::path::Path,
    path: PathBuf,
) -> Result<PathBuf, String> {
    let target = if path.is_absolute() {
        path
    } else {
        workspace.join(path)
    };
    let metadata = tokio::fs::metadata(&target)
        .await
        .map_err(|err| format!("failed to stat {}: {err}", target.display()))?;
    if metadata.is_dir() {
        Ok(target.join("rads.project.json"))
    } else {
        Ok(target)
    }
}

async fn runtime_response(state: &AppState) -> RuntimeResponse {
    let active_project = state
        .active
        .lock()
        .await
        .as_ref()
        .map(|project| project.root.clone());
    let build_possible = if let Some(project) = active_project.as_ref() {
        tokio::fs::metadata(project.join("Cargo.toml"))
            .await
            .is_ok()
            && tokio::fs::metadata(project.join("src/main.rs"))
                .await
                .is_ok()
    } else {
        false
    };
    let runtime = state.runtime.lock().await;
    let activity = state.jobs.current_activity().await;
    RuntimeResponse {
        full_auto: state.full_auto.load(Ordering::Relaxed),
        watch: runtime.watch,
        active_project,
        watched_project: runtime.watched_project.clone(),
        build_possible,
        current_stage: activity
            .as_ref()
            .and_then(|activity| activity.current_stage),
        current_status: activity.as_ref().map(|activity| activity.status),
        current_job_id: activity.map(|activity| activity.job_id),
    }
}

async fn refresh_watch(state: &AppState, project: Option<PathBuf>) -> Result<(), String> {
    let enabled = state.runtime.lock().await.watch;
    if enabled {
        set_watch(state, true, project).await?;
    }
    Ok(())
}

async fn set_watch(
    state: &AppState,
    enabled: bool,
    project: Option<PathBuf>,
) -> Result<(), String> {
    if !enabled {
        let mut runtime = state.runtime.lock().await;
        runtime.watch = false;
        runtime.watched_project = None;
        runtime.watcher = None;
        return Ok(());
    }

    let Some(project) = project else {
        let mut runtime = state.runtime.lock().await;
        runtime.watch = true;
        runtime.watched_project = None;
        runtime.watcher = None;
        return Ok(());
    };

    let active_watcher =
        watcher::watch_project(project.clone(), state.jobs.clone(), state.full_auto.clone())
            .await
            .map_err(|err| format!("failed to watch {}: {err}", project.display()))?;
    let mut runtime = state.runtime.lock().await;
    runtime.watch = true;
    runtime.watched_project = Some(project);
    runtime.watcher = Some(active_watcher);
    Ok(())
}

fn apply_window_property(window: &mut Ui2Window, property: Property) -> Result<(), String> {
    let key = property.key.replace('_', "-").to_ascii_lowercase();
    match key.as_str() {
        "name" => window.name = property.value,
        "caption" => window.caption = property.value,
        "title-twemoji" | "title-icon" | "twemoji" => {
            window.title_twemoji = (!property.value.trim().is_empty()).then_some(property.value)
        }
        "x" | "geometry.x" => window.geometry.x = parse_i32(&property.key, &property.value)?,
        "y" | "geometry.y" => window.geometry.y = parse_i32(&property.key, &property.value)?,
        "w" | "width" | "geometry.w" | "geometry.width" => {
            window.geometry.w = parse_u32(&property.key, &property.value)?
        }
        "h" | "height" | "geometry.h" | "geometry.height" => {
            window.geometry.h = parse_u32(&property.key, &property.value)?
        }
        "titlebar" | "decorations.titlebar" => {
            window.decorations.titlebar = parse_bool(&property.key, &property.value)?
        }
        "close" | "decorations.close" => {
            window.decorations.close = parse_bool(&property.key, &property.value)?
        }
        "minimize" | "decorations.minimize" => {
            window.decorations.minimize = parse_bool(&property.key, &property.value)?
        }
        "maximize" | "decorations.maximize" => {
            window.decorations.maximize = parse_bool(&property.key, &property.value)?
        }
        "resizable" | "decorations.resizable" => {
            window.decorations.resizable = parse_bool(&property.key, &property.value)?
        }
        "always-on-top" | "decorations.always-on-top" => {
            window.decorations.always_on_top = parse_bool(&property.key, &property.value)?
        }
        _ => return Err(format!("unknown window property `{}`", property.key)),
    }
    Ok(())
}

fn template_tags(app_kind: AppKind) -> Vec<&'static str> {
    match app_kind {
        AppKind::Ui2 => vec!["ui2", "blueprint", "package"],
        AppKind::Service => vec!["service", "background", "blueprint", "package"],
        AppKind::Shell => vec!["shell", "command", "blueprint", "package"],
    }
}

fn template_files(app_kind: AppKind, has_window: bool) -> Vec<&'static str> {
    let mut files = vec![
        "rads.project.json",
        "app.blueprint.json",
        "package/package.blueprint.json",
        "package/manifest.trueos.json",
        "Cargo.toml",
        "src/main.rs",
        "src/events.rs",
        "README.md",
    ];
    if app_kind.has_ui2() && has_window {
        files.extend([
            "ui/main.ui2",
            "ui/main.ui2.json",
            "ui/windows/*.ui2.json",
            "ui/windows/*.html",
            "ui/windows/*.css",
            "ui/index.html",
            "ui/styles.css",
            "src/ui.rs",
        ]);
    }
    files
}

fn trueos_twemoji_dir() -> PathBuf {
    FsPath::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|parent| parent.join("TRUEOS/src/gfx/althlasfont/twemoji-1x"))
        .unwrap_or_else(|| PathBuf::from("../TRUEOS/src/gfx/althlasfont/twemoji-1x"))
}

fn trueos_twemoji_path(file: &str) -> PathBuf {
    trueos_twemoji_dir().join(file)
}

fn unique_window_name(project: &RadsProject, requested: &str) -> String {
    let stem = window_name_stem(requested);
    if !project
        .windows
        .iter()
        .any(|window| window.name.eq_ignore_ascii_case(&stem))
    {
        return stem;
    }
    for index in 2..1000 {
        let candidate = format!("{stem}{index}");
        if !project
            .windows
            .iter()
            .any(|window| window.name.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
    }
    format!("{stem}{}", project.windows.len() + 1)
}

fn window_name_stem(input: &str) -> String {
    let mut name = String::new();
    let mut capitalize_next = true;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize_next {
                name.extend(ch.to_uppercase());
                capitalize_next = false;
            } else {
                name.push(ch);
            }
        } else {
            capitalize_next = true;
        }
    }
    if name.is_empty() {
        name.push_str("Window");
    }
    if name.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        name.insert_str(0, "Window");
    }
    if !name.to_ascii_lowercase().contains("window") {
        name.push_str("Window");
    }
    name
}

fn sanitize_asset_name(input: &str) -> Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("asset name is required".to_string());
    }
    let path = FsPath::new(trimmed);
    if path.components().count() != 1 {
        return Err("asset name cannot contain path separators".to_string());
    }
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return Err("asset name is not valid UTF-8".to_string());
    };
    Ok(name.to_string())
}

fn asset_extension(name: &str) -> Result<&'static str, String> {
    let extension = FsPath::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "jpg" => Ok("jpg"),
        "jpeg" => Ok("jpeg"),
        "png" => Ok("png"),
        "svg" => Ok("svg"),
        "bmp" => Ok("bmp"),
        _ => Err("asset type must be JPG, JPEG, PNG, SVG, or BMP".to_string()),
    }
}

fn asset_warning(extension: &str) -> Option<&'static str> {
    (extension == "bmp").then_some("BMP assets usually waste package space; prefer PNG or SVG.")
}

fn localcoder_status_code(kind: localcoder::LocalCoderErrorKind) -> StatusCode {
    match kind {
        localcoder::LocalCoderErrorKind::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        localcoder::LocalCoderErrorKind::InvalidArgs => StatusCode::BAD_REQUEST,
        localcoder::LocalCoderErrorKind::TimedOut => StatusCode::GATEWAY_TIMEOUT,
        localcoder::LocalCoderErrorKind::SpawnFailed
        | localcoder::LocalCoderErrorKind::Io
        | localcoder::LocalCoderErrorKind::ExitFailed => StatusCode::BAD_GATEWAY,
    }
}

fn escape_html_text(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn parse_i32(key: &str, value: &str) -> Result<i32, String> {
    value
        .parse()
        .map_err(|err| format!("invalid integer for `{key}`: {err}"))
}

fn parse_u32(key: &str, value: &str) -> Result<u32, String> {
    value
        .parse()
        .map_err(|err| format!("invalid unsigned integer for `{key}`: {err}"))
}

fn parse_bool(key: &str, value: &str) -> Result<bool, String> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(format!("invalid boolean for `{key}`: {value}")),
    }
}

async fn list_jobs(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.jobs.list().await)
}

async fn run_job(
    State(state): State<AppState>,
    Json(request): Json<RunJobRequest>,
) -> Result<Json<RunJobResponse>, String> {
    let active = state.active.lock().await;
    let project = active.as_ref().ok_or("no active project")?;
    let requested_kind = request.kind.to_ascii_lowercase();
    let kind = match requested_kind.as_str() {
        "generate" => JobKind::Generate {
            project: project.root.clone(),
        },
        "check" => JobKind::Check {
            project: project.root.clone(),
        },
        "build" => JobKind::Build {
            project: project.root.clone(),
        },
        "pack" => JobKind::Pack {
            project: project.root.clone(),
        },
        "dist" => JobKind::Dist {
            project: project.root.clone(),
        },
        "auto" => JobKind::FullAuto {
            project: project.root.clone(),
        },
        "full-auto" => JobKind::FullAuto {
            project: project.root.clone(),
        },
        _ => {
            return Err(
                "unknown job kind; expected generate, check, build, pack, dist, or auto"
                    .to_string(),
            );
        }
    };
    let job_id = state.jobs.spawn(kind).await;
    Ok(Json(RunJobResponse { job_id }))
}

async fn events(
    State(state): State<AppState>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.jobs.subscribe();
    let stream = futures_util::stream::unfold(rx, |mut rx| async {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let payload = serde_json::to_string(&event).unwrap_or_default();
                    return Some((Ok(Event::default().event("job").data(payload)), rx));
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}
