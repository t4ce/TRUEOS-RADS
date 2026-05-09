const fallbackPalette = [
  { kind: "button", label: "Button", default_w: 120, default_h: 34, category: "Basics" },
  { kind: "label", label: "Label", default_w: 160, default_h: 24, category: "Basics" },
  { kind: "text-box", label: "Text Box", default_w: 180, default_h: 34, category: "Inputs" },
  { kind: "check-box", label: "Check Box", default_w: 160, default_h: 30, category: "Inputs" },
  { kind: "panel", label: "Panel", default_w: 240, default_h: 160, category: "Containers" },
  { kind: "list-box", label: "List Box", default_w: 220, default_h: 160, category: "Data" },
  { kind: "canvas", label: "Canvas", default_w: 260, default_h: 180, category: "Visual" },
  { kind: "toolbar", label: "Toolbar", default_w: 360, default_h: 42, category: "Navigation" },
];

const defaultRustPaths = ["src/main.rs", "src/ui.rs", "src/events.rs"];

const state = {
  project: null,
  palette: fallbackPalette,
  templates: [],
  templatesLoaded: false,
  projects: [],
  projectsLoaded: false,
  selectedControlId: null,
  selectedWindowId: null,
  selectedPaletteCategory: "Basics",
  activeEditor: "design",
  logs: ["TRUEOS RADS ready."],
  jobs: [],
  fullAuto: false,
  snapToGrid: true,
  showDecorations: true,
  showGuides: true,
  grid: 12,
  zoom: 1,
  backendOnline: true,
  unavailableRoutes: new Set(),
  autoTimer: null,
  uiPersistTimer: null,
  fileRefreshTimer: null,
  pendingFileChangePaths: [],
  drag: null,
  newProjectOpen: false,
  openProjectOpen: false,
  newProjectDraft: { name: "Hello UI2", templateId: "" },
  glyphPickerOpen: false,
  glyphPickerTarget: null,
  glyphPickerQuery: "",
  trueosTwemoji: { available: false, fontStack: "Twitter Color Emoji, Twemoji Mozilla, Noto Color Emoji, Apple Color Emoji, Segoe UI Emoji", atlas: null, atlasPng: "" },
  assets: [],
  assetRoutesOnline: false,
  assetSaveLabel: "Local",
  uiHtml: "",
  uiCss: "",
  uiSourceProjectKey: "",
  uiSaveLabel: "Local",
  rustFiles: [],
  selectedRustPath: "src/main.rs",
  rustLoading: false,
  rustSaveLabel: "Local",
  rustRoutesOnline: false,
};

const app = document.querySelector("#app");

async function api(path, options = {}) {
  const response = await fetch(path, {
    headers: { "content-type": "application/json", ...(options.headers || {}) },
    ...options,
  });
  if (!response.ok) {
    throw new Error(`${response.status} ${response.statusText}: ${await response.text()}`);
  }
  const text = await response.text();
  return text ? JSON.parse(text) : null;
}

