# TRUEOS RADS API Sketch

The RADS API is a local HTTP interface served by the Rust binary at
`http://127.0.0.1:7377`. It is designed for the bundled browser UI, smoke
tests, and small local automation.

## Conventions

- Requests and responses use JSON unless noted.
- UUIDs are serialized as strings.
- Control kinds use kebab case, for example `text-box`, `check-box`, and `list-box`.
- Most failure paths currently return a non-JSON string body.
- Server-sent events are available at `/api/events`.

## Project Endpoints

### `GET /api/project`

Returns the active `RadsProject` or `null`.

### `POST /api/project`

Creates a project under `rads-workspace/<slug>/`, writes generated files, makes
it active, and refreshes runtime watch state.

Request:

```json
{
  "name": "Hello UI2"
}
```

Response:

```json
{
  "project": {
    "id": "00000000-0000-0000-0000-000000000000",
    "name": "Hello UI2",
    "slug": "hello-ui2",
    "root": "/path/to/rads-workspace/hello-ui2",
    "blueprint": {
      "schema": "trueos.app.blueprint/v1",
      "app_id": "dev.trueos.hello-ui2",
      "slug": "hello-ui2",
      "display_name": "Hello UI2",
      "version": "0.1.0",
      "entrypoint": "src/main.rs",
      "ui_layout": "ui/main.ui2"
    },
    "package": {
      "schema": "trueos.package.blueprint/v1",
      "package_id": "dev.trueos.hello-ui2.package"
    },
    "windows": []
  }
}
```

The real response includes blueprint metadata, capabilities, package artifacts,
and one starter window.

### `POST /api/project/load`

Loads an existing `rads.project.json` from inside the RADS workspace and makes
it active.

Request:

```json
{
  "path": "hello-ui2/rads.project.json"
}
```

Response: `{ "project": <RadsProject> }`.

### `POST /api/project/save`

Rewrites all generated files for the active project.

Response: `{ "project": <RadsProject> }`.

Failure case: `no active project`.

### `POST /api/project/control`

Adds a control to an active project window and rewrites generated files.

Request:

```json
{
  "window_id": "00000000-0000-0000-0000-000000000000",
  "kind": "text-box",
  "x": 57,
  "y": 159,
  "caption": "Search",
  "snap": {
    "enabled": true,
    "grid": 8
  }
}
```

Optional request fields:

- `id`: caller-provided UUID for deterministic tests or imports.
- `name`: caller-provided control name.
- `caption`: override the palette label.
- `snap`: snap position and size to a grid.

Response: the complete updated `RadsProject`.

### `PATCH /api/project/control`

Updates a control in an active project window and rewrites generated files.
Callers can either send a whole `control` replacement or selected fields.

Request:

```json
{
  "window_id": "00000000-0000-0000-0000-000000000000",
  "control_id": "00000000-0000-0000-0000-000000000001",
  "caption": "Run",
  "geometry": {
    "x": 64,
    "y": 160,
    "w": 128,
    "h": 34
  },
  "properties": [
    {
      "key": "enabled",
      "value": "true"
    }
  ]
}
```

Response: the complete updated `RadsProject`.

Failure case: `control not found`.

### `DELETE /api/project/control/{control_id}`

Deletes a control from whichever active project window contains it and rewrites
generated files.

Response: the complete updated `RadsProject`.

Failure case: `control not found`.

### `POST /api/project/window` and `PATCH /api/project/window`

Updates main-window data and rewrites generated files.

Request:

```json
{
  "window_id": "00000000-0000-0000-0000-000000000000",
  "caption": "Hello UI2",
  "geometry": {
    "x": 80,
    "y": 80,
    "w": 720,
    "h": 460
  },
  "decorations": {
    "titlebar": true,
    "close": true,
    "minimize": true,
    "maximize": true,
    "resizable": true,
    "always_on_top": false
  }
}
```

Callers can also provide a whole `window` object, or use `properties` entries
such as `caption`, `geometry.x`, `width`, `titlebar`, and `always-on-top`.

Response: the complete updated `RadsProject`.

## Palette Endpoint

### `GET /api/palette`

Returns palette items with kind, label, category, default size, and default
properties.

```json
[
  {
    "kind": "button",
    "label": "Button",
    "category": "standard",
    "default_w": 120,
    "default_h": 34,
    "default_properties": [
      {
        "key": "enabled",
        "value": "true"
      }
    ]
  }
]
```

Current palette kinds:

- `button`
- `label`
- `text-box`
- `check-box`
- `panel`
- `canvas`
- `list-box`
- `menu`
- `toolbar`

Current categories:

- `standard`
- `containers`
- `data`
- `navigation`

## Template Endpoint

### `GET /api/templates`

Returns the built-in project templates available to the New Project flow.

```json
{
  "source": "built-in",
  "templates": [
    {
      "id": "form-app",
      "name": "Form App",
      "description": "A small event-driven form with a label, button, and text input.",
      "tags": ["ui2", "blueprint", "package"],
      "files": [
        "rads.project.json",
        "app.blueprint.json",
        "ui/main.ui2",
        "ui/index.html",
        "ui/styles.css"
      ]
    }
  ]
}
```

## Project File Endpoints

These endpoints support the Code tab and local automation. They operate on the
active project root and reject paths that escape that root.

### `GET /api/project/files`

Lists project-relative files with size, modified time, generated status, and
writable status.

### `GET /api/project/file?path=ui/main.ui2`

Reads one project-relative file.

### `GET /api/project/file/{*path}`

Reads one project-relative file through a path route.

### `POST /api/project/file` and `PUT /api/project/file`

Writes a project-relative file.

Request:

```json
{
  "path": "ui/index.html",
  "contents": "<main></main>\n",
  "create_dirs": true
}
```

Response: the same shape as the read endpoint, including the final contents.

## Runtime Endpoints

### `GET /api/runtime`

Returns runtime automation state.

```json
{
  "full_auto": true,
  "watch": false,
  "active_project": "/path/to/rads-workspace/hello-ui2",
  "watched_project": null
}
```

### `POST /api/runtime`

Updates runtime automation toggles.

Request:

```json
{
  "full_auto": true,
  "watch": true
}
```

Response: the updated runtime state.

## Job Endpoints

### `GET /api/jobs`

Returns recent jobs, newest first.

### `POST /api/jobs`

Starts a job for the active project.

Request:

```json
{
  "kind": "check"
}
```

Valid `kind` values:

- `check`
- `build`
- `pack`
- `auto`

Response:

```json
{
  "job_id": "00000000-0000-0000-0000-000000000000"
}
```

## Events

### `GET /api/events`

Streams job updates as server-sent events. Each event has type `job` and JSON
event data.

```json
{
  "job_id": "00000000-0000-0000-0000-000000000000",
  "status": "running",
  "line": "started",
  "at": "2026-05-08T00:00:01Z"
}
```

## Curl Smoke

```sh
cargo run
curl http://127.0.0.1:7377/api/palette
curl -X POST http://127.0.0.1:7377/api/project \
  -H 'content-type: application/json' \
  -d '{"name":"Hello UI2"}'
curl -X POST http://127.0.0.1:7377/api/runtime \
  -H 'content-type: application/json' \
  -d '{"full_auto":true,"watch":false}'
curl -X POST http://127.0.0.1:7377/api/jobs \
  -H 'content-type: application/json' \
  -d '{"kind":"pack"}'
curl -N http://127.0.0.1:7377/api/events
```