async function flexibleApi(path, options = {}) {
  const response = await fetch(path, {
    headers: { "content-type": "application/json", ...(options.headers || {}) },
    ...options,
  });
  if (!response.ok) {
    throw new Error(`${response.status} ${response.statusText}: ${await response.text()}`);
  }
  const text = await response.text();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

async function probeApi(path, options = {}) {
  try {
    return await flexibleApi(path, options);
  } catch {
    return null;
  }
}

async function tryApi(path, options = {}) {
  try {
    return { ok: true, data: await flexibleApi(path, options) };
  } catch (error) {
    return { ok: false, error };
  }
}

async function optionalApi(path, options = {}, fallbackMessage = "") {
  try {
    return await api(path, options);
  } catch (error) {
    if (!state.unavailableRoutes.has(`${options.method || "GET"} ${path}`)) {
      state.unavailableRoutes.add(`${options.method || "GET"} ${path}`);
      state.logs.push(fallbackMessage || `${path} unavailable; keeping changes local.`);
    }
    return null;
  }
}

async function init() {
  try {
    const [palette, project, jobs, templates, twemoji, projects] = await Promise.all([
      api("/api/palette"),
      api("/api/project"),
      optionalApi("/api/jobs", {}, "Job history unavailable until the backend is running."),
      probeApi("/api/templates"),
      probeApi("/api/assets/trueos-twemoji"),
      probeApi("/api/projects"),
    ]);
    state.palette = normalizePalette(palette);
    state.templates = normalizeTemplates(templates);
    state.projects = normalizeProjects(projects);
    await hydrateTrueosTwemoji(twemoji);
    state.templatesLoaded = true;
    state.projectsLoaded = Boolean(projects);
    ensurePaletteCategory();
    state.project = project;
    state.jobs = Array.isArray(jobs) ? jobs : [];
    if (project?.windows?.length) {
      state.selectedWindowId = project.windows[0].id;
    }
    hydrateProjectEditors();
    loadAssetsForProject();
  } catch (error) {
    state.backendOnline = false;
    state.logs.push("Backend not reachable; running in local design mode.");
    state.project = createLocalProject("Local UI2 App");
    state.selectedWindowId = state.project.windows[0].id;
    state.projects = [];
    state.projectsLoaded = false;
    ensurePaletteCategory();
    hydrateProjectEditors();
    loadAssetsForProject();
  }
  connectEvents();
  render();
  loadRustFilesForProject();
}

function connectEvents() {
  if (!window.EventSource || !state.backendOnline) return;
  const source = new EventSource("/api/events");
  source.addEventListener("job", (event) => {
    const payload = JSON.parse(event.data);
    state.logs.push(`[${payload.status}] ${payload.line}`);
    refreshJobs().finally(render);
  });
  source.addEventListener("project-file", (event) => {
    handleProjectFileEvent(JSON.parse(event.data));
  });
  source.addEventListener("error", () => {
    state.backendOnline = false;
  });
}

function handleProjectFileEvent(payload) {
  const paths = normalizeProjectFileEventPaths(payload);
  if (!paths.length) return;

  state.pendingFileChangePaths = Array.from(new Set([
    ...state.pendingFileChangePaths,
    ...paths,
  ]));
  state.logs.push(`Project files changed: ${summarizePaths(paths)}`);
  window.clearTimeout(state.fileRefreshTimer);
  state.fileRefreshTimer = window.setTimeout(refreshChangedProjectFiles, 350);
  render();
}

function normalizeProjectFileEventPaths(payload) {
  const paths = Array.isArray(payload?.paths) ? payload.paths : [];
  return paths
    .map((path) => String(path || "").replace(/\\/g, "/"))
    .filter(Boolean);
}

function summarizePaths(paths) {
  const visible = paths.slice(0, 3).join(", ");
  return paths.length > 3 ? `${visible}, +${paths.length - 3}` : visible;
}

async function refreshChangedProjectFiles() {
  if (!state.project) return;
  const paths = state.pendingFileChangePaths.splice(0);
  if (!paths.length) return;

  try {
    const reloadProject = paths.some(isProjectModelPath);
    const reloadRust = paths.some((path) => path.endsWith(".rs"));
    const reloadAssets = paths.some((path) => path.startsWith("assets/"));

    if (reloadProject) {
      await reloadActiveProjectFromDisk();
    }
    if (reloadRust) {
      await loadRustFilesForProject();
    }
    if (reloadAssets) {
      await loadAssetsForProject();
    }
    await refreshProjects();
  } catch (error) {
    state.logs.push(`Project file refresh failed; ${shortError(error)}`);
  }
  render();
}

function isProjectModelPath(path) {
  return path === "rads.project.json"
    || path === "app.blueprint.json"
    || path === "package/package.blueprint.json"
    || path.startsWith("ui/");
}

async function reloadActiveProjectFromDisk() {
  if (!state.project?.root) return;
  const selectedWindowId = state.selectedWindowId;
  const selectedControlId = state.selectedControlId;
  const response = await api("/api/project/load", {
    method: "POST",
    body: JSON.stringify({ path: state.project.root }),
  });
  state.project = response.project;
  state.selectedWindowId = state.project.windows?.some((window) => window.id === selectedWindowId)
    ? selectedWindowId
    : state.project.windows?.[0]?.id || null;
  state.selectedControlId = selectedControlId && selectedWindow()?.controls?.some((control) => control.id === selectedControlId)
    ? selectedControlId
    : null;
  hydrateUiSourcesForWindow();
}

function openNewProject() {
  state.newProjectDraft = {
    name: state.project?.name || "Hello UI2",
    templateId: state.templates[0]?.id || "",
  };
  state.newProjectOpen = true;
  render();
}

function closeNewProject() {
  state.newProjectOpen = false;
  render();
}

function openProjectPicker() {
  state.openProjectOpen = true;
  refreshProjects().finally(render);
  render();
}

function closeOpenProject() {
  state.openProjectOpen = false;
  render();
}

async function newProject(form) {
  const data = new FormData(form);
  const name = String(data.get("projectName") || "").trim();
  if (!name) return;
  const templateId = String(data.get("templateId") || "");
  const template = state.templates.find((item) => item.id === templateId) || null;
  try {
    const response = await api("/api/project", {
      method: "POST",
      body: JSON.stringify({ name, template_id: templateId || undefined }),
    });
    state.project = response.project;
    state.backendOnline = true;
    state.logs.push(`Created ${response.project.name}`);
    await refreshProjects();
  } catch (error) {
    state.project = createLocalProject(name);
    applyTemplateToLocalProject(template);
    state.logs.push(`Created local project ${name}; backend create route was unavailable.`);
  }
  state.selectedWindowId = state.project.windows[0]?.id || null;
  state.selectedControlId = null;
  state.newProjectOpen = false;
  hydrateProjectEditors(template);
  loadAssetsForProject();
  loadRustFilesForProject();
  render();
}

async function loadProject(path) {
  if (!path) return;
  try {
    const response = await api("/api/project/load", {
      method: "POST",
      body: JSON.stringify({ path }),
    });
    state.project = response.project;
    state.backendOnline = true;
    state.selectedWindowId = state.project.windows?.[0]?.id || null;
    state.selectedControlId = null;
    state.selectedRustPath = "src/main.rs";
    state.activeEditor = isUi2Project(state.project) ? "design" : "rust";
    state.openProjectOpen = false;
    hydrateProjectEditors();
    state.logs.push(`Loaded ${state.project.name}`);
    render();
    await Promise.all([
      loadAssetsForProject(),
      loadRustFilesForProject(),
      refreshJobs(),
      refreshProjects(),
    ]);
  } catch (error) {
    state.logs.push(`Project load failed; ${shortError(error)}`);
  }
  render();
}

async function addControl(kind) {
  const activeWindow = selectedWindow();
  if (!activeWindow) return;
  const paletteItem = state.palette.find((item) => item.kind === kind) || fallbackPalette[0];
  const x = snap(36 + activeWindow.controls.length * 14);
  const y = snap(62 + activeWindow.controls.length * 12);
  try {
    state.project = await api("/api/project/control", {
      method: "POST",
      body: JSON.stringify({ window_id: activeWindow.id, kind, x, y }),
    });
    state.backendOnline = true;
    const updatedWindow = selectedWindow() || state.project.windows[0];
    state.selectedWindowId = updatedWindow.id;
    state.selectedControlId = updatedWindow.controls.at(-1)?.id || null;
    state.logs.push(`Added ${paletteItem.label}`);
  } catch (error) {
    const control = createLocalControl(kind, paletteItem, x, y, activeWindow);
    activeWindow.controls.push(control);
    state.selectedControlId = control.id;
    state.logs.push(`Added ${paletteItem.label} locally; backend add route unavailable.`);
  }
  scheduleAutoJob("control add");
  render();
}

async function addWindow() {
  if (!state.project || !isUi2Project()) return;
  const next = (state.project.windows?.length || 0) + 1;
  const caption = `Window ${next}`;
  try {
    state.project = await api("/api/project/window/new", {
      method: "POST",
      body: JSON.stringify({ caption }),
    });
    state.backendOnline = true;
    const created = state.project.windows.at(-1);
    state.selectedWindowId = created?.id || state.project.windows[0]?.id || null;
    state.logs.push(`Added ${created?.name || caption}`);
  } catch (error) {
    const window = createLocalWindow(caption, next);
    state.project.windows = state.project.windows || [];
    state.project.windows.push(window);
    state.selectedWindowId = window.id;
    state.logs.push(`Added ${caption} locally; backend window route unavailable.`);
  }
  state.selectedControlId = null;
  hydrateUiSourcesForWindow();
  loadRustFilesForProject();
  scheduleAutoJob("window add");
  render();
}

async function chooseTitleGlyph(glyph) {
  const window = selectedWindow();
  if (!window) return;
  window.title_twemoji = glyph || null;
  state.glyphPickerOpen = false;
  state.glyphPickerTarget = null;
  await persistWindow(window);
  scheduleAutoJob("window title glyph");
  render();
}

async function chooseControlGlyph(glyph) {
  const control = selectedControl();
  if (!control || control.kind !== "button") return;
  setControlProperty(control, "glyph", glyph || null);
  state.glyphPickerOpen = false;
  state.glyphPickerTarget = null;
  await persistControl(control);
  scheduleAutoJob("button glyph");
  render();
}

async function runJob(kind) {
  if (!state.project) return;
  try {
    const response = await api("/api/jobs", {
      method: "POST",
      body: JSON.stringify({ kind }),
    });
    state.backendOnline = true;
    state.logs.push(`Queued ${labelJob(kind)} job ${response.job_id}`);
    await refreshJobs();
  } catch (error) {
    state.logs.push(`${labelJob(kind)} job could not be queued; ${shortError(error)}`);
  }
  render();
}

async function refreshJobs() {
  const jobs = await optionalApi("/api/jobs", {}, "Job list unavailable; console will keep local messages.");
  if (Array.isArray(jobs)) state.jobs = jobs;
}

async function refreshProjects() {
  const payload = await probeApi("/api/projects");
  state.projects = normalizeProjects(payload);
  state.projectsLoaded = Boolean(payload);
  return state.projects;
}

function scheduleAutoJob(reason) {
  if (!state.fullAuto || !state.project) return;
  window.clearTimeout(state.autoTimer);
  state.autoTimer = window.setTimeout(() => {
    state.logs.push(`Full Auto reacting to ${reason}.`);
    runJob("auto");
  }, 700);
}

function selectedWindow() {
  if (!state.project?.windows?.length) return null;
  return state.project.windows.find((window) => window.id === state.selectedWindowId) || state.project.windows[0];
}

function selectedControl() {
  const activeWindow = selectedWindow();
  if (!activeWindow || !state.selectedControlId) return null;
  return activeWindow.controls.find((control) => control.id === state.selectedControlId) || null;
}

function selectWindow(windowId) {
  state.selectedWindowId = windowId;
  state.selectedControlId = null;
  hydrateUiSourcesForWindow();
  render();
}

function selectControl(controlId) {
  state.selectedControlId = controlId;
  render();
}

function render() {
  const project = state.project;
  const activeWindow = selectedWindow();
  const activeControl = selectedControl();

  app.innerHTML = `
    <main class="shell">
      <header class="topbar">
        <div class="brand">
          <span class="brand-mark">T</span>
          <span>TRUEOS RADS</span>
        </div>
        <button class="primary" data-action="new">New App</button>
        <button data-action="open-project">Open</button>
        <div class="topbar-divider"></div>
        <button data-job="check">Check</button>
        <button data-job="build">Build</button>
        <button data-job="pack">Pack</button>
        <label class="switch">
          <input type="checkbox" data-toggle="fullAuto" ${state.fullAuto ? "checked" : ""} />
          <span></span>
          Full Auto
        </label>
        <div class="status-pill ${state.backendOnline ? "online" : "offline"}">
          ${state.backendOnline ? "Backend online" : "Local mode"}
        </div>
      </header>

      <section class="workspace">
        <aside class="pane project-pane">
          ${renderProjectSidebar(project, activeWindow, activeControl)}
          ${isUi2Project(project) ? renderPalette() : renderAppKindPanel(project)}
        </aside>

        <section class="editor-area">
          ${renderEditorTabs()}
          <div class="editor-body">
            ${renderActiveEditor(project, activeWindow)}
          </div>
        </section>

        <aside class="pane inspector-pane">
          ${renderRightPane(activeWindow, activeControl)}
        </aside>
      </section>

      <section class="console">
        <div class="console-main">
          <div class="console-head">
            <strong>Job Console</strong>
            <button data-action="clear-log">Clear</button>
          </div>
          <div class="log">${state.logs.slice(-120).map(escapeHtml).join("<br />")}</div>
        </div>
        <div class="jobs">${renderJobs()}</div>
      </section>
      ${renderNewProjectModal()}
      ${renderOpenProjectModal()}
      ${renderGlyphPickerModal()}
    </main>`;

  bindEvents();
}

function renderEditorTabs() {
  const tabs = isUi2Project()
    ? [["design", "Design"], ["html", "HTML"], ["css", "CSS"], ["rust", "Rust"], ["assets", "Assets"]]
    : [["rust", "Rust"], ["assets", "Assets"]];
  if (!tabs.some(([id]) => id === state.activeEditor)) {
    state.activeEditor = tabs[0][0];
  }
  return `
    <div class="editor-tabs" role="tablist" aria-label="Project editors">
      ${tabs.map(([id, label]) => `
        <button role="tab" aria-selected="${state.activeEditor === id}" class="${state.activeEditor === id ? "active" : ""}" data-editor-tab="${id}">
          ${escapeHtml(label)}
        </button>
      `).join("")}
    </div>`;
}

function renderActiveEditor(project, activeWindow) {
  if (state.activeEditor === "html") return renderSourceEditor("html");
  if (state.activeEditor === "css") return renderSourceEditor("css");
  if (state.activeEditor === "rust") return renderRustEditor();
  if (state.activeEditor === "assets") return renderAssetsEditor();
  return renderDesignEditor(project, activeWindow);
}

function renderDesignEditor(project, activeWindow) {
  return `
    <section class="designer">
      <div class="designer-toolbar">
        <div>
          <strong>${project ? escapeHtml(project.name) : "No project"}</strong>
          <span>${activeWindow ? escapeHtml(activeWindow.caption) : "Create or load a UI2 window"}</span>
        </div>
        <div class="designer-tools">
          <label class="compact-check"><input type="checkbox" data-setting="snapToGrid" ${state.snapToGrid ? "checked" : ""} /> Snap</label>
          <label class="compact-check"><input type="checkbox" data-setting="showGuides" ${state.showGuides ? "checked" : ""} /> Guides</label>
          <label class="compact-check"><input type="checkbox" data-setting="showDecorations" ${state.showDecorations ? "checked" : ""} /> Deco</label>
          <select data-setting="zoom">
            ${[0.75, 0.9, 1, 1.15, 1.3].map((value) => `<option value="${value}" ${state.zoom === value ? "selected" : ""}>${Math.round(value * 100)}%</option>`).join("")}
          </select>
        </div>
      </div>
      <div class="design-stage ${state.showGuides ? "with-guides" : ""}" data-surface>
        ${activeWindow ? renderWindow(activeWindow) : renderEmpty()}
      </div>
    </section>`;
}

function renderSourceEditor(kind) {
  const isHtml = kind === "html";
  const value = isHtml ? state.uiHtml : state.uiCss;
  const label = isHtml ? "HTML" : "CSS";
  const sourceName = isHtml ? "index.html fragment" : "preview.css";
  return `
    <section class="source-workbench">
      <div class="source-pane">
        <div class="source-toolbar">
          <div>
            <strong>${label}</strong>
            <span>${escapeHtml(sourceName)}</span>
          </div>
          <span class="save-state" data-ui-status>${escapeHtml(state.uiSaveLabel)}</span>
        </div>
        <textarea class="code-editor" spellcheck="false" data-ui-source="${kind}">${escapeHtml(value)}</textarea>
      </div>
      <div class="preview-pane">
        <div class="source-toolbar">
          <div>
            <strong>Live Preview</strong>
            <span>HTML and CSS render together</span>
          </div>
        </div>
        <div class="live-preview">
          <iframe title="UI preview" data-live-preview sandbox srcdoc="${escapeHtml(composePreviewDocument())}"></iframe>
        </div>
      </div>
    </section>`;
}

function renderRustEditor() {
  const file = selectedRustFile();
  if (!state.project) {
    return `
      <section class="rust-workbench">
        <div class="empty-canvas">
          <b>No project loaded</b>
          <span>Create a UI2 app to edit generated Rust files.</span>
        </div>
      </section>`;
  }

  return `
    <section class="rust-workbench">
      <div class="source-toolbar">
        <div>
          <strong>Rust App Logic</strong>
          <span>${state.rustLoading ? "Loading source files" : `${state.rustFiles.length} files`}</span>
        </div>
        <div class="source-actions">
          <span class="save-state">${escapeHtml(state.rustSaveLabel)}</span>
          <button data-action="save-rust" ${file ? "" : "disabled"}>Save</button>
        </div>
      </div>
      <div class="file-tabs" role="tablist" aria-label="Rust files">
        ${state.rustFiles.map((item) => `
          <button role="tab" aria-selected="${item.path === state.selectedRustPath}" class="${item.path === state.selectedRustPath ? "active" : ""}" data-rust-file="${escapeHtml(item.path)}">
            ${escapeHtml(item.label || item.path)}${item.dirty ? " *" : ""}
          </button>
        `).join("")}
      </div>
      <textarea class="code-editor rust-editor" spellcheck="false" data-rust-editor ${file ? "" : "disabled"}>${escapeHtml(file?.content || "")}</textarea>
    </section>`;
}

function renderAssetsEditor() {
  if (!state.project) {
    return `
      <section class="assets-workbench">
        <div class="empty-canvas">
          <b>No project loaded</b>
          <span>Create an app to manage package assets.</span>
        </div>
      </section>`;
  }
  return `
    <section class="assets-workbench">
      <div class="source-toolbar">
        <div>
          <strong>Assets</strong>
          <span>JPG, JPEG, PNG, SVG, BMP</span>
        </div>
        <div class="source-actions">
          <span class="save-state">${escapeHtml(state.assetSaveLabel)}</span>
          <label class="file-pick">
            Import
            <input type="file" data-asset-import accept=".jpg,.jpeg,.png,.svg,.bmp,image/jpeg,image/png,image/svg+xml,image/bmp" />
          </label>
        </div>
      </div>
      <div class="asset-grid">
        ${state.assets.length ? state.assets.map(renderAssetCard).join("") : `
          <div class="empty-canvas">
            <b>No assets yet</b>
            <span>Import images for UI2 previews and package resources.</span>
          </div>`}
      </div>
    </section>`;
}

function renderAssetCard(asset) {
  const warning = asset.warning || (String(asset.extension).toLowerCase() === "bmp" ? "BMP assets usually waste package space; prefer PNG or SVG." : "");
  return `
    <article class="asset-card ${warning ? "warn" : ""}">
      <b>${escapeHtml(asset.name)}</b>
      <span>${escapeHtml(asset.path || `assets/${asset.name}`)}</span>
      <small>${escapeHtml(String(asset.extension || "").toUpperCase())} - ${formatBytes(asset.size || 0)}</small>
      ${warning ? `<em>${escapeHtml(warning)}</em>` : ""}
    </article>`;
}

function renderRightPane(activeWindow, activeControl) {
  if (state.activeEditor === "html" || state.activeEditor === "css") {
    return `
      <div class="pane-title">UI Description</div>
      ${renderUiDescriptionPanel()}`;
  }
  if (state.activeEditor === "rust") {
    return `
      <div class="pane-title">Source Files</div>
      ${renderRustSidePanel()}`;
  }
  if (state.activeEditor === "assets") {
    return `
      <div class="pane-title">Asset Manager</div>
      ${renderAssetSidePanel()}`;
  }
  return `
    <div class="pane-title">Object Inspector</div>
    ${renderInspector(activeWindow, activeControl)}`;
}

function renderUiDescriptionPanel() {
  if (!state.project) {
    return `<div class="inspector"><p>Create a project to describe HTML and CSS UI sources.</p></div>`;
  }
  const summary = state.project.ui_description?.summary || state.project.blueprint?.description || "";
  return `
    <div class="inspector">
      <div class="inspector-target">
        <b>${escapeHtml(state.project.blueprint?.display_name || state.project.name)}</b>
        <span>Project UI description</span>
      </div>
      <div class="description-box" data-ui-description>${escapeHtml(summary)}</div>
      <div class="metric-grid">
        <span><b data-html-count>${countHtmlElements(state.uiHtml)}</b> HTML nodes</span>
        <span><b data-css-count>${countCssRules(state.uiCss)}</b> CSS rules</span>
      </div>
    </div>`;
}

function renderRustSidePanel() {
  if (!state.project) {
    return `<div class="inspector"><p>Create a project to load source files.</p></div>`;
  }
  return `
    <div class="inspector">
      <div class="inspector-target">
        <b>${escapeHtml(state.project.slug || state.project.name)}</b>
        <span>${state.rustRoutesOnline ? "Backend file routes active" : "Local source fallback"}</span>
      </div>
      <div class="source-list">
        ${state.rustFiles.map((file) => `
          <button class="${file.path === state.selectedRustPath ? "active" : ""}" data-rust-file="${escapeHtml(file.path)}">
            <b>${escapeHtml(file.label || file.path)}</b>
            <span>${escapeHtml(file.path)}${file.dirty ? " - unsaved" : ""}</span>
          </button>
        `).join("")}
      </div>
    </div>`;
}

function renderAssetSidePanel() {
  if (!state.project) {
    return `<div class="inspector"><p>Create a project to manage assets.</p></div>`;
  }
  const bmpCount = state.assets.filter((asset) => String(asset.extension).toLowerCase() === "bmp").length;
  return `
    <div class="inspector">
      <div class="inspector-target">
        <b>${escapeHtml(state.project.slug || state.project.name)}</b>
        <span>${state.assetRoutesOnline ? "Backend asset store active" : "Local asset list"}</span>
      </div>
      <div class="metric-grid">
        <span><b>${state.assets.length}</b> assets</span>
        <span><b>${bmpCount}</b> BMP warnings</span>
      </div>
      <div class="description-box">
        Images are copied into the project assets directory. BMP is accepted, but RADS flags it because it usually makes packages larger than needed.
      </div>
    </div>`;
}

function renderNewProjectModal() {
  if (!state.newProjectOpen) return "";
  return `
    <div class="modal-backdrop">
      <form class="new-project-modal" data-new-project-form>
        <div class="modal-head">
          <strong>New TRUEOS App</strong>
          <button type="button" data-action="cancel-new">Close</button>
        </div>
        <label class="field">
          <span>Project Name</span>
          <input name="projectName" type="text" value="${escapeHtml(state.newProjectDraft.name)}" autocomplete="off" required />
        </label>
        ${state.templates.length ? `
          <label class="field">
            <span>Template</span>
            <select name="templateId">
              ${state.templates.map((template) => `
                <option value="${escapeHtml(template.id)}" ${template.id === state.newProjectDraft.templateId ? "selected" : ""}>
                  ${escapeHtml(template.name)} - ${escapeHtml(labelAppKind(template.app_kind))}
                </option>
              `).join("")}
            </select>
          </label>
          <div class="template-notes">
            ${state.templates.map((template) => `
              <div data-template-note="${escapeHtml(template.id)}" class="${template.id === state.newProjectDraft.templateId ? "active" : ""}">
                <b>${escapeHtml(labelAppKind(template.app_kind))}</b>
                <span>${escapeHtml(template.description || "Starter TRUEOS project template")}</span>
              </div>
            `).join("")}
          </div>
        ` : ""}
        <div class="modal-actions">
          <button type="button" data-action="cancel-new">Cancel</button>
          <button class="primary" type="submit">Create</button>
        </div>
      </form>
    </div>`;
}

function renderOpenProjectModal() {
  if (!state.openProjectOpen) return "";
  const projects = state.projects || [];
  return `
    <div class="modal-backdrop">
      <div class="open-project-modal">
        <div class="modal-head">
          <strong>Open RADS Project</strong>
          <button type="button" data-action="cancel-open-project">Close</button>
        </div>
        <div class="project-list">
          ${projects.length ? projects.map((project) => `
            <button type="button" class="project-choice ${isActiveProjectSummary(project) ? "active" : ""}" data-load-project="${escapeHtml(project.path)}">
              <span class="project-choice-main">
                <b>${escapeHtml(project.displayName || project.name)}</b>
                <small>${escapeHtml(labelAppKind(project.appKind))}</small>
              </span>
              <span class="project-choice-meta">
                <small>${escapeHtml(project.appId)}</small>
                <small>${escapeHtml(project.path)}</small>
                <small>${escapeHtml(`${project.windows} window${project.windows === 1 ? "" : "s"}${project.version ? `, v${project.version}` : ""}`)}</small>
              </span>
            </button>
          `).join("") : `
            <div class="empty-state">
              <b>No persisted projects found</b>
              <span>Saved RADS projects appear here once they exist in rads-workspace.</span>
            </div>
          `}
        </div>
        <div class="modal-actions">
          <span class="modal-status">${state.projectsLoaded ? `${projects.length} project${projects.length === 1 ? "" : "s"}` : "Project index unavailable"}</span>
          <button type="button" data-action="refresh-projects">Refresh</button>
        </div>
      </div>
    </div>`;
}

function renderGlyphPickerModal() {
  if (!state.glyphPickerOpen) return "";
  const glyphs = filteredTwemojiGlyphChoices();
  const target = state.glyphPickerTarget || { kind: "window-title" };
  const control = selectedControl();
  const active = target.kind === "control-glyph"
    ? controlProperty(control, "glyph")
    : selectedWindow()?.title_twemoji || "";
  const title = target.kind === "control-glyph" ? "Button Glyph" : "Title Glyph";
  return `
    <div class="modal-backdrop">
      <div class="glyph-modal">
        <div class="modal-head">
          <strong>${title}</strong>
          <button type="button" data-action="close-glyph-picker">Close</button>
        </div>
        <label class="field glyph-search">
          <span>Search</span>
          <input data-glyph-search type="search" value="${escapeHtml(state.glyphPickerQuery)}" placeholder="emoji or hex, e.g. 1f4be" autofocus />
        </label>
        <div class="glyph-grid">
          <button class="${!active ? "active" : ""}" data-glyph="">None</button>
          ${glyphs.length ? glyphs.map((glyph) => `
            <button class="${glyph === active ? "active" : ""}" data-glyph="${escapeHtml(glyph)}">
              ${renderTwemojiGlyph(glyph, `U+${glyph.codePointAt(0).toString(16).toUpperCase()}`)}
            </button>
          `).join("") : `<span class="glyph-empty">No matching Twemoji glyphs</span>`}
        </div>
      </div>
    </div>`;
}

function renderProjectSidebar(project, activeWindow, activeControl) {
  if (!project) {
    return `
      <section class="sidebar-section">
        <div class="pane-title">Project</div>
        <div class="empty-state">
          <b>No project loaded</b>
          <span>Create a UI2 app to begin designing.</span>
        </div>
      </section>`;
  }

  return `
    <section class="sidebar-section">
      <div class="pane-title">Project</div>
      <div class="project-card">
        <b>${escapeHtml(project.blueprint?.display_name || project.name)}</b>
        <span>${escapeHtml(labelAppKind(appKind(project)))}</span>
        <span>${escapeHtml(project.blueprint?.app_id || "dev.trueos.local")}</span>
        <span>${escapeHtml(project.root || "local workspace")}</span>
      </div>
      ${isUi2Project(project) ? `
        <div class="sidebar-actions">
          <button data-action="add-window">Add Window</button>
        </div>` : ""}
      <div class="tree">
        ${(project.windows || []).length ? (project.windows || []).map((window) => `
          <div>
            <button class="tree-row ${activeWindow?.id === window.id && !activeControl ? "active" : ""}" data-select-window="${window.id}">
              <span class="tree-icon">W</span>
              <span>${escapeHtml(window.name)}</span>
            </button>
            <div class="tree-children">
              ${(window.controls || []).map((control) => `
                <button class="tree-row ${activeControl?.id === control.id ? "active" : ""}" data-select-control="${control.id}">
                  <span class="tree-icon">${toolInitial(control.kind)}</span>
                  <span>${escapeHtml(control.name)}</span>
                </button>`).join("")}
            </div>
          </div>`).join("") : `<div class="tree-empty">No UI2 windows</div>`}
      </div>
      <div class="capabilities">
        ${(project.blueprint?.capabilities || []).map((capability) => `
          <span class="${capability.enabled ? "enabled" : ""}">${escapeHtml(capability.key)}</span>`).join("")}
      </div>
    </section>`;
}

function renderPalette() {
  const categories = paletteCategories();
  const items = state.palette.filter((item) => item.category === state.selectedPaletteCategory);
  return `
    <section class="sidebar-section palette-section">
      <div class="pane-title">Tool Palette</div>
      <div class="tabs">
        ${categories.map((category) => `
          <button class="${category === state.selectedPaletteCategory ? "active" : ""}" data-palette-category="${escapeHtml(category)}">${escapeHtml(category)}</button>
        `).join("")}
      </div>
      <div class="palette">
        ${items.map((item) => `
          <button class="tool" data-kind="${item.kind}">
            <span class="tool-icon">${toolInitial(item.kind)}</span>
            <span>
              <b>${escapeHtml(item.label)}</b>
              <small>${item.default_w} x ${item.default_h}</small>
            </span>
          </button>`).join("")}
      </div>
    </section>`;
}

function renderAppKindPanel(project) {
  if (!project) return "";
  return `
    <section class="sidebar-section">
      <div class="pane-title">App Surface</div>
      <div class="empty-state">
        <b>${escapeHtml(labelAppKind(appKind(project)))}</b>
        <span>This app has no UI2 design surface. Use Rust and Assets tabs for now.</span>
      </div>
    </section>`;
}

function renderEmpty() {
  return `
    <div class="empty-canvas">
      <b>Create a UI2 app</b>
      <span>The design surface, object tree, and inspector will activate here.</span>
    </div>`;
}

function renderWindow(window) {
  const rect = window.geometry;
  const scaledWidth = Math.round(rect.w * state.zoom);
  const scaledHeight = Math.round(rect.h * state.zoom);
  const decorations = normalizedDecorations(window.decorations);
  const titlebar = decorations.titlebar && state.showDecorations;
  const bottombar = decorations.bottom_bar && state.showDecorations;
  const chromeHeight = (titlebar ? 34 : 0) + (bottombar ? 22 : 0);
  return `
    <div class="surface-wrap" style="width:${scaledWidth}px;height:${scaledHeight}px">
      <div class="window-frame" style="width:${rect.w}px;height:${rect.h}px;--zoom:${state.zoom}">
        ${titlebar ? renderTitlebar(window) : ""}
        ${renderScrollbarPreview(window)}
        <div class="surface" data-window="${window.id}" style="height:calc(100% - ${chromeHeight}px)">
          ${(window.controls || []).map(renderControl).join("")}
        </div>
        ${bottombar ? renderBottombar(window) : ""}
      </div>
    </div>`;
}

function renderTitlebar(window) {
  const d = normalizedDecorations(window.decorations);
  return `
    <div class="titlebar">
      <span>${d.title_icon ? renderWindowTitleIcon(window) : ""}${escapeHtml(window.caption)}</span>
      <div class="traffic">
        ${d.toggle_composition ? renderWindowButton("🔄", "Toggle composition") : ""}
        ${d.fork ? renderWindowButton("🔀", "Fork") : ""}
        ${d.minimize ? renderWindowButton("➖", "Minimize") : ""}
        ${d.restore ? renderWindowButton("◽", "Restore") : ""}
        ${d.maximize ? renderWindowButton("⬜", "Maximize") : ""}
        ${d.preserve_vm ? renderWindowButton("💾", "Preserve VM") : ""}
        ${d.close ? renderWindowButton("❌", "Close") : ""}
      </div>
    </div>`;
}

function renderBottombar(window) {
  const d = normalizedDecorations(window.decorations);
  return `
    <div class="bottombar">
      <span></span>
      ${d.resize_button && d.resizable ? renderWindowButton("↘", "Resize", "resize-window-button") : ""}
      ${d.rotate_buttons ? renderWindowButton("↩", "Rotate left") + renderWindowButton("↪", "Rotate right") : ""}
    </div>`;
}

function renderWindowButton(label, title, extraClass = "") {
  return `<span class="window-button ${escapeHtml(extraClass)}" title="${escapeHtml(title)}">${renderTwemojiGlyph(label, title, "window-glyph")}</span>`;
}

function renderScrollbarPreview(window) {
  const options = normalizedWindowOptions(window.options);
  const showVertical = ["vertical", "both", "auto"].includes(options.scrollbars);
  const showHorizontal = ["horizontal", "both", "auto"].includes(options.scrollbars);
  if (!state.showDecorations || (!showVertical && !showHorizontal)) return "";
  return `
    ${showVertical ? `<i class="scrollbar-preview vertical ${options.vertical_scrollbar_side}"></i>` : ""}
    ${showHorizontal ? `<i class="scrollbar-preview horizontal ${options.horizontal_scrollbar_side}"></i>` : ""}`;
}

function renderWindowTitleIcon(window) {
  const glyph = window.title_twemoji || window.titleTwemoji || "";
  if (!glyph) return "";
  return `${renderTwemojiGlyph(glyph, "Title glyph", "title-icon")}`;
}

function renderTwemojiGlyph(glyph, label = "Twemoji", className = "") {
  const style = twemojiSpriteStyle(glyph);
  return `<span class="twemoji-glyph ${escapeHtml(className)}" title="${escapeHtml(label)}" ${style}>${style ? "" : escapeHtml(glyph)}</span>`;
}

function renderControl(control) {
  const rect = control.geometry;
  const selected = control.id === state.selectedControlId ? " selected" : "";
  const glyph = control.kind === "button" ? controlProperty(control, "glyph") : "";
  return `
    <div class="control ${control.kind}${selected}" data-control="${control.id}" style="left:${rect.x}px;top:${rect.y}px;width:${rect.w}px;height:${rect.h}px">
      <span>${glyph ? renderTwemojiGlyph(glyph, "Button glyph", "control-glyph") : ""}${escapeHtml(control.caption)}</span>
      <i class="resize-handle" data-resize="${control.id}"></i>
    </div>`;
}

function renderInspector(window, control) {
  if (!window) {
    return `<div class="inspector"><p>Create a project to inspect UI2 windows.</p></div>`;
  }

  if (!control) {
    const d = normalizedDecorations(window.decorations);
    const options = normalizedWindowOptions(window.options);
    return `
      <div class="inspector">
        <div class="inspector-target">
          <b>${escapeHtml(window.name)}</b>
          <span>UI2 Window</span>
        </div>
        ${field("Caption", "window.caption", window.caption)}
        <label class="field">
          <span>Title Glyph</span>
          <button type="button" data-action="open-glyph-picker">${window.title_twemoji ? renderTwemojiGlyph(window.title_twemoji, "Selected title glyph") : "Choose"}</button>
        </label>
        <div class="field-row">
          ${field("X", "window.geometry.x", window.geometry.x, "number")}
          ${field("Y", "window.geometry.y", window.geometry.y, "number")}
        </div>
        <div class="field-row">
          ${field("Width", "window.geometry.w", window.geometry.w, "number")}
          ${field("Height", "window.geometry.h", window.geometry.h, "number")}
        </div>
        <h3>Window Options</h3>
        ${selectField("Scrollbars", "window.options.scrollbars", options.scrollbars, ["none", "horizontal", "vertical", "both", "auto"])}
        <div class="field-row">
          ${selectField("Vertical Side", "window.options.vertical_scrollbar_side", options.vertical_scrollbar_side, ["left", "right"])}
          ${selectField("Horizontal Side", "window.options.horizontal_scrollbar_side", options.horizontal_scrollbar_side, ["top", "bottom"])}
        </div>
        <h3>Decorations</h3>
        ${selectField("Mode", "window.decorations.mode", d.mode, ["system", "client", "none"])}
        <div class="checks">
          ${check("Titlebar", "window.decorations.titlebar", d.titlebar)}
          ${check("Bottom Bar", "window.decorations.bottom_bar", d.bottom_bar)}
          ${check("Title Icon", "window.decorations.title_icon", d.title_icon)}
          ${check("Composition", "window.decorations.toggle_composition", d.toggle_composition)}
          ${check("Fork", "window.decorations.fork", d.fork)}
          ${check("Close", "window.decorations.close", d.close)}
          ${check("Minimize", "window.decorations.minimize", d.minimize)}
          ${check("Restore", "window.decorations.restore", d.restore)}
          ${check("Maximize", "window.decorations.maximize", d.maximize)}
          ${check("Resizable", "window.decorations.resizable", d.resizable)}
          ${check("Resize Button", "window.decorations.resize_button", d.resize_button)}
          ${check("Rotate", "window.decorations.rotate_buttons", d.rotate_buttons)}
          ${check("Preserve VM", "window.decorations.preserve_vm", d.preserve_vm)}
          ${check("Always Top", "window.decorations.always_on_top", d.always_on_top)}
        </div>
      </div>`;
  }

  return `
    <div class="inspector">
      <div class="inspector-target">
        <b>${escapeHtml(control.name)}</b>
        <span>${escapeHtml(labelKind(control.kind))}</span>
      </div>
      ${field("Name", "control.name", control.name)}
      ${field("Caption", "control.caption", control.caption)}
      <div class="field-row">
        ${field("X", "control.geometry.x", control.geometry.x, "number")}
        ${field("Y", "control.geometry.y", control.geometry.y, "number")}
      </div>
      <div class="field-row">
        ${field("Width", "control.geometry.w", control.geometry.w, "number")}
        ${field("Height", "control.geometry.h", control.geometry.h, "number")}
      </div>
      ${control.kind === "button" ? `
        <label class="field">
          <span>Glyph</span>
          <button type="button" data-action="open-control-glyph-picker">${controlProperty(control, "glyph") ? renderTwemojiGlyph(controlProperty(control, "glyph"), "Selected button glyph") : "Choose"}</button>
        </label>
      ` : ""}
      <h3>Events</h3>
      ${(control.events || []).map((event, index) => field(`on ${event.event}`, `control.events.${index}.handler`, event.handler)).join("")}
      <div class="inspector-actions">
        <button data-action="duplicate-control">Duplicate</button>
        <button class="danger" data-action="delete-control">Delete</button>
      </div>
    </div>`;
}

function renderJobs() {
  if (!state.jobs.length) {
    return `
      <div class="job empty">
        <b>No jobs yet</b>
        <span>Run Check, Build, Pack, or enable Full Auto.</span>
      </div>`;
  }
  return state.jobs.slice(-8).reverse().map((job) => `
    <div class="job">
      <b>${escapeHtml(job.status || "queued")}</b>
      <span>${escapeHtml(job.lines?.at(-1) || job.id || "")}</span>
    </div>`).join("");
}

function bindEvents() {
  app.querySelector("[data-action='new']")?.addEventListener("click", openNewProject);
  app.querySelector("[data-action='open-project']")?.addEventListener("click", openProjectPicker);
  app.querySelectorAll("[data-action='cancel-new']").forEach((button) => {
    button.addEventListener("click", closeNewProject);
  });
  app.querySelectorAll("[data-action='cancel-open-project']").forEach((button) => {
    button.addEventListener("click", closeOpenProject);
  });
  app.querySelector("[data-action='refresh-projects']")?.addEventListener("click", () => {
    refreshProjects().finally(render);
  });
  app.querySelectorAll("[data-load-project]").forEach((button) => {
    button.addEventListener("click", () => loadProject(button.dataset.loadProject));
  });
  app.querySelector("[data-new-project-form]")?.addEventListener("submit", (event) => {
    event.preventDefault();
    newProject(event.currentTarget);
  });
  app.querySelector("[name='templateId']")?.addEventListener("change", (event) => {
    state.newProjectDraft.templateId = event.target.value;
    app.querySelectorAll("[data-template-note]").forEach((note) => {
      note.classList.toggle("active", note.dataset.templateNote === event.target.value);
    });
  });
  app.querySelector("[data-action='clear-log']")?.addEventListener("click", () => {
    state.logs = [];
    render();
  });
  app.querySelector("[data-action='duplicate-control']")?.addEventListener("click", duplicateSelectedControl);
  app.querySelector("[data-action='delete-control']")?.addEventListener("click", deleteSelectedControl);
  app.querySelector("[data-action='save-rust']")?.addEventListener("click", saveSelectedRustFile);
  app.querySelector("[data-action='add-window']")?.addEventListener("click", addWindow);
  app.querySelector("[data-action='open-glyph-picker']")?.addEventListener("click", () => {
    state.glyphPickerTarget = { kind: "window-title" };
    state.glyphPickerQuery = "";
    state.glyphPickerOpen = true;
    render();
  });
  app.querySelector("[data-action='open-control-glyph-picker']")?.addEventListener("click", () => {
    state.glyphPickerTarget = { kind: "control-glyph", controlId: selectedControl()?.id || null };
    state.glyphPickerQuery = "";
    state.glyphPickerOpen = true;
    render();
  });
  app.querySelector("[data-action='close-glyph-picker']")?.addEventListener("click", () => {
    state.glyphPickerOpen = false;
    state.glyphPickerTarget = null;
    state.glyphPickerQuery = "";
    render();
  });
  app.querySelector("[data-glyph-search]")?.addEventListener("input", (event) => {
    state.glyphPickerQuery = event.target.value;
    render();
  });
  app.querySelectorAll("[data-glyph]").forEach((button) => {
    button.addEventListener("click", () => {
      const glyph = button.dataset.glyph;
      if (state.glyphPickerTarget?.kind === "control-glyph") {
        chooseControlGlyph(glyph);
      } else {
        chooseTitleGlyph(glyph);
      }
    });
  });
  app.querySelector("[data-asset-import]")?.addEventListener("change", (event) => {
    importAssetFile(event.target.files?.[0]);
  });

  app.querySelectorAll("[data-editor-tab]").forEach((button) => {
    button.addEventListener("click", () => {
      state.activeEditor = button.dataset.editorTab;
      render();
    });
  });
  app.querySelectorAll("[data-ui-source]").forEach((textarea) => {
    textarea.addEventListener("input", () => updateUiSource(textarea.dataset.uiSource, textarea.value));
  });
  app.querySelector("[data-rust-editor]")?.addEventListener("input", (event) => updateRustSource(event.target.value));
  app.querySelectorAll("[data-rust-file]").forEach((button) => {
    button.addEventListener("click", () => {
      state.selectedRustPath = button.dataset.rustFile;
      render();
    });
  });

  app.querySelectorAll("[data-job]").forEach((button) => {
    button.addEventListener("click", () => runJob(button.dataset.job));
  });
  app.querySelectorAll("[data-kind]").forEach((button) => {
    button.addEventListener("click", () => addControl(button.dataset.kind));
  });
  app.querySelectorAll("[data-palette-category]").forEach((button) => {
    button.addEventListener("click", () => {
      state.selectedPaletteCategory = button.dataset.paletteCategory;
      render();
    });
  });
  app.querySelectorAll("[data-select-window]").forEach((button) => {
    button.addEventListener("click", () => selectWindow(button.dataset.selectWindow));
  });
  app.querySelectorAll("[data-select-control]").forEach((button) => {
    button.addEventListener("click", () => selectControl(button.dataset.selectControl));
  });
  app.querySelector("[data-surface]")?.addEventListener("click", (event) => {
    if (event.target.closest("[data-control]")) return;
    if (selectedWindow()) {
      state.selectedControlId = null;
      render();
    }
  });

  app.querySelectorAll("[data-edit]").forEach((input) => {
    input.addEventListener("change", () => applyInspectorEdit(input));
  });
  app.querySelectorAll("[data-setting]").forEach((input) => {
    input.addEventListener("change", () => applySetting(input));
  });
  app.querySelector("[data-toggle='fullAuto']")?.addEventListener("change", (event) => {
    state.fullAuto = event.target.checked;
    state.logs.push(`Full Auto ${state.fullAuto ? "enabled" : "disabled"}.`);
    optionalApi(
      "/api/runtime",
      { method: "POST", body: JSON.stringify({ full_auto: state.fullAuto, watch: state.fullAuto }) },
      "Runtime toggle is local until the backend runtime route is available."
    );
    if (state.fullAuto) runJob("auto");
    render();
  });

  app.querySelectorAll("[data-control]").forEach((node) => {
    node.addEventListener("pointerdown", startDrag);
    node.addEventListener("click", (event) => {
      event.stopPropagation();
      selectControl(node.dataset.control);
    });
  });
  app.querySelectorAll("[data-resize]").forEach((node) => {
    node.addEventListener("pointerdown", startResize);
  });
}

function startDrag(event) {
  if (event.target.closest("[data-resize]")) return;
  const control = getControl(event.currentTarget.dataset.control);
  if (!control) return;
  event.preventDefault();
  event.currentTarget.setPointerCapture(event.pointerId);
  state.selectedControlId = control.id;
  state.drag = {
    mode: "move",
    id: control.id,
    startX: event.clientX,
    startY: event.clientY,
    rect: { ...control.geometry },
    node: event.currentTarget,
  };
  window.addEventListener("pointermove", movePointer);
  window.addEventListener("pointerup", stopPointer, { once: true });
}

function startResize(event) {
  const control = getControl(event.currentTarget.dataset.resize);
  if (!control) return;
  event.stopPropagation();
  event.preventDefault();
  state.drag = {
    mode: "resize",
    id: control.id,
    startX: event.clientX,
    startY: event.clientY,
    rect: { ...control.geometry },
    node: event.currentTarget.closest("[data-control]"),
  };
  window.addEventListener("pointermove", movePointer);
  window.addEventListener("pointerup", stopPointer, { once: true });
}

function movePointer(event) {
  if (!state.drag) return;
  const control = getControl(state.drag.id);
  const window = selectedWindow();
  if (!control || !window) return;
  const dx = (event.clientX - state.drag.startX) / state.zoom;
  const dy = (event.clientY - state.drag.startY) / state.zoom;

  if (state.drag.mode === "move") {
    control.geometry.x = clamp(snap(state.drag.rect.x + dx), 0, Math.max(0, window.geometry.w - control.geometry.w));
    control.geometry.y = clamp(snap(state.drag.rect.y + dy), 0, Math.max(0, window.geometry.h - control.geometry.h));
  } else {
    control.geometry.w = clamp(snap(state.drag.rect.w + dx), 40, Math.max(40, window.geometry.w - control.geometry.x));
    control.geometry.h = clamp(snap(state.drag.rect.h + dy), 24, Math.max(24, window.geometry.h - control.geometry.y));
  }

  state.drag.node.style.left = `${control.geometry.x}px`;
  state.drag.node.style.top = `${control.geometry.y}px`;
  state.drag.node.style.width = `${control.geometry.w}px`;
  state.drag.node.style.height = `${control.geometry.h}px`;
}

function stopPointer() {
  window.removeEventListener("pointermove", movePointer);
  if (state.drag) {
    persistControl(getControl(state.drag.id));
    scheduleAutoJob("layout edit");
  }
  state.drag = null;
  render();
}

function applyInspectorEdit(input) {
  const path = input.dataset.edit;
  const value = input.type === "checkbox" ? input.checked : coerceValue(input.value, input.type);
  const target = path.startsWith("window.") ? selectedWindow() : selectedControl();
  if (!target) return;
  setPath(target, path.replace(/^(window|control)\./, ""), value);
  if (path.startsWith("window.")) {
    persistWindow(target);
    scheduleAutoJob("window edit");
  } else {
    persistControl(target);
    scheduleAutoJob("control edit");
  }
  render();
}

function applySetting(input) {
  if (input.dataset.setting === "zoom") {
    state.zoom = Number(input.value);
  } else {
    state[input.dataset.setting] = input.checked;
  }
  render();
}

function duplicateSelectedControl() {
  const window = selectedWindow();
  const control = selectedControl();
  if (!window || !control) return;
  const copy = structuredClone(control);
  copy.id = crypto.randomUUID();
  copy.name = uniqueControlName(window, control.name.replace(/\d+$/, ""));
  copy.caption = `${control.caption} Copy`;
  copy.geometry.x = snap(control.geometry.x + 24);
  copy.geometry.y = snap(control.geometry.y + 24);
  window.controls.push(copy);
  state.selectedControlId = copy.id;
  state.logs.push(`Duplicated ${control.name} locally.`);
  persistWindow(window);
  scheduleAutoJob("control duplicate");
  render();
}

async function deleteSelectedControl() {
  const window = selectedWindow();
  const control = selectedControl();
  if (!window || !control) return;
  window.controls = window.controls.filter((item) => item.id !== control.id);
  state.selectedControlId = null;
  await optionalApi(`/api/project/control/${control.id}`, { method: "DELETE" }, "Delete route unavailable; removed control locally.");
  state.logs.push(`Deleted ${control.name}.`);
  scheduleAutoJob("control delete");
  render();
}

async function persistWindow(window) {
  await optionalApi(
    "/api/project/window",
    { method: "PATCH", body: JSON.stringify({ window_id: window.id, window }) },
    "Window edits are active locally; backend update route unavailable."
  );
}

async function persistControl(control) {
  if (!control) return;
  const window = selectedWindow();
  await optionalApi(
    "/api/project/control",
    { method: "PATCH", body: JSON.stringify({ window_id: window?.id, control_id: control.id, control }) },
    "Control edits are active locally; backend update route unavailable."
  );
}

function normalizeTemplates(payload) {
  const list = Array.isArray(payload) ? payload : payload?.templates || payload?.items || [];
  if (!Array.isArray(list)) return [];
  return list.map((template, index) => {
    if (typeof template === "string") {
      return { id: template, name: labelKind(template), description: "", raw: { id: template } };
    }
    const id = String(template.id || template.key || template.slug || template.name || `template-${index + 1}`);
    return {
      id,
      name: String(template.name || template.label || template.title || id),
      description: String(template.description || template.summary || template.note || ""),
      app_kind: String(template.app_kind || template.appKind || "ui2"),
      raw: template,
    };
  });
}

function normalizeProjects(payload) {
  const list = Array.isArray(payload) ? payload : payload?.projects || payload?.items || [];
  if (!Array.isArray(list)) return [];
  return list.map((project, index) => {
    const slug = String(project.slug || project.blueprint?.slug || project.name || `project-${index + 1}`);
    const path = String(project.path || project.project_file || project.projectFile || `${slug}/rads.project.json`);
    return {
      name: String(project.name || project.display_name || project.displayName || slug),
      slug,
      path,
      root: String(project.root || ""),
      appKind: String(project.app_kind || project.appKind || "ui2"),
      appId: String(project.app_id || project.appId || project.blueprint?.app_id || "dev.trueos.local"),
      displayName: String(project.display_name || project.displayName || project.name || slug),
      version: String(project.version || project.blueprint?.version || ""),
      windows: Number(project.windows || 0),
      modifiedUnixMs: Number(project.modified_unix_ms || project.modifiedUnixMs || 0),
      raw: project,
    };
  });
}

async function hydrateTrueosTwemoji(payload) {
  const asset = payload || {};
  state.trueosTwemoji = {
    available: Boolean(asset.available),
    fontStack: String(asset.font_stack || asset.fontStack || state.trueosTwemoji.fontStack),
    atlas: null,
    atlasPng: String(asset.atlas_png || asset.atlasPng || ""),
  };
  if (state.trueosTwemoji.available && asset.atlas_set) {
    const atlasSet = await probeApi(asset.atlas_set);
    state.trueosTwemoji.atlas = atlasSet?.atlas || null;
  }
}

function appKind(project = state.project) {
  const kind = String(project?.app_kind || project?.appKind || "").toLowerCase();
  if (kind) return kind;
  const runtime = String(project?.blueprint?.metadata?.ui_runtime || "").toLowerCase();
  if (runtime.includes("service")) return "service";
  if (runtime.includes("shell")) return "shell";
  return "ui2";
}

function isUi2Project(project = state.project) {
  return appKind(project) === "ui2" || Boolean(project?.windows?.length);
}

function labelAppKind(kind) {
  const normalized = typeof kind === "object" ? appKind(kind) : String(kind || "ui2").toLowerCase();
  if (normalized === "service") return "Background Service";
  if (normalized === "shell") return "Shell App";
  return "UI2 App";
}

function isActiveProjectSummary(project) {
  const active = state.project;
  if (!active || !project) return false;
  if (project.root && active.root && project.root === String(active.root)) return true;
  return Boolean(project.slug && active.slug && project.slug === active.slug);
}

function allTwemojiGlyphChoices() {
  const slots = state.trueosTwemoji.atlas?.slots || [];
  if (Array.isArray(slots) && slots.length) {
    return slots
      .filter((codepoint) => Number.isInteger(codepoint) && codepoint > 0)
      .map((codepoint) => String.fromCodePoint(codepoint));
  }
  return [
    0x1F4A0, 0x26A1, 0x1F4BB, 0x1F4C1, 0x1F4BF, 0x1F524,
    0x23F5, 0x23F8, 0x23F9, 0x23EF, 0x23CF, 0x2796,
    0x2797, 0x274C, 0x25FC, 0x25FB, 0x1F518, 0x1F4BE,
  ].map((codepoint) => String.fromCodePoint(codepoint));
}

function filteredTwemojiGlyphChoices() {
  const query = state.glyphPickerQuery.trim().toLowerCase().replace(/^u\+/, "");
  const glyphs = allTwemojiGlyphChoices();
  if (!query) return glyphs;
  return glyphs.filter((glyph) => {
    const hex = glyph.codePointAt(0).toString(16).toLowerCase();
    return glyph === query || hex.includes(query);
  });
}

function twemojiSpriteStyle(glyph) {
  const atlas = state.trueosTwemoji.atlas;
  if (!atlas || !state.trueosTwemoji.atlasPng || !glyph) return "";
  const codepoint = glyph.codePointAt(0);
  const slot = (atlas.slots || []).indexOf(codepoint);
  if (slot < 0) return "";
  const cellW = Number(atlas.cell_w || 0);
  const cellH = Number(atlas.cell_h || 0);
  const gridW = Number(atlas.grid_w || 1);
  const gridH = Number(atlas.grid_h || 1);
  const x = (slot % gridW) * cellW;
  const y = Math.floor(slot / gridW) * cellH;
  const atlasW = cellW * gridW;
  const atlasH = cellH * gridH;
  return `style="background-image:url('${escapeHtml(state.trueosTwemoji.atlasPng)}');background-size:${atlasW}px ${atlasH}px;background-position:-${x}px -${y}px"`;
}

function hydrateProjectEditors(template = null) {
  if (!state.project) {
    state.uiHtml = "";
    state.uiCss = "";
    state.uiSourceProjectKey = "";
    state.rustFiles = [];
    return;
  }

  hydrateUiSourcesForWindow(template);

  const localRustFiles = readLocalRustFiles();
  state.rustFiles = localRustFiles.length ? localRustFiles : defaultRustFilesForProject();
  state.selectedRustPath = state.rustFiles[0]?.path || "src/main.rs";
  state.rustSaveLabel = localRustFiles.length ? "Restored locally" : "Local";
}

function hydrateUiSourcesForWindow(template = null) {
  if (!state.project) return;
  if (!isUi2Project()) {
    state.uiSourceProjectKey = projectStorageKey();
    state.uiHtml = "";
    state.uiCss = "";
    state.uiSaveLabel = "No UI2 surface";
    return;
  }
  const key = projectStorageKey();
  const templateHtml = templateSource(template, ["html", "ui_html", "index.html", "ui/index.html"]);
  const templateCss = templateSource(template, ["css", "ui_css", "styles.css", "static/styles.css"]);
  const savedSources = template ? null : readJsonStorage(uiSourcesSection());
  const windowUi = selectedWindow()?.ui_description || {};
  const projectUi = state.project.ui_description || state.project.uiDescription || state.project.ui || {};
  const modelUi = windowUi.html || windowUi.css ? windowUi : projectUi;

  state.uiSourceProjectKey = key;
  state.uiHtml = savedSources?.html ?? templateHtml ?? modelUi.html ?? defaultHtmlForProject();
  state.uiCss = savedSources?.css ?? templateCss ?? modelUi.css ?? defaultCssForProject();
  state.uiSaveLabel = savedSources ? "Restored locally" : "Project UI description updated";
  syncProjectUiDescription();
  saveUiSourcesLocal();
}

function applyTemplateToLocalProject(template) {
  if (!state.project || !template) return;
  const raw = template.raw || {};
  const displayName = raw.display_name || raw.title || raw.name;
  if (displayName) {
    state.project.blueprint.display_name = String(displayName);
  }
  if (raw.description || template.description) {
    state.project.blueprint.description = String(raw.description || template.description);
  }
}

function templateSource(template, keys) {
  if (!template) return null;
  const raw = template.raw || template;
  for (const key of keys) {
    if (typeof raw[key] === "string") return raw[key];
  }
  const sources = raw.sources || raw.files || {};
  if (Array.isArray(sources)) {
    for (const name of keys) {
      const match = sources.find((file) => file.path === name || file.name === name || file.id === name);
      if (typeof match?.content === "string") return match.content;
      if (typeof match?.body === "string") return match.body;
    }
  } else {
    for (const name of keys) {
      if (typeof sources[name] === "string") return sources[name];
      if (typeof sources[name]?.content === "string") return sources[name].content;
    }
  }
  return null;
}

function defaultHtmlForProject() {
  const project = state.project;
  const window = selectedWindow() || project?.windows?.[0];
  if (!project || !window) {
    return `<main class="ui2-app">
  <h1>TRUEOS UI2 app</h1>
  <button>Click me</button>
</main>`;
  }
  const controls = (window.controls || []).map((control) => controlToHtml(control)).join("\n    ");
  return `<main class="ui2-app" data-window="${escapeHtml(window.name)}">
  <header class="ui2-titlebar">
    <h1>${escapeHtml(window.caption)}</h1>
  </header>
  <section class="ui2-surface">
    ${controls || "<p>Start designing your UI2 controls.</p>"}
  </section>
</main>`;
}

function controlToHtml(control) {
  const caption = escapeHtml(control.caption || control.name || labelKind(control.kind));
  const name = escapeHtml(control.name || control.kind);
  const glyph = controlProperty(control, "glyph");
  if (control.kind === "button") {
    const icon = glyph ? `<span class="emoji" aria-hidden="true">${escapeHtml(glyph)}</span>` : "";
    return `<button data-control="${name}">${icon}<span>${caption}</span></button>`;
  }
  if (control.kind === "text-box") return `<input data-control="${name}" placeholder="${caption}" />`;
  if (control.kind === "check-box") return `<label data-control="${name}"><input type="checkbox" /> ${caption}</label>`;
  if (control.kind === "list-box") return `<select data-control="${name}"><option>${caption}</option></select>`;
  if (control.kind === "canvas") return `<div class="canvas-preview" data-control="${name}">${caption}</div>`;
  if (control.kind === "panel") return `<section class="panel-preview" data-control="${name}">${caption}</section>`;
  if (control.kind === "toolbar") return `<nav data-control="${name}">${caption}</nav>`;
  return `<p data-control="${name}">${caption}</p>`;
}

function defaultCssForProject() {
  return `:root {
  color: #111416;
  background: #e8eef0;
  font-family: Inter, ui-sans-serif, system-ui, sans-serif;
}

body {
  margin: 0;
}

.ui2-app {
  min-height: 100vh;
  padding: 28px;
  background: #e8eef0;
}

.ui2-titlebar {
  margin-bottom: 18px;
  border-bottom: 1px solid #b8c5ca;
}

.ui2-titlebar h1 {
  margin: 0 0 10px;
  font-size: 24px;
}

.ui2-surface {
  display: grid;
  align-content: start;
  gap: 12px;
  max-width: 520px;
}

button,
input,
select {
  min-height: 34px;
  border: 1px solid #73838b;
  border-radius: 4px;
  padding: 0 12px;
  font: inherit;
}

button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  background: #253238;
  color: #ffffff;
}

.panel-preview,
.canvas-preview {
  min-height: 96px;
  border: 1px solid #91a2aa;
  padding: 12px;
  background: #f8fafb;
}`;
}

function updateUiSource(kind, value) {
  if (kind === "html") {
    state.uiHtml = value;
  } else {
    state.uiCss = value;
  }
  syncProjectUiDescription();
  saveUiSourcesLocal();
  state.uiSaveLabel = "Project UI description updated";
  app.querySelector("[data-ui-status]")?.replaceChildren(document.createTextNode(state.uiSaveLabel));
  app.querySelector("[data-ui-description]")?.replaceChildren(document.createTextNode(state.project?.ui_description?.summary || ""));
  app.querySelector("[data-html-count]")?.replaceChildren(document.createTextNode(String(countHtmlElements(state.uiHtml))));
  app.querySelector("[data-css-count]")?.replaceChildren(document.createTextNode(String(countCssRules(state.uiCss))));
  const preview = app.querySelector("[data-live-preview]");
  if (preview) preview.srcdoc = composePreviewDocument();
  persistUiDescriptionSoon();
  scheduleAutoJob("ui source edit");
}

function syncProjectUiDescription() {
  if (!state.project) return;
  const summary = summarizeUiSources(state.uiHtml, state.uiCss);
  state.project.ui_description = {
    html: state.uiHtml,
    css: state.uiCss,
    summary,
    updated_at: new Date().toISOString(),
  };
  const window = selectedWindow();
  if (window) {
    window.ui_description = {
      html: state.uiHtml,
      css: state.uiCss,
    };
  }
  if (state.project.blueprint) {
    state.project.blueprint.description = summary;
  }
}

function summarizeUiSources(html, css) {
  const text = String(html || "")
    .replace(/<script[\s\S]*?<\/script>/gi, " ")
    .replace(/<style[\s\S]*?<\/style>/gi, " ")
    .replace(/<[^>]+>/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  const cssRules = countCssRules(css);
  const fallback = state.project?.blueprint?.display_name || state.project?.name || "TRUEOS UI2 app";
  const summary = text ? text.slice(0, 120) : `${fallback} custom HTML/CSS UI`;
  const suffix = cssRules === 1 ? "1 CSS rule" : `${cssRules} CSS rules`;
  return `UI preview: ${summary}${text.length > 120 ? "..." : ""} (${suffix}).`;
}

function composePreviewDocument() {
  const html = state.uiHtml || "";
  const css = state.uiCss || "";
  const trueosStyle = `<style>:root{--trueos-emoji-font:${state.trueosTwemoji.fontStack};}.ui2-titlebar,.titlebar,.icon,.emoji{font-family:var(--trueos-emoji-font),system-ui,sans-serif;}</style>`;
  const style = `${trueosStyle}<style>${css}</style>`;
  if (/<html[\s>]/i.test(html)) {
    if (/<\/head>/i.test(html)) return html.replace(/<\/head>/i, `${style}</head>`);
    return html.replace(/<html[^>]*>/i, `$&<head>${style}</head>`);
  }
  return `<!doctype html>
<html>
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    ${style}
  </head>
  <body>
    ${html}
  </body>
</html>`;
}

function countHtmlElements(html) {
  return (String(html || "").match(/<[a-z][\w:-]*(\s|>|\/)/gi) || []).length;
}

function countCssRules(css) {
  return (String(css || "").match(/{/g) || []).length;
}

function saveUiSourcesLocal() {
  writeJsonStorage(uiSourcesSection(), {
    html: state.uiHtml,
    css: state.uiCss,
    updatedAt: new Date().toISOString(),
  });
}

function persistUiDescriptionSoon() {
  window.clearTimeout(state.uiPersistTimer);
  state.uiPersistTimer = window.setTimeout(async () => {
    const payload = {
      window_id: selectedWindow()?.id,
      description: state.project?.blueprint?.description || "",
      ui_description: state.project?.ui_description || {},
      html: state.uiHtml,
      css: state.uiCss,
    };
    const saved = await saveUiDescriptionToBackend(payload);
    state.uiSaveLabel = saved ? "Saved to backend" : "Saved locally";
    app.querySelector("[data-ui-status]")?.replaceChildren(document.createTextNode(state.uiSaveLabel));
  }, 650);
}

async function saveUiDescriptionToBackend(payload) {
  const routes = [
    ["/api/project/ui-description", "PATCH"],
    ["/api/project/ui", "PATCH"],
    ["/api/project/description", "PATCH"],
  ];
  for (const [path, method] of routes) {
    const response = await tryApi(path, { method, body: JSON.stringify(payload) });
    if (response.ok) return true;
  }
  return false;
}

async function loadAssetsForProject() {
  if (!state.project) return;
  const payload = await probeApi("/api/project/assets");
  const assets = normalizeAssets(payload);
  if (assets.length || payload?.assets) {
    state.assets = assets;
    state.assetRoutesOnline = true;
    state.assetSaveLabel = "Loaded from backend";
  } else {
    state.assets = readJsonStorage("assets") || [];
    state.assetRoutesOnline = false;
    state.assetSaveLabel = state.assets.length ? "Restored locally" : "Local";
  }
}

async function importAssetFile(file) {
  if (!file || !state.project) return;
  const extension = file.name.split(".").pop().toLowerCase();
  if (!["jpg", "jpeg", "png", "svg", "bmp"].includes(extension)) {
    state.logs.push("Asset type must be JPG, JPEG, PNG, SVG, or BMP.");
    render();
    return;
  }
  const contentsBase64 = await fileToBase64(file);
  const warning = extension === "bmp" ? "BMP assets usually waste package space; prefer PNG or SVG." : null;
  const localAsset = {
    name: file.name,
    path: `assets/${file.name}`,
    extension,
    size: file.size,
    warning,
  };
  const response = await tryApi("/api/project/assets", {
    method: "POST",
    body: JSON.stringify({ name: file.name, contents_base64: contentsBase64 }),
  });
  if (response.ok) {
    state.assets = normalizeAssets(response.data);
    state.assetRoutesOnline = true;
    state.assetSaveLabel = `Imported ${file.name}`;
  } else {
    state.assets = [...state.assets.filter((asset) => asset.name !== file.name), localAsset];
    state.assetRoutesOnline = false;
    state.assetSaveLabel = `Imported ${file.name} locally`;
    writeJsonStorage("assets", state.assets);
  }
  if (warning) state.logs.push(warning);
  scheduleAutoJob("asset import");
  render();
}

function normalizeAssets(payload) {
  const list = Array.isArray(payload) ? payload : payload?.assets || payload?.items || [];
  if (!Array.isArray(list)) return [];
  return list.map((asset) => {
    const name = String(asset.name || String(asset.path || "").split("/").at(-1) || "asset");
    const extension = String(asset.extension || name.split(".").pop() || "").toLowerCase();
    return {
      name,
      path: String(asset.path || `assets/${name}`),
      extension,
      size: Number(asset.size || 0),
      warning: asset.warning || (extension === "bmp" ? "BMP assets usually waste package space; prefer PNG or SVG." : null),
    };
  });
}

function fileToBase64(file) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.addEventListener("load", () => {
      const result = String(reader.result || "");
      resolve(result.includes(",") ? result.split(",").pop() : result);
    });
    reader.addEventListener("error", () => reject(reader.error));
    reader.readAsDataURL(file);
  });
}

async function loadRustFilesForProject() {
  if (!state.project) return;
  state.rustLoading = true;
  const localRustFiles = readLocalRustFiles();
  if (!state.rustFiles.length) {
    state.rustFiles = localRustFiles.length ? localRustFiles : defaultRustFilesForProject();
  }
  render();

  const backendFiles = await loadRustFilesFromBackend();
  if (backendFiles.length) {
    state.rustFiles = backendFiles;
    state.rustRoutesOnline = true;
    state.rustSaveLabel = "Loaded from backend";
  } else if (localRustFiles.length) {
    state.rustFiles = localRustFiles;
    state.rustRoutesOnline = false;
    state.rustSaveLabel = "Restored locally";
  } else {
    state.rustFiles = defaultRustFilesForProject();
    state.rustRoutesOnline = false;
    state.rustSaveLabel = "Local";
  }
  if (!state.rustFiles.some((file) => file.path === state.selectedRustPath)) {
    state.selectedRustPath = state.rustFiles[0]?.path || "src/main.rs";
  }
  state.rustLoading = false;
  saveRustFilesLocal();
  render();
}

async function loadRustFilesFromBackend() {
  const listRoutes = [
    "/api/project/files?kind=rust",
    "/api/project/source?kind=rust",
    "/api/files?kind=rust",
    "/api/project/rust-files",
  ];
  for (const route of listRoutes) {
    const payload = await probeApi(route);
    const files = await normalizeRustFiles(payload);
    if (files.length) return files;
  }

  const files = [];
  for (const path of defaultRustPaths) {
    const content = await readRustFileFromBackend(path);
    if (typeof content === "string") {
      files.push({ path, label: path.split("/").at(-1), content, dirty: false, backend: true });
    }
  }
  return files;
}

async function normalizeRustFiles(payload) {
  const list = Array.isArray(payload)
    ? payload
    : payload?.files || payload?.rust_files || payload?.sources || payload?.items || [];
  if (!Array.isArray(list)) return [];

  const files = [];
  for (const item of list) {
    const path = typeof item === "string" ? item : item.path || item.name || item.file;
    if (!path || !String(path).endsWith(".rs")) continue;
    let content = typeof item === "string" ? null : readContentPayload(item);
    if (content == null) content = await readRustFileFromBackend(path);
    files.push({
      path: String(path),
      label: String(path).split("/").at(-1),
      content: typeof content === "string" ? content : "",
      dirty: false,
      backend: true,
    });
  }
  return files;
}

async function readRustFileFromBackend(path) {
  const encoded = encodeURIComponent(path);
  const routes = [
    `/api/project/file?path=${encoded}`,
    `/api/project/source?path=${encoded}`,
    `/api/files?path=${encoded}`,
    `/api/project/files/${encoded}`,
  ];
  for (const route of routes) {
    const payload = await probeApi(route);
    const content = readContentPayload(payload);
    if (typeof content === "string") return content;
  }
  return null;
}

function readContentPayload(payload) {
  if (typeof payload === "string") return payload;
  if (!payload || typeof payload !== "object") return null;
  return payload.content ?? payload.contents ?? payload.body ?? payload.text ?? null;
}

function selectedRustFile() {
  return state.rustFiles.find((file) => file.path === state.selectedRustPath) || state.rustFiles[0] || null;
}

function updateRustSource(value) {
  const file = selectedRustFile();
  if (!file) return;
  file.content = value;
  file.dirty = true;
  state.rustSaveLabel = "Unsaved local edit";
  saveRustFilesLocal();
  const saveState = app.querySelector(".rust-workbench .save-state");
  if (saveState) saveState.textContent = state.rustSaveLabel;
}

async function saveSelectedRustFile() {
  const file = selectedRustFile();
  if (!file) return;
  saveRustFilesLocal();
  const saved = await saveRustFileToBackend(file);
  state.rustRoutesOnline = saved || state.rustRoutesOnline;
  state.rustSaveLabel = saved ? `Saved ${file.path}` : `Saved ${file.path} locally`;
  file.dirty = false;
  if (!saved && !state.unavailableRoutes.has("SOURCE file-save")) {
    state.unavailableRoutes.add("SOURCE file-save");
    state.logs.push("Source file save route unavailable; Rust edits are stored locally.");
  }
  saveRustFilesLocal();
  render();
}

async function saveRustFileToBackend(file) {
  const encoded = encodeURIComponent(file.path);
  const payload = { path: file.path, content: file.content };
  const routes = [
    ["/api/project/file", "PUT", payload],
    ["/api/project/file", "PATCH", payload],
    ["/api/project/files", "POST", payload],
    ["/api/files", "PUT", payload],
    [`/api/project/files/${encoded}`, "PUT", { content: file.content }],
  ];
  for (const [path, method, body] of routes) {
    const response = await tryApi(path, { method, body: JSON.stringify(body) });
    if (response.ok) return true;
  }
  return false;
}

function readLocalRustFiles() {
  const files = readJsonStorage("rust-files");
  if (!Array.isArray(files)) return [];
  return files
    .filter((file) => file?.path && typeof file.content === "string")
    .map((file) => ({
      path: String(file.path),
      label: String(file.label || file.path).split("/").at(-1),
      content: file.content,
      dirty: Boolean(file.dirty),
      backend: false,
    }));
}

function saveRustFilesLocal() {
  writeJsonStorage("rust-files", state.rustFiles.map((file) => ({
    path: file.path,
    label: file.label,
    content: file.content,
    dirty: file.dirty,
  })));
}

function defaultRustFilesForProject() {
  const files = [
    { path: "src/main.rs", label: "main.rs", content: generateMainRs(), dirty: false, backend: false },
    { path: "src/events.rs", label: "events.rs", content: generateEventsRs(), dirty: false, backend: false },
  ];
  if (isUi2Project()) {
    files.splice(1, 0, { path: "src/ui.rs", label: "ui.rs", content: generateUiRs(), dirty: false, backend: false });
  }
  return files;
}

function generateMainRs() {
  if (!isUi2Project()) {
    const name = escapeRustString(state.project?.blueprint?.display_name || state.project?.name || "TRUEOS app");
    if (appKind() === "service") {
      return `fn main() {
    v::vshell::line("service ${name} starting");
    v::vshell::line("background service loop is ready to wire to TRUEOS tasks");
}
`;
    }
    return `fn main() {
    v::vshell::line("${name} shell ready");
    v::vshell::line("wire commands here as the shell surface grows");
}
`;
  }
  const caption = escapeRustString(selectedWindow()?.caption || state.project?.name || "TRUEOS UI2 app");
  return `mod events;
mod ui;

fn main() {
    let windows = ui::create_all_windows();
    if windows.is_empty() {
        v::vshell::line("failed to create UI2 windows");
        return;
    }

    events::wire_main_window();
    v::vshell::line("started ${caption}");

    let _windows = windows;
}
`;
}

function generateUiRs() {
  const project = state.project || {};
  const windows = project.windows?.length ? project.windows : [createLocalWindow(project.name || "TRUEOS UI2 app", 1, "MainWindow")];
  const constants = windows.map((window, index) => {
    const stem = windowFileStem(window, index);
    const paths = index === 0
      ? ["../ui/main.ui2.json", "../ui/index.html", "../ui/styles.css"]
      : [`../ui/windows/${stem}.ui2.json`, `../ui/windows/${stem}.html`, `../ui/windows/${stem}.css`];
    return `pub const WINDOW_${index}_MODEL: &str = include_str!("${paths[0]}");
pub const WINDOW_${index}_HTML: &str = include_str!("${paths[1]}");
pub const WINDOW_${index}_CSS: &str = include_str!("${paths[2]}");
pub const WINDOW_${index}_DECORATIONS: &str = "${escapeRustString(decorationsLiteral(window.decorations || {}))}";
`;
  }).join("");
  const functions = windows.map((window, index) => {
    const geometry = window.geometry || { x: 80, y: 80, w: 720, h: 460 };
    const decorationOptions = windowDecorationOptionsLiteral(window);
    return `pub fn create_window_${index}() -> Option<vui2::OwnedWindow> {
    let rect = vui2::Rect {
        x: ${Number(geometry.x) || 0},
        y: ${Number(geometry.y) || 0},
        width: ${Number(geometry.w) || 720},
        height: ${Number(geometry.h) || 460},
    };
    let options = vui2::CreateOptions {
        decorations: ${decorationOptions},
        ..vui2::CreateOptions::default()
    };
    let window = vui2::OwnedWindow::create_with_options("${escapeRustString(window.caption || "TRUEOS UI2 app")}", rect, options)?;
    let id = window.id();
    id.set_title("${escapeRustString(window.caption || "TRUEOS UI2 app")}");
    Some(window)
}
`;
  }).join("\n");
  const pushes = windows.map((_window, index) => `    if let Some(window) = create_window_${index}() {
        windows.push(window);
    }
`).join("");
  return `use v::vui2;

pub const APP_ID: &str = "${escapeRustString(project.blueprint?.app_id || "dev.trueos.local")}";
pub const APP_DISPLAY_NAME: &str = "${escapeRustString(project.blueprint?.display_name || project.name || "TRUEOS UI2 app")}";
pub const MAIN_LAYOUT: &str = include_str!("../ui/main.ui2");
pub const MAIN_HTML: &str = WINDOW_0_HTML;
pub const MAIN_CSS: &str = WINDOW_0_CSS;
pub const MAIN_WINDOW_DECORATIONS: &str = WINDOW_0_DECORATIONS;
${constants}

pub fn create_main_window() -> Option<vui2::OwnedWindow> {
    create_window_0()
}

${functions}

pub fn create_all_windows() -> Vec<vui2::OwnedWindow> {
    let mut windows = Vec::new();
${pushes}    windows
}
`;
}

function generateEventsRs() {
  if (!isUi2Project()) {
    if (appKind() === "service") {
      return `pub fn register_service_handlers() {
    v::vshell::line("service handlers registered");
}
`;
    }
    return `pub fn register_shell_commands() {
    v::vshell::line("shell command table registered");
}
`;
  }
  const handlers = [];
  for (const window of state.project?.windows || []) {
    for (const control of window.controls || []) {
      for (const event of control.events || []) {
        if (!handlers.some((handler) => handler.name === event.handler)) {
          handlers.push({ name: event.handler, event: event.event, control: control.name });
        }
      }
    }
  }
  const body = handlers.map((handler) => `
pub fn ${handler.name}() {
    v::vshell::line("${escapeRustString(handler.event)} fired on ${escapeRustString(handler.control)}");
}
`).join("");
  return `pub fn wire_main_window() {
    v::vshell::line("UI2 event stubs registered");
}
${body}`;
}

function escapeRustString(value) {
  return String(value).replace(/\\/g, "\\\\").replace(/"/g, "\\\"");
}

function windowFileStem(window, index) {
  if (index === 0) return "main";
  return String(window.name || `window-${index + 1}`)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "") || `window-${index + 1}`;
}

function decorationsLiteral(decorations) {
  const d = normalizedDecorations(decorations);
  return `{ mode: ${d.mode}, titlebar: ${Boolean(d.titlebar)}, bottom_bar: ${Boolean(d.bottom_bar)}, title_icon: ${Boolean(d.title_icon)}, toggle_composition: ${Boolean(d.toggle_composition)}, fork: ${Boolean(d.fork)}, close: ${Boolean(d.close)}, minimize: ${Boolean(d.minimize)}, restore: ${Boolean(d.restore)}, maximize: ${Boolean(d.maximize)}, preserve_vm: ${Boolean(d.preserve_vm)}, resizable: ${Boolean(d.resizable)}, resize_button: ${Boolean(d.resize_button)}, rotate_buttons: ${Boolean(d.rotate_buttons)}, always_on_top: ${Boolean(d.always_on_top)} }`;
}

function windowDecorationOptionsLiteral(window) {
  const d = normalizedDecorations(window.decorations);
  const options = normalizedWindowOptions(window.options);
  return `vui2::WindowDecorationOptions {
            mode: vui2::WindowDecorationMode::${rustVariant(d.mode)},
            titlebar_visible: ${Boolean(d.titlebar)},
            bottom_bar_visible: ${Boolean(d.bottom_bar)},
            title_icon_visible: ${Boolean(d.title_icon)},
            buttons: vui2::WindowDecorationButtons {
                toggle_composition: ${Boolean(d.toggle_composition)},
                fork: ${Boolean(d.fork)},
                minimize: ${Boolean(d.minimize)},
                restore: ${Boolean(d.restore)},
                toggle_maximize: ${Boolean(d.maximize)},
                preserve_vm: ${Boolean(d.preserve_vm)},
                close: ${Boolean(d.close)},
            },
            resize_button_visible: ${Boolean(d.resizable && d.resize_button)},
            rotate_buttons_visible: ${Boolean(d.rotate_buttons)},
            vertical_scrollbar_visible: ${["vertical", "both", "auto"].includes(options.scrollbars)},
            horizontal_scrollbar_visible: ${["horizontal", "both", "auto"].includes(options.scrollbars)},
            vertical_scrollbar_side: vui2::VerticalScrollbarSide::${rustVariant(options.vertical_scrollbar_side)},
            horizontal_scrollbar_side: vui2::HorizontalScrollbarSide::${rustVariant(options.horizontal_scrollbar_side)},
            resize_mode: vui2::WindowResizeMode::Auto,
            resize_maintain_aspect: false,
            content_preserve_scale: ${Boolean(options.preserve_scale)},
        }`;
}

function normalizedDecorations(decorations = {}) {
  return {
    mode: "system",
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
    ...decorations,
  };
}

function normalizedWindowOptions(options = {}) {
  return {
    min_size: { width: 320, height: 240 },
    max_size: null,
    resize_mode: "both",
    scrollbars: "none",
    vertical_scrollbar_side: "left",
    horizontal_scrollbar_side: "bottom",
    hit_test_visible: true,
    preserve_scale: false,
    ...options,
  };
}

function rustVariant(value) {
  const normalized = String(value || "").replace(/_/g, "-").toLowerCase();
  if (normalized === "bottom") return "Bottom";
  if (normalized === "right") return "Right";
  if (normalized === "top") return "Top";
  if (normalized === "left") return "Left";
  if (normalized === "client") return "Client";
  if (normalized === "none") return "None";
  return "System";
}

function projectStorageKey() {
  const project = state.project;
  if (!project) return "none";
  return String(project.id || project.slug || project.blueprint?.slug || project.name || "local")
    .toLowerCase()
    .replace(/[^a-z0-9._-]+/g, "-");
}

function storageKey(section) {
  return `trueos-rads:${projectStorageKey()}:${section}`;
}

function uiSourcesSection() {
  return selectedWindow()?.id ? `ui-sources:${selectedWindow().id}` : "ui-sources";
}

function readJsonStorage(section) {
  try {
    const raw = window.localStorage.getItem(storageKey(section));
    return raw ? JSON.parse(raw) : null;
  } catch {
    return null;
  }
}

function writeJsonStorage(section, value) {
  try {
    window.localStorage.setItem(storageKey(section), JSON.stringify(value));
  } catch {
    // Local storage can be unavailable in hardened/private browser contexts.
  }
}

function normalizePalette(palette) {
  if (!Array.isArray(palette) || !palette.length) return fallbackPalette;
  const categoryMap = {
    button: "Basics",
    label: "Basics",
    "text-box": "Inputs",
    "check-box": "Inputs",
    panel: "Containers",
    "list-box": "Data",
    canvas: "Visual",
    toolbar: "Navigation",
    menu: "Navigation",
  };
  return palette.map((item) => ({
    ...item,
    category: item.category || categoryMap[item.kind] || "Other",
  }));
}

function ensurePaletteCategory() {
  const categories = paletteCategories();
  if (!categories.includes(state.selectedPaletteCategory)) {
    state.selectedPaletteCategory = categories[0] || "Other";
  }
}

function createLocalProject(name) {
  const appSlug = name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/(^-|-$)/g, "") || "local";
  const window = createLocalWindow(name, 1, "MainWindow");
  window.controls.push(createLocalControl("label", fallbackPalette[1], 32, 34, window, "titleLabel", "TRUEOS UI2 app"));
  window.controls.push(createLocalControl("button", fallbackPalette[0], 32, 86, window, "runButton", "Click me"));
  return {
    id: crypto.randomUUID(),
    name,
    slug: appSlug,
    root: "local",
    app_kind: "ui2",
    blueprint: {
      schema: "trueos.app.blueprint/v1",
      app_id: `dev.trueos.${appSlug}`,
      slug: appSlug,
      display_name: name,
      version: "0.1.0",
      entrypoint: "src/main.rs",
      ui_layout: "ui/main.ui2",
      description: `${name} generated with TRUEOS RADS.`,
      license: "MIT OR Apache-2.0",
      authors: ["TRUEOS RADS"],
      capabilities: [
        { key: "ui2.window", enabled: true, note: "Create and manage UI2 windows" },
        { key: "ui2.events", enabled: true, note: "Bind generated UI2 event handlers" },
        { key: "fs.user", enabled: false, note: "Read and write user-selected files" },
        { key: "net.client", enabled: false, note: "Open outbound network connections" },
      ],
      metadata: {
        generator: "trueos-rads",
        generator_version: "local",
        ui_runtime: "TRUEOS/UI2",
        schema_version: "0.1",
      },
    },
    windows: [window],
  };
}

function createLocalWindow(caption, index = 1, name = "") {
  const offset = Math.min((index - 1) * 28, 160);
  return {
    id: crypto.randomUUID(),
    name: name || `Window${index}`,
    caption,
    title_twemoji: null,
    geometry: { x: 80 + offset, y: 80 + offset, w: index === 1 ? 720 : 680, h: index === 1 ? 460 : 420 },
    decorations: normalizedDecorations(),
    options: {
      min_size: { width: 320, height: 240 },
      max_size: null,
      resize_mode: "both",
      scrollbars: "none",
      vertical_scrollbar_side: "left",
      horizontal_scrollbar_side: "bottom",
      hit_test_visible: true,
      preserve_scale: false,
    },
    ui_description: { html: "", css: "" },
    controls: [],
  };
}

function createLocalControl(kind, paletteItem, x, y, window, name = "", caption = "") {
  const base = kind.replaceAll("-", "");
  const finalName = name || uniqueControlName(window, base);
  const eventName = kind === "button" || kind === "check-box" ? "click" : kind === "text-box" ? "change" : "ready";
  return {
    id: crypto.randomUUID(),
    kind,
    name: finalName,
    caption: caption || paletteItem.default_caption || paletteItem.label,
    geometry: { x, y, w: paletteItem.default_w, h: paletteItem.default_h },
    properties: [],
    events: [{ event: eventName, handler: `on_${finalName}_${eventName}` }],
  };
}

function uniqueControlName(window, base) {
  const stem = (base || "control").replace(/[^a-z0-9]/gi, "").toLowerCase() || "control";
  for (let i = 1; i < 1000; i += 1) {
    const candidate = `${stem}${i}`;
    if (!window.controls.some((control) => control.name === candidate)) return candidate;
  }
  return `${stem}${Date.now()}`;
}

function getControl(id) {
  return selectedWindow()?.controls?.find((control) => control.id === id) || null;
}

function controlProperty(control, key) {
  return (control?.properties || []).find((property) => property.key === key)?.value || "";
}

function setControlProperty(control, key, value) {
  if (!control) return;
  control.properties = Array.isArray(control.properties) ? control.properties : [];
  const existing = control.properties.find((property) => property.key === key);
  if (value == null || value === "") {
    control.properties = control.properties.filter((property) => property.key !== key);
  } else if (existing) {
    existing.value = value;
  } else {
    control.properties.push({ key, value });
  }
}

function field(label, path, value, type = "text") {
  return `
    <label class="field">
      <span>${escapeHtml(label)}</span>
      <input data-edit="${path}" type="${type}" value="${escapeHtml(String(value ?? ""))}" />
    </label>`;
}

function selectField(label, path, value, options) {
  return `
    <label class="field">
      <span>${escapeHtml(label)}</span>
      <select data-edit="${path}">
        ${options.map((option) => `<option value="${escapeHtml(option)}" ${option === value ? "selected" : ""}>${escapeHtml(labelKind(option))}</option>`).join("")}
      </select>
    </label>`;
}

function check(label, path, checked) {
  return `
    <label class="check">
      <input data-edit="${path}" type="checkbox" ${checked ? "checked" : ""} />
      <span>${escapeHtml(label)}</span>
    </label>`;
}

function paletteCategories() {
  return [...new Set(state.palette.map((item) => item.category || "Other"))];
}

function setPath(target, path, value) {
  const parts = path.split(".");
  let cursor = target;
  for (const part of parts.slice(0, -1)) {
    cursor = cursor[part];
  }
  cursor[parts.at(-1)] = value;
}

function coerceValue(value, type) {
  if (type !== "number") return value;
  return Number.isFinite(Number(value)) ? Number(value) : 0;
}

function snap(value) {
  if (!state.snapToGrid) return Math.round(value);
  return Math.round(value / state.grid) * state.grid;
}

function clamp(value, min, max) {
  return Math.min(Math.max(Math.round(value), min), max);
}

function labelKind(kind) {
  return kind.split("-").map((part) => part[0].toUpperCase() + part.slice(1)).join(" ");
}

function labelJob(kind) {
  return kind === "auto" ? "Full Auto" : labelKind(kind);
}

function toolInitial(kind) {
  const labels = {
    button: "B",
    label: "L",
    "text-box": "T",
    "check-box": "C",
    panel: "P",
    "list-box": "D",
    canvas: "V",
    toolbar: "N",
    menu: "M",
  };
  return labels[kind] || kind.slice(0, 1).toUpperCase();
}

function shortError(error) {
  return String(error.message || error).slice(0, 140);
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (ch) => (
    { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#039;" }[ch]
  ));
}

init();
